# Design System: OpenClaw / opencode-web

This design direction supersedes the original "Instrument" theme from plan 003, moving towards a denser, more technical aesthetic suited for a workspace-oriented agent interface.

## Core Principles

1. **Dark by Default**: The interface assumes a dark environment, aligning with developer tools and terminal-based workflows. The `signal` and `graphite` scales from plan 003 are retained but optimized for the dark surface.
2. **Density**: UI elements, especially in the sidebar (workspaces, conversations), use tighter padding and smaller heights. This allows more information to fit on screen without feeling cluttered, using semantic colors (`text-muted`, `bg-elevated`) rather than raw palettes to maintain clarity.
3. **Typography**:
   - **Chrome**: Sans-serif (`Geist`) remains the default for the application shell, settings, and navigation.
   - **Content**: The chat surface itself is strictly monospace-forward (`Geist Mono`). This gives the interaction a terminal-like feel, emphasizing the technical nature of the output.
4. **Interaction Patterns**:
   - **Command-Palette First**: ⌘K is the primary way to navigate between workspaces and conversations, reducing reliance on point-and-click sidebar navigation.
   - **Workspace Scoping**: Chats are grouped into user-defined workspaces, switchable via a compact dropdown in the sidebar header, keeping the URL structure flat while providing logical separation.

## Tokens (app/assets/css/main.css)

The underlying color primitives (`--color-signal-*`, `--color-graphite-*`) remain unchanged from plan 003, as they already provide a strong foundation for a dark-mode-first aesthetic.

- **Background**: `#0B0E14` (neutral-900 / graphite-900)
- **Surface**: `#151A23` (neutral-800 / graphite-800)
- **Accent**: `#262D3A` (neutral-700 / graphite-700)
- **Primary Signal**: `#21a8f0` (signal-500 in dark mode)
