CREATE INDEX IF NOT EXISTS posts_title_idx ON posts USING GIN (to_tsvector('simple', title));
CREATE INDEX IF NOT EXISTS posts_created_by_idx ON posts USING btree (created_by);
CREATE INDEX IF NOT EXISTS comments_post_id_idx ON comments USING btree (post_id);
CREATE INDEX IF NOT EXISTS tokens_user_id_idx ON tokens USING btree (user_id);
CREATE INDEX IF NOT EXISTS users_username_activated_idx ON users USING btree (user_name) WHERE is_activated = true;
CREATE INDEX IF NOT EXISTS issue_delivery_queue_execute_after_idx ON issue_delivery_queue USING btree (execute_after);
CREATE INDEX IF NOT EXISTS idempotency_created_at_idx ON idempotency USING btree (created_at);
CREATE INDEX IF NOT EXISTS newsletter_issues_created_at_idx ON newsletter_issues USING btree (created_at);

