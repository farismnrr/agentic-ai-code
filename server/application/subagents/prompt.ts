import { buildToolSelectionPolicy } from '../chat/tool-selection-policy.ts'

/** Called after child authority, effects, ownership and profile tool filtering. */
export function buildSubagentPrompt(input: { instructions: string, skills: string[], context: unknown, toolNames: string[], maxContextTokens: number }) {
  const system = [input.instructions, ...input.skills, buildToolSelectionPolicy(input.toolNames), 'Return JSON with keys status, summary, findings, evidence, validation, remaining_risks. Never include hidden reasoning.'].filter(Boolean).join('\n')
  const prompt = JSON.stringify(input.context)
  if (Buffer.byteLength(system + prompt, 'utf8') > input.maxContextTokens * 4) throw new Error('child context exceeds effective token bound')
  return { system, prompt }
}
