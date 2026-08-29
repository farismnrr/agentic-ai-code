// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/ui',
    // Renders assistant markdown incrementally as tokens arrive; plain
    // rendering flickers and breaks mid-token during streaming.
    '@comark/nuxt',
    'nuxt-auth-utils'
  ],

  devtools: {
    enabled: true
  },

  css: ['~/assets/css/main.css'],

  colorMode: {
    preference: 'dark',
    fallback: 'dark'
  },

  // Values here are overridden at runtime by NUXT_-prefixed env vars.
  // Public keys are exposed to the browser; add private keys at the top level.
  runtimeConfig: {
    // postgres.js connection string — read via useRuntimeConfig().databaseUrl
    // in server/utils/db.ts. Never expose this to the client.
    databaseUrl: '',
    routerBaseUrl: 'http://localhost:20128/v1',
    routerApiKey: '',
    modelProviderSecretKey: '',
    // Dedicated server-side key for encrypting source-bearing activity
    // evidence at rest. It is never exposed to the browser or relay events.
    activityPayloadSecret: '',
    activityRetentionDays: 90,
    searxngBaseUrl: 'http://127.0.0.1:8888',
    workspacesRoot: '',
    // Plan 036: private credentials for the one first-party public MCP
    // resource backed by the laptop relay. `url` and `ownerUserId` are both
    // matched before `accessToken` is ever attached, preventing another
    // authenticated ai-code user from reusing the operator's relay token by
    // creating an MCP row with the same URL. All three values are server-only;
    // never move them under `public`.
    remoteMcp: {
      url: '',
      ownerUserId: '',
      accessToken: '',
      // Per HTTP round-trip deadline only. Durable task execution may outlive
      // this budget and is retrieved through the MCP task lifecycle.
      requestTimeoutMs: 45_000
    },
    // nuxt-auth-utils sealed-cookie session key — NUXT_SESSION_PASSWORD must
    // be ≥ 32 characters. Generated once per environment, never reused.
    session: {
      password: '',
      cookie: {
        // h3's session cookie defaults to Secure unconditionally, which
        // silently drops the cookie on any non-HTTPS origin (plain
        // http://<lan-ip>:3333 during local dev) — the browser and curl
        // both refuse to store or resend it, so login "succeeds" but no
        // session ever persists. Only relax this outside production;
        // real deployments must keep Secure.
        secure: process.env.NUXT_SESSION_COOKIE_SECURE === 'true'
      }
    },
    // SMTP for email verification + password reset. All values come from
    // NUXT_SMTP_* env vars — nothing is hardcoded here.
    smtpHost: '',
    smtpPort: '',
    smtpSecure: '',
    smtpUser: '',
    smtpPassword: '',
    smtpFrom: '',
    oauth: {
      google: {
        clientId: '',
        clientSecret: ''
      },
      github: {
        clientId: '',
        clientSecret: ''
      }
    },
    public: {
      siteUrl: 'http://localhost:3333'
    }
  },

  routeRules: {
    // The landing page is static and public, so it can be built once at
    // deploy time. This reverses the change made in plan 001, when `/` was
    // the stateful chat screen.
    '/': { prerender: true },
    '/settings/local-terminal': { redirect: '/settings/mcp' }

    // /chat/** and /settings/** no longer carry ssr: false.
    //
    // Old reason they had it: the session lived in localStorage, which the
    // server can't read, so an SSR pass would render the guarded shell for a
    // signed-in visitor before the client middleware redirected — a visible
    // flash of the wrong state.
    //
    // Why it's gone now (plan 005, phase 1): the session is now an httpOnly
    // cookie set by nuxt-auth-utils. The server reads it on every request, so
    // requireUserSession() / useUserSession() work the same way on both sides.
    // The global middleware can guard on the server, so the SSR pass already
    // produces the right result — no flash, no need for client-only rendering.
  },

  // Default dev port. NUXT_PORT (or --port) in .env overrides this.
  devServer: {
    port: 3333
  },

  compatibilityDate: '2026-06-30',

  nitro: {
    errorHandler: '~~/server/core/errors/index',
    // Nuxt's fnv1a-64 dependency uses BigInt literals (for example, 32n).
    // Keep the server bundle target aligned with that ES2020 language feature
    // instead of asking esbuild to warn about code it cannot lower to ES2019.
    esbuild: {
      options: {
        target: 'es2020'
      }
    },
    rollupConfig: {
      onLog(level, log, defaultHandler) {
        const generatedAnnotationWarning
          = level === 'warn'
            && log.code === 'INVALID_ANNOTATION'
            && log.id?.includes('/.nuxt/dist/server/')
        const nuxtUnusedH3Import
          = level === 'warn'
            && log.message.includes('"H3Error" and "H3Event"')
            && log.message.includes('but never used in')

        if (generatedAnnotationWarning || nuxtUnusedH3Import) {
          return
        }

        defaultHandler(level, log)
      }
    },
    externals: {
      traceInclude: ['@opentelemetry'],
      external: [
        '@opentelemetry/sdk-trace-node',
        '@opentelemetry/sdk-logs',
        '@opentelemetry/api',
        '@opentelemetry/api-logs',
        '@opentelemetry/semantic-conventions',
        '@opentelemetry/exporter-trace-otlp-grpc',
        '@opentelemetry/resources',
        '@opentelemetry/instrumentation',
        '@opentelemetry/instrumentation-http'
      ]
    }
  },

  vite: {
    build: {
      // These diagnostics describe Nuxt/Rolldown's generated dependency
      // graph, not application code. Keep other build warnings enabled.
      rolldownOptions: {
        checks: {
          invalidAnnotation: false,
          pluginTimings: false
        }
      }
    }
  },

  // @nuxt/ui pulls in @nuxt/fonts, whose `fontless` dependency spawns an
  // esbuild service that never shuts itself down. `pnpm build` finishes and
  // writes .output/ correctly, but the CLI process hangs forever afterward —
  // upstream issue, not ours: https://github.com/nuxt/nuxt/issues/33987.
  hooks: {
    close: () => {
      if (process.argv.includes('build')) {
        process.exit(0)
      }
    }
  },

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
