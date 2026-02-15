-- Webhooks
CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    provider TEXT NOT NULL, -- 'github' or 'gitlab'
    secret TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(application_id)
);

-- Webhook Deliveries (for tracking webhook attempts)
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    event_type TEXT NOT NULL, -- 'push', 'pull_request', etc.
    branch TEXT,
    commit_sha TEXT,
    commit_message TEXT,
    author TEXT,
    status TEXT NOT NULL, -- 'success', 'failed', 'skipped'
    response_code INTEGER,
    error_message TEXT,
    deployment_id TEXT REFERENCES deployments(id),
    delivered_at TEXT NOT NULL DEFAULT (datetime('now'))
);
