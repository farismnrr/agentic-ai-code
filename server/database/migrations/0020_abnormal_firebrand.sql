CREATE TABLE "ai_code"."relay_activity_sources" (
	"id" uuid PRIMARY KEY NOT NULL,
	"user_id" uuid NOT NULL,
	"device_id" uuid,
	"label" varchar(80) NOT NULL,
	"kind" varchar(32) NOT NULL,
	"token_hash" text NOT NULL,
	"token_prefix" varchar(16) NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	CONSTRAINT "relay_activity_sources_token_hash_unique" UNIQUE("token_hash")
);
--> statement-breakpoint
CREATE TABLE "ai_code"."relay_activity_workspace_bindings" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"source_id" uuid NOT NULL,
	"workspace_id" uuid NOT NULL,
	"root_fingerprint" varchar(128) NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "relay_activity_binding_unique" UNIQUE("source_id","root_fingerprint")
);
--> statement-breakpoint
CREATE TABLE "ai_code"."workspace_activity" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"source_id" uuid NOT NULL,
	"activity_id" varchar(160) NOT NULL,
	"workspace_id" uuid NOT NULL,
	"source_sequence" integer NOT NULL,
	"contract_version" varchar(24) NOT NULL,
	"actor" varchar(80) NOT NULL,
	"channel" varchar(40) NOT NULL,
	"tool" varchar(80) NOT NULL,
	"category" varchar(40) NOT NULL,
	"effects" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"status" text NOT NULL,
	"target" varchar(512) NOT NULL,
	"started_at" timestamp with time zone NOT NULL,
	"finished_at" timestamp with time zone,
	"duration_ms" integer,
	"change_evidence" jsonb,
	"occurred_at" timestamp with time zone NOT NULL,
	"ingested_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "workspace_activity_source_activity_unique" UNIQUE("source_id","activity_id")
);
--> statement-breakpoint
CREATE TABLE "ai_code"."workspace_activity_payloads" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"activity_id" uuid NOT NULL,
	"payload_kind" varchar(40) NOT NULL,
	"payload_version" varchar(24) NOT NULL,
	"encrypted_envelope" text NOT NULL,
	"checksum" varchar(128) NOT NULL,
	"byte_count" integer NOT NULL,
	"complete" boolean DEFAULT true NOT NULL,
	"chunk_index" integer DEFAULT 0 NOT NULL,
	"chunk_count" integer DEFAULT 1 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "workspace_activity_payload_unique" UNIQUE("activity_id","payload_kind","chunk_index")
);
--> statement-breakpoint
ALTER TABLE "ai_code"."relay_activity_sources" ADD CONSTRAINT "relay_activity_sources_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "ai_code"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."relay_activity_sources" ADD CONSTRAINT "relay_activity_sources_device_id_user_devices_id_fk" FOREIGN KEY ("device_id") REFERENCES "ai_code"."user_devices"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."relay_activity_workspace_bindings" ADD CONSTRAINT "relay_activity_workspace_bindings_source_id_relay_activity_sources_id_fk" FOREIGN KEY ("source_id") REFERENCES "ai_code"."relay_activity_sources"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."relay_activity_workspace_bindings" ADD CONSTRAINT "relay_activity_workspace_bindings_workspace_id_workspaces_id_fk" FOREIGN KEY ("workspace_id") REFERENCES "ai_code"."workspaces"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ADD CONSTRAINT "workspace_activity_source_id_relay_activity_sources_id_fk" FOREIGN KEY ("source_id") REFERENCES "ai_code"."relay_activity_sources"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity" ADD CONSTRAINT "workspace_activity_workspace_id_workspaces_id_fk" FOREIGN KEY ("workspace_id") REFERENCES "ai_code"."workspaces"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."workspace_activity_payloads" ADD CONSTRAINT "workspace_activity_payloads_activity_id_workspace_activity_id_fk" FOREIGN KEY ("activity_id") REFERENCES "ai_code"."workspace_activity"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "relay_activity_sources_user_idx" ON "ai_code"."relay_activity_sources" USING btree ("user_id","created_at");--> statement-breakpoint
CREATE INDEX "relay_activity_binding_workspace_idx" ON "ai_code"."relay_activity_workspace_bindings" USING btree ("workspace_id");--> statement-breakpoint
CREATE INDEX "workspace_activity_cursor_idx" ON "ai_code"."workspace_activity" USING btree ("workspace_id","started_at","id");--> statement-breakpoint
CREATE INDEX "workspace_activity_source_sequence_idx" ON "ai_code"."workspace_activity" USING btree ("source_id","source_sequence");