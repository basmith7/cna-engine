//! The game routes over HTTP, signed in with the `test-auth` header.
//! Needs `DATABASE_URL` and `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::Router;
use axum::http::StatusCode;
use cna_data::scenario::Side;
use cna_game::view::leaks;
use cna_server::accounts;
use cna_server::games::{GameId, Games};
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

/// Discord ids of the seated accounts `seated` makes, and an outsider's.
const AXIS: &str = "1";
const CW: &str = "2";
const OUTSIDER: &str = "3";

/// What a Commonwealth answer `body` says that 3.61 hides about the Axis.
async fn cw_leaks(games: &Games, id: GameId, body: &Value) -> Vec<String> {
    let live = games.get(id).await.unwrap();
    leaks(&live.state, body, Side::Axis)
}

/// The golden game seated as in `common::seated`, with its router and
/// an outsider's account.
async fn app(pool: &PgPool) -> (Router, Games, GameId) {
    let (games, id, _, _) = seated(pool).await;
    accounts::insert(pool, OUTSIDER, "Outsider").await.unwrap();
    (router_for_play(games.clone()), games, id)
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_game_is_made_listed_and_joined_once_per_seat(pool: PgPool) {
    for (d, n) in [("10", "A"), ("11", "B"), ("12", "C")] {
        accounts::insert(&pool, d, n).await.unwrap();
    }
    let app = router(Games::new(pool.clone()));
    let (s, _) = call(
        &app,
        "POST",
        "/api/games",
        None,
        Some(json!({"side": "axis"})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, "GET", "/api/games?mine", Some("99"), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "an unknown account");

    let (s, made) = call(
        &app,
        "POST",
        "/api/games",
        Some("10"),
        Some(json!({"side": "axis"})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{made}");
    let id = made["id"].as_i64().unwrap();
    let (_, open) = call(&app, "GET", "/api/games?open", Some("11"), None).await;
    assert_eq!(open[0]["id"], id, "{open}");
    assert_eq!(open[0]["seats"], json!({"axis": "A", "cw": null}));
    let (_, open) = call(&app, "GET", "/api/games?open", Some("10"), None).await;
    assert_eq!(open, json!([]), "not one's own game");
    let (_, mine) = call(&app, "GET", "/api/games?mine", Some("10"), None).await;
    assert_eq!(mine[0]["id"], id);
    let (s, _) = call(&app, "GET", "/api/games", Some("10"), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let join = format!("/api/games/{id}/join");
    let (s, _) = call(
        &app,
        "POST",
        &join,
        Some("11"),
        Some(json!({"side": "axis"})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "the Axis seat is held");
    let (s, b) = call(&app, "POST", &join, Some("11"), Some(json!({"side": "cw"}))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, _) = call(&app, "POST", &join, Some("12"), Some(json!({"side": "cw"}))).await;
    assert_eq!(s, StatusCode::CONFLICT, "the game is full");
    assert_eq!(status(&pool, id).await, "active");
    let (s, _) = call(
        &app,
        "POST",
        "/api/games/999999/join",
        Some("12"),
        Some(json!({"side": "cw"})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_game_is_shown_only_to_its_seats_each_its_own_view(pool: PgPool) {
    let (app, games, id) = app(&pool).await;
    let uri = format!("/api/games/{id}");
    let (s, axis) = call(&app, "GET", &uri, Some(AXIS), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(axis["side"], "axis");
    assert!(axis["version"].is_string(), "{axis}");
    assert!(axis.get("seq").is_none(), "no seq to count orders by");
    assert!(axis["view"]["pieces"].is_object(), "{}", axis["view"]);
    let (s, cw) = call(&app, "GET", &uri, Some(CW), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(cw["side"], "cw");
    assert_eq!(cw_leaks(&games, id, &cw).await, Vec::<String>::new());
    assert!(cw["view"].get("rng").is_none() && cw["view"].get("log").is_none());
    assert_ne!(axis["view"], cw["view"], "each side its own view");
    let (s, _) = call(&app, "GET", &uri, Some(OUTSIDER), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // No game at all looks like a game you are not in: an outsider
    // cannot tell real ids from missing ones, nor make us load them.
    let (s, _) = call(&app, "GET", "/api/games/999999", Some(AXIS), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn orders_come_only_from_their_seat_in_turn_and_current(pool: PgPool) {
    let (app, games, id) = app(&pool).await;
    let uri = format!("/api/games/{id}/orders");
    let log = golden();
    let first = json!({
        "side": "axis",
        "order": log.orders[0].order,
        "version": version(&app, id, AXIS).await,
    });
    let (s, _) = call(&app, "POST", &uri, Some(CW), Some(first.clone())).await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "the Commonwealth ordering as the Axis"
    );
    let (s, _) = call(&app, "POST", &uri, Some(OUTSIDER), Some(first.clone())).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, ok) = call(&app, "POST", &uri, Some(AXIS), Some(first.clone())).await;
    assert_eq!(s, StatusCode::OK, "{ok}");
    assert!(ok["ok"].is_array(), "{ok}");
    let (s, stale) = call(&app, "POST", &uri, Some(AXIS), Some(first)).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(stale["stale"], true, "{stale}");
    assert!(stale.get("seq").is_none(), "{stale}");

    let cw_order = &log
        .orders
        .iter()
        .find(|o| o.player == Side::Cw)
        .unwrap()
        .order;
    let (s, refused) = post_order(&app, id, CW, "cw", cw_order).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(refused["illegal"]["case"].is_string(), "{refused}");
    assert_eq!(cw_leaks(&games, id, &refused).await, Vec::<String>::new());
    let (s, _) = post_order(&app, id, AXIS, "axis", "nonsense").await;
    assert!(s.is_client_error(), "{s}");
}

/// The whole golden game over HTTP: after every order the Commonwealth's
/// view, its answers and finally its event stream say nothing 3.61
/// hides about the Axis, and the stream holds every order.
#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_commonwealth_learns_no_axis_secret_over_a_whole_game(pool: PgPool) {
    let (app, games, id) = app(&pool).await;
    let log = golden();
    let view = format!("/api/games/{id}");
    let save = format!("/api/games/{id}/save");
    let (s, _) = call(&app, "GET", &save, Some(AXIS), None).await;
    assert_eq!(s, StatusCode::CONFLICT, "no save while the game runs");
    for (i, o) in log.orders.iter().enumerate() {
        let who = if o.player == Side::Axis { AXIS } else { CW };
        let sealed_step = matches!(
            games.get(id).await.unwrap().state.pending,
            Some(cna_game::state::Pending::Sealed { .. })
        );
        if sealed_step {
            // Previewing a sealed answer would show the other side's.
            let (s, p) = call(
                &app,
                "POST",
                &format!("/api/games/{id}/preview"),
                Some(who),
                Some(json!({"order": o.order})),
            )
            .await;
            assert_eq!(
                (s, p),
                (StatusCode::OK, json!({"hidden": "sealed"})),
                "order {i}"
            );
        }
        let (s, answer) = post_order(&app, id, who, o.player, &o.order).await;
        assert_eq!(s, StatusCode::OK, "order {i}: {answer}");
        if o.player == Side::Cw {
            assert_eq!(
                cw_leaks(&games, id, &answer).await,
                Vec::<String>::new(),
                "order {i}"
            );
        }
        let (_, seen) = call(&app, "GET", &view, Some(CW), None).await;
        if games.get(id).await.unwrap().state.is_over() {
            // Over, the view hides nothing (`view_for`).
            assert_eq!(i + 1, log.orders.len(), "the game ends on its last order");
            continue;
        }
        assert_eq!(
            cw_leaks(&games, id, &seen).await,
            Vec::<String>::new(),
            "after order {i}"
        );
        let (_, sealed) = call(
            &app,
            "GET",
            &format!("/api/games/{id}/sealed-options"),
            Some(CW),
            None,
        )
        .await;
        assert_eq!(
            cw_leaks(&games, id, &sealed).await,
            Vec::<String>::new(),
            "after order {i}"
        );
    }
    let (s, stream) = call(
        &app,
        "GET",
        &format!("/api/games/{id}/events"),
        Some(CW),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rows = stream.as_array().unwrap();
    // A row for each Commonwealth order, and for each Axis order that
    // shows the Commonwealth something; none left empty to be counted.
    let cw_orders: Vec<String> = sqlx::query_scalar(
        "SELECT cursor FROM orders WHERE game_id = $1 AND side = 'cw' ORDER BY seq",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let cw_rows: Vec<_> = rows
        .iter()
        .filter(|r| r["side"] == "cw")
        .map(|r| r["cursor"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(cw_rows, cw_orders);
    assert!(
        rows.iter().all(|r| r.get("seq").is_none()),
        "no seq in a row"
    );
    assert!(
        rows.iter()
            .all(|r| r["side"] == "cw" || !r["events"].as_array().unwrap().is_empty()),
        "an empty Axis row"
    );
    assert!(rows.len() < log.orders.len(), "some Axis order is secret");
    assert_eq!(cw_leaks(&games, id, &stream).await, Vec::<String>::new());
    let at_455 = cursor_at(&pool, id, 455).await.unwrap();
    let (_, tail) = call(
        &app,
        "GET",
        &format!("/api/games/{id}/events?after={at_455}"),
        Some(CW),
        None,
    )
    .await;
    // Rebuilt from the snapshot at 450, the tail's rows are those the
    // replay from the seed gives.
    let later: Vec<String> =
        sqlx::query_scalar("SELECT cursor FROM orders WHERE game_id = $1 AND seq > 455")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
    let after: Vec<_> = rows
        .iter()
        .filter(|r| later.iter().any(|c| r["cursor"] == c.as_str()))
        .cloned()
        .collect();
    assert!(!after.is_empty());
    assert_eq!(tail, json!(after), "events after a snapshot");
    let (s, _) = call(
        &app,
        "GET",
        &format!("/api/games/{id}/events"),
        Some(OUTSIDER),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = call(
        &app,
        "GET",
        &format!("/api/games/{id}/events?after=455"),
        Some(CW),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "a seq is no cursor");
    let over = games.get(id).await.unwrap().state.is_over();
    let (s, _) = call(&app, "GET", &save, Some(CW), None).await;
    assert_eq!(
        s,
        if over {
            StatusCode::OK
        } else {
            StatusCode::CONFLICT
        }
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn nothing_the_other_side_reads_counts_secret_orders(pool: PgPool) {
    // Dummy tanks placed in phase C (16.42) are secret: the enemy's feed
    // shows no row for them, not an empty row each, and the game it
    // reads (its version too) is the same as before them. Graziani's
    // Axis is Italian, so the Commonwealth places them.
    let (games, id, axis, cw) = seated(&pool).await;
    let app = router_for_play(games.clone());
    let (at, hexes) = before_dummy_tanks();
    play(&games, id, axis, cw, at).await;
    let game = format!("/api/games/{id}");
    let (_, before) = call(&app, "GET", &game, Some(AXIS), None).await;
    let cw_before = version(&app, id, CW).await;
    for hex in &hexes {
        let order = cna_game::order::Order::PlaceDummyTank { hex: hex.clone() };
        let (s, answer) = post_order(&app, id, CW, "cw", order).await;
        assert_eq!(s, StatusCode::OK, "{answer}");
    }
    let (_, after) = call(&app, "GET", &game, Some(AXIS), None).await;
    assert_eq!(after, before, "the Axis's game changed with the dummies");
    assert_ne!(
        version(&app, id, CW).await,
        cw_before,
        "the Commonwealth's did"
    );
    let feed = format!(
        "/api/games/{id}/events?after={}",
        cursor_at(&pool, id, at as i64 - 1).await.unwrap()
    );
    let (_, seen) = call(&app, "GET", &feed, Some(AXIS), None).await;
    assert_eq!(seen, json!([]), "the Axis counts the dummies");
    let (_, own) = call(&app, "GET", &feed, Some(CW), None).await;
    let own = own.as_array().unwrap();
    assert_eq!(own.len(), 2, "{own:?}");
    for row in own {
        assert!(row["events"][0].get("dummy-tank-placed").is_some(), "{row}");
    }
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn queries_run_as_the_callers_seat(pool: PgPool) {
    let (app, games, id) = app(&pool).await;
    let (axis_piece, cw_piece) = {
        let live = games.get(id).await.unwrap();
        let of = |side| {
            live.state
                .pieces
                .iter()
                .find(|(_, p)| p.side == side)
                .map(|(id, _)| id.as_str().to_owned())
                .unwrap()
        };
        (of(Side::Axis), of(Side::Cw))
    };
    let placements = |piece: &str| format!("/api/games/{id}/placements?piece={}", urlencode(piece));
    let (s, hexes) = call(&app, "GET", &placements(&axis_piece), Some(AXIS), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(hexes.is_array(), "{hexes}");
    let (s, _) = call(&app, "GET", &placements(&axis_piece), Some(CW), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "another side's piece");
    let (s, _) = call(&app, "GET", &placements(&cw_piece), Some(CW), None).await;
    assert_eq!(s, StatusCode::OK);

    // The Axis is setting up: a Commonwealth stack cannot move, and
    // the refusal says nothing of the Axis.
    let pieces = serde_json::to_string(&[&cw_piece]).unwrap();
    let uri = format!(
        "/api/games/{id}/reachable?purpose=move&pieces={}",
        urlencode(&pieces)
    );
    let (s, r) = call(&app, "GET", &uri, Some(CW), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert_eq!(cw_leaks(&games, id, &r).await, Vec::<String>::new());
    let (s, _) = call(&app, "GET", &uri, Some(OUTSIDER), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Fuel sources: none for a side not to act; not for outsiders.
    let fuel = format!("/api/games/{id}/fuel-sources?pieces={}", urlencode(&pieces));
    let (s, f) = call(&app, "GET", &fuel, Some(CW), None).await;
    assert_eq!((s, f), (StatusCode::OK, json!({"ok": []})));
    let (s, _) = call(&app, "GET", &fuel, Some(OUTSIDER), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let axis_pieces = serde_json::to_string(&[&axis_piece]).unwrap();
    let fuel = format!(
        "/api/games/{id}/fuel-sources?pieces={}",
        urlencode(&axis_pieces)
    );
    let (s, _) = call(&app, "GET", &fuel, Some(CW), None).await;
    assert_eq!(
        s,
        StatusCode::UNPROCESSABLE_ENTITY,
        "a piece the caller cannot name"
    );

    let (s, sealed) = call(
        &app,
        "GET",
        &format!("/api/games/{id}/sealed-options"),
        Some(CW),
        None,
    )
    .await;
    assert_eq!(
        s,
        StatusCode::UNPROCESSABLE_ENTITY,
        "no sealed step yet: {sealed}"
    );

    // A preview answers as the order would and changes nothing.
    let first = &golden().orders[0].order;
    let preview = format!("/api/games/{id}/preview");
    let (s, p) = call(
        &app,
        "POST",
        &preview,
        Some(AXIS),
        Some(json!({"order": first})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert!(p["ok"].is_array() || p["hidden"] == "dice", "{p}");
    let (s, p) = call(
        &app,
        "POST",
        &preview,
        Some(CW),
        Some(json!({"order": first})),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::UNPROCESSABLE_ENTITY,
        "the Axis's order, from the Commonwealth: {p}"
    );
    assert_eq!(cw_leaks(&games, id, &p).await, Vec::<String>::new());
    assert_eq!(games.get(id).await.unwrap().seq(), 0);
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn alerts_are_switched_off_and_on_and_clear_unreachable(pool: PgPool) {
    accounts::insert(&pool, "10", "A").await.unwrap();
    let app = router(Games::new(pool.clone()));
    let (_, me) = call(&app, "GET", "/api/me", Some("10"), None).await;
    assert_eq!(
        (me["alerts"].clone(), me["alerts_unreachable"].clone()),
        (json!(true), json!(false))
    );

    sqlx::query("UPDATE accounts SET alerts_unreachable_at = now() WHERE discord_id = '10'")
        .execute(&pool)
        .await
        .unwrap();
    let (_, me) = call(&app, "GET", "/api/me", Some("10"), None).await;
    assert_eq!(me["alerts_unreachable"], json!(true));

    let (s, me) = call(
        &app,
        "PATCH",
        "/api/me",
        Some("10"),
        Some(json!({"alerts": false})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        (me["alerts"].clone(), me["alerts_unreachable"].clone()),
        (json!(false), json!(false))
    );
    let (_, me) = call(&app, "GET", "/api/me", Some("10"), None).await;
    assert_eq!(me["alerts"], json!(false), "the switch is kept");

    let (s, _) = call(
        &app,
        "PATCH",
        "/api/me",
        Some("10"),
        Some(json!({"alerts": "yes"})),
    )
    .await;
    assert!(s.is_client_error(), "a malformed body is refused: {s}");
    let (s, _) = call(
        &app,
        "PATCH",
        "/api/me",
        None,
        Some(json!({"alerts": true})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}
