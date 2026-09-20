import type { CreativeCanvasInvokeInput } from '#server/application/creative-canvas'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const body = await readBody<CreativeCanvasInvokeInput>(event)
  if (!body || typeof body !== 'object' || !body.workspaceId || !body.tool || !body.arguments) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid Creative Canvas request' })
  }
  if (body.tool !== 'creative_graph' && body.tool !== 'creative_project') {
    throw createError({ statusCode: 400, statusMessage: 'Unsupported Creative Canvas tool' })
  }
  return event.context.application.creativeCanvas.invoke(session.user.id, body)
})
