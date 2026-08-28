CREATE TABLE "ai_code"."task_notification_outbox" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"source" varchar(16) NOT NULL,
	"task_id" varchar(128) NOT NULL,
	"title" varchar(160) NOT NULL,
	"summary" varchar(2000) NOT NULL,
	"completed_at" timestamp with time zone NOT NULL,
	"result_url" varchar(2048),
	"status" text DEFAULT 'pending' NOT NULL,
	"attempts" integer DEFAULT 0 NOT NULL,
	"next_attempt_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_error" varchar(64),
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	"sent_at" timestamp with time zone,
	CONSTRAINT "task_notification_source_task_unique" UNIQUE("source","task_id")
);
--> statement-breakpoint
CREATE INDEX "task_notification_delivery_idx" ON "ai_code"."task_notification_outbox" USING btree ("status","next_attempt_at","created_at");