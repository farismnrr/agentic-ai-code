# 006 — Error handling: stop leaking internals, make status codes match RFC 9110/9457

> **Status: belum mulai.**

## Context

Kena 500 yang nge-expose detail internal ke client. Investigasi ke `server/` (lihat [`005-backend-auth.md`](005-backend-auth.md) untuk konteks endpoint yang ada) dan ke source `h3@1.15.11` + `nitropack@2.13.4` yang benar-benar terpasang (bukan tebak dari dokumentasi web yang campur versi) menemukan akar masalahnya persis di `nitropack/dist/runtime/utils.mjs` fungsi `normalizeError`:

```js
const message = !isDev && error.unhandled ? "internal server error" : error.message || error.toString();
```

Artinya: Nitro **hanya** menyamarkan pesan error kalau (a) `NODE_ENV=production` **dan** (b) error itu ditandai `unhandled` (exception mentah yang tidak lewat `createError`). Di local dev — persis kondisi yang kejadian — `message` **selalu** dikirim apa adanya, termasuk pesan driver Postgres, `ValiError` valibot, atau `Error` polos. Dan bahkan di production, error yang dilempar lewat `createError({ statusCode: 500, message: rawThing })` **tidak** disamarkan sama sekali karena dia bukan `unhandled` — jadi siapa pun yang nulis `createError({ message: err.message })` di masa depan tetap bocor walau sudah di prod.

Sumber kebocoran konkret yang ditemukan (bukan hipotetis):
- `server/api/auth/register.post.ts` — cek email duplikat dulu (`existing.length > 0`), baru insert. Race: dua request bersamaan lolos cek, request kedua kena `PostgresError` unique-violation (`users.email` unique di `server/database/schema.ts`) yang **tidak ditangkap** → lempar mentah ke client. `PostgresError` (lihat `postgres@3.4.9/src/errors.js`) meng-assign semua field wire-protocol Postgres ke object, termasuk `detail` yang **bisa berisi nilai kolom yang bentrok** (`Key (email)=(user@x.com) already exists.`) — bukan cuma bocor struktur DB, tapi berpotensi bocor data.
- Pola race yang sama di `server/routes/auth/github.get.ts`, `server/routes/auth/google.get.ts` (linking OAuth account), dan `server/api/mcp-servers/index.post.ts` (id hasil slug bisa collide).
- **10 route** yang manggil `v.parse(schema, data)` langsung (bukan `v.safeParse`) — kalau body tidak valid, `ValiError` mentah terlempar tanpa ada yang menangkap, jadi hari ini semua "bad input" di route-route itu jatuh ke exception tak tertangani (statusCode default 500 dari h3, bukan 400/422 yang seharusnya).
- `server/utils/db.ts` — `useDb()` lempar `new Error('NUXT_DATABASE_URL is not set...')` polos kalau env belum di-set, tidak pernah ditangkap di pemanggilnya.
- `server/api/chat.post.ts` — `ReadableStream.start()` tidak punya try/catch. Kalau gagal di tengah jalan (mis. `db.insert` pesan asisten gagal setelah stream mulai), tidak ada cara ubah status HTTP lagi (header sudah terkirim) dan client cuma dapat stream yang mati tanpa sinyal.
- `server/utils/mailer.ts` — kegagalan SMTP ditelan (`return false`), tapi kedua pemanggilnya (`register.post.ts`, `forgot.post.ts`) tidak pernah mengecek return value-nya — kebalikan dari masalah di atas (bukan bocor, tapi disembunyikan total dari monitoring).
- Status code sekarang tidak konsisten dengan skenarionya: 400 dipakai untuk "token invalid" *dan* "token kedaluwarsa" (harusnya beda kasus), tidak ada satu pun endpoint yang pakai 409 padahal race condition duplicate-nya nyata, `data` yang dikirim ke `createError()` ternyata **tidak pernah keluar ke client** karena `normalizeError` cuma pakai `message`/`stack`/`statusCode`/`statusMessage` — jadi kalaupun ada kode yang coba taruh detail aman di `data`, itu percuma dengan handler default Nitro yang sekarang.

