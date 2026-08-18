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
COPY . .
RUN pnpm install --config.ignore-scripts=true
RUN pnpm build

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
COPY --from=build /app/.output ./.output
# otel-preload.mjs runs via `node --import` before Nitro's own build output
# ever loads, entirely outside Nitro's bundler — so Nitro's dependency
# tracer never sees its imports and won't copy @opentelemetry/* into
# .output/server/node_modules the way it does for packages the app itself
# imports. Copy the full node_modules from the build stage instead, so both
# the preload script and the app resolve everything the normal Node way.
COPY --from=build /app/node_modules ./node_modules
# Agent profiles and approved skill instructions are runtime inputs for the
# bounded subagent executor. Keep only these reviewed instruction roots in the
# production image; plans/contracts/history remain build-time repository data.
COPY --from=build /app/.agents/agents ./.agents/agents
COPY --from=build /app/.agents/skills ./.agents/skills
COPY --from=build /app/ai-self/skills ./ai-self/skills
COPY otel-preload.mjs ./otel-preload.mjs
EXPOSE 3333
ENV HOST=0.0.0.0
ENV PORT=3333
CMD ["node", "--import", "./otel-preload.mjs", ".output/server/index.mjs"]
