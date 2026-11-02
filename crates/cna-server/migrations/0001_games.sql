-- Accounts, games, seats, the order log and snapshots. A game is its
-- scenario, ruleset, modules, seed and orders; snapshots only save
-- replay time and can always be rebuilt.

CREATE TABLE accounts (
    id bigserial PRIMARY KEY,
    discord_id text NOT NULL UNIQUE,
    name text NOT NULL,
    avatar text,
    -- is_admin is not stored: it is derived at runtime from config.
    suspended_at timestamptz,
    alerts boolean NOT NULL DEFAULT true,
    alerts_unreachable_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE games (
    id bigserial PRIMARY KEY,
    scenario text NOT NULL,
    ruleset jsonb NOT NULL,
    modules jsonb NOT NULL,
    open_information boolean NOT NULL DEFAULT false,
    -- The engine's u64 seed, bit-cast to bigint. Never sent to a client.
    seed bigint NOT NULL,
    -- 32 random bytes keying the supply-unit aliases; not derived from the seed.
    alias_key bytea NOT NULL CHECK (octet_length(alias_key) = 32),
    rules_commit text NOT NULL,
    created_by bigint REFERENCES accounts (id),
    status text NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'active', 'over', 'abandoned', 'stale')),
    to_act text CHECK (to_act IN ('axis', 'cw')),
    invite_token_hash bytea,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE seats (
    game_id bigint NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    side text NOT NULL CHECK (side IN ('axis', 'cw')),
    account_id bigint NOT NULL REFERENCES accounts (id),
    PRIMARY KEY (game_id, side),
    UNIQUE (game_id, account_id)
);

CREATE TABLE orders (
    game_id bigint NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    -- 0-based: the order's index in the game's log.
    seq bigint NOT NULL CHECK (seq >= 0),
    side text NOT NULL CHECK (side IN ('axis', 'cw')),
    "order" jsonb NOT NULL,
    at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, seq)
);

CREATE TABLE snapshots (
    game_id bigint NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    -- The number of orders applied: the state after orders 0..seq.
    seq bigint NOT NULL CHECK (seq > 0),
    rules_commit text NOT NULL,
    -- GameState::to_json, kept as text so it reads back byte for byte.
    state text NOT NULL,
    PRIMARY KEY (game_id, seq, rules_commit)
);