## Keputusan desain

- **RFC 9457 Problem Details for HTTP APIs** (`application/problem+json`) untuk semua body error: `{ type, title, status, detail, instance, ...extension }`.
- **Model default-deny, bukan default-trust.** Sekarang arsitekturnya "kirim `message` kecuali ditandai unhandled+prod". Dibalik: **tidak ada apa pun yang keluar ke client kecuali eksplisit ditandai aman oleh kode aplikasi.** Central error handler Nitro (`nitro.errorHandler`) jadi garis pertahanan terakhir — default-nya selalu balas 500 generik `"Internal Server Error"` tanpa detail apa pun, KECUALI error itu punya `data.problem` yang di-set lewat helper factory sendiri.
- **Satu factory module** (`server/utils/http-errors.ts`) buat semua titik lempar error, jadi tidak ada lagi `createError({...})` ad hoc yang bisa lupa disamarkan.
- **Validasi (valibot):** ganti semua `v.parse` → `v.safeParse`, gagal → **422 Unprocessable Content**. Standar industri (Stripe, GitHub, banyak API modern) buat "syntactically valid tapi semantically invalid", beda dari 400 yang dipakai buat request benar-benar malformed (param routing hilang, dsb). Detail per-field yang dikirim ke client dibatasi ke `{ path, message }` saja — bukan objek `ValiError` mentah (yang bisa berisi info skema internal).
- **Not-owned = 404, dipertahankan.** Pola yang sudah benar di `login.post.ts` (anti user-enumeration) dan konsisten dipakai GitHub API dkk buat private resource — 403 justru bocorin "resource ini ada". Didokumentasikan eksplisit di tiap endpoint kenapa begitu.
- **Race condition unique-violation → 409 Conflict.** Deteksi `error.code === '23505'` (SQLSTATE unique_violation, dari `postgres` driver — lihat `PostgresError` di `postgres@3.4.9/src/errors.js`) di titik-titik yang sudah diidentifikasi rawan race, ubah jadi `conflict()` generik. Pre-check yang sudah ada (mis. cek email dulu) **tetap dipertahankan** untuk UX cepat di kasus umum; catch di sekitar insert jadi jaring pengaman buat race yang sebenarnya.
- **Token verifikasi/reset:** pisahkan makna — token tidak dikenal → 400 (request salah, bukan resource), token dikenal tapi kedaluwarsa/sudah dipakai → **410 Gone** (RFC 9110: resource itu memang pernah ada, sekarang tidak lagi tersedia). Sinyal UX lebih jelas ("link kadaluwarsa, minta baru" vs "link salah").
- **429** dapat header `Retry-After` yang benar (RFC 9110 §10.2.3) selain body, bukan cuma angka ditulis di dalam teks pesan.
- **`chat.post.ts` streaming:** bungkus `start()` dengan try/catch. Wire format yang dipakai file ini adalah protokol lama Vercel AI SDK (prefix `0:` untuk text — lihat baris 46/50), error di tengah stream dikirim dengan prefix `3:` (error part di protokol yang sama, sudah didukung `useChat` versi lama) berisi pesan generik aman, lalu stream ditutup — bukan bikin protokol baru.
- **SMTP:** `sendEmail()` di `mailer.ts` tetap non-blocking buat `register`/`forgot` (jangan bongkar pola anti-enumeration yang sudah ada — respons harus tetap sukses generik apa pun hasil pengiriman), tapi return value-nya sekarang **dicek dan di-log** (bukan didiamkan) supaya kegagalan SMTP kelihatan di server log/observability, bukan hilang total.

## Peta status code final per skenario

