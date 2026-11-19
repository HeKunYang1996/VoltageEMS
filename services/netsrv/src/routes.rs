use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::{
    Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::Serialize;
use serde_json::{Value, json};
use tracing::error;
use utoipa::OpenApi;
#[cfg(feature = "swagger-ui")]
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::db_config;
use crate::models::{AlarmBroadcastRequest, CertUploadForm, NetConfig, SystemMetrics};
use crate::mqtt::{do_inst_sync, publish_json};
use crate::state::AppState;

// ============================================================================
// Router
// ============================================================================

pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/", get(root))
        .route("/ping", get(ping))
        .route("/netApi/health", get(health))
        // Alarm
        .route("/netApi/alarm/broadcast", post(alarm_broadcast))
        .route("/netApi/alarm/config", get(alarm_config))
        // MQTT
        .route("/netApi/mqtt/config", get(mqtt_get_config).post(mqtt_update_config))
        .route("/netApi/mqtt/status", get(mqtt_status))
        .route("/netApi/mqtt/disconnect", post(mqtt_disconnect))
        .route("/netApi/mqtt/reconnect", post(mqtt_reconnect))
        // Certificate
        .route("/netApi/certificate/upload", post(cert_upload))
        .route("/netApi/certificate/info", get(cert_info))
        .route("/netApi/certificate/{cert_type}", delete(cert_delete))
        // Device sync
        .route("/netApi/inst-sync", post(inst_sync_push))
        // Admin API (shared endpoints from common lib)
        .route("/api/admin/logs/level", get(common::admin_api::get_log_level).post(common::admin_api::set_log_level))
        .route("/api/admin/logs/files", get(common::admin_api::list_log_files))
        .route("/api/admin/logs/view", get(common::admin_api::view_log_file))
        .with_state(state);

    #[cfg(feature = "swagger-ui")]
    let api = api.merge(
        SwaggerUi::new("/docs")
            .url("/openapi.json", ApiDoc::openapi())
            .config(
                Config::default()
                    .default_model_rendering("model")
                    .default_models_expand_depth(1),
            ),
    );

    api
}

// ============================================================================
// OpenAPI document (only consumed when swagger-ui feature is enabled)
// ============================================================================

#[cfg_attr(not(feature = "swagger-ui"), allow(dead_code))]
#[derive(OpenApi)]
#[openapi(
    paths(
        root,
        ping,
        health,
        alarm_broadcast,
        alarm_config,
        inst_sync_push,
        mqtt_get_config,
        mqtt_update_config,
        mqtt_status,
        mqtt_disconnect,
        mqtt_reconnect,
        cert_upload,
        cert_info,
        cert_delete,
    ),
    components(schemas(
        NetConfig,
        AlarmBroadcastRequest,
        CertUploadForm,
        SystemMetrics,
    )),
    tags(
        (name = "Health",      description = "Health checks and service information"),
        (name = "Alarm",       description = "Alarm broadcast and configuration"),
        (name = "MQTT",        description = "MQTT connection configuration and control"),
        (name = "Certificate", description = "TLS certificate management"),
    ),
    info(
        title = "VoltageEMS Network Service",
        version = "1.0.0",
        description = "MQTT gateway service: forwards Redis real-time data to the cloud MQTT broker \
                       and routes cloud-issued read/write commands to local devices."
    )
)]
pub struct ApiDoc;

// ============================================================================
// Root / ping
// ============================================================================

/// netsrv service banner.
///
/// Returns service name, version, and status. Use this to confirm the netsrv
/// process is online and running the expected version. Does not depend on the
/// MQTT connection — returns 200 even if the broker is unreachable. For MQTT
/// status see `/netApi/health` or `/netApi/mqtt/status`.
#[utoipa::path(get, path = "/", tag = "Health",
    responses((status = 200, description = "Basic service information")))]
