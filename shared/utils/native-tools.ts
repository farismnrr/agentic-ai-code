export const NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native.local_terminal'

export interface NativeTool {
  id: string
  /** Human-readable label, shown in the tool picker. */
  name: string
  /** The model-facing tool key registered in the ToolSet. */
  toolName: string
  description: string
  /** Omit or set true when the capability is user-toggleable in the picker. */
  pickerVisible?: boolean
}

// Shell execution only happens on the user's machine through the loopback
// relay agent. Unlike the old pairing-driven behavior, this registry entry is
// now an explicit per-conversation capability switch: enabling the terminal
// here unlocks Agent Mode once the relay is also live.
export const nativeTools: NativeTool[] = [
  {
    id: NATIVE_LOCAL_TERMINAL_TOOL_ID,
    name: 'Terminal relay',
    toolName: 'local_terminal',
    description: 'Let AI Code use the local CLI relay for terminal commands.',
    pickerVisible: true
  }
]

export function isNativeToolId(toolId: string) {
  return nativeTools.some(tool => tool.id === toolId)
}
