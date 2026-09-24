<script lang="ts">
  import type { AuthenticatedUser } from '../model/auth-state'
  import AuthPanel from './AuthPanel.svelte'
  import LogoutButton from './LogoutButton.svelte'

  export let user: AuthenticatedUser
</script>

<AuthPanel
  badge="Authenticated"
  badgeClass="badge-success badge-outline"
  title={`@${user.login}`}
  message="Your GitHub session is active."
>
  <div class="flex flex-col gap-5">
    {#if user.avatarUrl}
      <div class="avatar self-start">
        <div class="w-20 rounded-full ring ring-base-300 ring-offset-2 ring-offset-base-100">
          <img src={user.avatarUrl} alt="" />
        </div>
      </div>
    {/if}

    <div class="rounded-box bg-base-200 p-4 text-base-content">
      <div class="text-xs font-medium uppercase tracking-wide text-base-content/65">
        Signed in with
      </div>
      <div class="mt-1 font-semibold">GitHub</div>
    </div>

    <LogoutButton />
  </div>
</AuthPanel>
