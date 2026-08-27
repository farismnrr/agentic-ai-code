ALTER TABLE "ai_code"."conversations" ALTER COLUMN "mode" SET DEFAULT 'chat';--> statement-breakpoint
UPDATE "ai_code"."conversations"
SET "permission_mode" = 'manual'
WHERE "permission_mode" IN ('workspace', 'autonomous');--> statement-breakpoint
UPDATE "ai_code"."conversations"
SET "mode" = 'chat'
WHERE "mode" = 'agent'
  AND NOT ("enabled_tool_ids" ? 'native.local_terminal');
