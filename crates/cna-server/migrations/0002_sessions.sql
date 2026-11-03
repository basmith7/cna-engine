-- Sign-in sessions (tower-sessions records). account_id is copied out
-- of the record's data so suspending an account can delete its sessions.

CREATE TABLE sessions (
    id text PRIMARY KEY,
    account_id bigint REFERENCES accounts (id) ON DELETE CASCADE,
    data jsonb NOT NULL,
    expiry_date timestamptz NOT NULL
);

CREATE INDEX sessions_account ON sessions (account_id);
