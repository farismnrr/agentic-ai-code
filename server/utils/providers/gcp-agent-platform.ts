import { createGoogleGenerativeAI } from '@ai-sdk/google'
import { decryptSecret } from '../crypto'

export function getGcpAgentPlatformModel(modelId: string, encryptedApiKey: string) {
  const provider = createGoogleGenerativeAI({
    apiKey: decryptSecret(encryptedApiKey)
  })
  return provider(modelId)
}