async fn root() -> Json<Value> {
    Json(json!({
        "service": "netsrv",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

/// Minimal liveness probe — returns the string "pong".
///
/// Unlike `/`, the response body is a plain string with no JSON overhead,
/// suitable for high-frequency liveness probes and load-balancer health checks.
#[utoipa::path(get, path = "/ping", tag = "Health",
    responses((status = 200, description = "pong")))]
async fn ping() -> &'static str {
    "pong"
}

// ============================================================================
// Health
// ============================================================================

/// Health check: returns MQTT connection status and device identity.
///
/// Reflects the live MQTT broker connection state (not a cached value). Returns
/// `mqtt_connected` (bool), broker address, and device `client_id`. When the
/// process is alive but MQTT is not connected, responds 200 with
/// `connected: false` — allowing dashboards to distinguish a dead process from
/// a live process with a broken cloud link.
#[utoipa::path(get, path = "/netApi/health", tag = "Health",
    responses((status = 200, description = "MQTT connection status and device identity")))]
async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mqtt_ok = state.mqtt_connected.load(Ordering::Relaxed);
    Json(json!({
        "success": mqtt_ok,
        "message": if mqtt_ok { "MQTT 连接正常" } else { "MQTT 未连接" },
        "data": {
            "mqtt_connected": mqtt_ok,
            "product_sn":     state.device.product_sn,
            "device_sn":      state.device.device_sn,
        }
    }))
}

// ============================================================================
// Alarm
// ============================================================================

/// Forward an alarm JSON payload to the MQTT alarm topic.
///
/// The request body is an arbitrary JSON object; content is not validated and
/// is published as-is to the configured alarm topic (see `GET /netApi/alarm/config`
/// for the topic name). The cloud subscriber is responsible for parsing the
/// payload. Alarm events from upstream alarmsrv travel this path to the cloud.
/// If MQTT is not connected the call still returns 200 but the message may be
/// lost (delivery guarantee depends on the QoS configured for the broker).
#[utoipa::path(post, path = "/netApi/alarm/broadcast", tag = "Alarm",
    request_body = AlarmBroadcastRequest,
    responses(
        (status = 200, description = "Alarm published successfully"),
        (status = 503, description = "MQTT not connected — publish failed"),
    ))]
async fn alarm_broadcast(
    State(state): State<Arc<AppState>>,
    Json(AlarmBroadcastRequest(body)): Json<AlarmBroadcastRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    publish_json(&state, &state.topics.alarm, &body)
        .await
        .map(|_| Json(json!({"success": true, "message": "Alarm broadcast"})))
        .map_err(|e| {
            error!("Alarm broadcast failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"success": false, "message": e.to_string()})),
            )
        })
}

/// Retrieve alarm cloud-forwarding configuration (topic name and MQTT status).
///
/// Returns the MQTT topic used for alarm broadcasts (e.g.
/// `voltageems/alarm/{device_id}`) and whether the MQTT connection is currently
/// online. Useful for the cloud-config UI to confirm where alarms are sent and
/// whether the link is healthy.
#[utoipa::path(get, path = "/netApi/alarm/config", tag = "Alarm",
    responses((status = 200, description = "Alarm topic name and MQTT connection status")))]
async fn alarm_config(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "success": true,
        "message": "OK",
        "data": {
            "alarm_topic":    state.topics.alarm,
            "mqtt_connected": state.mqtt_connected.load(Ordering::Relaxed),
        }
    }))
}

// ============================================================================
// MQTT config
// ============================================================================

/// Retrieve current MQTT configuration (broker, certificate paths, topic prefix, etc.).
///
/// Read-only. Returns a `NetConfig` object containing broker_host, port,
/// client_id, TLS flag, certificate filenames, and topic templates. Sensitive
/// material (e.g. certificate private-key contents) is never included. To
/// update the configuration use `POST /netApi/mqtt/config`.
#[utoipa::path(get, path = "/netApi/mqtt/config", tag = "MQTT",
    responses((status = 200, description = "Current MQTT configuration", body = NetConfig)))]
async fn mqtt_get_config(State(state): State<Arc<AppState>>) -> Json<Value> {
    let cfg = state.config.read().await.clone();
    Json(json!({ "success": true, "message": "OK", "data": cfg }))
}

