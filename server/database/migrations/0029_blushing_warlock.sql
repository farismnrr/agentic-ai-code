ALTER TABLE "ai_code"."task_notification_outbox" ADD COLUMN "workspace" varchar(160) DEFAULT 'Workspace unavailable';
--> statement-breakpoint
UPDATE "ai_code"."task_notification_outbox" SET "workspace" = 'Workspace unavailable' WHERE "workspace" IS NULL;
--> statement-breakpoint
ALTER TABLE "ai_code"."task_notification_outbox" ALTER COLUMN "workspace" SET NOT NULL;
--> statement-breakpoint
ALTER TABLE "ai_code"."task_notification_outbox" ALTER COLUMN "workspace" DROP DEFAULT;
