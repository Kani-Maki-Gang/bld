use crate::generated::login_attempts::{self, Entity as LoginAttemptsEntity};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use tracing::{debug, error};

pub use crate::generated::login_attempts::Model as LoginAttempt;

pub struct InsertLoginAttempt {
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

pub async fn select_by_csrf_token(
    conn: &DatabaseConnection,
    csrf_token: &str,
) -> Result<LoginAttempt> {
    debug!("loading login attempt with csrf_token: {csrf_token} from the database");

    let model = LoginAttemptsEntity::find()
        .filter(login_attempts::Column::CsrfToken.eq(csrf_token))
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load login attempt due to {e}");
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load login attempt due to not found");
            anyhow!("login attempt not found")
        })
        .map(|p| {
            debug!("loaded login attempt successfully");
            p
        })
}

pub async fn insert(conn: &DatabaseConnection, model: InsertLoginAttempt) -> Result<()> {
    debug!("inserting login attemp to the database");

    let utc_now = Utc::now();
    let expires_at = utc_now + Duration::minutes(10);
    let active_model = login_attempts::ActiveModel {
        csrf_token: Set(model.csrf_token),
        nonce: Set(model.nonce),
        pkce_verifier: Set(model.pkce_verifier),
        created_at: Set(utc_now.naive_utc()),
        expires_at: Set(expires_at.naive_utc()),
        ..Default::default()
    };

    active_model
        .insert(conn)
        .await
        .map(|_| {
            debug!("created new login attempt entry successfully");
        })
        .map_err(|e| {
            error!("could not insert login attempt due to: {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_csrf_token(conn: &DatabaseConnection, csrf_token: &str) -> Result<()> {
    debug!("deleting login attempt with csrf_token: {csrf_token}");

    LoginAttemptsEntity::delete_many()
        .filter(login_attempts::Column::CsrfToken.eq(csrf_token))
        .exec(conn)
        .await
        .map(|_| {
            debug!("deleted login attempt successfully");
        })
        .map_err(|e| {
            error!("could not delete login attempt due to: {e}");
            anyhow!(e)
        })
}

pub async fn delete_expired(conn: &DatabaseConnection) -> Result<()> {
    debug!("deleting expired login attempts");

    LoginAttemptsEntity::delete_many()
        .filter(login_attempts::Column::ExpiresAt.lt(Utc::now().naive_utc()))
        .exec(conn)
        .await
        .map(|_| {
            debug!("deleted expired login attempts successfully");
        })
        .map_err(|e| {
            error!("could not delete expired login attempts due to: {e}");
            anyhow!(e)
        })
}
