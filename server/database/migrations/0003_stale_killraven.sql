ALTER TABLE "ai_code"."workspaces" ADD COLUMN "path" text;--> statement-breakpoint
ALTER TABLE "ai_code"."workspaces" ADD COLUMN "path_confirmed" boolean DEFAULT true NOT NULL;--> statement-breakpoint
UPDATE "ai_code"."workspaces" SET "path" = '/workspace', "path_confirmed" = false;--> statement-breakpoint
ALTER TABLE "ai_code"."workspaces" ALTER COLUMN "path" SET NOT NULL;