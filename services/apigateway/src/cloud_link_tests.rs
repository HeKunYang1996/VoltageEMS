use super::*;

use std::sync::Arc;

use axum::{Json, Router, http::HeaderMap, routing::post};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::mpsc;

const KEY: &str = "ABCD1234567890123456789WXYZ";
const OTHER_KEY: &str = "abcd1234567890123456789wxyz";
const MASK: &str = "ABCD*******************WXYZ";

fn store(directory: &TempDir) -> CloudLink {
    let mut link = CloudLink::new(directory.path().join(".env")).unwrap();
    // No test reads the host's serial or registers a real device.
    link.serial_path = directory.path().join("hardware-serial");
    link
}

fn write_config(link: &CloudLink, url: &str, key: &str, status: bool) {
    fs::write(
        &link.env_path,
        format!("# deployment settings\nJWT_SECRET_KEY=keep-me\nDEVICE_SN=00098888\nCLOUD_LINK_URL={url}\nCLOUD_LINK_KEY={key}\nCLOUD_LINK_STATUS={status}\n"),
    ).unwrap();
}

fn request(url: Option<&str>, key: Option<&str>) -> ActivateRequest {
    ActivateRequest {
        url: url.map(str::to_owned),
        key: key.map(str::to_owned),
    }
}

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(router: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Server { url, task }
}

async fn cloud(
    status: StatusCode,
    response: Value,
) -> (Server, mpsc::UnboundedReceiver<(HeaderMap, Value)>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let server = serve(Router::new().route(
        "/user-app/common/register",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let sender = sender.clone();
            let response = response.clone();
            async move {
                let _ = sender.send((headers, body));
                (status, Json(response))
            }
        }),
    ))
    .await;
    (server, receiver)
}

#[test]
fn initializes_missing_settings_without_resetting_other_values() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    fs::write(
        &link.env_path,
        "# preserve\nJWT_SECRET_KEY=unchanged\nDEVICE_SN=000123\nCLOUD_LINK_STATUS=true\n",
    )
    .unwrap();
    link.initialize().unwrap();
    let first = fs::read_to_string(&link.env_path).unwrap();
    assert!(first.contains("JWT_SECRET_KEY=unchanged"));
    let config = link.config().unwrap();
    assert_eq!(config.device_sn, "000123");
    assert_eq!(config.url, DEFAULT_URL);
    assert!(config.status);
    assert_eq!(config.key, "");
    link.initialize().unwrap();
    assert_eq!(fs::read_to_string(&link.env_path).unwrap(), first);
}

#[test]
fn initialization_preserves_existing_key_and_url_and_reads_current_file() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    write_config(&link, "https://cloud.example/prefix", KEY, true);
    let original = fs::read_to_string(&link.env_path).unwrap();
    link.initialize().unwrap();
    assert_eq!(fs::read_to_string(&link.env_path).unwrap(), original);
    assert_eq!(link.config().unwrap().key, MASK);
    write_config(&link, "https://new.example", OTHER_KEY, false);
    assert_eq!(link.config().unwrap().url, "https://new.example");
    assert!(!link.config().unwrap().status);
}

#[test]
fn keys_have_exact_length_charset_and_equal_length_masks() {
    assert_eq!(KEY.len(), 27);
    assert_eq!(mask_key(KEY), MASK);
    assert_eq!(MASK.len(), 27);
    assert_eq!(MASK.matches('*').count(), 19);
    assert!(valid_key("ABCD-_34567890123456789WXYZ"));
    for bad in [
        "short",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ12",
        "ABCD+234567890123456789WXYZ",
        "ABCD/234567890123456789WXYZ",
        "ABCD=234567890123456789WXYZ",
        MASK,
        "中文1234567890123456789WXYZ",
    ] {
        assert!(!valid_key(bad), "{bad}");
    }
    for submitted in [None, Some(""), Some(MASK)] {
        assert_eq!(resolve_key(submitted, KEY).unwrap(), KEY);
    }
    assert_eq!(resolve_key(Some(OTHER_KEY), KEY).unwrap(), OTHER_KEY);
    assert!(resolve_key(Some("ABCD*******************xxxx"), KEY).is_err());
    assert!(resolve_key(Some(MASK), "").is_err());
    assert!(resolve_key(None, "").is_err());
    assert_eq!(mask_key("bad-secret"), "**********");
}

