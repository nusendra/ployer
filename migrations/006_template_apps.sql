-- Service template installs: store rendered compose + template origin on the application
ALTER TABLE applications ADD COLUMN compose_content TEXT;
ALTER TABLE applications ADD COLUMN template_slug TEXT;
