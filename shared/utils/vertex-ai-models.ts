/**
 * Vertex AI Express Mode has no ListModels/discovery endpoint reachable with
 * an API key (confirmed directly against the real API — ListPublisherModels
 * rejects API-key auth, OAuth2-only) — see .agents/plans/023, so unlike the
 * OpenAI/Anthropic-compatible providers this can't be fetched live. These
 * IDs were curl-verified one by one against the real
 * aiplatform.googleapis.com Express Mode endpoint, not taken from docs alone.
 */
export const VERTEX_AI_CHAT_MODELS = [
  { label: 'gemini-3.5-flash-lite', value: 'gemini-3.5-flash-lite' },
  { label: 'gemini-3.6-flash', value: 'gemini-3.6-flash' },
  { label: 'gemini-3.1-pro-preview', value: 'gemini-3.1-pro-preview' }
]

/**
 * Embedding models use the `:predict` endpoint, not `:generateContent` —
 * not a chat model, so deliberately excluded from VERTEX_AI_CHAT_MODELS.
 * Parked here unused until an embeddings use case (e.g. RAG/semantic
 * search) lands and needs it.
 */
export const VERTEX_AI_EMBEDDING_MODELS = [
  { label: 'gemini-embedding-001', value: 'gemini-embedding-001' }
]
