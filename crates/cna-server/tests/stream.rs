//! The live channel, `GET /api/games/{id}/stream`, signed in with the
//! `test-auth` header. Needs `DATABASE_URL` and
//! `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::Router;
use axum::body::{Body, BodyDataStream};
use axum::http::{Request, StatusCode};
use cna_data::scenario::Side;
use cna_game::view::leaks;
use cna_server::games::{GameId, Games};
use common::*;
use futures_util::StreamExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;

const AXIS: &str = "1";
const CW: &str = "2";

async fn send(app: &Router, method: &str, uri: &str, who: &str, body: Option<Value>) -> Value {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-cna-test-account", who)
        .header("origin", PUBLIC_URL)
        .header("content-type", "application/json")
        .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "{method} {uri}");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Opens `uri` as `who`: its status, and its stream if it opened.
async fn open_uri(
    app: &Router,
    uri: &str,
    who: &str,
    last: Option<&str>,
) -> (StatusCode, Option<Sse>) {
    let mut req = Request::get(uri).header("x-cna-test-account", who);
    if let Some(l) = last {
        req = req.header("last-event-id", l);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let s = res.status();
    let body = s.is_success().then(|| Sse {
        body: res.into_body().into_data_stream(),
        buf: String::new(),
    });
    (s, body)
}

async fn open(
    app: &Router,
    id: GameId,
    who: &str,
    last: Option<&str>,
) -> (StatusCode, Option<Sse>) {
    open_uri(app, &format!("/api/games/{id}/stream"), who, last).await
}

struct Sse {
    body: BodyDataStream,
    buf: String,
}

impl Sse {
    /// The next message with an id: the id and the parsed data.
    /// Keep-alive comments are skipped.
    async fn next(&mut self) -> (i64, Value) {
        loop {
            if let Some(end) = self.buf.find("\n\n") {
                let frame: String = self.buf.drain(..end + 2).collect();
                let (mut id, mut data) = (None, String::new());
                for line in frame.lines() {
                    if let Some(v) = line.strip_prefix("id:") {
                        id = Some(v.trim().parse().unwrap());
                    } else if let Some(v) = line.strip_prefix("data:") {
                        data.push_str(v.trim_start());
                    }
                }
                if let Some(id) = id {
                    return (id, serde_json::from_str(&data).unwrap());
                }
                continue;
            }
            let chunk = tokio::time::timeout(Duration::from_secs(30), self.body.next())
                .await
                .expect("a message within 30 s")
                .expect("the stream stays open")
                .unwrap();
            self.buf.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// The next frame that names an event: its name and data, and
    /// whether it carried an id. Keep-alives do not extend the 30 s wait.
    async fn event(&mut self) -> (String, Value, bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(end) = self.buf.find("\n\n") {
                let frame: String = self.buf.drain(..end + 2).collect();
                let (mut name, mut data, mut id) = (None, String::new(), false);
                for line in frame.lines() {
                    if let Some(v) = line.strip_prefix("event:") {
                        name = Some(v.trim().to_string());
                    } else if let Some(v) = line.strip_prefix("data:") {
                        data.push_str(v.trim_start());
                    } else if line.starts_with("id:") {
                        id = true;
                    }
                }
                if let Some(name) = name {
                    return (name, serde_json::from_str(&data).unwrap(), id);
                }
                continue;
            }
            let chunk = tokio::time::timeout_at(deadline, self.body.next())
                .await
                .expect("an event within 30 s")
                .expect("the stream stays open")
                .unwrap();
            self.buf.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// Nothing more arrives within a short wait.
    async fn quiet(&mut self) {
        assert!(!self.buf.contains("data:"), "{}", self.buf);
        let r = tokio::time::timeout(Duration::from_millis(300), self.body.next()).await;
        assert!(r.is_err(), "a message nobody sent: {r:?}");
    }
}

fn who(side: Side) -> &'static str {
    if side == Side::Axis { AXIS } else { CW }
}

/// What `who` reads from `/events` for the orders after `after`, as one
/// flat list.
async fn events_since(app: &Router, id: GameId, who: &str, after: i64) -> Value {
    let v = send(
        app,
        "GET",
        &format!("/api/games/{id}/events?after={after}"),
        who,
        None,
    )
    .await;
    json!(
        v.as_array()
            .unwrap()
            .iter()
            .flat_map(|o| o["events"].as_array().unwrap().clone())
            .collect::<Vec<_>>()
    )
}

async fn order(app: &Router, id: GameId, i: usize) {
    let o = &golden().orders[i];
    let body = json!({"side": o.player, "order": o.order, "seq": i});
    send(
        app,
        "POST",
        &format!("/api/games/{id}/orders"),
        who(o.player),
        Some(body),
    )
    .await;
}

async fn cw_leaks(games: &Games, id: GameId, body: &Value) -> Vec<String> {
    let live = games.get(id).await.unwrap();
    leaks(&live.state, body, Side::Axis)
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn each_seat_streams_only_its_own_side_of_an_order(pool: PgPool) {
    let (games, id, _, _) = seated(&pool).await;
    let app = router(games.clone());
    let (s, axis) = open(&app, id, AXIS, None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, cw) = open(&app, id, CW, None).await;
    assert_eq!(s, StatusCode::OK);
    let (mut axis, mut cw) = (axis.unwrap(), cw.unwrap());

    // A few orders, so both sides' orders go out.
    for i in 0..4 {
        order(&app, id, i).await;
        for (stream, side) in [(&mut axis, Side::Axis), (&mut cw, Side::Cw)] {
            let (seq, msg) = stream.next().await;
            assert_eq!((seq, &msg["seq"]), (i as i64, &json!(i)), "order {i}");
            assert_eq!(
                msg["events"],
                events_since(&app, id, who(side), i as i64 - 1).await,
                "order {i}, {side:?}"
            );
            let game = send(&app, "GET", &format!("/api/games/{id}"), who(side), None).await;
            assert_eq!(msg["view"], game["view"], "order {i}, {side:?}");
            if side == Side::Cw {
                assert_eq!(cw_leaks(&games, id, &msg).await, Vec::<String>::new());
            }
        }
    }
    axis.quiet().await;
    cw.quiet().await;
    cna_server::accounts::insert(&pool, "3", "Outsider")
        .await
        .unwrap();
    assert_eq!(
        open(&app, id, "3", None).await.0,
        StatusCode::FORBIDDEN,
        "an outsider has no stream"
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_reconnecting_stream_first_catches_up(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 3).await;
    let app = router(games.clone());

    let (s, cw_stream) = open(&app, id, CW, Some("0")).await;
    assert_eq!(s, StatusCode::OK);
    let mut cw_stream = cw_stream.unwrap();
    let (seq, msg) = cw_stream.next().await;
    assert_eq!(seq, 2, "the last order's seq");
    assert_eq!(msg["events"], events_since(&app, id, CW, 0).await);
    assert_eq!(cw_leaks(&games, id, &msg).await, Vec::<String>::new());
    cw_stream.quiet().await;

    // `?after=` does the same; then both go on live.
    let (s, ax) = open_uri(&app, &format!("/api/games/{id}/stream?after=1"), AXIS, None).await;
    assert_eq!(s, StatusCode::OK);
    let mut ax = ax.unwrap();
    let (seq, msg) = ax.next().await;
    assert_eq!(seq, 2);
    assert_eq!(msg["events"], events_since(&app, id, AXIS, 1).await);
    order(&app, id, 3).await;
    assert_eq!(ax.next().await.0, 3);
    assert_eq!(cw_stream.next().await.0, 3);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn streams_are_capped_per_game_and_per_account(pool: PgPool) {
    let (games, id, axis, _) = seated(&pool).await;
    let app = router(games.clone());
    let mut held = Vec::new();
    for _ in 0..4 {
        let (s, b) = open(&app, id, AXIS, None).await;
        assert_eq!(s, StatusCode::OK);
        held.push(b.unwrap());
    }
    assert_eq!(
        open(&app, id, AXIS, None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let (s, other) = open(&app, id, CW, None).await;
    assert_eq!(s, StatusCode::OK, "the cap is the account's");
    drop(other);
    assert_eq!(
        games.evict_idle(Duration::ZERO),
        0,
        "a streamed game stays in memory however idle"
    );
    held.pop();
    let (s, b) = open(&app, id, AXIS, None).await;
    assert_eq!(s, StatusCode::OK, "a closed stream frees its place");
    held.push(b.unwrap());

    // 32 in all: 4 here and 4 on each of 7 more games, then no more.
    let mut more = Vec::new();
    for _ in 0..8 {
        let g = games
            .create_seeded(&golden().scenario, axis, false, golden().seed)
            .await
            .unwrap();
        games.seat(g, Side::Axis, axis).await.unwrap();
        more.push(g);
    }
    for &g in &more[..7] {
        for _ in 0..4 {
            let (s, b) = open(&app, g, AXIS, None).await;
            assert_eq!(s, StatusCode::OK);
            held.push(b.unwrap());
        }
    }
    assert_eq!(
        open(&app, more[7], AXIS, None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    drop(held);
    assert_eq!(open(&app, more[7], AXIS, None).await.0, StatusCode::OK);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_stream_hears_a_player_join_and_leave(pool: PgPool) {
    let games = Games::new(pool.clone());
    let axis = cna_server::accounts::insert(&pool, AXIS, "Axis")
        .await
        .unwrap();
    cna_server::accounts::insert(&pool, CW, "Commonwealth")
        .await
        .unwrap();
    let id = games
        .create_seeded(&golden().scenario, axis, false, golden().seed)
        .await
        .unwrap();
    games.seat(id, Side::Axis, axis).await.unwrap();
    let app = router(games.clone());
    let (s, axis_stream) = open(&app, id, AXIS, None).await;
    assert_eq!(s, StatusCode::OK, "the creator streams the open game");
    let mut axis_stream = axis_stream.unwrap();

    let join = json!({"side": Side::Cw});
    send(
        &app,
        "POST",
        &format!("/api/games/{id}/join"),
        CW,
        Some(join),
    )
    .await;
    let (name, data, id_set) = axis_stream.event().await;
    assert_eq!((name.as_str(), &data, id_set), ("seats", &json!({}), false));
    axis_stream.quiet().await;

    // The joiner streams too; the leave reaches the creator.
    let (s, cw_stream) = open(&app, id, CW, None).await;
    assert_eq!(s, StatusCode::OK);
    let mut cw_stream = cw_stream.unwrap();
    send(&app, "POST", &format!("/api/games/{id}/leave"), CW, None).await;
    let (name, _, id_set) = axis_stream.event().await;
    assert_eq!((name.as_str(), id_set), ("seats", false));
    assert_eq!(cw_stream.event().await.0, "seats");
    assert_eq!(status(&pool, id).await, "abandoned");
}
