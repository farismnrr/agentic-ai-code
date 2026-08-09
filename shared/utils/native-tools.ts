export const NATIVE_TERMINAL_TOOL_ID = 'native.terminal'

export interface NativeTool {
  id: string
  name: string
  description: string
}

export const nativeTools: NativeTool[] = [
  {
    id: NATIVE_TERMINAL_TOOL_ID,
    name: 'Terminal (full shell access)',
    description: 'Execute shell commands within the current workspace directory.'
  }
]
