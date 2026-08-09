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
EXPOSE 3333
ENV HOST=0.0.0.0
ENV PORT=3333
CMD ["node", ".output/server/index.mjs"]
