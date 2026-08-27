CREATE TABLE "ai_code"."mcp_oauth_flows" (
	"state_hash" text PRIMARY KEY NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"description" text DEFAULT '' NOT NULL,
	"transport" text NOT NULL,
	"server_url" text NOT NULL,
	"redirect_uri" text NOT NULL,
	"authorization_server" text NOT NULL,
	"resource" text NOT NULL,
	"client_information_encrypted" text NOT NULL,
	"code_verifier_encrypted" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_servers" ADD COLUMN "oauth_authorization_server" text;--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_servers" ADD COLUMN "oauth_resource" text;--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_servers" ADD COLUMN "oauth_redirect_uri" text;--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_servers" ADD COLUMN "oauth_client_information_encrypted" text;--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_servers" ADD COLUMN "oauth_tokens_encrypted" text;--> statement-breakpoint
ALTER TABLE "ai_code"."mcp_oauth_flows" ADD CONSTRAINT "mcp_oauth_flows_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "ai_code"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "mcp_oauth_flows_user_idx" ON "ai_code"."mcp_oauth_flows" USING btree ("user_id","expires_at");