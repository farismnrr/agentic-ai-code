FROM mcr.microsoft.com/playwright:v1.55.0-noble

WORKDIR /e2e

RUN npm init -y \
    && npm install --no-audit --no-fund playwright@1.55.0

COPY sso-auth/e2e/smoke.cjs ./smoke.cjs

ENTRYPOINT ["node", "smoke.cjs"]
