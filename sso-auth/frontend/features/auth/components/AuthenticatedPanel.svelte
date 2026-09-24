<script lang="ts">
  import type { AuthenticatedUser } from '../model/auth-state'

  export let user: AuthenticatedUser
</script>

<section class="card w-full border border-base-300 bg-base-100 shadow-xl">
  <div class="card-body items-center gap-5 p-8 text-center sm:p-10">
    {#if user.avatarUrl}
      <div class="avatar">
        <div class="w-20 rounded-full ring ring-base-300 ring-offset-2 ring-offset-base-100">
          <img src={user.avatarUrl} alt="" />
        </div>
      </div>
    {/if}

    <div class="space-y-2">
      <div class="badge badge-success badge-outline">Authenticated</div>
      <h1 class="text-2xl font-bold">@{user.login}</h1>
      <p class="text-base-content/70">Your GitHub session is active.</p>
    </div>
  </div>
</section>
