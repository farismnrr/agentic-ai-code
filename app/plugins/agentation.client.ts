import { createApp } from 'vue'

/**
 * Mounts the Agentation annotation toolbar — click an element, leave a note,
 * copy selectors an agent can grep for.
 *
 * Dev-only and client-only, by three independent mechanisms, so it cannot
 * reach production even if one is changed:
 *
 * 1. `.client.ts` keeps it out of the server bundle
 * 2. the `import.meta.dev` guard makes Vite drop the branch in a prod build
 * 3. `agentation-vue` is a devDependency, so a production install has no
 *    package to resolve even if the import survived
 *
 * The toolbar is mounted here rather than placed in `app.vue`. A tag there
 * compiles to a `resolveComponent('AgentationVue')` call that survives into
 * the production bundle as dead code — harmless, but it left the component
 * name in the server entry, which made the claim above less than exactly
 * true. Mounting into its own root leaves no trace at all.
 *
 * A separate Vue app is correct for an overlay: it sits outside the page's
 * component tree on purpose, so it can inspect that tree without appearing
 * in it.
 *
 * `agentation-vue` is an unofficial community port — the official package is
 * React-only. It was scanned before adoption: no network calls, no eval, no
 * external URLs anywhere across its 107 dist files. It stores settings in
 * localStorage and writes output to the clipboard, which is all it claims to
 * do. Re-scan on upgrade; that guarantee is version-specific.
 */
export default defineNuxtPlugin(async () => {
  if (!import.meta.dev) return

  const { AgentationVue, AgentationVuePlugin } = await import('agentation-vue')
  await import('agentation-vue/style.css')

  const container = document.createElement('div')
  container.id = 'agentation-root'
  document.body.appendChild(container)

  createApp(AgentationVue).use(AgentationVuePlugin).mount(container)
})
