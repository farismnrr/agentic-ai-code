#!/usr/bin/env node

import { parseArgs } from 'node:util'
import { createTerminalTool } from '../../terminal-tool/src/index.ts'

const { values, positionals } = parseArgs({
  args: process.argv.slice(2),
  options: {
    'cwd': { type: 'string', default: process.cwd() },
    'no-guard': { type: 'boolean' },
    'timeout': { type: 'string' }
  },
  allowPositionals: true
})

const command = positionals[0]
const args = positionals.slice(1)

if (!command) {
  console.error('Usage: terminal-tool <command> [args...] [--cwd <path>] [--no-guard]')
  process.exit(1)
}

const assertSafeCommand = async () => {
  if (!values['no-guard']) {
    console.warn('WARN: Exec guard is enabled but no external validation is provided in CLI. Pass --no-guard if you want to bypass exec protection.')
    throw new Error('Exec guard blocked request. Use --no-guard to bypass.')
  }
}

const t = createTerminalTool({ assertSafeCommand, cwd: values.cwd })

t.invoke({
  command,
  args
}).then(res => console.log(res)).catch(err => console.error(err))
