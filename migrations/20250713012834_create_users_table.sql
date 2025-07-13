CREATE TABLE IF NOT EXISTS "users" (
id uuid NOT NULL,
PRIMARY KEY (id),
"created_at"     timestamptz NOT NULL DEFAULT NOW(),
"name"           text NOT NULL,
"email"          text UNIQUE NOT NULL,
"password_hash"  bytea,
"activated"      BOOLEAN NOT NULL DEFAULT TRUE,
"is_subscribed"  BOOLEAN NOT NULL DEFAULT FALSE,
"version"        integer NOT NULL DEFAULT 1
);