/// Update MQTT configuration and immediately trigger a reconnect — no service restart needed.
///
/// Persists the new configuration, then disconnects the current MQTT session and
/// reconnects with the new parameters. There will be a brief MQTT unavailability
/// window of a few seconds during which in-flight alarms may be lost (subject to
/// the configured QoS). Use this endpoint to change the broker address,
/// certificates, or TLS settings. If the new parameters are invalid and the
/// connection fails, netsrv remains in the disconnected state until a correct
/// configuration is submitted.
#[utoipa::path(post, path = "/netApi/mqtt/config", tag = "MQTT",
    request_body = NetConfig,
    responses(
        (status = 200, description = "Configuration saved; reconnecting"),
        (status = 500, description = "Failed to save configuration"),
    ))]
async fn mqtt_update_config(
    State(state): State<Arc<AppState>>,
    Json(mut new_cfg): Json<NetConfig>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Without this the in-memory copy bypasses load_config() and chunks(0) can panic.
    new_cfg.normalize();
    if let Err(e) = db_config::save_config(&state.sqlite, &new_cfg).await {
        error!("Save config failed: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }
    *state.config.write().await = new_cfg;
    state.reconnect_signal.notify_one();
    Ok(Json(
        json!({"success": true, "message": "Config updated, reconnecting"}),
    ))
}

/// Real-time MQTT connection status (polling endpoint).
///
/// Returns connected/disconnected state, broker address, TLS flag, and device
/// identity. Intended for the cloud-status indicator on the operations dashboard.
/// More detailed than `/netApi/health` but with the same update frequency (no
/// background cache).
#[utoipa::path(get, path = "/netApi/mqtt/status", tag = "MQTT",
    responses((status = 200, description = "Current MQTT connection state, broker address, and device identity")))]
async fn mqtt_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let cfg = state.config.read().await;
    Json(json!({
        "success": true,
        "message": "OK",
        "data": {
            "connected":  state.mqtt_connected.load(Ordering::Relaxed),
            "broker":     format!("{}:{}", cfg.broker_host, cfg.broker_port),
            "ssl":        cfg.ssl_enabled,
            "product_sn": state.device.product_sn,
            "device_sn":  state.device.device_sn,
        }
    }))
}

/// Manually disconnect MQTT and suspend automatic reconnection.
///
/// Counterpart to `POST /netApi/mqtt/reconnect`. Closes the current MQTT
/// session and sets a "reconnect inhibit" flag — netsrv will not attempt to
/// reconnect even if the broker is reachable, until `reconnect` is explicitly
/// called. Intended for maintenance windows such as broker upgrades or
/// temporarily suppressing cloud alarm forwarding.
#[utoipa::path(post, path = "/netApi/mqtt/disconnect", tag = "MQTT",
    responses((status = 200, description = "MQTT disconnected; auto-reconnect suspended until reconnect is called")))]
async fn mqtt_disconnect(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Mark disconnection intent first, then wake the mqtt loop so it stops reconnecting.
    state
        .disconnect_requested
        .store(true, std::sync::atomic::Ordering::Relaxed);
    // Drop the current client to force the event loop to exit.
    *state.mqtt_client.lock().await = None;
    state
        .mqtt_connected
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state.reconnect_signal.notify_one();
    Json(json!({"success": true, "message": "MQTT disconnected, auto-reconnect paused"}))
}

/// Trigger MQTT reconnection and resume automatic reconnection.
///
/// Counterpart to `POST /netApi/mqtt/disconnect`. Clears the reconnect-inhibit
/// flag and immediately schedules a connection attempt. A 200 response does not
/// mean the connection succeeded — reconnection runs asynchronously in the
/// background; poll `GET /netApi/mqtt/status` to confirm. Call this after a
/// maintenance window to restore cloud link.
#[utoipa::path(post, path = "/netApi/mqtt/reconnect", tag = "MQTT",
    responses((status = 200, description = "Reconnect command issued; executing in background")))]
async fn mqtt_reconnect(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Clear the disconnect flag, then wake the mqtt loop to reconnect.
    state
        .disconnect_requested
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state.reconnect_signal.notify_one();
    Json(json!({"success": true, "message": "Reconnect command sent, executing in background"}))
}

// ============================================================================
// Certificate management
// ============================================================================

