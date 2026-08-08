import { sql } from 'drizzle-orm'
import { drizzle } from 'drizzle-orm/postgres-js'
import postgres from 'postgres'
import 'dotenv/config'

async function backfill() {
  const url = process.env.NUXT_DATABASE_URL
  if (!url) {
    console.error('NUXT_DATABASE_URL is not set')
    process.exit(1)
  }

  const client = postgres(url)
  const db = drizzle(client)

  console.log('Backfilling defaultModelId in ai_code.user_settings...')
  await db.execute(sql`UPDATE ai_code.user_settings SET default_model_id = 'vx/gemini-3-flash-preview' WHERE default_model_id = 'legacy-model-id';`)

  console.log('Backfilling model_id in ai_code.conversations...')
  await db.execute(sql`UPDATE ai_code.conversations SET model_id = 'vx/gemini-3-flash-preview' WHERE model_id = 'legacy-model-id';`)

  console.log('Done.')
  await client.end()
}

backfill().catch(console.error)
