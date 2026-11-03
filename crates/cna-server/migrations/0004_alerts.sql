-- The turn-alert outbox: a row is written in the transaction of the
-- order that made it `account_id`'s move in `game_id` (`seq` is that
-- order's), and the sender DMs it. `next_attempt_at` is when the sender
-- may try it next; `given_up_at` ends a row that is never to be sent
-- (DMs closed, or 24 hours of failures).

CREATE TABLE alerts (
    id bigserial PRIMARY KEY,
    account_id bigint NOT NULL REFERENCES accounts (id),
    game_id bigint NOT NULL REFERENCES games (id),
    seq bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    sent_at timestamptz,
    attempts integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    last_error text,
    given_up_at timestamptz
);

-- The sender's queue.
CREATE INDEX alerts_due ON alerts (next_attempt_at)
    WHERE sent_at IS NULL AND given_up_at IS NULL;
-- The 15-minute check at each order.
CREATE INDEX alerts_recent ON alerts (account_id, game_id, created_at);
