# 9Router Config Convention

**Context**: In plan 007, we wired up the chat interface to a real model served by `9router`.

**Convention**:
- The router base URL and API key are configured in `.env` as `NUXT_ROUTER_BASE_URL` and `NUXT_ROUTER_API_KEY`.
- The real API key (`sk-b779a94bf4382cee-...`) is found in `~/.9router/db.json` under `apiKeys[0].key`.
- `NUXT_ROUTER_BASE_URL` defaults to `http://localhost:20128/v1`.
- The frontend chat streaming is untouched; `server/api/chat.post.ts` translates between the frontend's text-delta stream (`0:...`) and the OpenAI-compatible Server-Sent Events from `9router`.
