import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '../..')
const read = (path: string) => readFileSync(resolve(root, path), 'utf8')

const createRoute = read('server/api/mcp-servers/index.post.ts')
assert.match(createRoute, /v\.strictObject/)
assert.match(createRoute, /v\.picklist\(\['http', 'sse'\]/)
assert.match(createRoute, /v\.maxLength\(2048/)
assert.doesNotMatch(createRoute, /status:|tools:|command:/)
assert.match(createRoute, /Unable to connect to MCP server/)

const updateRoute = read('server/api/mcp-servers/[id].put.ts')
assert.match(updateRoute, /v\.strictObject/)
assert.match(updateRoute, /enabled: v\.optional\(v\.boolean\(\)\)/)
assert.doesNotMatch(updateRoute, /status: v\.|tools: v\.|command: v\./)
assert.match(updateRoute, /Unable to verify MCP server changes/)

const scanRoute = read('server/api/mcp-servers/scan.post.ts')
assert.match(scanRoute, /application\.mcp\.scanServer/)
assert.doesNotMatch(scanRoute, /createServer|updateServer|deleteServer/)
assert.match(scanRoute, /Unable to connect to MCP server/)

const management = read('server/infrastructure/mcp/server-management.ts')
assert.match(management, /createMcpClient/)
assert.match(management, /createVerifiedMcpServer/)
assert.match(management, /discoverStoredTools\(userId, id, config\)/)
assert.match(management, /createMcpServer\(userId, \{ \.\.\.config, id, tools \}\)/)
assert.match(management, /status: 'error', enabled: false, tools: \[\]/)
assert.match(management, /connectionChanged/)
assert.match(management, /status: 'connected'/)

const database = read('server/infrastructure/database/mcp-servers.ts')
assert.match(database, /const unsupported = server\.transport === 'stdio'/)
assert.match(database, /enabled: unsupported \? false : server\.enabled/)
assert.match(database, /tools: unsupported \? \[\] : parseTools/)
assert.match(database, /status: 'connected'/)

const serverConfig = read('server/infrastructure/mcp/server-config.ts')
assert.match(serverConfig, /ne\(mcpServers\.transport, 'stdio'\)/)

const client = read('server/infrastructure/mcp/client.ts')
assert.match(client, /assertSafeUrl/)
assert.match(client, /createSsrfSafeFetch/)
assert.match(client, /resolveFirstPartyRemote/)
assert.match(client, /transport === 'stdio'/)

const inbound = read('server/api/mcp/index.ts')
assert.match(inbound, /enum: \['http', 'sse'\]/)
assert.doesNotMatch(inbound.match(/name: 'create_mcp_server'[\s\S]*?name: 'delete_mcp_server'/)?.[0] ?? '', /command:|status:/)

console.log('MCP server management contract: pass')
