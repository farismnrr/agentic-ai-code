# 005 — Backend: auth sungguhan, lalu persistensi sungguhan

> **Status: complete.** Shipped to `dev` via PR #26 (fase 0), #27 (fase 1), #28 (fase 2), #29 (fase 3), #30 (fase 4), #31 (docs follow-up).

## Context

Semua yang ada sekarang frontend murni. `app/composables/useAuth.ts` menerima kredensial apa pun dan menyimpan `{ name, email }` ke `localStorage`; `app/middleware/auth.global.ts` bahkan `return` lebih awal di server karena server tidak bisa tahu siapa yang login. Konsekuensinya `routeRules` memaksa `/chat/**` dan `/settings/**` jadi `ssr: false`. Conversation, settings, dan MCP server semuanya `useState` in-memory yang reset tiap reload (`app/composables/useConversations.ts`).

Plan ini memberi produk backend sungguhan: sesi cookie httpOnly, akun tersimpan di Postgres, OAuth, verifikasi email + reset password, lalu memindahkan data chat dari memori ke database. Transport AI tetap mock — ditarik ke server route supaya nanti tinggal ditukar provider asli tanpa menyentuh frontend.

## Keputusan yang sudah diambil

- **`nuxt-auth-utils`** untuk sesi. Modul Nuxt resmi, sealed cookie httpOnly, `useUserSession()` di client, `requireUserSession()` di server, `hashPassword`/`verifyPassword` (scrypt, tanpa dependensi native), dan handler OAuth bawaan. Sesuai [`../knowledge/nuxt-way.md`](../knowledge/nuxt-way.md).
- **Postgres + Drizzle ORM** dengan driver `postgres` (postgres.js, pure-JS — penting karena `pnpm-workspace.yaml` mematikan build native).
- **Infrastruktur yang sudah jalan di laptop**, tidak menambah container:
  - DB: container `sensio-postgres` (`timescale/timescaledb-ha:pg17`, `localhost:5432`), database **`masihawam`**, user `postgres` / `devpassword`. Schema `public` masih kosong.
  - **Schema baru `ai_code`** di dalam DB `masihawam`, dipilih lewat `?search_path=ai_code`. Ini mengikuti pola project tetangga (`sensio-iot`, `sensio-notes`, `tuya_manager` semuanya schema, bukan DB terpisah). Drizzle-kit diarahkan ke schema yang sama.
  - SMTP: reuse milik ATJA — `smtp.farismunir.my.id:587`, STARTTLS, user `farismunir@farismnrr.com`, referensi implementasi di `~/Projects/ATJA/Backend/src/modules/auth/infrastructure/services/email.service.ts` (nodemailer). Kredensial masuk `.env`, **bukan** hardcode seperti di ATJA.
- **Sesi cookie, bukan tabel session.** Trade-off yang diterima: tidak ada "logout dari semua perangkat" / revoke paksa. Kalau nanti perlu, tambah tabel `session` menyusul — bukan bagian plan ini.
- **Chat pindah ke DB, transport tetap mock.** `app/utils/mock-transport.ts` pindah ke `server/utils/`; frontend bicara ke `/api/chat` lewat AI SDK.

## Yang perlu disiapkan pemilik repo

- **Password SMTP** ATJA (di kodenya default `devpassword`) — isi di `.env`.
- **OAuth credentials** sebelum fase 2: Google Cloud Console (OAuth client, redirect `http://localhost:3333/api/auth/google`) dan GitHub Developer Settings (redirect `http://localhost:3333/api/auth/github`).

## Skema database (schema `ai_code`)

