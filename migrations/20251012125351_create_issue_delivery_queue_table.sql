CREATE TABLE IF NOT EXISTS issue_delivery_queue(
newsletter_issue_id UUID NOT NULL REFERENCES newsletter_issues(id),
user_email TEXT NOT NULL,
PRIMARY KEY (newsletter_issue_id, user_email)
);

