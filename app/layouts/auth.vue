<template>
  <div class="flex min-h-svh flex-col bg-muted">
    <header class="p-4 sm:p-6">
      <NuxtLink
        to="/"
        class="inline-flex items-center gap-2 text-sm text-muted hover:text-default"
      >
        <UIcon name="i-lucide-arrow-left" />
        Back to home
      </NuxtLink>
    </header>

    <main class="flex flex-1 items-center justify-center p-4">
      <div class="w-full max-w-sm">
        <slot />
      </div>
    </main>

    <footer class="p-4 text-center text-xs text-dimmed sm:p-6">
      A prototype — no real accounts, any credentials work.
    </footer>
  </div>
</template>
