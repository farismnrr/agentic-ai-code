import { ChatOpenAI } from '@langchain/openai'

export function getLanggraphModel(modelId: string) {
  const config = useRuntimeConfig()
  return new ChatOpenAI({
    modelName: modelId,
    configuration: {
      baseURL: config.routerBaseUrl,
      apiKey: config.routerApiKey
    }
  })
}