| Tabel | Isi |
| --- | --- |
| `users` | `id` (uuid pk), `email` (citext unique), `name`, `password_hash` (nullable — akun OAuth-only), `avatar_url`, `email_verified_at`, `created_at`, `updated_at` |
| `oauth_accounts` | `provider` + `provider_account_id` (unique bersama), `user_id` fk cascade |
| `verification_tokens` | `token_hash` (pk), `user_id`, `type` (`email_verify` \| `password_reset`), `expires_at`, `consumed_at`. Token mentah dikirim lewat email, **hanya hash yang disimpan** |
| `conversations` | `id`, `user_id` fk, `title`, `model_id`, `enabled_tool_ids` (jsonb), `approvals` (jsonb), `created_at`, `updated_at` |
| `messages` | `id`, `conversation_id` fk cascade, `role`, `parts` (jsonb — bentuk `UIMessage` di `app/types/chat.ts`), `created_at`, indeks `(conversation_id, created_at)` |
| `user_settings` | `user_id` pk fk, kolom-kolom dari `app/composables/useSettings.ts` |
| `mcp_servers` | per-user, dari `app/composables/useMcpServers.ts` |

Semua query difilter `user_id` — tidak ada endpoint yang menerima id tanpa cek kepemilikan.

## Git: branch, worktree, commit

Semua mengikuti [`../knowledge/git.md`](../knowledge/git.md). Ditulis eksplisit di sini karena plan ini panjang dan menyentuh dependensi, migrasi, dan `.env`.

### Worktree

Tiap fase dikerjakan di worktree terpisah, bukan di worktree utama:

```sh
git worktree add ../ai-code-005-p1 -b feat/005-p1-auth-foundation dev
```

Alasannya: worktree utama tetap berada di `dev` dan tetap bisa dijalankan, jadi perilaku lama dan baru bisa dibandingkan berdampingan. Tiga konsekuensi yang harus ditangani, bukan diabaikan:

- **`node_modules` tidak ikut.** Jalankan `pnpm install` di worktree baru. Fase 1 memang menambah dependensi, jadi ini bukan biaya tambahan.
- **`.env` gitignored, jadi tidak ikut tersalin.** Salin manual (`cp ../ai-code/.env .`) lalu tambahkan variabel baru fase itu. Jangan pernah `git add` file ini.
- **Port 3333 cuma satu.** Kalau dua dev server jalan bersamaan, jalankan yang di worktree dengan `NUXT_PORT=3334 pnpm dev` — jangan ubah default di `nuxt.config.ts`.

Setelah PR di-squash-merge: `git worktree remove ../ai-code-005-p1 && git worktree prune`, lalu `git switch dev && git pull --ff-only` di worktree utama. Bagian "Clean up after every merge" di `git.md` dijalankan penuh, tanpa diminta.

### Branch per fase

| Fase | Branch |
| --- | --- |
| 0 | `docs/005-plan` |
| 1 | `feat/005-p1-auth-foundation` |
| 2 | `feat/005-p2-oauth` |
| 3 | `feat/005-p3-email-verification` |
| 4 | `feat/005-p4-chat-persistence` |

Selalu bercabang dari `dev` terbaru. Satu PR per fase (`--base dev`), squash merge begitu CI hijau — itu izin berdiri, tidak perlu tanya. `dev` → `main` tidak pernah dibuka tanpa permintaan eksplisit.

### Commit

Conventional Commits, atomik, subject imperatif huruf kecil ≤72 char. Plan ini memperkenalkan dua scope baru, **`auth`** dan **`db`**, menyusul `chat`, `mcp`, `settings`, `ui`, `agents`, `deps`, `config` — ditambahkan ke `git.md` di fase 0 supaya daftar scope tidak diam-diam melebar.

Pemecahan commit yang diharapkan di fase 1, bukan satu commit besar:

```
build(deps): add drizzle, postgres driver and nuxt-auth-utils
feat(db): add ai_code schema with users and oauth tables
feat(auth): add register, login and logout endpoints
refactor(auth): back useAuth with server session instead of localStorage
feat(config): drop ssr:false now that the guard runs server-side
```

Body menjelaskan **kenapa**. Commit yang mencabut `ssr: false` wajib punya body — komentar di `nuxt.config.ts` saat ini beralasan panjang kenapa flag itu ada, dan alasan pencabutannya harus terekam.

