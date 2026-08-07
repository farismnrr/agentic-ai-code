The `nuxt-auth-utils` module reads its `User`/`UserSession` shape from module augmentation of `#auth-utils`, but Nuxt only picks that augmentation up from files inside a real source root — `app/types/` and the repo root are *not* scanned reliably, so `session.user.id` etc. type as `unknown`/missing even though the runtime value is correct.

**Fix:** put the augmentation at `shared/types/auth.d.ts`:

```ts
declare module '#auth-utils' {
  interface User {
    id: string
    email?: string
    name?: string
    avatarUrl?: string | null
    emailVerifiedAt?: string | null
  }
}

export {}
```

`shared/` is auto-scanned by both the Nitro and app TypeScript projects, so server routes (`server/api/**`) and composables see the same `User` type without a manual import. Symptom if this regresses: `pnpm typecheck` reports `Property 'id' does not exist on type 'User'` across every `server/api/**` file that calls `requireUserSession`, and someone "fixes" it by adding `(session.user as any).id` casts instead of moving the file — don't do that, it defeats the point of augmenting the type.

Same reasoning applies to any type shared between `app/` and `server/` (e.g. `Conversation`, `UIMessage` for [plan 005](../plans/005-backend-auth.md)) — it belongs in `shared/types/`, not `app/types/`, the moment a server route needs it too.