/// Upload a single TLS certificate file (multipart/form-data).
///
/// Upload one certificate per request; use `cert_type` to specify the type.
/// The original filename is ignored — files are saved under fixed names in
/// `cert_dir`:
///
/// | cert_type     | Saved filename         |
/// |---------------|------------------------|
/// | `ca_cert`     | `AmazonRootCA1.pem`    |
/// | `client_cert` | `certificate.pem.crt`  |
/// | `client_key`  | `private.pem.key`      |
#[utoipa::path(post, path = "/netApi/certificate/upload", tag = "Certificate",
    request_body(
        content_type = "multipart/form-data",
        content = inline(CertUploadForm),
    ),
    responses(
        (status = 200, description = "Certificate uploaded successfully"),
        (status = 400, description = "Invalid request — unknown cert_type, empty file, unsupported format, or file exceeds 1 MB"),
        (status = 500, description = "Certificate directory not writable or file write failed"),
    ))]
async fn cert_upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    const MAX_SIZE: usize = 1024 * 1024; // 1 MB
    const ALLOWED_EXT: &[&str] = &[".pem", ".crt", ".key", ".cer", ".p12", ".pfx"];

    let cert_dir = state.env.cert_dir.clone();

    // Collect all multipart fields first.
    let mut cert_type_val: Option<String> = None;
    let mut file_data: Option<(String, Vec<u8>)> = None; // (original_filename, bytes)

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "cert_type" => {
                let text = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"success": false, "message": e.to_string()})),
                    )
                })?;
                cert_type_val = Some(text);
            },
            "file" => {
                let orig_name = field.file_name().unwrap_or("").to_string();
                let data = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"success": false, "message": e.to_string()})),
                    )
                })?;
                file_data = Some((orig_name, data.to_vec()));
            },
            _ => {},
        }
    }

    // Validate cert_type.
    let cert_type = cert_type_val.ok_or_else(|| (
        StatusCode::BAD_REQUEST,
        Json(json!({"success": false, "message": "Missing cert_type field. Valid values: ca_cert | client_cert | client_key"})),
    ))?;

    let save_name = match cert_type.as_str() {
        "ca_cert" => "AmazonRootCA1.pem",
        "client_cert" => "certificate.pem.crt",
        "client_key" => "private.pem.key",
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"success": false, "message": format!("Unsupported cert_type: '{}'. Valid: ca_cert | client_cert | client_key", cert_type)}),
                ),
            ));
        },
    };

    // Validate file.
    let (orig_name, data) = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "Missing file field"})),
        )
    })?;

    if orig_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "Filename cannot be empty"})),
        ));
    }

    let ext = std::path::Path::new(&orig_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();
    if !ALLOWED_EXT.contains(&ext.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": format!("Unsupported file format '{}'. Supported: {}", ext, ALLOWED_EXT.join(", "))
            })),
        ));
    }

    if data.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "File content is empty"})),
        ));
    }
    if data.len() > MAX_SIZE {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": format!("File exceeds 1MB limit (current: {} bytes)", data.len())
            })),
        ));
    }

    // Ensure directory exists.
    std::fs::create_dir_all(&cert_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Cannot create cert directory '{}': {} (check path permissions)", cert_dir, e)
            })),
        )
    })?;

    // Save file.
    let dest = format!("{}/{}", cert_dir, save_name);
    std::fs::write(&dest, &data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": format!("Failed to write file: {}", e)})),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "message": "Certificate uploaded",
        "data": {
            "cert_type": cert_type,
            "saved_as":  save_name,
            "path":      dest,
        }
    })))
}

/// List certificate directory status: path and per-file existence flags.
///
/// Checks whether the CA certificate, client certificate, and private key are
/// present in the configured certificate directory. Certificate contents and
/// fingerprints are never returned (to avoid private-key exposure) — only
/// `exists: true/false` per file. Use this on the cloud-config pre-flight page
/// to confirm all required certificates have been uploaded.
#[utoipa::path(get, path = "/netApi/certificate/info", tag = "Certificate",
    responses((status = 200, description = "Certificate directory path and per-file existence status")))]
