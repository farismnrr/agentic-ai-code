import { tool as langchainTool } from '@langchain/core/tools'
import { tool as aiTool } from 'ai'
import { z } from 'zod'
import { execa } from 'execa'

export const terminalToolSchema = z.object({
  command: z.string().describe('The command to run.'),
  args: z.array(z.string()).optional().describe('Arguments for the command.')
})

export const runTerminalCommand = async ({
  command,
  args = [],
  cwd,
  assertSafeCommand
}: {
  command: string
  args?: string[]
  cwd: string
  assertSafeCommand: (command: string, args: string[]) => Promise<void>
}) => {
  try {
    await assertSafeCommand(command, args)

    // minimal env passthrough
    const env: Record<string, string> = {}
    if (process.env.PATH) env.PATH = process.env.PATH
    if (process.env.HOME) env.HOME = process.env.HOME
    if (process.env.LANG) env.LANG = process.env.LANG

    const { exitCode, stdout, stderr } = await execa(command, args, {
      shell: false,
      cwd,
      env,
      extendEnv: false,
      timeout: 30000,
      killSignal: 'SIGKILL',
      reject: false
    })

    const truncate = (str: string) => str.length > 20000 ? str.slice(0, 20000) + '... (truncated)' : str

    return `Exit: ${exitCode}\nStdout: ${truncate(stdout)}\nStderr: ${truncate(stderr)}`
  } catch (e: unknown) {
    return `Error: ${(e as Error).message}`
  }
}

export const createTerminalTool = ({
  assertSafeCommand,
  cwd
}: {
  assertSafeCommand: (command: string, args: string[]) => Promise<void>
  cwd: string
}) => {
  return langchainTool(
    async ({ command, args }) => runTerminalCommand({ command, args, cwd, assertSafeCommand }),
    {
      name: 'terminal',
      description: 'Run a shell command within the workspace directory.',
      schema: terminalToolSchema
    }
  )
}

export const createTerminalAiTool = ({
  assertSafeCommand,
  cwd
}: {
  assertSafeCommand: (command: string, args: string[]) => Promise<void>
  cwd: string
}) => {
  return aiTool({
    description: 'Run a shell command within the workspace directory.',
    parameters: terminalToolSchema,
    execute: async ({ command, args }) => runTerminalCommand({ command, args, cwd, assertSafeCommand })
  })
}