Sebelum staging selalu `git status` dulu. Jangan `git add -A` setelah build atau `pnpm install`; `.env` dan artefak build tidak boleh ikut.

## Fase

Tiap fase berakhir hijau (`pnpm lint && pnpm typecheck && pnpm audit`) dan punya PR sendiri ke `dev`.

### Fase 0 — Plan ini masuk repo ✅ done

Sebelum kode apa pun:

- File ini ada di `.agents/plans/005-backend-auth.md`.
- Terdaftar di [`README.md`](README.md) di bawah **In Flight**; dipindahkan ke Completed saat fase 4 mendarat.
- Scope `auth` dan `db` ditambahkan ke daftar scope di `../knowledge/git.md`.
- Branch `docs/005-plan`, satu commit `docs(agents): add plan 005 for the auth and persistence backend`, PR ke `dev`.

*(Fase 1–4 masing-masing di worktree + branch sesuai tabel di atas.)*

### Fase 1 — Fondasi: DB + email/password auth ✅ done

1. `pnpm dlx nuxi module add auth-utils`; `pnpm add drizzle-orm postgres`; `pnpm add -D drizzle-kit`. (`valibot` sudah terpasang.)
2. `server/database/schema.ts` (`pgSchema('ai_code')`), `drizzle.config.ts`, script `db:generate` / `db:migrate` di `package.json`. Migrasi awal: `users`, `oauth_accounts`, `verification_tokens`.
3. `server/utils/db.ts` — singleton koneksi postgres.js yang dibaca dari `useRuntimeConfig().databaseUrl`, bukan `process.env`.
4. `runtimeConfig` di `nuxt.config.ts`: `databaseUrl`, `session.password` (`NUXT_SESSION_PASSWORD`, ≥32 char). `.env.example` diperbarui dengan komentar sesuai gayanya sekarang.
5. Endpoint: `server/api/auth/register.post.ts`, `login.post.ts`, `logout.post.ts`, `server/api/me.get.ts`. Validasi body pakai **valibot** (sudah dipakai di `app/pages/login.vue`) — skema ditaruh di `shared/` supaya client dan server memakai aturan yang sama.
6. Ganti isi `app/composables/useAuth.ts` jadi pembungkus tipis di atas `useUserSession()` supaya call-site (`app/pages/login.vue`, `register.vue`, `app/layouts/default.vue`) tidak perlu ditulis ulang. Hapus jalur `localStorage` dan plugin `app/plugins/auth.client.ts`.
7. `app/middleware/auth.global.ts`: buang guard `import.meta.server` — cookie terbaca di server, jadi guard jalan di kedua sisi. **Hapus `ssr: false`** untuk `/chat/**` dan `/settings/**` di `nuxt.config.ts`; komentar di sana yang menjelaskan alasan lama ikut diperbarui.
8. Pengerasan: rate limit login/register (in-memory per IP+email, cukup untuk single-node), pesan error seragam supaya tidak membocorkan email mana yang terdaftar, cookie `sameSite: 'lax'` + `secure` di produksi.

### Fase 2 — OAuth Google & GitHub ✅ done

- `server/routes/auth/google.get.ts` dan `github.get.ts` pakai `defineOAuthGoogleEventHandler` / `defineOAuthGitHubEventHandler`.
- Logika penautan akun: cocokkan `oauth_accounts`, kalau tidak ada cocokkan email terverifikasi lalu tautkan, kalau tidak ada juga buat user baru dengan `email_verified_at` terisi dari provider. Email yang **belum** terverifikasi dari provider tidak boleh otomatis menautkan akun password — itu jalur pengambilalihan akun.
- Tombol social di `app/pages/login.vue` dan `register.vue` (di plan 002 sengaja ditolak; sekarang jadi nyata), pakai ikon `simple-icons`.

### Fase 3 — Verifikasi email & reset password ✅ done

