export type CreativeCanvasTool = 'creative_graph' | 'creative_project'

export type CreativeCanvasInvokeInput = {
  workspaceId: string
  tool: CreativeCanvasTool
  arguments: Record<string, unknown>
}

export type CreativeCanvasInvokeResult = {
  tool: CreativeCanvasTool
  content: unknown[]
  isError: boolean
}

export interface CreativeCanvasUseCases {
  invoke(userId: string, input: CreativeCanvasInvokeInput): Promise<CreativeCanvasInvokeResult>
}
