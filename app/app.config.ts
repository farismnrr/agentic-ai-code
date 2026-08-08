export default defineAppConfig({
  ui: {
    colors: {
      // `signal` is reserved for things that are actually happening —
      // streaming text, a running tool, a connected server, a focused
      // control. It is the product's only saturated colour, so if it starts
      // appearing on decoration the idea behind the palette is broken.
      primary: 'signal',
      // Cool grey with a blue cast, so the signal reads as the same family
      // rather than a sticker on top of a neutral.
      neutral: 'graphite'
    },
    dashboardPanel: {
      slots: {
        // Default theme sets `min-h-svh` on the root. That's fine standalone,
        // but app/layouts/default.vue nests this panel inside a flex-col
        // wrapper alongside the "verify your email" banner — a sibling that
        // eats real height above the panel. `min-h-svh` forces the panel to
        // claim a full viewport's worth of height regardless of what the
        // banner already consumed, so the panel (and its #footer, the chat
        // prompt) overflows past the bottom of the screen by exactly the
        // banner's height. `min-h-0` cancels that floor via tailwind-merge
        // so the panel's own `flex-1` sizing (fill the remaining flex space)
        // is what actually governs its height.
        root: 'min-h-0'
      }
    }
  }
})
