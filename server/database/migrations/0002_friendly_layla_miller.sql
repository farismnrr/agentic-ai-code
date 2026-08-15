CREATE TABLE "ai_code"."workspaces" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "ai_code"."workspaces" ADD CONSTRAINT "workspaces_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "ai_code"."users"("id") ON DELETE cascade ON UPDATE no action;
--> statement-breakpoint
ALTER TABLE "ai_code"."conversations" ADD COLUMN "workspace_id" uuid;
--> statement-breakpoint
INSERT INTO "ai_code"."workspaces" (id, user_id, name) SELECT gen_random_uuid(), id, 'Personal' FROM "ai_code"."users";
--> statement-breakpoint
UPDATE "ai_code"."conversations" c SET workspace_id = w.id FROM "ai_code"."workspaces" w WHERE c.user_id = w.user_id;
--> statement-breakpoint
ALTER TABLE "ai_code"."conversations" ALTER COLUMN "workspace_id" SET NOT NULL;
--> statement-breakpoint
ALTER TABLE "ai_code"."conversations" ADD CONSTRAINT "conversations_workspace_id_workspaces_id_fk" FOREIGN KEY ("workspace_id") REFERENCES "ai_code"."workspaces"("id") ON DELETE cascade ON UPDATE no action;