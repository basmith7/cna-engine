//! A stand-in for Discord (the `fake-discord` binary, `fake` feature):
//! the OAuth2 endpoints the sign-in uses and the bot endpoints alerts
//! use, in memory, so tests and compose run the real flow offline. The
//! token endpoint checks the PKCE verifier against the challenge, so the
//! server's PKCE is tested rather than assumed.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Form, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand_core::{OsRng, RngCore};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// A test user: Discord id and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub name: String,
}

/// `FAKE_DISCORD_USERS`: `id:name,…`.
pub fn parse_users(s: &str) -> Result<Vec<User>, String> {
    s.split(',')
        .filter(|u| !u.trim().is_empty())
        .map(|u| match u.trim().split_once(':') {
            Some((id, name)) if !id.is_empty() && !name.is_empty() => Ok(User {
                id: id.to_owned(),
                name: name.to_owned(),
            }),
            _ => Err(format!("not id:name: {u:?}")),
        })
        .collect()
}

/// The users when `FAKE_DISCORD_USERS` is unset.
pub fn default_users() -> Vec<User> {
    parse_users("1001:alice,1002:bob").unwrap()
}

struct Grant {
    user: String,
    redirect_uri: String,
    challenge: String,
}

#[derive(Default)]
struct Inner {
    codes: BTreeMap<String, Grant>,
    tokens: BTreeMap<String, String>,
    members: Vec<Value>,
    sent: Vec<Value>,
}

#[derive(Clone)]
struct Fake {
    users: Arc<Vec<User>>,
    inner: Arc<Mutex<Inner>>,
}

/// The fake's routes over `users`.
pub fn router(users: Vec<User>) -> Router {
    let fake = Fake {
        users: Arc::new(users),
        inner: Arc::default(),
    };
    Router::new()
        .route("/oauth2/authorize", get(authorize))
        .route("/api/oauth2/token", post(token))
        .route("/api/users/@me", get(me))
        .route("/api/guilds/{guild}/members/{user}", put(add_member))
        .route("/api/users/@me/channels", post(open_dm))
        .route("/api/channels/{channel}/messages", post(send))
        .route("/_sent", get(sent))
        .route("/_members", get(members))
        .with_state(fake)
}

fn random() -> String {
    let mut b = [0u8; 16];
    OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

/// RFC 7636 S256: the challenge a verifier must hash to.
pub fn s256(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[derive(Deserialize)]
struct AuthorizeQuery {
    redirect_uri: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

/// The consent page: one link per user, each to `redirect_uri` with a
/// fresh code and the caller's `state`.
async fn authorize(State(fake): State<Fake>, Query(q): Query<AuthorizeQuery>) -> Response {
    if q.code_challenge_method != "S256" {
        return (
            StatusCode::BAD_REQUEST,
            "code_challenge_method must be S256",
        )
            .into_response();
    }
    let mut inner = fake.inner.lock().unwrap();
    let mut items = String::new();
    for u in fake.users.iter() {
        let code = random();
        inner.codes.insert(
            code.clone(),
            Grant {
                user: u.id.clone(),
                redirect_uri: q.redirect_uri.clone(),
                challenge: q.code_challenge.clone(),
            },
        );
        let mut to = url::Url::parse(&q.redirect_uri).expect("redirect_uri is a URL");
        to.query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", &q.state);
        items += &format!(
            "<li><a href=\"{}\">{}</a></li>",
            escape(to.as_str()),
            escape(&u.name)
        );
    }
    Html(format!(
        "<!doctype html><title>Fake Discord</title><h1>Sign in as</h1><ul>{items}</ul>"
    ))
    .into_response()
}

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: String,
    redirect_uri: String,
    code_verifier: Option<String>,
}

fn oauth_error(error: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
}

/// Trades a code for a token once, if the verifier matches its challenge.
async fn token(State(fake): State<Fake>, Form(f): Form<TokenForm>) -> Response {
    if f.grant_type != "authorization_code" {
        return oauth_error("unsupported_grant_type");
    }
    let mut inner = fake.inner.lock().unwrap();
    let Some(grant) = inner.codes.remove(&f.code) else {
        return oauth_error("invalid_grant");
    };
    if grant.redirect_uri != f.redirect_uri
        || f.code_verifier.as_deref().map(s256) != Some(grant.challenge)
    {
        return oauth_error("invalid_grant");
    }
    let access = random();
    inner.tokens.insert(access.clone(), grant.user);
    Json(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": 604800,
        "scope": "identify guilds.join",
    }))
    .into_response()
}