| Skenario | Status | Catatan |
|---|---|---|
| Body/JSON request rusak, param routing wajib hilang | 400 Bad Request | `badRequest()` |
| Body valid JSON tapi gagal validasi skema (valibot) | 422 Unprocessable Content | `unprocessable(issues)` |
| Tidak ada sesi / cookie tidak valid | 401 Unauthorized | sudah ditangani `requireUserSession`, dibungkus biar konsisten formatnya |
| Rate limit | 429 Too Many Requests | + header `Retry-After` |
| Kredensial salah (login) | 401 Unauthorized | sudah benar, pesan tetap generik (tidak beda antara "email tak ada" vs "password salah") |
| Resource tak ada / bukan milik user | 404 Not Found | dipertahankan, didokumentasikan alasannya |
| Token verifikasi/reset tidak dikenal | 400 Bad Request | `badRequest()` |
| Token verifikasi/reset kedaluwarsa/sudah dipakai | 410 Gone | `gone()` — baru |
| Duplicate email / unique-violation lain (race) | 409 Conflict | `conflict()` — baru, ditangkap dari `PostgresError.code === '23505'` |
| Email OAuth provider tidak ada | 400 Bad Request | sudah benar |
| Email OAuth belum verified saat linking | 403 Forbidden | sudah benar (ini memang kasus "paham tapi menolak", bukan not-found) |
| Exception tak terduga (DB down, insert gagal aneh, dll) | 500 Internal Server Error | `internal()` — pesan **selalu** generik ke client, detail asli cuma di server log |

## Git: branch, worktree, commit

Ikut [`../knowledge/git.md`](../knowledge/git.md), sama seperti plan 005: worktree terpisah per fase, satu PR per fase ke `dev`, squash merge begitu CI hijau.

| Fase | Branch |
| --- | --- |
| 0 | `docs/006-plan` |
| 1 | `feat/006-p1-central-error-handler` |
| 2 | `feat/006-p2-status-code-audit` |

Scope commit baru yang relevan: `auth`, `db`, `chat` (sudah ada), tidak perlu scope baru — semua perubahan di sini masuk salah satu dari itu tergantung file yang disentuh, atau tanpa scope kalau lintas `server/` (mis. `server/error.ts`, `nuxt.config.ts`).

## Fase

Tiap fase berakhir hijau (`pnpm lint && pnpm typecheck && pnpm audit`) dan punya PR sendiri ke `dev`.

### Fase 0 — Plan ini masuk repo

- File ini ada di `.agents/plans/006-error-handling.md`, terdaftar di [`README.md`](README.md) di bawah **In Flight**.
- Branch `docs/006-plan`, commit `docs(agents): add plan 006 for RFC-consistent error handling`, PR ke `dev`.

### Fase 1 — Central error handler + factory (fondasi)

1. `server/utils/http-errors.ts` — factory RFC 9457:

```ts
interface ProblemInit {
  status: number
  title: string
  detail?: string
  type?: string        // default 'about:blank' per RFC 9457 §4.2
  extra?: Record<string, unknown>  // extension members, mis. { errors: [...] } atau retryAfter
}

function problem(init: ProblemInit) {
  return createError({
    statusCode: init.status,
    statusMessage: init.title,
    data: {
      problem: true,
      type: init.type ?? 'about:blank',
      title: init.title,
      status: init.status,
      detail: init.detail,
      ...init.extra
    }
  })
}

export const badRequest = (detail?: string) => problem({ status: 400, title: 'Bad Request', detail })
export const unauthorized = (detail?: string) => problem({ status: 401, title: 'Unauthorized', detail })
export const forbidden = (detail?: string) => problem({ status: 403, title: 'Forbidden', detail })
export const notFound = (detail?: string) => problem({ status: 404, title: 'Not Found', detail })
export const conflict = (detail?: string) => problem({ status: 409, title: 'Conflict', detail })
export const gone = (detail?: string) => problem({ status: 410, title: 'Gone', detail })
export function unprocessable(issues: v.BaseIssue<unknown>[]) { /* map ke [{ path, message }], status 422 */ }
export function tooManyRequests(retryAfterSeconds: number) { /* status 429, extra: { retryAfter: retryAfterSeconds } */ }
export const internal = (cause?: unknown) => { console.error('[internal]', cause); return problem({ status: 500, title: 'Internal Server Error' }) }
```

