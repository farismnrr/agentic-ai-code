import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// Port-scoped, not a single fixed path — the relay-agent has no reason it
// couldn't run more than one instance (different `--port`, different
// `--dir`) on the same machine, and each needs its own pidfile.
export function pidFilePath(port) {
  return path.join(os.tmpdir(), `relay-agent-${port}.pid`)
}

export function isProcessAlive(pid) {
  try {
    // Signal 0 sends nothing — it only checks whether the process exists
    // and this user has permission to signal it, without actually killing it.
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

export function readPidFile(port) {
  try {
    const raw = fs.readFileSync(pidFilePath(port), 'utf8').trim()
    const pid = parseInt(raw, 10)
    return isNaN(pid) ? null : pid
  } catch {
    return null
  }
}

export function writePidFile(port) {
  fs.writeFileSync(pidFilePath(port), String(process.pid))
}

export function removePidFile(port) {
  try {
    fs.unlinkSync(pidFilePath(port))
  } catch {
    // Already gone — fine.
  }
}
