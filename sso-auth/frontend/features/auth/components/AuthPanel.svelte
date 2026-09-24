<script lang="ts">
  export let title: string
  export let message: string
  export let eyebrow = ''
</script>

<section class="w-full rounded-2xl border border-slate-200 bg-white shadow-sm">
  <div class="p-6 sm:p-8">
    <header class="space-y-2">
      {#if eyebrow}
        <p class="text-xs font-semibold uppercase tracking-[0.14em] text-blue-600">{eyebrow}</p>
      {/if}
      <h1 class="text-2xl font-semibold tracking-tight text-slate-950 sm:text-[1.75rem]">{title}</h1>
      <p class="max-w-md text-sm leading-6 text-slate-600">{message}</p>
    </header>

    <div class="mt-7">
      <slot />
    </div>
  </div>
</section>
