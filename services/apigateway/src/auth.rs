use anyhow::{Result, anyhow};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db;
use crate::models::{RefreshTokenInfo, TokenResponse, UserWithRole};

// ── JWT Claims ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    pub auth_version: i64,
    pub exp: usize,
    pub iat: usize,
    #[serde(rename = "type")]
    pub token_type: String,
}

// ── Password Hashing ──────────────────────────────────────────────────────────

/// Hash a password (frontend sends MD5, we bcrypt the MD5 for storage).
pub fn hash_password(md5_password: &str) -> Result<String> {
    bcrypt::hash(md5_password, bcrypt::DEFAULT_COST)
        .map_err(|e| anyhow!("bcrypt hash failed: {}", e))
}

/// Verify a password against a stored bcrypt hash.
pub fn verify_password(md5_password: &str, hash: &str) -> bool {
    bcrypt::verify(md5_password, hash).unwrap_or(false)
}

// ── Token Creation ────────────────────────────────────────────────────────────

pub fn create_access_token(
    user: &UserWithRole,
    auth_version: i64,
    secret: &str,
    expire_minutes: i64,
) -> Result<String> {
    let now = Utc::now().timestamp() as usize;
    let exp = (Utc::now().timestamp() + expire_minutes * 60) as usize;

    let claims = Claims {
        user_id: user.id,
        username: user.username.clone(),
        role: Some(user.role.name_en.clone()),
        token_id: None,
        auth_version,
        exp,
        iat: now,
        token_type: "access".to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("JWT encode failed: {}", e))
}

/// Creates a refresh token and returns (token_string, token_id, token_info).
pub fn create_refresh_token(
    user: &UserWithRole,
    auth_version: i64,
    secret: &str,
    expire_days: i64,
) -> Result<(String, String, RefreshTokenInfo)> {
    let token_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    let exp = (now + expire_days * 86400) as usize;

    let claims = Claims {
        user_id: user.id,
        username: user.username.clone(),
        role: None,
        token_id: Some(token_id.clone()),
        auth_version,
        exp,
        iat: now as usize,
        token_type: "refresh".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("JWT encode failed: {}", e))?;

    let info = RefreshTokenInfo {
        user_id: user.id,
        username: user.username.clone(),
        expires_at: now + expire_days * 86400,
    };

    Ok((token, token_id, info))
}

pub fn create_token_pair(
    user: &UserWithRole,
    auth_version: i64,
    secret: &str,
    access_expire_minutes: i64,
    refresh_expire_days: i64,
) -> Result<(TokenResponse, String, RefreshTokenInfo)> {
    let access_token = create_access_token(user, auth_version, secret, access_expire_minutes)?;
    let (refresh_token, token_id, token_info) =
        create_refresh_token(user, auth_version, secret, refresh_expire_days)?;

    let response = TokenResponse {
        access_token,
        refresh_token,
        token_type: "bearer".to_string(),
        expires_in: access_expire_minutes * 60,
    };

    Ok((response, token_id, token_info))
}

// ── Token Verification ────────────────────────────────────────────────────────

/// Verifies an access token and returns the Claims on success.
pub fn verify_access_token(token: &str, secret: &str) -> Option<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .and_then(|data| {
        if data.claims.token_type == "access" {
            Some(data.claims)
        } else {
            None
        }
    })
}

/// Verify the JWT and ensure the account is still active and its authentication
/// version has not changed since this token was issued.
pub async fn validate_access_token(token: &str, secret: &str, pool: &SqlitePool) -> Option<Claims> {
    let claims = verify_access_token(token, secret)?;
    validate_current_claims(pool, claims).await
}

async fn validate_current_claims(pool: &SqlitePool, claims: Claims) -> Option<Claims> {
    let (is_active, auth_version) = db::get_user_auth_state(pool, claims.user_id).await.ok()??;

    (is_active && auth_version == claims.auth_version).then_some(claims)
}

/// Verifies a refresh token and returns the Claims on success.
pub fn verify_refresh_token(token: &str, secret: &str) -> Option<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .and_then(|data| {
        if data.claims.token_type == "refresh" {
            Some(data.claims)
        } else {
            None
        }
    })
}

pub async fn validate_refresh_token(
    token: &str,
    secret: &str,
    pool: &SqlitePool,
) -> Option<Claims> {
    let claims = verify_refresh_token(token, secret)?;
    validate_current_claims(pool, claims).await
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    const SECRET: &str = "test-secret";

    async fn setup_user() -> (SqlitePool, UserWithRole, i64) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::create_tables(&pool).await.unwrap();
        db::init_roles(&pool).await.unwrap();
        let user_id = db::create_user(&pool, "test-user", "password-hash", 3)
            .await
            .unwrap();
        let user = db::get_user_with_role(&pool, user_id)
            .await
            .unwrap()
            .unwrap();
        (pool, user, user_id)
    }

    #[tokio::test]
    async fn password_change_invalidates_existing_access_token() {
        let (pool, user, user_id) = setup_user().await;
        let access_token = create_access_token(&user, 0, SECRET, 30).unwrap();
        let (refresh_token, _, _) = create_refresh_token(&user, 0, SECRET, 7).unwrap();

        assert!(
            validate_access_token(&access_token, SECRET, &pool)
                .await
                .is_some()
        );
        assert!(
            validate_refresh_token(&refresh_token, SECRET, &pool)
                .await
                .is_some()
        );

        db::update_user_password(&pool, user_id, "new-password-hash")
            .await
            .unwrap();

        assert!(
            validate_access_token(&access_token, SECRET, &pool)
                .await
                .is_none()
        );
        assert!(
            validate_refresh_token(&refresh_token, SECRET, &pool)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn disabling_user_invalidates_existing_access_token() {
        let (pool, user, user_id) = setup_user().await;
        let token = create_access_token(&user, 0, SECRET, 30).unwrap();

        db::update_user_active(&pool, user_id, false).await.unwrap();

        assert!(validate_access_token(&token, SECRET, &pool).await.is_none());
    }
}
