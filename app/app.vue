<script setup lang="ts">
useHead({
  meta: [
    { name: 'viewport', content: 'width=device-width, initial-scale=1' }
  ],
  link: [
    { rel: 'icon', href: '/favicon.ico' }
  ],
  htmlAttrs: {
    lang: 'en'
  }
})

const title = 'AI Code'
const description = 'A chat interface for AI models with MCP tool support.'

useSeoMeta({
  title,
  titleTemplate: chunk => (chunk && chunk !== title ? `${chunk} · ${title}` : title),
  description,
  ogTitle: title,
  ogDescription: description
})
</script>

<template>
  <!-- UApp must stay outermost: toasts, tooltips and programmatic overlays
       are mounted from its context. -->
  <UApp>
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
  </UApp>
</template>
