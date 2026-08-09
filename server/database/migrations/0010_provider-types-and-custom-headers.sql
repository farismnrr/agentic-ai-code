ALTER TABLE "ai_code"."model_providers" ADD COLUMN "custom_headers" jsonb DEFAULT '{}'::jsonb NOT NULL;--> statement-breakpoint
UPDATE "ai_code"."model_providers" SET "type" = 'openai_compatible' WHERE "type" = '9router';--> statement-breakpoint
UPDATE "ai_code"."model_providers" SET "type" = 'vertex_ai' WHERE "type" = 'gcp_agent_platform';