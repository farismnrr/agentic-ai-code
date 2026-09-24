<script lang="ts">
  export let badge: string
  export let badgeClass = 'badge-neutral badge-outline'
  export let title: string
  export let message: string
</script>

<section class="card w-full border border-base-300 bg-base-100 shadow-xl">
  <div class="card-body gap-6 p-8 sm:p-10">
    <header class="space-y-3">
      <div class={`badge ${badgeClass}`}>{badge}</div>
      <div class="space-y-2">
        <h1 class="card-title text-3xl font-bold">{title}</h1>
        <p class="text-base-content/70">{message}</p>
      </div>
    </header>

    <slot />
  </div>
</section>