async fn me(State(fake): State<Fake>, headers: HeaderMap) -> Response {
    let user = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|t| fake.inner.lock().unwrap().tokens.get(t).cloned())
        .and_then(|id| fake.users.iter().find(|u| u.id == id).cloned());
    match user {
        Some(u) => Json(json!({
            "id": u.id,
            "username": u.name,
            "global_name": u.name,
            "avatar": null,
        }))
        .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "message": "401: Unauthorized" })),
        )
            .into_response(),
    }
}

async fn add_member(
    State(fake): State<Fake>,
    Path((guild, user)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> StatusCode {
    fake.inner.lock().unwrap().members.push(json!({
        "guild": guild,
        "user": user,
        "access_token": body.get("access_token").is_some(),
    }));
    StatusCode::CREATED
}

#[derive(Deserialize)]
struct OpenDm {
    recipient_id: String,
}

/// A DM channel's id is `dm-<user>`.
async fn open_dm(Json(b): Json<OpenDm>) -> Json<Value> {
    Json(json!({ "id": format!("dm-{}", b.recipient_id), "type": 1 }))
}

async fn send(
    State(fake): State<Fake>,
    Path(channel): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let content = body.get("content").cloned().unwrap_or(Value::Null);
    fake.inner.lock().unwrap().sent.push(json!({
        "channel": channel,
        "content": content,
    }));
    Json(json!({ "id": random(), "channel_id": channel }))
}

/// The DMs sent so far, oldest first.
async fn sent(State(fake): State<Fake>) -> Json<Value> {
    Json(Value::from(fake.inner.lock().unwrap().sent.clone()))
}

/// The guild joins made so far, oldest first.
async fn members(State(fake): State<Fake>) -> Json<Value> {
    Json(Value::from(fake.inner.lock().unwrap().members.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    const REDIRECT: &str = "http://cna.test/auth/discord/callback";

    async fn body(res: Response) -> String {
        let b = axum::body::to_bytes(res.into_body(), 1 << 16)
            .await
            .unwrap();
        String::from_utf8(b.to_vec()).unwrap()
    }

    /// The code the consent page offers the first user, for `verifier`.
    async fn code_for(app: &Router, verifier: &str) -> String {
        let mut url = url::Url::parse("http://fake/oauth2/authorize").unwrap();
        url.query_pairs_mut()
            .append_pair("redirect_uri", REDIRECT)
            .append_pair("state", "st")
            .append_pair("code_challenge", &s256(verifier))
            .append_pair("code_challenge_method", "S256");
        let res = app
            .clone()
            .oneshot(Request::get(url.as_str()).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let page = body(res).await;
        let href = page
            .split("href=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let link = url::Url::parse(&href.replace("&amp;", "&")).unwrap();
        assert!(link.as_str().starts_with(REDIRECT), "{link}");
        let q: BTreeMap<_, _> = link.query_pairs().into_owned().collect();
        assert_eq!(q["state"], "st");
        q["code"].clone()
    }

    async fn exchange(app: &Router, code: &str, verifier: &str) -> (StatusCode, Value) {
        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "authorization_code")
            .append_pair("code", code)
            .append_pair("redirect_uri", REDIRECT)
            .append_pair("code_verifier", verifier)
            .finish();
        let res = app
            .clone()
            .oneshot(
                Request::post("/api/oauth2/token")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        (status, serde_json::from_str(&body(res).await).unwrap())
    }

    #[tokio::test]
    async fn the_right_verifier_gets_a_token_for_the_user() {
        let app = router(default_users());
        let verifier = random();
        let code = code_for(&app, &verifier).await;
        let (status, tok) = exchange(&app, &code, &verifier).await;
        assert_eq!(status, StatusCode::OK, "{tok}");
        let res = app
            .clone()
            .oneshot(
                Request::get("/api/users/@me")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", tok["access_token"].as_str().unwrap()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let me: Value = serde_json::from_str(&body(res).await).unwrap();
        assert_eq!(
            (me["id"].as_str(), me["username"].as_str()),
            (Some("1001"), Some("alice"))
        );
    }

    #[tokio::test]
    async fn a_pkce_mismatch_is_refused() {
        let app = router(default_users());
        let code = code_for(&app, &random()).await;
        let (status, err) = exchange(&app, &code, &random()).await;
        assert_eq!(
            (status, err["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("invalid_grant"))
        );
    }

    #[tokio::test]
    async fn a_code_is_good_once() {
        let app = router(default_users());
        let verifier = random();
        let code = code_for(&app, &verifier).await;
        assert_eq!(exchange(&app, &code, &verifier).await.0, StatusCode::OK);
        assert_eq!(
            exchange(&app, &code, &verifier).await.0,
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn users_parse_and_bad_ones_are_refused() {
        assert_eq!(default_users()[1].name, "bob");
        assert!(parse_users("1001").is_err());
        assert!(parse_users("").unwrap().is_empty());
    }
}
