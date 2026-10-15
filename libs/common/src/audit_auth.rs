//! Authenticate direct HTTP clients before handlers run and pass the verified
//! actor to the operation logger. Local service and monarch calls remain local.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::api_types::{AppError, ErrorInfo};
use crate::operation_log::{AuditActor, decode_header_value};

#[derive(Clone)]
pub struct AuditAuth {
    client: reqwest::Client,
    validate_url: String,
}

impl AuditAuth {
    pub fn from_env() -> anyhow::Result<Arc<Self>> {
        let gateway =
            std::env::var("APIGATEWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:6005".to_owned());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()?;
        Ok(Arc::new(Self {
            client,
            validate_url: format!("{}/api/v1/auth/validate", gateway.trim_end_matches('/')),
        }))
    }

    async fn validate(&self, token: &str) -> Result<(AuditActor, String), Response> {
        let response = self
            .client
            .get(&self.validate_url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!("JWT validation service unavailable: {}", error);
                AppError::service_unavailable("Authentication service unavailable").into_response()
            })?;

        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => return Err(auth_error(StatusCode::UNAUTHORIZED)),
            reqwest::StatusCode::FORBIDDEN => return Err(auth_error(StatusCode::FORBIDDEN)),
            reqwest::StatusCode::OK => {},
            status => {
                tracing::warn!(%status, "Unexpected JWT validation response");
                return Err(
                    AppError::service_unavailable("Authentication service unavailable")
                        .into_response(),
                );
            },
        }

        let headers = response.headers();
        let actor = headers
            .get("x-voltage-audit-user-id")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|user_id| *user_id > 0)
            .zip(
                headers
                    .get("x-voltage-audit-username")
                    .and_then(|value| value.to_str().ok()),
            )
            .map(|(user_id, username)| AuditActor {
                user_id,
                username: decode_header_value(username),
            })
            .ok_or_else(|| {
                tracing::warn!("JWT validation response missing identity");
                AppError::service_unavailable("Authentication service unavailable").into_response()
            })?;
        let role = headers
            .get("x-voltage-audit-role")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        Ok((actor, role))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Access {
    Public,
    Viewer,
    Engineer,
    Admin,
}

fn access_for(method: &Method, path: &str) -> Access {
    if method == Method::OPTIONS
        || matches!(
            path,
            "/" | "/health" | "/ping" | "/hisApi/health" | "/netApi/health"
        )
        || path == "/hisApi/metrics"
        || path == "/docs"
        || path.starts_with("/docs/")
        || path == "/openapi.json"
    {
        Access::Public
    } else if path.starts_with("/api/admin/") {
        Access::Admin
    } else if matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    ) {
        Access::Engineer
    } else {
        Access::Viewer
    }
}

fn is_local_call(req: &Request) -> bool {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|peer| peer.0.ip().is_loopback())
        && !req.headers().contains_key("x-forwarded-for")
        && !req.headers().contains_key("x-real-ip")
}

fn auth_error(status: StatusCode) -> Response {
    let message = if status == StatusCode::FORBIDDEN {
        "Insufficient permissions"
    } else {
        "Missing or invalid access token"
    };
    AppError::new(status, ErrorInfo::new(message).with_code(status.as_u16())).into_response()
}

