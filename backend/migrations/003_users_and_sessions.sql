-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
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

-- Insert default user (password: changeme)
INSERT INTO users (id, username, password_hash, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'default',
    '$argon2id$v=19$m=19456,t=2,p=1$EKhZgOG2wAY+i4dUmV/6xw$G2nKzxje8vjk4zHlsjWgzWGbD9BQPEEKUMaT8cR8Z94',
    NOW()
);

-- Migrate existing repertoires to default user
UPDATE repertoires SET user_id = '00000000-0000-0000-0000-000000000001' WHERE user_id IS NULL;

-- Make user_id NOT NULL
ALTER TABLE repertoires ALTER COLUMN user_id SET NOT NULL;

-- Index for user lookups
CREATE INDEX idx_repertoires_user_id ON repertoires(user_id);