async fn cert_info(State(state): State<Arc<AppState>>) -> Json<Value> {
    let cert_dir = state.env.cert_dir.clone();
    let files = [
        "AmazonRootCA1.pem",
        "certificate.pem.crt",
        "private.pem.key",
    ];
    let info: Vec<Value> = files
        .iter()
        .map(|f| {
            let exists = std::path::Path::new(&format!("{}/{}", cert_dir, f)).exists();
            json!({ "file": f, "exists": exists })
        })
        .collect();
    Json(json!({
        "success": true,
        "message": "OK",
        "data": {
            "cert_dir": cert_dir,
            "files":    info,
        }
    }))
}

/// Delete a certificate file by type.
///
/// `cert_type` must be one of: `ca_cert` / `client_cert` / `client_key`.
#[utoipa::path(delete, path = "/netApi/certificate/{cert_type}", tag = "Certificate",
    params(
        ("cert_type" = String, Path, description = "Certificate type: ca_cert | client_cert | client_key")
    ),
    responses(
        (status = 200, description = "Deleted successfully (also returned when the file did not exist)"),
        (status = 400, description = "Unknown cert_type"),
        (status = 500, description = "Delete failed"),
    ))]
async fn cert_delete(
    State(state): State<Arc<AppState>>,
    Path(cert_type): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let cert_dir = state.env.cert_dir.clone();
    let filename = match cert_type.as_str() {
        "ca_cert" => "AmazonRootCA1.pem",
        "client_cert" => "certificate.pem.crt",
        "client_key" => "private.pem.key",
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"success": false, "message": "Unknown cert_type. Valid: ca_cert | client_cert | client_key"}),
                ),
            ));
        },
    };

    match std::fs::remove_file(format!("{}/{}", cert_dir, filename)) {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Deleted successfully",
            "data": { "deleted": filename }
        }))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Json(
            json!({"success": true, "message": "File does not exist, nothing to delete"}),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        )),
    }
}

// ============================================================================
// Device sync
// ============================================================================

#[derive(Serialize)]
struct InstSyncError {
    success: bool,
    message: String,
}

impl InstSyncError {
    fn response(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: false,
                message: message.into(),
            }),
        )
    }
}

async fn require_sync_registration(
    state: &AppState,
) -> Result<(), (StatusCode, Json<InstSyncError>)> {
    let url = format!(
        "{}/api/v1/internal/cloud-link/status",
        state.env.apigateway_url.trim_end_matches('/')
    );
    let check = state
        .http_client
        .get(url)
        .query(&[("device_sn", state.device.device_sn.as_str())])
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;

    match check {
        Ok(response) => match response.status() {
            StatusCode::NO_CONTENT => return Ok(()),
            StatusCode::CONFLICT => {
                return Err(InstSyncError::response(
                    StatusCode::CONFLICT,
                    "Device is not activated. Activate it before synchronizing.",
                ));
            },
            status => error!(%status, "Activation status check failed"),
        },
        Err(error) => error!(error = %error.without_url(), "Activation status check failed"),
    }
    Err(InstSyncError::response(
        StatusCode::SERVICE_UNAVAILABLE,
        "Unable to verify device activation status. Please try again.",
    ))
}

/// Publish the current device list and topology to the cloud for an activated device.
#[utoipa::path(post, path = "/netApi/inst-sync", tag = "MQTT",
    responses(
        (status = 200, description = "inst-sync-reply submitted for MQTT publication"),
        (status = 409, description = "Device is not activated"),
        (status = 503, description = "Activation status, modsrv, or MQTT is unavailable"),
    ))]
