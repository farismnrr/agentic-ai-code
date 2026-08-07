// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/ui'
  ],

  devtools: {
    enabled: true
  },

  css: ['~/assets/css/main.css'],

  // Values here are overridden at runtime by NUXT_-prefixed env vars.
  // Public keys are exposed to the browser; add private keys at the top level.
  runtimeConfig: {
    public: {
      siteUrl: 'http://localhost:3333'
    }
  },

  routeRules: {
    '/': { prerender: true }
  },

  // Default dev port. NUXT_PORT (or --port) in .env overrides this.
  devServer: {
    port: 3333
  },

  compatibilityDate: '2026-06-30',

  eslint: {
    // Surface lint errors in the dev server and browser overlay, not just in `pnpm lint`
    checker: true,

    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      },
      // typescript-eslint strict preset. Type-aware rules are intentionally not
      // enabled: Nuxt 4's root tsconfig.json is references-only (`files: []`),
      // so there is no single project for the type checker to resolve against.
      typescript: {
        strict: true
      },
      // Formats css/json/markdown via eslint-plugin-format (Prettier under the hood)
      formatters: true
    }
  }
})
