-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sessions table
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_token ON sessions(token);

-- Add user_id to repertoires
ALTER TABLE repertoires ADD COLUMN user_id UUID REFERENCES users(id);

-- Delete any orphaned repertoires (no user to assign them to)
DELETE FROM repertoires WHERE user_id IS NULL;

-- Make user_id NOT NULL
ALTER TABLE repertoires ALTER COLUMN user_id SET NOT NULL;

-- Index for user lookups
CREATE INDEX idx_repertoires_user_id ON repertoires(user_id);
