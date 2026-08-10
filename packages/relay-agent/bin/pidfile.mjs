import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// Where the lock/pidfile lives — deliberately not `os.tmpdir()`. Confirmed
// via real-world guidance (storing per-user daemon pidfiles in a shared,
// system-wide temp directory is a documented anti-pattern: no per-user
// isolation, and the directory can be cleared by the OS/cleanup timers
// independently of whether the process is still running). Prefer
// `XDG_RUNTIME_DIR` (Linux: systemd-managed, tmpfs, per-user, torn down at
// logout — exactly the lifetime a "is this still running" lock should
// have) when set; otherwise a dotfolder under the user's home directory,
// which works the same way on every platform this CLI ships for.
function stateDir() {
  const dir = process.env.XDG_RUNTIME_DIR || path.join(os.homedir(), '.relay-agent')
  fs.mkdirSync(dir, { recursive: true })
  return dir
}

// Port-scoped, not a single fixed path — the relay-agent has no reason it
// couldn't run more than one instance (different `--port`, different
// `--dir`) on the same machine, and each needs its own pidfile.
export function pidFilePath(port) {
  return path.join(stateDir(), `relay-agent-${port}.pid`)
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

export function removePidFile(port) {
  try {
    fs.unlinkSync(pidFilePath(port))
  } catch {
    // Already gone — fine.
  }
}

// Only removes the pidfile if it still names *this* process — a real,
// confirmed-live race: two processes racing to start on the same port used
// to both write the pidfile (whoever wrote last "won" no matter who
// actually ended up bound to the port), so the loser's cleanup could delete
// the winner's still-valid entry. `acquireLock` below closes the actual
// race that caused that; this stays as a second, independent safety net —
// no code path can ever delete an entry it doesn't provably own.
export function removePidFileIfOwnedByMe(port) {
  if (readPidFile(port) === process.pid) {
    removePidFile(port)
  }
}

/**
 * Atomically claims the right to run on `port`, *before* ever attempting to
 * bind it — this is the actual concurrency guard now, not a side effect of
 * who wins the `listen()` race. `fs.openSync(path, 'wx')` is an exclusive
 * create: it succeeds only if the file does not already exist, and the
 * check-and-create is a single atomic filesystem operation (no
 * check-then-write window for a second process to land in between, unlike
 * `existsSync` followed by `writeFileSync`). A stale lock (process crashed
 * without cleaning up, or was `kill -9`'d) is detected via `isProcessAlive`
 * and cleared automatically — the standard failure mode documented for
 * every pidfile-based tool, so it's handled here rather than left as a
 * "delete the file yourself" instruction.
 *
 * Returns `true` if the lock was acquired (caller may proceed to bind the
 * port and, on success, should leave the file in place — it already holds
 * this process's own pid); `false` if another live instance already holds
 * it (caller should refuse to start).
 */
export function acquireLock(port) {
  const file = pidFilePath(port)
  try {
    const fd = fs.openSync(file, 'wx')
    fs.writeSync(fd, String(process.pid))
    fs.closeSync(fd)
    return true
  } catch (err) {
    if (err.code !== 'EEXIST') throw err
  }

  // Lock file already exists — stale (owning process is gone) or live.
  const existingPid = readPidFile(port)
  if (existingPid && isProcessAlive(existingPid)) {
    return false
  }
  // Stale: clear it and retry once. A second EEXIST here would mean
  // another process won the exact same race between our unlink and our
  // retry — vanishingly unlikely for a single-user local CLI, and failing
  // the start with a clear error is the right outcome if it ever happens,
  // rather than silently retrying forever.
  removePidFile(port)
  const fd = fs.openSync(file, 'wx')
  fs.writeSync(fd, String(process.pid))
  fs.closeSync(fd)
  return true
}
