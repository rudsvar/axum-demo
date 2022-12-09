-- Drop and rename old columns
ALTER TABLE requests DROP COLUMN client;
ALTER TABLE requests DROP COLUMN server;

-- Add host column
ALTER TABLE requests ADD COLUMN host TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ALTER COLUMN host DROP DEFAULT;

-- Add method column
ALTER TABLE requests ADD COLUMN method TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ALTER COLUMN method DROP DEFAULT;
