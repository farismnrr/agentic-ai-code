export default defineEventHandler(async (event) => {
  await requireUserSession(event)
  return [
    { label: 'OpenAI Compatible', value: 'openai_compatible' },
    { label: 'Anthropic Compatible', value: 'anthropic_compatible' },
    { label: 'Vertex AI', value: 'vertex_ai' }
  ]
})
