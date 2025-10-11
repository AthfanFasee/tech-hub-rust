CREATE TYPE header_pair AS (
name TEXT,
value BYTEA
);

CREATE TABLE idempotency (
user_id uuid NOT NULL REFERENCES users(id),
idempotency_key TEXT NOT NULL,
response_status_code SMALLINT NOT NULL,
response_headers header_pair[] NOT NULL,
response_body BYTEA NOT NULL,
"created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
PRIMARY KEY(user_id, idempotency_key)
);