#[test]
fn env_edits_preserve_unrelated_lines_and_parse_quotes_and_duplicates() {
    let mut env = EnvFile("# settings\nexport DEVICE_SN='0000123' # serial\nJWT_SECRET_KEY=do-not-change\nCLOUD_LINK_URL=old\nCLOUD_LINK_URL=\"http://cloud.local\" # current\n".into());
    assert_eq!(env.get("DEVICE_SN").unwrap().as_deref(), Some("0000123"));
    assert_eq!(
        env.get(URL_KEY).unwrap().as_deref(),
        Some("http://cloud.local")
    );
    env.set(URL_KEY, "https://cloud.local/a$b");
    assert_eq!(
        env.get(URL_KEY).unwrap().as_deref(),
        Some("https://cloud.local/a$b")
    );
    assert_eq!(env.0.matches("CLOUD_LINK_URL=").count(), 1);
    assert!(env.0.contains("JWT_SECRET_KEY=do-not-change\n"));
    assert!(env.0.contains("export DEVICE_SN='0000123' # serial\n"));
}

#[test]
fn sn_is_never_invented_and_hardware_fallback_preserves_leading_zeroes() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    link.initialize().unwrap();
    assert_eq!(link.config().unwrap().device_sn, "");
    assert!(!link.config().unwrap().status);
    fs::write(&link.serial_path, "0000456\0\n").unwrap();
    link.initialize().unwrap();
    assert_eq!(link.config().unwrap().device_sn, "0000456");
    assert_eq!(
        link.read_env()
            .unwrap()
            .get("DEVICE_SN")
            .unwrap()
            .as_deref(),
        Some("0000456")
    );
}

#[test]
fn url_validation_accepts_base_paths_and_rejects_credentials_or_wrong_schemes() {
    assert_eq!(
        normalize_url(" http://127.0.0.1:9100/prefix/ ").unwrap(),
        "http://127.0.0.1:9100/prefix"
    );
    for bad in [
        "",
        "ftp://host",
        "http://user:secret@host",
        "http://host?key=secret",
        "http://host/#secret",
        "http://ho\nst",
    ] {
        assert!(normalize_url(bad).is_err());
    }
}

#[tokio::test]
async fn registration_uses_raw_key_string_sn_and_saves_only_after_cloud_success() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let (server, mut received) = cloud(
        StatusCode::OK,
        json!({"code":200,"msg":"success","data":{"stationSN":"00098888","orgId":3}}),
    )
    .await;
    write_config(&link, "https://old.example", "", false);
    let result = link
        .activate(request(Some(&server.url), Some(KEY)))
        .await
        .unwrap();
    let (headers, body) = received.recv().await.unwrap();
    assert_eq!(body, json!({"stationSN":"00098888","activation":KEY}));
    assert!(!headers.contains_key("authorization"));
    assert!(result.status);
    assert_eq!(result.url, server.url);
    assert_eq!(result.key, MASK);
    assert!(!serde_json::to_string(&result).unwrap().contains(KEY));
    let saved = link.read_env().unwrap();
    assert_eq!(saved.get(ACTIVATION_KEY).unwrap().as_deref(), Some(KEY));
    assert!(
        fs::read_to_string(&link.env_path)
            .unwrap()
            .contains("JWT_SECRET_KEY=keep-me")
    );
    assert!(link.config().unwrap().status);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&link.env_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn current_mask_or_omitted_key_reuses_real_key() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let (server, mut received) = cloud(StatusCode::OK, json!({"code":200,"msg":"success"})).await;
    write_config(&link, &server.url, KEY, false);
    for key in [Some(MASK), None] {
        let result = link.activate(request(None, key)).await.unwrap();
        assert_eq!(result.key, MASK);
        assert_eq!(received.recv().await.unwrap().1["activation"], KEY);
    }
}

#[tokio::test]
async fn cloud_rejections_preserve_previous_configuration_including_true_status() {
    for (status, reason) in [
        (StatusCode::OK, "Activation code has expired"),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Activation code is disabled",
        ),
    ] {
        for previously_active in [false, true] {
            let directory = TempDir::new().unwrap();
            let link = store(&directory);
            write_config(
                &link,
                "https://previous.example",
                OTHER_KEY,
                previously_active,
            );
            let original = fs::read_to_string(&link.env_path).unwrap();
            let (server, _) = cloud(status, json!({"code":500,"msg":reason,"data":null})).await;
            let error = link
                .activate(request(Some(&server.url), Some(KEY)))
                .await
                .err()
                .unwrap();
            assert_eq!(error.status, StatusCode::BAD_GATEWAY);
            assert_eq!(
                error.error.message,
                format!("Device activation failed: {reason}")
            );
            assert_eq!(fs::read_to_string(&link.env_path).unwrap(), original);
        }
    }
}

#[tokio::test]
async fn missing_sn_or_invalid_key_is_rejected_before_network_request() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    link.initialize().unwrap();
    let error = link.activate(request(None, Some(KEY))).await.err().unwrap();
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(
        error
            .error
            .message
            .contains("Device serial number is missing")
    );
    let error = link
        .activate(request(None, Some(MASK)))
        .await
        .err()
        .unwrap();
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(
        error
            .error
            .message
            .contains("activation key must be 27 characters long")
    );
}

