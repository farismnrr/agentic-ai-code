---
name: terminal-tool
description: Standalone LangChain and AI SDK tool for executing terminal commands.
license: MIT
---

# @ai-code/terminal-tool

A standalone LangChain and AI SDK tool that runs a shell command within a specified working directory, guarded by an injected validation function.

## Usage

### In Code

```ts
import { createTerminalTool, createTerminalAiTool } from '@ai-code/terminal-tool'

const myTerminalTool = createTerminalTool({
  cwd: '/my/workspace',
  assertSafeCommand: async (command, args) => {
    // Implement your own security guard
    if (command === 'rm') throw new Error('Blocked')
  }
})

const result = await myTerminalTool.invoke({ command: 'ls', args: ['-la'] })
```

### CLI

The CLI provides a way to run commands directly. Use `--no-guard` to explicitly bypass validations when running locally.

```bash
npx @ai-code/terminal-tool ls -la --cwd /tmp --no-guard
```
