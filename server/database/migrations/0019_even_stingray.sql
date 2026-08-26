CREATE INDEX "auth_sessions_user_idx" ON "ai_code"."auth_sessions" USING btree ("user_id","created_at");--> statement-breakpoint
CREATE INDEX "mfa_factors_user_idx" ON "ai_code"."mfa_factors" USING btree ("user_id","created_at");--> statement-breakpoint
CREATE INDEX "mfa_recovery_codes_user_idx" ON "ai_code"."mfa_recovery_codes" USING btree ("user_id","used_at");--> statement-breakpoint
CREATE INDEX "security_events_user_created_idx" ON "ai_code"."security_events" USING btree ("user_id","created_at");