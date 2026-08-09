import { consola } from 'consola'
import { getLogger } from './otel'

/**
 * Single logging entry point for server code — replaces raw `console.*`
 * calls, which only ever reached `docker compose logs` and were invisible
 * in Loki (see server/utils/http-errors.ts's `problem()` for the same fix
 * applied to thrown API errors). `consola` gives readable, leveled stdout
 * output in dev (it's already a Nitro/Nuxt-ecosystem convention — bundled
 * transitively via `nuxt`), and every call is also forwarded through the
 * existing OTel → Loki bridge so it shows up there too. `getLogger()`
 * no-ops when NUXT_OTEL_ENABLED isn't 'true', so this is safe to call
 * unconditionally in every environment.
 */

type LogAttributes = Record<string, unknown>

function errorAttributes(err: unknown): LogAttributes {
  if (err instanceof Error) return { error: err.message, stack: err.stack }
  if (err === undefined) return {}
  return { error: String(err) }
}

function emit(severityNumber: number, severityText: string, message: string, attributes: LogAttributes) {
  getLogger('ai-code-server').emit({
    severityNumber,
    severityText,
    body: message,
    attributes: { 'service.name': 'ai-code-server', ...attributes }
  })
}

export const logger = {
  error(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.error(message)
    else consola.error(message, err)
    emit(17, 'ERROR', message, { ...errorAttributes(err), ...attributes })
  },
  warn(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.warn(message)
    else consola.warn(message, err)
    emit(13, 'WARN', message, { ...errorAttributes(err), ...attributes })
  },
  info(message: string, attributes: LogAttributes = {}) {
    consola.info(message)
    emit(9, 'INFO', message, attributes)
  },
  // Forwards to Loki only, no consola print — for wrapping output Node/a
  // dependency already prints on its own (e.g. process.emitWarning), where
  // calling logger.warn() would duplicate every line on stdout.
  forwardOnly(severityNumber: number, severityText: string, message: string, attributes: LogAttributes = {}) {
    emit(severityNumber, severityText, message, attributes)
  }
}
