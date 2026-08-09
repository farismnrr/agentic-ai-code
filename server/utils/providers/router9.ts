import { createOpenAICompatible } from '@ai-sdk/openai-compatible'
import { decryptSecret } from '../crypto'

export function getRouter9Model(modelId: string, baseUrl: string, encryptedApiKey: string) {
  const provider = createOpenAICompatible({
    name: '9router',
    baseURL: baseUrl,
    apiKey: decryptSecret(encryptedApiKey)
  })
  return provider.chatModel(modelId)
}
