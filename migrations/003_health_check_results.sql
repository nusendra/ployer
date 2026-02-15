-- Health Check Results (for tracking health check history)
CREATE TABLE IF NOT EXISTS health_check_results (
    id TEXT PRIMARY KEY NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    container_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 'healthy', 'unhealthy', 'unknown'
    response_time_ms INTEGER,
    status_code INTEGER,
    error_message TEXT,
    checked_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying recent health checks by application
CREATE INDEX IF NOT EXISTS idx_health_check_results_app ON health_check_results(application_id, checked_at DESC);

-- Index for querying by container
CREATE INDEX IF NOT EXISTS idx_health_check_results_container ON health_check_results(container_id, checked_at DESC);

-- Container Stats (for storing aggregated metrics)
CREATE TABLE IF NOT EXISTS container_stats (
    id TEXT PRIMARY KEY NOT NULL,
    container_id TEXT NOT NULL,
    application_id TEXT,
    cpu_percent REAL NOT NULL,
    memory_mb REAL NOT NULL,
    memory_limit_mb REAL,
    network_rx_mb REAL,
    network_tx_mb REAL,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying stats by container and time
CREATE INDEX IF NOT EXISTS idx_container_stats_container ON container_stats(container_id, recorded_at DESC);

-- Index for querying stats by application
CREATE INDEX IF NOT EXISTS idx_container_stats_app ON container_stats(application_id, recorded_at DESC);
