import { tool } from 'ai'
import { z } from 'zod'
import { ORCHESTRATOR_BUDGET_CLASSES, ORCHESTRATOR_ROLES } from '../../../shared/types/orchestration.ts'
import { SUBAGENT_PROFILES } from '../../../shared/types/subagents.ts'
import { ORCHESTRATOR_CAPS, replaceOrchestratorGraph } from './task-graph.ts'

const effectSchema = z.enum(['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge'])

export function buildOrchestratorPlanTool(input: { userId: string, conversationId: string, parentSessionId: string }) {
  return tool({
    description: 'Define a bounded parent-owned orchestration task graph and return currently ready nodes. This tool only plans/schedules state; it does not spawn children or grant capabilities.',
    inputSchema: z.object({
      nodes: z.array(z.object({
        id: z.string().min(1).max(ORCHESTRATOR_CAPS.id),
        role: z.enum(ORCHESTRATOR_ROLES),
        objective: z.string().min(1).max(ORCHESTRATOR_CAPS.objective),
        depends_on: z.array(z.string().min(1).max(ORCHESTRATOR_CAPS.id)).max(ORCHESTRATOR_CAPS.dependencies).default([]),
        budget_class: z.enum(ORCHESTRATOR_BUDGET_CLASSES).default('medium'),
        profile: z.enum(SUBAGENT_PROFILES).optional(),
        required_tools: z.array(z.string().min(1).max(160)).max(ORCHESTRATOR_CAPS.requiredTools).default([]),
        required_effects: z.array(effectSchema).max(ORCHESTRATOR_CAPS.requiredEffects).default([])
      })).min(1).max(ORCHESTRATOR_CAPS.nodes)
    }),
    execute: async ({ nodes }) => replaceOrchestratorGraph({ ...input, nodes })
  })
}
