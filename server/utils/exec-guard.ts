export const assertSafeCommand = async (command: string, args: string[], mode: 'read-only' | 'full'): Promise<void> => {
  if (mode === 'full') {
    // In 'full' mode (agent mode), no binary allowlist applies. Safety is enforced by the cwd jail + execa's shell: false.
    return
  }

  // mode === 'read-only' (chat mode)
  const allowlist = ['ls', 'cat', 'pwd', 'echo', 'grep', 'rg', 'head', 'tail', 'wc', 'stat', 'file', 'tree', 'diff', 'find', 'sed', 'git']

  if (!allowlist.includes(command)) {
    throw new Error(`assertSafeCommand: Command '${command}' is not allowed in read-only mode.`)
  }

  // Additional per-binary checks
  if (command === 'find') {
    // Exact-match blocklists are bypassable by GNU-style flags with attached
    // values (e.g. `-fprint` vs `-fprintf`), so block by prefix instead —
    // every one of these can execute a program or write to the filesystem.
    const blockedPrefixes = ['-delete', '-exec', '-execdir', '-ok', '-okdir', '-fprint', '-fls']
    if (args.some(arg => blockedPrefixes.some(prefix => arg.startsWith(prefix)))) {
      throw new Error(`assertSafeCommand: 'find' arguments starting with ${blockedPrefixes.join(', ')} are not allowed.`)
    }
  }

  if (command === 'sed') {
    // Only allow -n, block -i. GNU sed also accepts a suffix glued directly
    // onto -i (e.g. -i.bak), so an exact '-i' match alone is bypassable —
    // block on prefix instead.
    if (args.some(arg => arg.startsWith('-i'))) {
      throw new Error(`assertSafeCommand: 'sed -i' is not allowed.`)
    }
  }

  if (command === 'git') {
    const allowedGitCommands = ['status', 'log', 'diff', 'show', 'branch', 'remote']
    // git subcommand is usually the first arg, or after options. Keep it simple: check the first non-flag arg
    const subcommand = args.find(a => !a.startsWith('-'))
    if (!subcommand) {
      // Just 'git' with no subcommand is fine
      return
    }

    if (!allowedGitCommands.includes(subcommand)) {
      throw new Error(`assertSafeCommand: git subcommand '${subcommand}' is not allowed in read-only mode.`)
    }

    // `remote` is only allowed as the read-only `remote -v` (or bare
    // `remote`) — `remote add`/`remove`/`set-url` mutate repo config and
    // must not slip through just because the subcommand name matches.
    if (subcommand === 'remote') {
      const remoteArgs = args.filter(a => a !== 'remote')
      const allowedRemoteArgs = new Set(['-v', '--verbose'])
      if (remoteArgs.some(a => !allowedRemoteArgs.has(a))) {
        throw new Error(`assertSafeCommand: 'git remote' only allows '-v' in read-only mode.`)
      }
    }

    // Additional checks for blocked global flags. Prefix-matched because
    // git accepts `--flag=value` glued forms (e.g. `--exec=/bin/sh`,
    // `--output=file`) that an exact string match would miss entirely.
    const blockedFlagPrefixes = ['--global', '-C', '--exec', '--output', '--upload-pack', '-c']
    if (args.some(arg => blockedFlagPrefixes.some(prefix => arg.startsWith(prefix)))) {
      throw new Error(`assertSafeCommand: git flags like --global, -C, --exec, --output, --upload-pack, or -c are not allowed.`)
    }
  }
}

/**
 * Classifies a command as read-only by re-running it through the same
 * allowlist `assertSafeCommand` enforces in chat mode's read-only mode —
 * one implementation of "what counts as read-only", not a second one. Used
 * by agent mode to skip the approval prompt for commands that can't mutate
 * anything regardless of full write access being granted (e.g. `bash -c`
 * is never classified as read-only here, since an arbitrary shell script
 * can't be statically judged safe).
 */
export const isReadOnlyCommand = async (command: string, args: string[]): Promise<boolean> => {
  try {
    await assertSafeCommand(command, args, 'read-only')
    return true
  } catch {
    return false
  }
}
