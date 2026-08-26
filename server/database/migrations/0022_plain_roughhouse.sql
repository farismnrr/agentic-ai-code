ALTER TABLE "ai_code"."relay_activity_sources" ADD COLUMN "source_key" varchar(128);--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ADD COLUMN "actor_source" varchar(80);--> statement-breakpoint
ALTER TABLE "ai_code"."relay_activity_sources" ADD CONSTRAINT "relay_activity_sources_source_key_unique" UNIQUE("source_key");--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ADD CONSTRAINT "workspace_activity_source_sequence_unique" UNIQUE("source_id","source_sequence");