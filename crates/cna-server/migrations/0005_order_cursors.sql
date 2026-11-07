-- Each order's cursor: a random name clients page and reconnect by, so
-- that no client sees `seq` and counts from it the orders its view hides.
ALTER TABLE orders ADD COLUMN cursor text NOT NULL DEFAULT gen_random_uuid()::text;
CREATE UNIQUE INDEX orders_cursor ON orders (game_id, cursor);
