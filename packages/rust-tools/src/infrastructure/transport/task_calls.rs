//! Removed async MCP task-call surface.
//!
//! Agent tool execution is synchronous and bounded; long-running work is
//! handed to the human/operator as an explicit foreground command instead.
