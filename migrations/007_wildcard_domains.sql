-- Wildcard / custom-domain support.
-- When set, the domain is served as a wildcard (*.<domain>) plus the apex,
-- so an app can host unbounded tenant subdomains behind a single route.
ALTER TABLE domains ADD COLUMN wildcard INTEGER NOT NULL DEFAULT 0;
