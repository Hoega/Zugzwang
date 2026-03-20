-- Add email column to users
ALTER TABLE users ADD COLUMN email TEXT;

-- Set email for existing users
UPDATE users SET email = username || '@zugzwang.local' WHERE email IS NULL;

-- Make email NOT NULL and UNIQUE
ALTER TABLE users ALTER COLUMN email SET NOT NULL;
ALTER TABLE users ADD CONSTRAINT users_email_key UNIQUE (email);

-- Remove default user (existing repertoires will be deleted by CASCADE... but there's no CASCADE on repertoires FK)
-- Instead, delete repertoires owned by default user first, then the user
DELETE FROM repertoires WHERE user_id = '00000000-0000-0000-0000-000000000001';
DELETE FROM users WHERE id = '00000000-0000-0000-0000-000000000001';
