<script lang="ts">
  import type { AuthenticatedUser } from '../model/auth-state'
  import GitHubIcon from '../../../shared/components/icons/GitHubIcon.svelte'
  import AuthPanel from './AuthPanel.svelte'
  import LogoutButton from './LogoutButton.svelte'

  export let user: AuthenticatedUser
  export let onSignOut: () => void
</script>

<AuthPanel title="You're signed in" message="Your GitHub authentication session is active.">
  <div class="flex items-center gap-4">
    {#if user.avatarUrl}
      <div class="avatar">
        <div class="w-16 rounded-full ring-1 ring-slate-200 ring-offset-2 ring-offset-white">
          <img src={user.avatarUrl} alt="" />
        </div>
      </div>
    {/if}

    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <p class="truncate text-lg font-semibold text-slate-950">@{user.login}</p>
        <GitHubIcon />
      </div>
      <div class="mt-1 inline-flex items-center gap-2 rounded-full bg-emerald-50 px-2.5 py-1 text-xs font-medium text-emerald-700">
        <span class="size-2 rounded-full bg-emerald-500"></span>
        Active session
      </div>
    </div>
  </div>

  <div class="my-6 h-px bg-slate-100"></div>

  <dl class="space-y-4 text-sm">
    <div class="flex items-center justify-between gap-4">
      <dt class="text-slate-500">Signed in with</dt>
      <dd class="font-medium text-slate-800">GitHub</dd>
    </div>
    <div class="flex items-center justify-between gap-4">
      <dt class="text-slate-500">Session expires</dt>
      <dd class="font-medium text-slate-800">In 7 days</dd>
    </div>
    <div class="flex items-center justify-between gap-4">
      <dt class="text-slate-500">Last refreshed</dt>
      <dd class="font-medium text-slate-800">Just now</dd>
    </div>
  </dl>

  <div class="mt-7">
    <LogoutButton {onSignOut} />
  </div>
</AuthPanel>
