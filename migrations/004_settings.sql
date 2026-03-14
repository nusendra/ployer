CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Default: registration is open
INSERT OR IGNORE INTO settings (key, value) VALUES ('allow_registration', 'true');
