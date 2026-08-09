export const NATIVE_TERMINAL_TOOL_ID = 'native.terminal'

export interface NativeTool {
  id: string
  /** Human-readable label, shown in the tool picker. */
  name: string
  /** The model-facing tool key registered in the `ToolSet` (see server/api/chat.post.ts) — distinct from `name`, and how a pending approval part is matched back to this entry. */
  toolName: string
  description: string
}

export const nativeTools: NativeTool[] = [
  {
    id: NATIVE_TERMINAL_TOOL_ID,
    name: 'Terminal (full shell access)',
    toolName: 'terminal',
    description: 'Execute shell commands within the current workspace directory.'
  }
]
