FROM node:22-slim AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
ENV CI=true
RUN corepack enable

FROM base AS build
WORKDIR /app
COPY . .
RUN pnpm install --config.ignore-scripts=true
RUN pnpm build

FROM base AS runtime
WORKDIR /app
COPY --from=build /app/.output ./.output
# otel-preload.mjs runs via `node --import` before Nitro's own build output
# ever loads, entirely outside Nitro's bundler — so Nitro's dependency
# tracer never sees its imports and won't copy @opentelemetry/* into
# .output/server/node_modules the way it does for packages the app itself
# imports. Copy the full node_modules from the build stage instead, so both
# the preload script and the app resolve everything the normal Node way.
COPY --from=build /app/node_modules ./node_modules
COPY otel-preload.mjs ./otel-preload.mjs
EXPOSE 3333
ENV HOST=0.0.0.0
ENV PORT=3333
CMD ["node", "--import", "./otel-preload.mjs", ".output/server/index.mjs"]
