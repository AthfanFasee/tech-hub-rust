CREATE TYPE header_pair AS (
name TEXT,
value BYTEA
);

CREATE TABLE IF NOT EXISTS idempotency (
user_id UUID NOT NULL REFERENCES users(id),
idempotency_key TEXT NOT NULL,
response_status_code SMALLINT,
response_headers header_pair[],
response_body BYTEA,
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
PRIMARY KEY(user_id, idempotency_key)
);

