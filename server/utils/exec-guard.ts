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
    const blockedArgs = ['-delete', '-exec', '-fprintf']
    if (args.some(arg => blockedArgs.includes(arg))) {
      throw new Error(`assertSafeCommand: 'find' arguments ${blockedArgs.join(', ')} are not allowed.`)
    }
  }

  if (command === 'sed') {
    // Only allow -n, block -i
    if (args.includes('-i')) {
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

    // Additional checks for blocked global flags
    if (args.includes('--global') || args.includes('-C') || args.includes('--exec')) {
      throw new Error(`assertSafeCommand: git global flags like --global, -C, or --exec are not allowed.`)
    }
  }
}
