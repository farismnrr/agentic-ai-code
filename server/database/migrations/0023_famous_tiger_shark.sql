ALTER TABLE "ai_code"."workspace_activity" ALTER COLUMN "activity_id" SET DATA TYPE varchar(256);--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ALTER COLUMN "actor" SET DATA TYPE varchar(256);--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ALTER COLUMN "actor_source" SET DATA TYPE varchar(256);--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ALTER COLUMN "tool" SET DATA TYPE varchar(256);--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ALTER COLUMN "target" SET DATA TYPE varchar(4096);