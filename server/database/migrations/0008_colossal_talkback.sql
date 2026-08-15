CREATE TABLE "ai_code"."model_providers" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"type" text NOT NULL,
	"name" text NOT NULL,
	"base_url" text,
	"api_key_encrypted" text NOT NULL,
	"enabled" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "ai_code"."models" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"provider_id" uuid NOT NULL,
	"model_id" text NOT NULL,
	"label" text NOT NULL,
	"description" text DEFAULT '' NOT NULL,
	"icon" text DEFAULT 'i-lucide-sparkles' NOT NULL,
	"context_window" integer,
	"max_output_tokens" integer,
	"thinking_enabled" boolean,
	"thinking_min_tokens" integer,
	"thinking_max_tokens" integer,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ALTER COLUMN "default_model_id" SET DATA TYPE uuid USING NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ALTER COLUMN "default_model_id" DROP NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD COLUMN "default_context_window" integer DEFAULT 128000 NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD COLUMN "default_max_output_tokens" integer DEFAULT 8192 NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD COLUMN "default_thinking_enabled" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD COLUMN "default_thinking_min_tokens" integer DEFAULT 1024 NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD COLUMN "default_thinking_max_tokens" integer DEFAULT 8192 NOT NULL;--> statement-breakpoint
ALTER TABLE "ai_code"."model_providers" ADD CONSTRAINT "model_providers_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "ai_code"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."models" ADD CONSTRAINT "models_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "ai_code"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."models" ADD CONSTRAINT "models_provider_id_model_providers_id_fk" FOREIGN KEY ("provider_id") REFERENCES "ai_code"."model_providers"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_code"."user_settings" ADD CONSTRAINT "user_settings_default_model_id_models_id_fk" FOREIGN KEY ("default_model_id") REFERENCES "ai_code"."models"("id") ON DELETE set null ON UPDATE no action;