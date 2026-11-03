//! Sign-in with Discord: OAuth2 authorization code with PKCE and
//! `state`, scopes `identify guilds.join`. The first sign-in makes the
//! account; each one refreshes its name and avatar and rotates the
//! session id. With the bot configured and the account's alerts on, it
//! also adds the player to the bot's guild (Discord lets a bot DM only
//! users it shares a server with); that failing does not fail sign-in.
//! Turning alerts off later removes no one: leaving the guild is the
//! player's.

use super::store::ACCOUNT;
use crate::App;
use crate::accounts;
use crate::games::AccountId;
use axum::Router;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_sessions::{Expiry, Session};

/// The session key holding a sign-in in progress.
const PENDING: &str = "oauth";

/// How long a sign-in in progress lives unused: the session behind it
/// gets the full `SESSION_DAYS` only once it is signed in, so sign-in
/// starts nobody finishes do not pile up for a month.
const PENDING_MINUTES: i64 = 10;

pub const SCOPES: [&str; 2] = ["identify", "guilds.join"];

pub fn routes() -> Router<App> {
    Router::new()
        .route("/auth/discord", get(start))
        .route("/auth/discord/callback", get(callback))
        .route("/auth/sign-out", post(sign_out))
}

#[derive(Serialize, Deserialize)]
struct Pending {
    state: String,
    verifier: String,
}

fn found(to: &str) -> Response {
    (StatusCode::FOUND, [(header::LOCATION, to.to_owned())]).into_response()
}

fn page(status: StatusCode, title: &str, text: &str) -> Response {
    (
        status,
        Html(format!(
            "<!doctype html><title>{title}</title><h1>{title}</h1><p>{text}</p><p><a href=\"/\">Back</a></p>"
        )),
    )
        .into_response()
}

fn failed(e: impl std::fmt::Display) -> Response {
    tracing::error!("sign-in: {e}");
    page(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Sign-in failed",
        "Something went wrong. Try again.",
    )
}

/// 302 to Discord's consent page, the `state` and PKCE verifier kept in
/// the session; 429 past the per-IP limit.
async fn start(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    session: Session,
) -> Response {
    let ip = crate::limits::client_ip(&headers, peer, app.auth.trust_proxy);
    if !app.limits.sign_in(ip) {
        return page(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many sign-ins",
            "Too many sign-ins from here. Wait a minute and try again.",
        );
    }
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state) = app
        .auth
        .oauth
        .authorize_url(CsrfToken::new_random)
        .add_scopes(SCOPES.map(|s| Scope::new(s.to_owned())))
        .set_pkce_challenge(challenge)
        .url();
    let pending = Pending {
        state: state.secret().clone(),
        verifier: verifier.secret().clone(),
    };
    session.set_expiry(Some(Expiry::OnInactivity(time::Duration::minutes(
        PENDING_MINUTES,
    ))));
    if let Err(e) = session.insert(PENDING, pending).await {
        return failed(e);
    }
    found(url.as_str())
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

/// Discord sends the player back here: the code is traded (with the
/// verifier) for a token, the token for the user, the user for an
/// account; then 302 to `/`, signed in.
async fn callback(
    State(app): State<App>,
    session: Session,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let pending: Option<Pending> = match session.remove(PENDING).await {
        Ok(p) => p,
        Err(e) => return failed(e),
    };
    let (Some(pending), Some(code)) = (pending, q.code) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in failed",
            "This sign-in was not started here or has expired. Try again.",
        );
    };
    if q.state.as_deref() != Some(pending.state.as_str()) {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in failed",
            "The sign-in's state does not match. Try again.",
        );
    }
    let token = match app
        .auth
        .oauth
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(pending.verifier))
        .request_async(&app.auth.http)
        .await
    {
        Ok(t) => t,
        Err(e) => return failed(format!("token exchange: {e}")),
    };
    let user = async {
        app.auth
            .http
            .get(format!("{}/users/@me", app.auth.discord_api))
            .bearer_auth(token.access_token().secret())
            .send()
            .await?
            .error_for_status()?
            .json::<DiscordUser>()
            .await
    }
    .await;
    let user = match user {
        Ok(u) => u,
        Err(e) => return failed(format!("users/@me: {e}")),
    };
    let name = user.global_name.as_deref().unwrap_or(&user.username);
    let (id, suspended) =
        match accounts::sign_in(app.games.pool(), &user.id, name, user.avatar.as_deref()).await {
            Ok(a) => a,
            Err(e) => return failed(e),
        };
    if suspended {
        return page(
            StatusCode::FORBIDDEN,
            "Account suspended",
            "This account is suspended and cannot sign in.",
        );
    }
    join_guild(&app, id, &user.id, token.access_token().secret()).await;
    if let Err(e) = session.cycle_id().await {
        return failed(e);
    }
    session.set_expiry(Some(Expiry::OnInactivity(time::Duration::days(
        super::SESSION_DAYS,
    ))));
    if let Err(e) = session.insert(ACCOUNT, id).await {
        return failed(e);
    }
    found("/")
}

/// Adds `discord_id` to the bot's guild with the user's `access_token`
/// (the `guilds.join` grant) if the bot is configured and `account` has
/// alerts on. Failures are logged, not returned: sign-in goes on.
async fn join_guild(app: &App, account: AccountId, discord_id: &str, access_token: &str) {
    let Some((bot_token, guild)) = &app.auth.bot else {
        return;
    };
    let alerts: Result<bool, _> = sqlx::query_scalar("SELECT alerts FROM accounts WHERE id = $1")
        .bind(account)
        .fetch_one(app.games.pool())
        .await;
    match alerts {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            tracing::warn!("sign-in: reading account {account}'s alerts: {e}");
            return;
        }
    }
    let r = app
        .auth
        .http
        .put(format!(
            "{}/guilds/{guild}/members/{discord_id}",
            app.auth.discord_api
        ))
        .header(header::AUTHORIZATION, format!("Bot {bot_token}"))
        .json(&serde_json::json!({ "access_token": access_token }))
        .send()
        .await
        .and_then(|r| r.error_for_status());
    if let Err(e) = r {
        tracing::warn!("sign-in: adding account {account} to the guild: {e}");
    }
}

/// Ends the caller's session: 204.
async fn sign_out(session: Session) -> Response {
    match session.flush().await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => failed(e),
    }
}
