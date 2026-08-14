import { tool as langchainTool } from '@langchain/core/tools'
import { tool as aiTool } from 'ai'
import { z } from 'zod'
import { execa } from 'execa'

import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const terminalToolSchema = z.object({
  command: z.string().describe('The binary to run, e.g. "ls" — do not include flags/arguments here, put them in `args` instead.'),
  args: z.array(z.string()).optional().describe('Arguments for the command, e.g. ["-la", "."].'),
  // Only meaningful for a tool that has no fixed `cwd` closed over server-side
  // (e.g. @ai-code/relay-agent's `local_terminal`, which has no directory jail
  // at all) — createTerminalTool/createTerminalAiTool below ignore this and
  // always run in their own fixed, server-controlled `cwd`.
  cwd: z.string().optional().describe('Absolute path to run the command in, for a tool with no fixed working directory. Omit to use that tool\'s own default.')
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
    const [binary, ...gluedArgs] = command.trim().split(/\s+/)
    const finalCommand = binary ?? command
    const finalArgs = [...gluedArgs, ...args]

    await assertSafeCommand(finalCommand, finalArgs)

    const __dirname = path.dirname(fileURLToPath(import.meta.url))
    const rustBin = path.join(__dirname, '../../../target/release/ai-tools')

    const cliArgs = [
      'terminal',
      '--cwd', cwd,
      '--no-guard',
      '--',
      command,
      ...args
    ]

    const result = await execa(rustBin, cliArgs, {
      reject: false
    })

    if (result.failed || result.stdout.startsWith('Error:')) return 'Tool execution failed'

    return result.stdout
  } catch {
    return 'Tool execution failed'
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
    inputSchema: terminalToolSchema,
    execute: async ({ command, args }) => runTerminalCommand({ command, args, cwd, assertSafeCommand })
  })
}
