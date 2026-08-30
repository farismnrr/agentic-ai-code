FROM node:22-slim AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
ENV CI=true
RUN corepack enable

FROM base AS runtime-tools
# node:22-slim ships almost nothing beyond coreutils/findutils/grep/sed/awk —
# the terminal tool's own system prompt (see buildWorkspaceSystemPrompt in
# server/api/chat.post.ts) tells the model to use `tree`, `grep`/`rg`, `git`,
# none of which existed in the image, so every such call failed to even
# spawn (see packages/terminal-tool/src/index.ts's execa `.failed` handling).
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    tree \
    ripgrep \
    less \
    unzip \
    jq \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

FROM base AS build
WORKDIR /app
# The native Rust workspace, relay, and native-tool adapter packages are
# deliberately excluded by .dockerignore. Rust is deployed as the separate
# ai-tools systemd service; this image builds and runs only the Nuxt app.
# Nuxt prerenders `/`, and nuxt-auth-utils validates its password during that
# build-time request. This throwaway value is scoped to the build command; the
# final image receives the real NUXT_SESSION_PASSWORD from Compose/runtime env.
COPY . .
RUN pnpm install --frozen-lockfile --config.ignore-scripts=true
RUN NUXT_SESSION_PASSWORD="build-only-session-password-not-for-runtime" NUXT_DATABASE_ENFORCE_LEAST_PRIVILEGE=false pnpm build
# Runtime keeps only declared production dependencies; build/lint/type tooling
# must not expand the deployed attack surface.
RUN pnpm prune --prod --config.ignore-scripts=true

FROM runtime-tools AS runtime
ARG VERSION=dev
ARG REVISION=unknown
ARG CREATED=unknown
LABEL org.opencontainers.image.title="AI Code" \
      org.opencontainers.image.description="MasihAwam AI Code web application" \
      org.opencontainers.image.source="https://github.com/farismnrr/ai-code" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${CREATED}"
ENV APP_VERSION="${VERSION}"
WORKDIR /app
COPY --chown=node:node --from=build /app/.output ./.output
# otel-preload.mjs runs via `node --import` before Nitro's own build output
# ever loads, entirely outside Nitro's bundler — so Nitro's dependency
# tracer never sees its imports and won't copy @opentelemetry/* into
# .output/server/node_modules the way it does for packages the app itself
# imports. Copy the full node_modules from the build stage instead, so both
# the preload script and the app resolve everything the normal Node way.
COPY --chown=node:node --from=build /app/node_modules ./node_modules
# Agent profiles and approved skill instructions are runtime inputs for the
# bounded subagent executor. Keep only these reviewed instruction roots in the
# production image; plans/contracts/history remain build-time repository data.
COPY --chown=node:node --from=build /app/.agents/agents ./.agents/agents
COPY --chown=node:node --from=build /app/.agents/skills ./.agents/skills
COPY --chown=node:node --from=build /app/ai-self/skills ./ai-self/skills
COPY --chown=node:node otel-preload.mjs ./otel-preload.mjs
COPY --chown=node:node ops/runtime-web-entry.mjs ./ops/runtime-web-entry.mjs
COPY --chown=node:node server/application/database-role-policy.mjs ./server/application/database-role-policy.mjs
USER node
EXPOSE 3333
ENV HOST=0.0.0.0
ENV PORT=3333
CMD ["node", "--import", "./otel-preload.mjs", "./ops/runtime-web-entry.mjs"]
