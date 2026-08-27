export const LOCAL_RELAY_PORT = 47821
export const LOCAL_RELAY_BINARY = 'ai-tools-x86_64-unknown-linux-gnu'
export const LOCAL_RELAY_DOWNLOAD_URL = `https://github.com/farismnrr/ai-code/releases/latest/download/${LOCAL_RELAY_BINARY}`

export interface LocalRelayCommandOptions {
  origin: string
  allowTerminalNetwork?: boolean
  background?: boolean
  port?: number
}

export function buildLocalRelayCommand(options: LocalRelayCommandOptions) {
  const port = options.port ?? LOCAL_RELAY_PORT
  const flags = [
    '--mode local',
    '--dir /path/to/project',
    '--execution-root $HOME',
    `--origin ${options.origin}`,
    ...(port !== LOCAL_RELAY_PORT ? [`--port ${port}`] : []),
    ...(options.allowTerminalNetwork ? ['--allow-terminal-network'] : [])
  ]

  const lines = flags.map((flag, index) => {
    const continues = index < flags.length - 1 || options.background
    return `  ${flag}${continues ? ' \\' : ''}`
  })

  return [
    `${options.background ? 'nohup ' : ''}./${LOCAL_RELAY_BINARY} relay \\`,
    ...lines,
    ...(options.background ? ['  > relay-agent.log 2>&1 & disown'] : [])
  ].join('\n')
}