/// Apply only to the five backend HTTP servers. The apigateway already has its
/// own JWT middleware. This layer must be inside `http_request_logger` so the
/// logger also records rejected write attempts.
pub async fn authenticate_operation(
    State(auth): State<Arc<AuditAuth>>,
    req: Request,
    next: Next,
) -> Response {
    let access = access_for(req.method(), req.uri().path());
    if access == Access::Public {
        return next.run(req).await;
    }

    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        })
        .filter(|value| !value.is_empty());
    let Some(token) = token else {
        return if is_local_call(&req) {
            next.run(req).await
        } else {
            auth_error(StatusCode::UNAUTHORIZED)
        };
    };
    let (actor, role) = match auth.validate(token).await {
        Ok(verified) => verified,
        Err(response) => return response,
    };
    let allowed = match access {
        Access::Public | Access::Viewer => true,
        Access::Engineer => role == "Engineer" || role == "Admin",
        Access::Admin => role == "Admin",
    };
    if !allowed {
        let mut response = auth_error(StatusCode::FORBIDDEN);
        response.extensions_mut().insert(actor);
        return response;
    }

    let mut response = next.run(req).await;
    response.extensions_mut().insert(actor);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        routing::{get, post},
    };

    #[test]
    fn access_policy_keeps_health_open_and_requires_roles() {
        assert_eq!(access_for(&Method::GET, "/health"), Access::Public);
        assert_eq!(access_for(&Method::GET, "/api/instances"), Access::Viewer);
        assert_eq!(
            access_for(&Method::POST, "/api/instances"),
            Access::Engineer
        );
        assert_eq!(
            access_for(&Method::GET, "/api/admin/logs/files"),
            Access::Admin
        );
    }

    #[test]
    fn proxy_requests_do_not_get_local_access() {
        let mut request = Request::builder()
            .uri("/api/instances")
            .body(axum::body::Body::empty())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
        ));
        assert!(is_local_call(&request));
        request
            .headers_mut()
            .insert("x-forwarded-for", "192.0.2.10".parse().unwrap());
        assert!(!is_local_call(&request));
    }

    #[tokio::test]
    async fn direct_client_is_verified_before_a_write() {
        let gateway_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_addr = gateway_listener.local_addr().unwrap();
        let gateway = Router::new().route(
            "/api/v1/auth/validate",
            get(|headers: axum::http::HeaderMap| async move {
                let token = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                match token {
                    "Bearer engineer" => (
                        StatusCode::OK,
                        [
                            ("x-voltage-audit-user-id", "7"),
                            ("x-voltage-audit-username", "alice"),
                            ("x-voltage-audit-role", "Engineer"),
                        ],
                    )
                        .into_response(),
                    "Bearer viewer" => (
                        StatusCode::OK,
                        [
                            ("x-voltage-audit-user-id", "8"),
                            ("x-voltage-audit-username", "bob"),
                            ("x-voltage-audit-role", "Viewer"),
                        ],
                    )
                        .into_response(),
                    _ => StatusCode::UNAUTHORIZED.into_response(),
                }
            }),
        );
        let gateway_task = tokio::spawn(axum::serve(gateway_listener, gateway).into_future());

        let auth = Arc::new(AuditAuth {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            validate_url: format!("http://{gateway_addr}/api/v1/auth/validate"),
        });
        let service_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let service_addr = service_listener.local_addr().unwrap();
        let service = Router::new()
            .route("/api/test", post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                auth,
                authenticate_operation,
            ))
            .layer(axum::middleware::from_fn(
                |req: Request, next: Next| async move {
                    let mut response = next.run(req).await;
                    if let Some(user_id) = response
                        .extensions()
                        .get::<AuditActor>()
                        .map(|actor| actor.user_id)
                    {
                        response
                            .headers_mut()
                            .insert("x-test-user-id", user_id.to_string().parse().unwrap());
                    }
                    response
                },
            ));
        let service_task = tokio::spawn(
            axum::serve(
                service_listener,
                service.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .into_future(),
        );
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let url = format!("http://{service_addr}/api/test");
        let request = |token: Option<&str>| {
            let mut request = client.post(&url).header("x-forwarded-for", "192.0.2.1");
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
            request
        };

        let no_token = request(None)
            .header("x-voltage-audit-user-id", "999")
            .header("x-voltage-audit-username", "forged")
            .send()
            .await
            .unwrap();
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
        assert!(no_token.headers().get("x-test-user-id").is_none());

        let viewer = request(Some("viewer")).send().await.unwrap();
        assert_eq!(viewer.status(), StatusCode::FORBIDDEN);
        assert_eq!(viewer.headers()["x-test-user-id"], "8");

        let engineer = request(Some("engineer")).send().await.unwrap();
        assert_eq!(engineer.status(), StatusCode::OK);
        assert_eq!(engineer.headers()["x-test-user-id"], "7");

        let local_tool = client.post(&url).send().await.unwrap();
        assert_eq!(local_tool.status(), StatusCode::OK);
        assert!(local_tool.headers().get("x-test-user-id").is_none());

        let local_client = client
            .post(&url)
            .bearer_auth("engineer")
            .send()
            .await
            .unwrap();
        assert_eq!(local_client.status(), StatusCode::OK);
        assert_eq!(local_client.headers()["x-test-user-id"], "7");

        gateway_task.abort();
        service_task.abort();
    }
}
