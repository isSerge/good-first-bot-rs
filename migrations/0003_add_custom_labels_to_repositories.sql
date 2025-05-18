-- Add migration script here
ALTER TABLE repositories
ADD COLUMN tracked_labels TEXT;

-- Backfill existing repositories
UPDATE repositories
SET tracked_labels = '["good first issue","beginner-friendly","help wanted"]'
WHERE tracked_labels IS NULL;
