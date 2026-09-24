<script lang="ts">
  import AuthPanel from './AuthPanel.svelte'
</script>

<AuthPanel
  badge="GitHub"
  badgeClass="border-info/60 text-info"
  title="Signing you in"
  message="We're finishing your GitHub authentication and preparing your session."
>
  <div class="flex items-center gap-3 rounded-box bg-base-200 p-4 text-base-content">
    <span class="loading loading-spinner loading-md shrink-0"></span>
    <span class="text-sm text-base-content/80">Please wait a moment…</span>
  </div>
</AuthPanel>
