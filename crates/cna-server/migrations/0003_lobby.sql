-- The lobby. `public`: listed in the open games (a private game is
-- joined only with its invite token, whose SHA-256 is invite_token_hash).
-- `creator_either`: the creator takes whichever side the joiner leaves,
-- so until someone joins they hold no seat. `clock`: the game's
-- cna_game Clock after its last order, for the lobby (it is public).

ALTER TABLE games
    ADD COLUMN public boolean NOT NULL DEFAULT true,
    ADD COLUMN creator_either boolean NOT NULL DEFAULT false,
    ADD COLUMN clock jsonb;