#[tokio::test]
async fn timeout_reports_unknown_outcome_and_keeps_saved_config() {
    let directory = TempDir::new().unwrap();
    let mut link = store(&directory);
    link.client = reqwest::Client::builder()
        .timeout(Duration::from_millis(150))
        .build()
        .unwrap();
    let server = serve(Router::new().route(
        "/user-app/common/register",
        post(|| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Json(json!({"code":200,"msg":"success"}))
        }),
    ))
    .await;
    write_config(&link, "https://previous.example", OTHER_KEY, true);
    let original = fs::read_to_string(&link.env_path).unwrap();
    let error = link
        .activate(request(Some(&server.url), Some(KEY)))
        .await
        .err()
        .unwrap();
    assert_eq!(error.status, StatusCode::GATEWAY_TIMEOUT);
    assert!(
        error
            .error
            .message
            .contains("Activation could not be confirmed")
    );
    assert_eq!(fs::read_to_string(&link.env_path).unwrap(), original);
}

#[tokio::test]
async fn concurrent_activation_is_rejected_instead_of_queued() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let _guard = link.activation.lock().await;
    let error = link.activate(request(None, Some(KEY))).await.err().unwrap();
    assert_eq!(error.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn cloud_success_followed_by_local_save_failure_is_not_reported_as_success() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let path = link.env_path.clone();
    let server = serve(Router::new().route(
        "/user-app/common/register",
        post(move || {
            let path = path.clone();
            async move {
                // Simulate the configuration mount disappearing after cloud registration.
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
                Json(json!({"code":200,"msg":"success"}))
            }
        }),
    ))
    .await;
    write_config(&link, &server.url, KEY, false);
    let error = link.activate(request(None, None)).await.err().unwrap();
    assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.error.message.contains("Cloud registration succeeded"));
    assert!(
        error
            .error
            .message
            .contains("local configuration could not be saved")
    );
}

#[tokio::test]
async fn unrelated_env_edits_during_cloud_call_are_preserved() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let path = link.env_path.clone();
    let server = serve(Router::new().route(
        "/user-app/common/register",
        post(move || {
            let path = path.clone();
            async move {
                let mut text = fs::read_to_string(&path).unwrap();
                text.push_str("NEW_SETTING=keep-this-too\n");
                fs::write(path, text).unwrap();
                Json(json!({"code":200,"msg":"success"}))
            }
        }),
    ))
    .await;
    write_config(&link, &server.url, KEY, false);
    link.activate(request(None, None)).await.unwrap();
    assert!(
        fs::read_to_string(&link.env_path)
            .unwrap()
            .contains("NEW_SETTING=keep-this-too")
    );
}

#[tokio::test]
async fn invalid_cloud_json_and_redirects_do_not_activate_or_forward_key() {
    for status in [StatusCode::OK, StatusCode::TEMPORARY_REDIRECT] {
        let directory = TempDir::new().unwrap();
        let link = store(&directory);
        let server = serve(Router::new().route(
            "/user-app/common/register",
            post(move || async move {
                (
                    status,
                    [("location", "http://127.0.0.1:1/never-follow")],
                    "invalid json",
                )
            }),
        ))
        .await;
        write_config(&link, &server.url, KEY, false);
        let error = link.activate(request(None, None)).await.err().unwrap();
        assert_eq!(error.status, StatusCode::BAD_GATEWAY);
        assert!(error.error.message.contains("Invalid cloud response"));
        assert!(!link.config().unwrap().status);
    }
}