async fn inst_sync_push(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<InstSyncError>)> {
    require_sync_registration(&state).await?;
    let msg_id = chrono::Utc::now().timestamp_millis().to_string();

    do_inst_sync(Arc::clone(&state), Some(msg_id.clone()))
        .await
        .map(|_| {
            Json(json!({
                "success": true,
                "message": "inst-sync-reply published",
                "data": { "msgId": msg_id }
            }))
        })
        .map_err(|e| {
            error!("inst-sync-push failed: {}", e);
            InstSyncError::response(StatusCode::SERVICE_UNAVAILABLE, e.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Query;
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicBool, AtomicU16, AtomicUsize},
    };
    use tokio::sync::{Mutex, Notify, RwLock};

    #[tokio::test]
    async fn instance_sync_requires_confirmed_activation_before_querying_modsrv() {
        let registration = Arc::new(AtomicU16::new(StatusCode::CONFLICT.as_u16()));
        let activation_fixture = Arc::clone(&registration);
        let modsrv_reads = Arc::new(AtomicUsize::new(0));
        let instance_reads = Arc::clone(&modsrv_reads);
        let topology_reads = Arc::clone(&modsrv_reads);
        let upstream = Router::new()
            .route(
                "/api/v1/internal/cloud-link/status",
                get(move |Query(params): Query<HashMap<String, String>>| {
                    let status = activation_fixture.load(Ordering::Relaxed);
                    async move {
                        assert_eq!(
                            params.get("device_sn").map(String::as_str),
                            Some("00098888")
                        );
                        StatusCode::from_u16(status).unwrap()
                    }
                }),
            )
            .route(
                "/api/instances/properties",
                get(move || {
                    instance_reads.fetch_add(1, Ordering::Relaxed);
                    async { Json(json!({"data": {"list": []}})) }
                }),
            )
            .route(
                "/api/station/topology",
                get(move || {
                    topology_reads.fetch_add(1, Ordering::Relaxed);
                    async { Json(json!({"data": {"flow_json": {}}})) }
                }),
            );
        let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_url = format!("http://{}", upstream_listener.local_addr().unwrap());
        let upstream_task =
            tokio::spawn(async move { axum::serve(upstream_listener, upstream).await.unwrap() });

        let redis =
            common::redis::RedisClient::with_config_no_ping(common::redis::RedisPoolConfig {
                url: "redis://127.0.0.1:1".into(),
                min_idle: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        let device = Arc::new(crate::device::DeviceIdentity::resolve(
            "MonarchHub",
            "00098888",
        ));
        let topics = Arc::new(device.topics());
        let (mqtt, _event_loop) =
            rumqttc::AsyncClient::new(rumqttc::MqttOptions::new("sync-test", "127.0.0.1", 1), 4);
        let state = Arc::new(AppState {
            sqlite: sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
            rtdb: Arc::new(voltage_rtdb::RedisRtdb::from_client(Arc::new(redis))),
            env: Arc::new(crate::config::EnvConfig {
                apigateway_url: upstream_url.clone(),
                ..Default::default()
            }),
            config: Arc::new(RwLock::new(NetConfig {
                apigateway_url: "http://127.0.0.1:1".into(),
                modsrv_url: upstream_url,
                ..Default::default()
            })),
            device,
            topics,
            mqtt_client: Arc::new(Mutex::new(Some(mqtt))),
            mqtt_connected: Arc::new(AtomicBool::new(true)),
            reconnect_signal: Arc::new(Notify::new()),
            disconnect_requested: Arc::new(AtomicBool::new(false)),
            http_client: reqwest::Client::builder().no_proxy().build().unwrap(),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/netApi/inst-sync", listener.local_addr().unwrap());
        let netsrv_task =
            tokio::spawn(async move { axum::serve(listener, build_router(state)).await.unwrap() });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();

        for (gateway_status, http_status, message) in [
            (
                StatusCode::CONFLICT,
                StatusCode::CONFLICT,
                "Device is not activated. Activate it before synchronizing.",
            ),
            (
                StatusCode::NOT_FOUND,
                StatusCode::SERVICE_UNAVAILABLE,
                "Unable to verify device activation status. Please try again.",
            ),
        ] {
            registration.store(gateway_status.as_u16(), Ordering::Relaxed);
            let response = client.post(&endpoint).send().await.unwrap();
            assert_eq!(response.status(), http_status);
            assert_eq!(
                response.json::<Value>().await.unwrap(),
                json!({"success": false, "message": message})
            );
            assert_eq!(modsrv_reads.load(Ordering::Relaxed), 0);
        }

        registration.store(StatusCode::NO_CONTENT.as_u16(), Ordering::Relaxed);
        let response = client.post(&endpoint).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json::<Value>().await.unwrap()["success"], true);
        assert_eq!(modsrv_reads.load(Ordering::Relaxed), 2);
        netsrv_task.abort();
        upstream_task.abort();
    }
}
