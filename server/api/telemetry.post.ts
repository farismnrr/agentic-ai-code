import * as v from 'valibot'
import { getLogger } from '../utils/otel'

const logEventSchema = v.object({
  level: v.string(),
  message: v.string(),
  attributes: v.optional(v.record(v.string(), v.any())),
  timestamp: v.optional(v.number())
})

const telemetrySchema = v.array(logEventSchema)

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const body = await readValidatedBody(event, data => v.parse(telemetrySchema, data))

  const logger = getLogger('ai-code-frontend')

  for (const log of body) {
    let severityNumber: number
    switch (log.level.toLowerCase()) {
      case 'fatal':
        severityNumber = 21
        break
      case 'error':
        severityNumber = 17
        break
      case 'warn':
      case 'warning':
        severityNumber = 13
        break
      case 'debug':
        severityNumber = 5
        break
      case 'info':
      default:
        severityNumber = 9
        break // INFO
    }

    logger.emit({
      severityNumber,
      severityText: log.level.toUpperCase(),
      body: log.message,
      attributes: {
        ...log.attributes,
        'userId': session.user?.id,
        'service.name': 'ai-code-frontend'
      },
      timestamp: log.timestamp ? new Date(log.timestamp) : undefined
    })
  }

  return { success: true }
})
