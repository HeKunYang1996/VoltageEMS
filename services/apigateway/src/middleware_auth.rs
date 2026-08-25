//! Axum middleware that enforces JWT auth on protected routes.
//!
//! Accepts Bearer tokens in the `Authorization` header for REST calls.
//! WebSocket upgrades cannot set custom headers from a browser, so the
//! middleware also accepts `?token=...` as a query-string fallback.
//!
//! `require_loopback` is a separate, lighter guard for internal service-to-
//! service endpoints (e.g. `/broadcast`). All VoltageEMS services run with
//! host networking on the same machine, so checking that the peer is 127.0.0.1
//! is sufficient — external clients cannot reach the loopback address.

use std::net::IpAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::auth::validate_access_token;
use crate::state::AppState;

fn extract_bearer(req: &Request) -> Option<String> {
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(|s| s.to_string())
}

fn extract_query_token(req: &Request) -> Option<String> {
    // JWTs are base64url (A-Za-z0-9-_.) — URL-safe, no decoding needed.
    let q = req.uri().query()?;
    q.split('&')
        .find_map(|kv| kv.strip_prefix("token=").map(|s| s.to_string()))
}

pub async fn require_jwt(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let token = extract_bearer(&req).or_else(|| extract_query_token(&req));

    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    if validate_access_token(&token, &state.config.jwt_secret, &state.db)
        .await
        .is_none()
    {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    next.run(req).await
}

/// Middleware that requires Admin or Engineer role.
///
/// Viewer accounts receive 403. Used as a route-layer guard on write operations
/// in apigateway (homepage edits, etc.). For nginx auth_request integration the
/// equivalent is `GET /api/v1/auth/validate/engineer`.
pub async fn require_engineer(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let token = extract_bearer(&req).or_else(|| extract_query_token(&req));

    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    let Some(claims) = validate_access_token(&token, &state.config.jwt_secret, &state.db).await
    else {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    };

    let role = claims.role.as_deref().unwrap_or("");
    if role != "Admin" && role != "Engineer" {
        return (StatusCode::FORBIDDEN, "Engineer or Admin role required").into_response();
    }

    next.run(req).await
}

/// Middleware that requires Admin role only.
///
/// Non-admin accounts receive 403. Used as a route-layer guard on destructive or
/// system-level operations. For nginx auth_request integration the equivalent is
/// `GET /api/v1/auth/validate/admin`.
pub async fn require_admin_role(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let token = extract_bearer(&req).or_else(|| extract_query_token(&req));

    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    let Some(claims) = validate_access_token(&token, &state.config.jwt_secret, &state.db).await
    else {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    };

    if claims.role.as_deref() != Some("Admin") {
        return (StatusCode::FORBIDDEN, "Admin role required").into_response();
    }

    next.run(req).await
}

/// Guard for internal service-to-service endpoints (e.g. `/broadcast`).
///
/// Allows requests that originate from the loopback address (127.0.0.1 / ::1).
/// All VoltageEMS microservices run on the same host with host networking, so
/// loopback-only access is a sufficient boundary without requiring JWT tokens
/// on every internal call.
pub async fn require_loopback(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = addr.ip();
    let is_local = match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    };

    if !is_local {
        return (StatusCode::FORBIDDEN, "broadcast endpoint is internal-only").into_response();
    }

    next.run(req).await
}
