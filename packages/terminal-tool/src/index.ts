import { tool as langchainTool } from '@langchain/core/tools'
import { tool as aiTool } from 'ai'
import { z } from 'zod'
import { execa } from 'execa'

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
    // Models frequently glue the whole invocation into `command` (e.g.
    // "ls -la") instead of splitting it into `args` as instructed — rather
    // than reject that outright (which just makes the model retry blindly),
    // split it here so both call shapes reach the same argv. This is plain
    // whitespace splitting, not shell parsing — execa still runs with
    // `shell: false`, so this doesn't reintroduce metacharacter handling.
    const [binary, ...gluedArgs] = command.trim().split(/\s+/)
    const finalCommand = binary ?? command
    const finalArgs = [...gluedArgs, ...args]

    await assertSafeCommand(finalCommand, finalArgs)

    // minimal env passthrough
    const env: Record<string, string> = {}
    if (process.env.PATH) env.PATH = process.env.PATH
    if (process.env.HOME) env.HOME = process.env.HOME
    if (process.env.LANG) env.LANG = process.env.LANG

    const timeoutMs = 30000
    const result = await execa(finalCommand, finalArgs, {
      shell: false,
      cwd,
      env,
      extendEnv: false,
      timeout: timeoutMs,
      killSignal: 'SIGKILL',
      reject: false
    })
    const { exitCode, stdout, stderr, timedOut } = result

    // `exitCode` is undefined when the process was killed for timing out
    // rather than exiting on its own — surface that explicitly instead of
    // an ambiguous "Exit: undefined" that reads the same as an unrelated
    // failure and gives the model nothing to act on.
    if (timedOut) {
      return `Error: command timed out after ${timeoutMs / 1000}s and was killed.`
    }

    // `reject: false` means execa never throws, even when the process
    // couldn't be spawned at all (bad `cwd`, missing binary, ENOENT, …) —
    // that case also has `exitCode: undefined` but with empty stdout/stderr
    // and no signal that anything actually went wrong. Without checking
    // `.failed` here, this silently returned "Exit: undefined / Stdout: /
    // Stderr: " for every single command, indistinguishable from a command
    // that legitimately produced no output — the model had no way to tell
    // "this ran and did nothing" from "this never ran at all", and neither
    // did anyone reading the logs.
    if (result.failed) {
      return `Error: ${result.shortMessage}`
    }

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
    inputSchema: terminalToolSchema,
    execute: async ({ command, args }) => runTerminalCommand({ command, args, cwd, assertSafeCommand })
  })
}