- `server/utils/mailer.ts` — nodemailer + SMTP dari runtime config, template HTML sederhana bergaya produk ini (bukan salinan ATJA).
- Alur: register mengirim link verifikasi; `/api/auth/verify` menandai `email_verified_at`; `/api/auth/forgot` + `/api/auth/reset` memakai token sekali pakai berumur pendek. Respons `forgot` selalu sukses apa pun emailnya (anti user-enumeration).
- Halaman `app/pages/verify-email.vue` dan `reset-password.vue` di layout `auth`, ditambah banner "verifikasi email lu" di layout `default` selama `email_verified_at` kosong.
- Keputusan: akun **belum terverifikasi tetap boleh login**, hanya diberi banner. Memblokir login sebelum verifikasi bikin demo macet kalau SMTP lagi rewel.

### Fase 4 — Data chat pindah ke Postgres ✅ done

- Migrasi tabel `conversations`, `messages`, `user_settings`, `mcp_servers`.
- CRUD di `server/api/conversations/**`, `server/api/settings.*`, `server/api/mcp-servers/**`; semuanya di balik `requireUserSession`.
- `app/utils/mock-transport.ts` → `server/utils/mock-transport.ts`, diekspos sebagai `server/api/chat.post.ts` yang melakukan streaming lalu menyimpan pesan asisten setelah selesai. `app/composables/useConversationChat.ts` beralih ke transport HTTP default AI SDK.
- `useConversations`/`useSettings`/`useMcpServers` diubah dari `useState` seed ke `useFetch`/`$fetch` dengan pembaruan optimistis; helper murni seperti `titleFrom` dan `app/utils/group-conversations.ts` tetap dipakai apa adanya.
- Data seed di `app/utils/fixtures/` jadi seed sisi server untuk akun yang baru dibuat, supaya user baru tetap punya isi. Tombol "Reset demo data" di settings diarahkan ke endpoint reseed.

## Verifikasi

Per fase: `pnpm lint && pnpm typecheck && pnpm audit`, plus `pnpm build` sebelum PR terakhir. Sesuai [`../memories/verify-in-a-browser.md`](../memories/verify-in-a-browser.md), tiap route dibuka di browser dan log dev-server digrep untuk `Hydration`, `Failed to resolve component`, `[console.error]`.

Alur end-to-end di http://localhost:3333:

1. Register email baru → cek baris di `ai_code.users` (`docker exec sensio-postgres psql -U postgres -d masihawam -c 'select email, email_verified_at from ai_code.users'`), password tersimpan sebagai hash, bukan plaintext.
2. Hard refresh di `/chat` → tetap login, dan HTML hasil SSR sudah berisi konten yang dijaga (bukti `ssr: false` tidak lagi diperlukan).
3. Login dengan password salah → ditolak; pesannya sama persis dengan pesan email tidak terdaftar.
4. `curl` ke `/api/conversations` tanpa cookie → 401.
5. Login sebagai user A, catat id percakapan, lalu `curl` id itu sebagai user B → 401/404, bukan datanya.
6. Login via Google dan GitHub dengan email yang sama dengan akun password → tertaut ke user yang sama, bukan bikin duplikat.
7. Email verifikasi masuk ke inbox asli; klik link → banner hilang. Link yang sudah dipakai atau kedaluwarsa ditolak.
8. Reset password: minta link, ganti password, login dengan yang baru, yang lama gagal.
9. Kirim pesan → balasan streaming; **reload halaman → percakapan dan pesannya masih ada** (ini yang belum pernah bisa sebelumnya).
10. Sign out → cookie hilang, `/chat` tidak bisa diakses lagi.

## Di luar cakupan

Provider LLM asli, MCP tool-calling sungguhan, tabel session/revoke, 2FA, organisasi/tim, deploy ke luar laptop, Playwright.

## Setelah selesai

Centang tiap fase di file ini saat mendarat, lalu pindahkan entrinya dari In Flight ke Completed di [`README.md`](README.md). Catat di [`../memories/`](../memories/): koordinat Postgres + konvensi schema `ai_code`, host SMTP yang dipakai bersama ATJA, dan alasan `ssr: false` dicabut — supaya agen berikutnya tidak mengembalikannya.
