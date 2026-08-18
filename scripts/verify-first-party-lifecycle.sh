#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
node --input-type=module <<'NODE'
import { readFileSync } from 'node:fs'
const rust = readFileSync('packages/rust-tools/infrastructure/src/transport/tools.rs', 'utf8')
const dispatch = readFileSync('packages/rust-tools/application/src/dispatcher.rs', 'utf8')
const hooks = readFileSync('packages/rust-tools/application/src/hooks.rs', 'utf8')
const client = readFileSync('app/composables/useRelayAgent.ts', 'utf8')
const chat = readFileSync('app/composables/useConversationChat.ts', 'utf8')
const controller = readFileSync('app/composables/chat/local-tool-controller.ts', 'utf8')
const server = readFileSync('server/application/chat/execute-chat-turn.ts', 'utf8')
const endpoint = readFileSync('server/api/chat.post.ts', 'utf8')
const approvalUi = readFileSync('app/components/chat/ChatToolApproval.vue', 'utf8')
const required = [
  [dispatch, '"agent/session_start"', 'explicit session-start RPC'],
  [dispatch, '"agent/pre_stop"', 'explicit pre-stop RPC'],
  [rust, '"control": { "type": "approval_required"', 'typed approval control result'],
  [hooks, 'bounded_context', 'bounded structured context'],
  [hooks, 'issue_approval', 'one-use hook approval token'],
  [hooks, 'current_attempt == 2', 'bounded stop boundary'],
  [client, "'io.modelcontextprotocol/agentSession'", 'lifecycle metadata channel'],
  [chat, 'agentContext', 'context consumed by chat transport'],
  [server, 'Bounded repository hook context', 'context reaches agent system context'],
  [endpoint, 'agentContext', 'chat endpoint accepts structured context'],
  [client, 'async function startSession', 'first-party session start client'],
  [client, 'async function preAgentStop', 'first-party stop client'],
  [chat, 'relayAgent.preAgentStop', 'agent completion boundary']
]
for (const [source, needle, label] of required) if (!source.includes(needle)) throw new Error(`missing ${label}`)
if (rust.includes('session_id') || hooks.includes('session_id')) throw new Error('lifecycle identity leaked into undeclared argument contract')
if (!client.includes('approvalRequired') || !client.includes('hookApprovalToken') || !controller.includes('requestApproval') || !approvalUi.includes('hookAnswer')) throw new Error('approval control has no first-party consumer')
console.log('first-party lifecycle/approval/context acceptance: pass')
NODE
bash scripts/verify-hooks.sh >/dev/null
