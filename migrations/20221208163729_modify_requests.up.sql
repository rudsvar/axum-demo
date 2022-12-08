-- Drop and rename old columns
ALTER TABLE requests DROP COLUMN client;
ALTER TABLE requests DROP COLUMN server;
ALTER TABLE requests ADD COLUMN host TEXT NOT NULL;
ALTER TABLE requests ADD COLUMN method TEXT NOT NULL DEFAULT 'N/A';
ALTER TABLE requests ALTER COLUMN method DROP DEFAULT;
