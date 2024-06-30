use crate::generated::login_attempts::{self, Entity as LoginAttemptsEntity};
use anyhow::{anyhow, Result};
use bld_migrations::Expr;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::fmt::Display;
use tracing::{debug, error};

pub use crate::generated::login_attempts::Model as LoginAttempt;

pub struct InsertLoginAttempt {
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

#[derive(Debug, Clone)]
pub enum LoginAttemptStatus {
    Active,
    Failed,
    Completed,
}

impl Display for LoginAttemptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginAttemptStatus::Active => write!(f, "active"),
            LoginAttemptStatus::Failed => write!(f, "failed"),
            LoginAttemptStatus::Completed => write!(f, "completed"),
        }
    }
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
    let date_expires = utc_now + Duration::minutes(10);
    let active_model = login_attempts::ActiveModel {
        csrf_token: Set(model.csrf_token),
        nonce: Set(model.nonce),
        pkce_verifier: Set(model.pkce_verifier),
        status: Set(LoginAttemptStatus::Active.to_string()),
        date_created: Set(utc_now.naive_utc()),
        date_expires: Set(date_expires.naive_utc()),
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

pub async fn update_status_by_csrf_token(
    conn: &DatabaseConnection,
    csrf_token: &str,
    status: LoginAttemptStatus,
) -> Result<()> {
    debug!("updating the status of login attempt with csrf_token: {csrf_token}");

    LoginAttemptsEntity::update_many()
        .col_expr(
            login_attempts::Column::Status,
            Expr::value(status.to_string()),
        )
        .col_expr(
            login_attempts::Column::DateUpdated,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(login_attempts::Column::CsrfToken.eq(csrf_token))
        .exec(conn)
        .await
        .map(|_| {
            debug!("updated login attempt status successfully");
        })
        .map_err(|e| {
            error!("could not update login attempt status due to: {e}");
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
        .filter(login_attempts::Column::DateExpires.lt(Utc::now().naive_utc()))
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
