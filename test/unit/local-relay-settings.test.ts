import { strict as assert } from 'node:assert'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { buildLocalRelayCommand, LOCAL_RELAY_BINARY, LOCAL_RELAY_PORT } from '../../shared/utils/local-relay.ts'

const root = resolve(import.meta.dirname, '../..')
const read = (path: string) => readFileSync(resolve(root, path), 'utf8')

const foreground = buildLocalRelayCommand({ origin: 'http://100.99.88.53:3333' })
assert.match(foreground, new RegExp(`\\./${LOCAL_RELAY_BINARY} relay`))
assert.match(foreground, /--mode local/)
assert.match(foreground, /--execution-root \$HOME/)
assert.match(foreground, /--origin http:\/\/100\.99\.88\.53:3333/)
assert.doesNotMatch(foreground, /--allow-terminal-network/)
assert.doesNotMatch(foreground, /--port 47821/)

const networked = buildLocalRelayCommand({
  origin: 'http://localhost:3333',
  allowTerminalNetwork: true,
  port: LOCAL_RELAY_PORT + 1
})
assert.match(networked, /--allow-terminal-network/)
assert.match(networked, /--port 47822/)

const background = buildLocalRelayCommand({ origin: 'http://localhost:3333', background: true })
assert.match(background, /^nohup /)
assert.ok(background.includes('--origin http://localhost:3333 \\\n  > relay-agent.log 2>&1 & disown'))

const relay = read('app/composables/useRelayAgent.ts')
assert.match(relay, /server\/discover/)
assert.match(relay, /supportedVersions/)
assert.match(relay, /if \(name\) headers\['mcp-name'\] = name/)
assert.doesNotMatch(relay, /startJob|terminal_job_start|terminal_job_get|terminal_job_cancel/)

const setup = read('app/components/settings/SettingsLocalRelaySetup.vue')
assert.match(setup, /Install relay/)
assert.match(setup, /Start relay/)
assert.match(setup, /Verify connection/)
assert.match(setup, /Changing this toggle|only changes the generated command|restart an already-running relay/i)
assert.doesNotMatch(setup, /\/api\/mcp-servers/)
assert.equal(existsSync(resolve(root, 'app/pages/settings/local-terminal.vue')), false)

console.log('local relay settings behavior: pass')
