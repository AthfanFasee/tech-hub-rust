CREATE TABLE IF NOT EXISTS issue_delivery_queue(
newsletter_issue_id UUID NOT NULL REFERENCES newsletter_issues(id),
user_email TEXT NOT NULL,
n_retries INT NOT NULL DEFAULT 0,
execute_after TIMESTAMPTZ NOT NULL DEFAULT NOW(),
PRIMARY KEY (newsletter_issue_id, user_email)
);

