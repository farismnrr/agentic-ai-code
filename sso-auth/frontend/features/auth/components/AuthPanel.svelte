<script lang="ts">
  export let badge: string
  export let badgeClass = 'badge-neutral badge-outline'
  export let title: string
  export let message: string
</script>

<section class="card w-full overflow-hidden border border-base-300 bg-base-100 text-base-content shadow-xl">
  <div class="card-body gap-6 p-6 sm:p-10">
    <header class="space-y-3">
      <div class={`badge ${badgeClass}`}>{badge}</div>
      <div class="min-w-0 space-y-2">
        <h1 class="card-title break-words text-3xl font-bold leading-tight">{title}</h1>
        <p class="text-base-content/75">{message}</p>
      </div>
    </header>

    <slot />
  </div>
</section>
