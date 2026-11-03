//! `/admin`: the accounts list, suspension and restore, for the admins
//! (the Discord ids in `CNA_ADMINS`).

use crate::App;
use crate::api::ApiError;
use crate::auth::CurrentAccount;
use crate::games::AccountId;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/accounts", get(list))
        .route("/accounts/{id}/suspend", post(suspend))
        .route("/accounts/{id}/restore", post(restore))
}

/// A signed-in admin; 403 for anyone else.
pub struct Admin;

impl FromRequestParts<App> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Admin, ApiError> {
        let CurrentAccount(id) = CurrentAccount::from_request_parts(parts, app).await?;
        let discord_id: String =
            sqlx::query_scalar("SELECT discord_id FROM accounts WHERE id = $1")
                .bind(id)
                .fetch_one(app.games.pool())
                .await?;
        if app.auth.admins.contains(&discord_id) {
            Ok(Admin)
        } else {
            Err(ApiError::new(StatusCode::FORBIDDEN, "admins only"))
        }
    }
}

/// Every account: `[{id, discord_id, name, admin, suspended}]`, oldest first.
async fn list(State(app): State<App>, _: Admin) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(AccountId, String, String, bool)> = sqlx::query_as(
        "SELECT id, discord_id, name, suspended_at IS NOT NULL FROM accounts ORDER BY id",
    )
    .fetch_all(app.games.pool())
    .await?;
    Ok(Json(json!(
        rows.into_iter()
            .map(|(id, discord_id, name, suspended)| json!({
                "id": id,
                "admin": app.auth.admins.contains(&discord_id),
                "discord_id": discord_id,
                "name": name,
                "suspended": suspended,
            }))
            .collect::<Vec<_>>()
    )))
}

async fn set_suspended(app: &App, id: AccountId, suspended: bool) -> Result<(), ApiError> {
    let r = sqlx::query(
        "UPDATE accounts SET suspended_at = CASE WHEN $2 THEN coalesce(suspended_at, now()) END \
         WHERE id = $1",
    )
    .bind(id)
    .bind(suspended)
    .execute(app.games.pool())
    .await?;
    if r.rows_affected() == 0 {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "no such account"));
    }
    Ok(())
}

/// Suspends an account: its sessions are deleted and its open streams
/// closed; its games stay as they are.
async fn suspend(
    State(app): State<App>,
    _: Admin,
    Path(id): Path<AccountId>,
) -> Result<Json<Value>, ApiError> {
    set_suspended(&app, id, true).await?;
    app.auth.store.delete_account(id).await?;
    app.games.streams().close_account(id);
    Ok(Json(json!({ "id": id, "suspended": true })))
}

/// Lifts a suspension: the account may sign in again.
async fn restore(
    State(app): State<App>,
    _: Admin,
    Path(id): Path<AccountId>,
) -> Result<Json<Value>, ApiError> {
    set_suspended(&app, id, false).await?;
    Ok(Json(json!({ "id": id, "suspended": false })))
}
