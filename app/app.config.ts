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
    }
  }
})
