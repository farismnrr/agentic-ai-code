import { readFileSync } from 'node:fs'
import { toolEffects } from '../shared/utils/capability-policy.ts'

type Case = { tool: string, destructive: boolean, open_world: boolean, effects: string[] }
const cases = JSON.parse(readFileSync('.agents/contracts/039e-effect-classification.json', 'utf8')) as Case[]
const actual = cases.map(item => ({ ...item, effects: toolEffects(item.tool, { destructiveHint: item.destructive, openWorldHint: item.open_world }, 'external') }))
if (JSON.stringify(actual) !== JSON.stringify(cases)) {
  console.error('effect classification parity mismatch')
  process.exit(1)
}
console.log('effect classification parity: pass')
