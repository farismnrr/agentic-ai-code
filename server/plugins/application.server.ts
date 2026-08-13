import { createApplicationAdapters } from '../infrastructure/composition/application'

export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('request', (event) => {
    event.context.application = createApplicationAdapters()
  })
})
