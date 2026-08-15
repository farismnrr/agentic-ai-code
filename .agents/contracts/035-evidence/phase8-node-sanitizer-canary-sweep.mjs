import { redactSecrets, sanitizeAttributes, sanitizeMessage } from '/home/farismnrr/Projects/MasihAwam/ai-code/server/infrastructure/observability/sanitize.ts'

const RS = 'x7q9zK2'
const cases = {
  bearer: `Bearer CANARY-BEARER-${RS}`,
  basic: `Basic CANARY-BASIC-${RS}`,
  apikey_header: `x-api-key: CANARY-APIKEY-${RS}`,
  apikey_json: `{"apiKey":"CANARY-APIKEY-${RS}"}`,
  cookie: `Cookie: session=CANARY-COOKIE-${RS}`,
  password_field: `password=CANARY-PASSWORD-${RS}`,
  token_field: `token=CANARY-TOKEN-${RS}`,
  secret_field: `secret=CANARY-SECRET-${RS}`,
  jwt: `eyJhbGciOiJIUzI1NiJ9.CANARY-JWT-${RS}.sigCANARY${RS}`,
  db_conn: `postgres://user:CANARY-DBPASS-${RS}@dbhost:5432/mydb`,
  url_secret_query: `https://api.example.com/x?api_key=CANARY-QUERYKEY-${RS}&other=1`,
  fs_path: `/home/someuser/CANARY-PATH-${RS}/project/file.ts:12:3`,
  route_path_regression: `/api/auth/register`
}

console.log('=== redactSecrets() per-category ===')
for (const [name, input] of Object.entries(cases)) {
  const out = redactSecrets(input)
  const leaked = out.includes(`CANARY`) && !name.startsWith('route')
  console.log(`${name}: leaked=${leaked} out="${out}"`)
}

console.log('\n=== sanitizeAttributes (error.message field) ===')
for (const [name, input] of Object.entries(cases)) {
  const attrs = sanitizeAttributes({ 'error.message': input })
  console.log(`${name}: "${attrs['error.message']}"`)
}

console.log('\n=== sanitizeMessage ===')
console.log(sanitizeMessage(`failed auth Bearer CANARY-BEARER-${RS} for user`))