`instance` (RFC 9457, opsional) diisi otomatis di central handler dari `event.path`, tidak perlu di-set manual tiap panggilan.

2. `server/error.ts` + `nuxt.config.ts` `nitro.errorHandler` — central handler, default-deny:

```ts
// server/error.ts
export default defineNitroErrorHandler((error, event) => {
  const isProblem = (error.data as any)?.problem === true
  const status = isProblem ? error.statusCode : 500
  const body = isProblem
    ? { type: error.data.type, title: error.data.title, status, detail: error.data.detail, instance: event.path, ...extensionFields(error.data) }
    : { type: 'about:blank', title: 'Internal Server Error', status: 500, instance: event.path }

  // Full detail (message asli, stack, error object) HANYA ke server log — tidak pernah ke client.
  if (!isProblem) console.error('[unhandled]', error)

  setResponseHeader(event, 'Content-Type', 'application/problem+json')
  if (isProblem && error.data.retryAfter) setResponseHeader(event, 'Retry-After', String(error.data.retryAfter))
  setResponseStatus(event, status)
  return send(event, JSON.stringify(body))
})
```

Didaftarkan lewat `nitro: { errorHandler: '~/server/error' }` di `nuxt.config.ts`. Titik pertahanan terakhir: **apa pun** yang lolos tanpa lewat factory di atas otomatis jadi 500 generik, tidak pernah bocor `message`/`stack` asli — beda dari perilaku Nitro default yang bocor `message` di dev tanpa syarat.

3. `server/utils/is-unique-violation.ts`:

```ts
export function isUniqueViolation(err: unknown): boolean {
  return typeof err === 'object' && err !== null && 'code' in err && (err as { code: unknown }).code === '23505'
}
```

### Fase 2 — Audit semua titik error di server/, pasang status code yang benar

1. Ganti semua situs `createError({...})` ad hoc ke factory dari `server/utils/http-errors.ts`:
   - `server/api/auth/{login,register,forgot,reset,verify}.post.ts`
   - `server/api/conversations/*.ts`, `server/api/mcp-servers/*.ts`
   - `server/api/settings.{get,put}.ts`
   - `server/routes/auth/{github,google}.get.ts`
   - `server/api/chat.post.ts` (bagian sebelum stream mulai)

2. Validasi — `v.parse` → `v.safeParse` di 10 titik:

```ts
const result = v.safeParse(schema, await readBody(event))
if (!result.success) throw unprocessable(result.issues)
const data = result.output
```

Sekalian rapikan `forgot.post.ts`/`reset.post.ts`/`verify.post.ts` yang saat ini mendefinisikan skema inline duplikat — pindah pakai schema yang sudah ada di `shared/schemas/auth.ts`.

3. Tangkap race condition unique-violation → 409 di titik yang perlu try/catch di sekitar insert:
   - `server/api/auth/register.post.ts` (insert `users`)
   - `server/routes/auth/github.get.ts`, `google.get.ts` (insert `oauthAccounts` / `users` saat linking)
   - `server/api/mcp-servers/index.post.ts` (insert `mcpServers`)

   Pola: `try { await db.insert(...) } catch (err) { if (isUniqueViolation(err)) throw conflict('...'); throw err }` — `throw err` di akhir supaya exception lain tetap jatuh ke central handler sebagai 500 generik, bukan disembunyikan.

4. `reset.post.ts` / `verify.post.ts` — pisahkan invalid vs expired: ganti pasangan `400`/`400` (baris `reset.post.ts:36,40` dan `verify.post.ts:35,39`) jadi `badRequest()` (token tidak dikenal) dan `gone()` (token ada tapi expired/consumed).

5. `chat.post.ts` — try/catch di `start()`:

