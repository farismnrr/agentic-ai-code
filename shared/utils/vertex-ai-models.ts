/**
 * Vertex AI Express Mode has no ListModels/discovery endpoint reachable with
 * an API key (confirmed directly against the real API — ListPublisherModels
 * rejects API-key auth, OAuth2-only). This was established during historical
 * Plan 023, now compacted in .agents/plans/030-previous-plans-summary.md, so
 * unlike OpenAI/Anthropic-compatible providers this list can't be fetched
 * live with the same credential. IDs were curl-verified individually against
 * the real aiplatform.googleapis.com Express Mode endpoint.
 */
export const VERTEX_AI_CHAT_MODELS = [
  { label: 'gemini-3.5-flash-lite', value: 'gemini-3.5-flash-lite' },
  { label: 'gemini-3.6-flash', value: 'gemini-3.6-flash' },
  { label: 'gemini-3.1-pro-preview', value: 'gemini-3.1-pro-preview' }
]

/**
 * Context window / max output token limits per Google's own model pages
 * (ai.google.dev/gemini-api/docs/models/<id>) — all three currently share
 * the same Gemini 3 family limits. `thinkingEnabled: true` because thinking
 * is a native, always-available part of the Gemini 3 family (unlike Gemini
 * 2.x, where it was opt-in per model). Used to auto-fill the model form's
 * Overrides section when a Vertex AI model is picked — still plain,
 * editable fields afterward, not locked.
 */
export const VERTEX_AI_MODEL_DEFAULTS: Record<string, { contextWindow: number, maxOutputTokens: number, thinkingEnabled: boolean }> = {
  'gemini-3.5-flash-lite': { contextWindow: 1048576, maxOutputTokens: 65536, thinkingEnabled: true },
  'gemini-3.6-flash': { contextWindow: 1048576, maxOutputTokens: 65536, thinkingEnabled: true },
  'gemini-3.1-pro-preview': { contextWindow: 1048576, maxOutputTokens: 65536, thinkingEnabled: true }
}

/**
 * Embedding models use the `:predict` endpoint, not `:generateContent` —
 * not a chat model, so deliberately excluded from VERTEX_AI_CHAT_MODELS.
 * Parked here unused until an embeddings use case (e.g. RAG/semantic
 * search) lands and needs it.
 */
export const VERTEX_AI_EMBEDDING_MODELS = [
  { label: 'gemini-embedding-001', value: 'gemini-embedding-001' }
]
