//! Who is calling: a session cookie (`cna_session`) set by the Discord
//! sign-in in `discord`, or, in the server's own tests, the `test-auth`
//! header.

use crate::App;
use crate::api::ApiError;
use crate::games::AccountId;
use anyhow::{Context, Result};
use axum::Router;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl};
use std::collections::BTreeSet;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, Session, SessionManagerLayer};

pub mod discord;
pub mod store;
#[cfg(feature = "test-auth")]
mod test_auth;

pub use store::PgStore;

/// The session cookie's name.
pub const COOKIE: &str = "cna_session";
/// How long a session lasts unused.
pub const SESSION_DAYS: i64 = 30;

pub type OAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

/// The sign-in: Discord's OAuth2 client, the session store, the admins.
pub struct Auth {
    /// Where players reach the server, without a trailing `/`: the OAuth
    /// redirect's base and the one `Origin` state changes are taken from.
    pub public_url: String,
    pub oauth: OAuthClient,
    /// Calls Discord; never follows redirects (the oauth2 crate's advice).
    pub http: reqwest::Client,
    pub discord_api: String,
    pub dev: bool,
    pub trust_proxy: bool,
    pub admins: BTreeSet<String>,
    pub store: PgStore,
    /// The bot token and the guild id players join at sign-in, when
    /// both are configured (alerts are off otherwise).
    pub bot: Option<(String, String)>,
}

impl Auth {
    pub fn new(config: &crate::Config, pool: sqlx::PgPool) -> Result<Auth> {
        let public_url = match (&config.public_url, config.dev) {
            (Some(u), _) => u.trim_end_matches('/').to_owned(),
            (None, true) => "http://localhost:3240".to_owned(),
            (None, false) => anyhow::bail!("CNA_PUBLIC_URL is not set (and CNA_DEV is not 1)"),
        };
        let client_id = config
            .discord_client_id
            .clone()
            .context("CNA_DISCORD_CLIENT_ID is not set")?;
        let api = config.discord_api.trim_end_matches('/').to_owned();
        let mut oauth = BasicClient::new(ClientId::new(client_id))
            .set_auth_uri(
                AuthUrl::new(config.discord_authorize.clone()).context("CNA_DISCORD_AUTHORIZE")?,
            )
            .set_token_uri(TokenUrl::new(format!("{api}/oauth2/token")).context("CNA_DISCORD_API")?)
            .set_redirect_uri(
                RedirectUrl::new(format!("{public_url}/auth/discord/callback"))
                    .context("CNA_PUBLIC_URL")?,
            );
        if let Some(secret) = &config.discord_client_secret {
            oauth = oauth.set_client_secret(ClientSecret::new(secret.clone()));
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("building the HTTP client")?;
        Ok(Auth {
            public_url,
            oauth,
            http,
            discord_api: api,
            dev: config.dev,
            trust_proxy: config.trust_proxy,
            admins: config.admins.clone(),
            store: PgStore::new(pool),
            bot: config
                .discord_bot_token
                .clone()
                .zip(config.discord_guild_id.clone()),
        })
    }

    /// The tower-sessions layer: cookie `cna_session`, `HttpOnly`,
    /// `SameSite=Lax`, `Secure` unless in dev, 30 days unused.
    pub fn session_layer(&self) -> SessionManagerLayer<PgStore> {
        SessionManagerLayer::new(self.store.clone())
            .with_name(COOKIE)
            .with_http_only(true)
            .with_same_site(SameSite::Lax)
            .with_secure(!self.dev)
            .with_expiry(Expiry::OnInactivity(time::Duration::days(SESSION_DAYS)))
    }
}

/// `/auth/…`: the sign-in and sign-out.
pub fn routes() -> Router<App> {
    discord::routes()
}

/// The signed-in caller's account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentAccount(pub AccountId);

fn unauthorized() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "not signed in")
}

impl FromRequestParts<App> for CurrentAccount {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, ApiError> {
        #[cfg(feature = "test-auth")]
        if let Some(id) = test_auth::account(parts, &app.games).await? {
            return Ok(CurrentAccount(id));
        }
        let session = Session::from_request_parts(parts, app)
            .await
            .map_err(|(_, e)| ApiError::internal(e))?;
        let Some(id) = session
            .get::<AccountId>(store::ACCOUNT)
            .await
            .map_err(ApiError::internal)?
        else {
            return Err(unauthorized());
        };
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM accounts WHERE id = $1 AND suspended_at IS NULL)",
        )
        .bind(id)
        .fetch_one(app.games.pool())
        .await?;
        if !active {
            session.flush().await.map_err(ApiError::internal)?;
            return Err(unauthorized());
        }
        Ok(CurrentAccount(id))
    }
}
