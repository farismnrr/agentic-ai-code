<template>
  <!-- The ground, same as every other screen. `bg-muted` here was the one
       use of that token in the whole app, and it put auth two steps lighter
       than the app it leads into. The card carries the elevation instead. -->
  <div class="flex min-h-svh flex-col bg-default">
    <header class="p-4 sm:p-6">
      <NuxtLink
        to="/"
        class="inline-flex items-center gap-2 text-sm text-muted hover:text-default"
      >
        <UIcon name="i-lucide-arrow-left" />
        Back to home
      </NuxtLink>
    </header>

    <main class="flex flex-1 items-center justify-center p-6 sm:p-8">
      <div class="w-full max-w-sm">
        <slot />
      </div>
    </main>

    <footer class="p-4 text-center text-xs text-dimmed sm:p-6">
      A prototype — no real accounts, any credentials work.
    </footer>
  </div>
</template>
