export const NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native.local_terminal'

export interface NativeTool {
  id: string
  /** Human-readable label, shown in the tool picker. */
  name: string
  /** The model-facing tool key registered in the `ToolSet` (see server/api/chat.post.ts) — distinct from `name`, and how a pending approval part is matched back to this entry. */
  toolName: string
  description: string
  /**
   * `false` hides this entry from the Tool Picker's checkbox list (see
   * app/components/ChatToolPicker.vue) without removing it from this
   * registry — approval-id resolution (ChatToolApproval.vue's
   * `resolveToolId`) and the "N tools" count still need every entry here
   * regardless of picker visibility. Omit (defaults to visible) for any
   * tool that's meant to be manually toggled per conversation.
   */
  pickerVisible?: boolean
}

// The AI never executes shell commands on this server — `native.terminal`
// (workspace-sandboxed, server-side) was removed by deliberate decision: the
// only shell execution path is now `local_terminal`, which runs on the
// user's own machine via their relay-agent CLI. The historical Plan 026
// decision is compacted in .agents/plans/030-previous-plans-summary.md.
export const nativeTools: NativeTool[] = [
  {
    id: NATIVE_LOCAL_TERMINAL_TOOL_ID,
    name: 'Local Terminal (relay agent)',
    toolName: 'local_terminal',
    description: 'Execute shell commands on your local machine via the local CLI relay agent.',
    // No picker toggle — the Settings → Local Terminal page is already
    // where a user manages this device, so a second on/off switch here
    // would just be a redundant control for the same thing. Availability
    // is instead driven server-side by whether the user has a paired
    // device (see server/api/chat.post.ts) — this entry stays in the
    // registry purely so approval decisions ("Always allow"/"Always deny")
    // still resolve and persist correctly.
    pickerVisible: false
  }
]