```ts
async start(controller) {
  try {
    // ...loop chunks + db.insert seperti sekarang...
    controller.close()
  } catch (err) {
    console.error('[chat stream]', err)
    controller.enqueue(`3:${JSON.stringify('Something went wrong while generating a response.')}\n`)
    controller.close()
  }
}
```

6. `mailer.ts` callers — log kegagalan SMTP: `register.post.ts` dan `forgot.post.ts`, setelah `await sendEmail(...)`, kalau `false` → `console.warn('[email] delivery failed', { to, purpose })`. Response ke client **tidak berubah** (tetap sukses generik, anti-enumeration tidak boleh rusak).

7. `server/utils/db.ts` — tidak disentuh. `useDb()` yang throw `new Error(...)` polos kalau `NUXT_DATABASE_URL` belum di-set adalah startup misconfiguration, bukan skenario yang diakses user biasa, dan sekarang central handler default-deny sudah otomatis menyamarkannya ke client tanpa perlu ubah file ini.

## File yang disentuh

- Baru: `server/utils/http-errors.ts`, `server/utils/is-unique-violation.ts`, `server/error.ts`
- Ubah: `nuxt.config.ts` (tambah `nitro.errorHandler`), semua file di Fase 2 §1, `server/api/chat.post.ts`, `server/api/auth/{register,forgot}.post.ts` (logging mailer), `shared/schemas/auth.ts` (kalau perlu skema tambahan buat `reset`/`verify`)

## Verifikasi

`pnpm lint && pnpm typecheck && pnpm audit` per fase, plus `pnpm build` sebelum PR terakhir.

Manual, per skenario di tabel status code:
1. Body kosong/salah tipe ke `POST /api/conversations` → 422, body `application/problem+json` berisi `errors: [{ path, message }]`, **tidak** ada field valibot internal.
2. Dua `curl -X POST /api/auth/register` konkuren dengan email sama (mis. lewat `xargs -P2`) → satu 201, satu 409, **tidak ada** teks constraint/nama kolom/nilai email di response.
3. Reset password dengan token acak → 400. Reset dengan token yang sudah dipakai → 410. Body beda pesan, keduanya tanpa detail token asli.
4. `GET /api/conversations/<id-milik-user-lain>` → 404, sama persis dengan `GET /api/conversations/<id-random>`.
5. Matikan Postgres sesaat (`docker stop sensio-postgres`), panggil endpoint apa pun yang query DB → 500, body `{ type: 'about:blank', title: 'Internal Server Error', status: 500, instance }` — **tidak** ada connection string / host / pesan driver. Cek server log (dev console) tetap punya detail lengkapnya. Nyalakan lagi postgres setelahnya.
6. Login gagal 6x dalam window rate limit → 429 dengan header `Retry-After` (`curl -i` cek header-nya beneran ada, bukan cuma di body).
7. Kirim chat lalu paksa gagal (mis. matikan DB di tengah stream) → response stream tetap kebaca sampai selesai dengan pesan error generik lewat prefix `3:`, tidak hang/putus mendadak tanpa sinyal.
8. Semua respons error di atas: header `Content-Type: application/problem+json`, dan `curl -s | jq .stack` selalu `null`/absent di **setiap** environment (bukan cuma prod) — beda dari perilaku Nitro default sebelumnya.

## Di luar cakupan

Structured logging/correlation-id ke sistem observability eksternal (cukup `console.error` server-side untuk sekarang), rewrite protokol streaming `chat.post.ts` ke `UIMessageChunk` JSON asli (masalah terpisah dari error handling), retry/backoff otomatis di client untuk 429.

## Setelah selesai

Centang tiap fase di file ini saat mendarat, lalu pindahkan entrinya dari In Flight ke Completed di [`README.md`](README.md). Catat di [`../memories/`](../memories/): fakta bahwa Nitro default HANYA menyamarkan `message` untuk error `unhandled` di production (bukan untuk `createError` eksplisit, dan tidak sama sekali di dev) — jebakan yang gampang keulang kalau ada yang nambah endpoint baru tanpa lewat `http-errors.ts`.
