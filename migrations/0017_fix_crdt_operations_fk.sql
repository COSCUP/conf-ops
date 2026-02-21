-- Drop the foreign key constraint on crdt_operations.created_by.
-- The column stores member IDs (not account IDs) and Uuid::nil() for compaction snapshots,
-- so a FK to accounts(id) is incorrect.
ALTER TABLE crdt_operations DROP CONSTRAINT crdt_operations_created_by_fkey;
