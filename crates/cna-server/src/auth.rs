//! Who is calling. Slice 3 signs players in with Discord; until then the
//! only way in is the `test-auth` header, and a build without that
//! feature lets nobody in.

use crate::api::ApiError;
use crate::games::{AccountId, Games};
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;

#[cfg(feature = "test-auth")]
mod test_auth;

/// The signed-in caller's account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentAccount(pub AccountId);

impl FromRequestParts<Games> for CurrentAccount {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, games: &Games) -> Result<Self, ApiError> {
        #[cfg(feature = "test-auth")]
        if let Some(id) = test_auth::account(parts, games).await? {
            return Ok(CurrentAccount(id));
        }
        let _ = (parts, games);
        Err(ApiError::new(StatusCode::UNAUTHORIZED, "not signed in"))
    }
}