#[tokio::test]
async fn real_router_requires_login_and_accepts_every_role_with_uniform_masked_responses() {
    use crate::{auth, config::GatewayConfig, db, state::AppState, ws::WsHub};
    use dashmap::DashMap;
    let directory = TempDir::new().unwrap();
    let link = Arc::new(store(&directory));
    let (cloud_server, mut received) =
        cloud(StatusCode::OK, json!({"code":200,"msg":"success"})).await;
    write_config(&link, &cloud_server.url, KEY, false);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db::create_tables(&db).await.unwrap();
    db::init_roles(&db).await.unwrap();
    let redis = common::redis::RedisClient::with_config_no_ping(common::redis::RedisPoolConfig {
        url: "redis://127.0.0.1:1".into(),
        min_idle: Some(0),
        ..Default::default()
    })
    .await
    .unwrap();
    let rtdb = Arc::new(voltage_rtdb::RedisRtdb::from_client(Arc::new(redis)));
    let config = Arc::new(GatewayConfig::default());
    let app = crate::build_router(Arc::new(AppState {
        cloud_link: link.clone(),
        db: db.clone(),
        ws_hub: WsHub::new(rtdb.clone(), db.clone()),
        rtdb,
        config: config.clone(),
        refresh_tokens: DashMap::new(),
    }));
    let gateway = serve(app).await;
    let client = reqwest::Client::new();
    let endpoint = format!("{}/api/v1/cloud-link", gateway.url);
    let check = format!("{}/api/v1/internal/cloud-link/status", gateway.url);
    let check_serial = |serial: &str| client.get(&check).query(&[("device_sn", serial)]);
    assert_eq!(
        check_serial("00098888").send().await.unwrap().status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client.get(&endpoint).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(format!("{endpoint}/activate"))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for (role_id, role_name) in [(1, "Admin"), (2, "Engineer"), (3, "Viewer")] {
        let id = db::create_user(&db, role_name, "unused-test-password", role_id)
            .await
            .unwrap();
        let user = db::get_user_with_role(&db, id).await.unwrap().unwrap();
        let token = auth::create_access_token(&user, 0, &config.jwt_secret, 5).unwrap();
        let response = client
            .post(format!("{endpoint}/activate"))
            .bearer_auth(&token)
            .json(&json!({"key":MASK}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{role_name}");
        let activated: Value = response.json().await.unwrap();
        assert_eq!(activated["success"], true);
        assert_eq!(activated["data"]["key"], MASK);
        assert_eq!(activated["data"]["status"], true);
        assert_eq!(activated["data"]["device_sn"], "00098888");
        assert!(
            !received
                .recv()
                .await
                .unwrap()
                .0
                .contains_key("authorization")
        );
        let fetched: Value = client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(fetched, activated);
        let failure = client
            .post(format!("{endpoint}/activate"))
            .bearer_auth(&token)
            .json(&json!({"key":"wrong"}))
            .send()
            .await
            .unwrap();
        assert_eq!(failure.status(), StatusCode::BAD_REQUEST);
        let failure: Value = failure.json().await.unwrap();
        assert_eq!(failure["success"], false);
        assert_eq!(failure["error"]["code"], 400);
    }
    assert_eq!(
        check_serial("00098888").send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        check_serial("98888").send().await.unwrap().status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        check_serial("00098888")
            .header("x-forwarded-for", "192.0.2.10")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    let id = db::get_user_by_username(&db, "Viewer")
        .await
        .unwrap()
        .unwrap()
        .id;
    let user = db::get_user_with_role(&db, id).await.unwrap().unwrap();
    let token = auth::create_access_token(&user, 0, &config.jwt_secret, 5).unwrap();
    let invalid = client
        .post(format!("{endpoint}/activate"))
        .bearer_auth(&token)
        .json(&json!({"device_sn":"client-override","key":KEY}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn changed_cloud_configuration_during_registration_is_not_overwritten() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let path = link.env_path.clone();
    let server = serve(Router::new().route(
        "/user-app/common/register",
        post(move || {
            let path = path.clone();
            async move {
                let mut env = EnvFile(fs::read_to_string(&path).unwrap());
                env.set(ACTIVATION_KEY, OTHER_KEY);
                fs::write(&path, env.0).unwrap();
                Json(json!({"code":200,"msg":"success"}))
            }
        }),
    ))
    .await;
    write_config(&link, &server.url, KEY, false);
    let error = link.activate(request(None, None)).await.err().unwrap();
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert!(error.error.message.contains("Cloud registration succeeded"));
    assert_eq!(link.config().unwrap().key, mask_key(OTHER_KEY));
    assert!(!link.config().unwrap().status);
}

#[tokio::test]
async fn registration_appends_api_path_to_configured_base_path() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let server = serve(Router::new().route(
        "/prefix/user-app/common/register",
        post(|| async { Json(json!({"code":200,"msg":"success"})) }),
    ))
    .await;
    write_config(&link, &format!("{}/prefix/", server.url), KEY, false);
    let config = link.activate(request(None, None)).await.unwrap();
    assert_eq!(config.url, format!("{}/prefix", server.url));
}

#[tokio::test]
async fn unknown_cloud_error_messages_are_replaced_with_safe_english() {
    let directory = TempDir::new().unwrap();
    let link = store(&directory);
    let (server, _) = cloud(
        StatusCode::BAD_REQUEST,
        json!({
            "code":400,"msg":format!("云端内部错误 {KEY}, saved {OTHER_KEY}")
        }),
    )
    .await;
    write_config(&link, &server.url, OTHER_KEY, true);
    let error = link.activate(request(None, Some(KEY))).await.err().unwrap();
    assert_eq!(error.status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        error.error.message,
        "Device activation failed: Cloud rejected the request (HTTP 400, code 400)"
    );
}
