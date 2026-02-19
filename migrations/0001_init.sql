-- Enable required PostgreSQL extensions
-- Note: uuid-ossp is NOT used; UUID v7 is generated at the application layer
-- pgcrypto provides gen_random_uuid() and cryptographic functions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
