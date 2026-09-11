use std::sync::Arc;
/// MQTT connection lifecycle and incoming-message dispatch.
///
/// Architecture:
/// - `run_mqtt_loop` runs forever, reconnecting whenever the connection drops
///   or `state.reconnect_signal` fires (triggered by config-change API).
/// - Incoming `Publish` events are dispatched to the appropriate handler
///   based on topic.
/// - A shared `Arc<Mutex<Option<AsyncClient>>>` in `AppState` is updated
///   every time a new connection is established, so other tasks can publish.
use std::sync::atomic::Ordering;

use bytes::Bytes;
use chrono::Utc;
use rumqttc::{
    AsyncClient, Event, Incoming, LastWill, MqttOptions, NetworkOptions, QoS, TlsConfiguration,
    Transport,
};
use serde_json::json;
use tokio::time::{self, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use voltage_rtdb::Rtdb;

use crate::models::{
    CommandReply, FuncRequest, InstSyncItem, InstSyncProperty, InstSyncReply, ReadReply,
    ReadReplyProperty, ReadRequest, StatusPayload, WriteReply, WriteRequest,
};
use crate::state::AppState;

const MQTT_MAX_PACKET_SIZE_BYTES: usize = 1024 * 1024;

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run_mqtt_loop(state: Arc<AppState>, shutdown: CancellationToken) {
    loop {
        let cfg = state.config.read().await.clone();
        let delay = Duration::from_secs(cfg.reconnect_delay_secs);

        match connect_and_run(Arc::clone(&state), shutdown.clone()).await {
            Ok(_) => {
                if shutdown.is_cancelled() {
                    info!("MQTT task shut down cleanly");
                    return;
                }
                info!("MQTT event loop exited");
            },
            Err(e) => {
                warn!("MQTT connection error: {}", e);
            },
        }

        if shutdown.is_cancelled() {
            return;
        }

        // If an explicit disconnect was requested, stay idle until a reconnect
        // signal arrives (do NOT auto-reconnect after the delay).
        if state
            .disconnect_requested
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            info!("Disconnect requested – waiting for reconnect signal");
            state.mqtt_connected.store(false, Ordering::Relaxed);
            tokio::select! {
                _ = state.reconnect_signal.notified() => {}
                _ = shutdown.cancelled() => return,
            }
            // If still disconnected after wakeup, loop back and check again.
            continue;
        }

        // Normal auto-reconnect: wait for delay or an early trigger.
        tokio::select! {
            _ = time::sleep(delay) => {}
            _ = state.reconnect_signal.notified() => {
                info!("Reconnect signal received");
            }
            _ = shutdown.cancelled() => return,
        }
    }
}

// ── Inner connect + run ───────────────────────────────────────────────────────

async fn connect_and_run(state: Arc<AppState>, shutdown: CancellationToken) -> anyhow::Result<()> {
    let cfg = state.config.read().await.clone();

    // Resolve client ID
    let client_id = if cfg.client_id == "auto" {
        state.device.device_sn.clone()
    } else {
        cfg.client_id.clone()
    };

    let mut options = MqttOptions::new(&client_id, &cfg.broker_host, cfg.broker_port);
    apply_packet_size_limit(&mut options);
    options.set_keep_alive(Duration::from_secs(cfg.broker_keepalive_secs));
    options.set_clean_session(true);

    // MQTT username/password auth (only set when both are non-empty)
    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password)
        && !user.is_empty()
    {
        options.set_credentials(user, pass);
    }

    // Last Will Testament: broker publishes this automatically on unexpected disconnect.
    let lwt_payload = serde_json::to_string(&StatusPayload {
        msg_type: "offline".to_string(),
        gateway: state.device.device_sn.clone(),
        timestamp: Utc::now().timestamp(),
        reason: Some("unexpected".to_string()),
    })
    .unwrap_or_default();
    options.set_last_will(LastWill::new(
        &state.topics.status,
        lwt_payload.into_bytes(),
        QoS::AtLeastOnce,
        true,
    ));

    // TLS – cert_dir is fixed at startup from EnvConfig (not API-editable).
    // If ssl_enabled but cert loading fails, abort the connection attempt rather
    // than silently downgrading to plaintext.
    if cfg.ssl_enabled {
        let tls = build_tls(&state.env.cert_dir)?;
        options.set_transport(Transport::tls_with_config(tls));
    }

    let (client, mut event_loop) = AsyncClient::new(options, 64);
    // ARM64 设备无硬件加速时 rustls RSA 握手可能超过默认 5s，调大连接超时避免误报
    // NetworkOptions 是 rumqttc 0.24 设置连接超时的正确 API（不在 MqttOptions 上）
    let mut network_options = NetworkOptions::new();
    network_options.set_connection_timeout(30);
    event_loop.set_network_options(network_options);
    *state.mqtt_client.lock().await = Some(client.clone());

    info!("MQTT connecting to {}:{}", cfg.broker_host, cfg.broker_port);

    loop {
        tokio::select! {
            event_result = event_loop.poll() => {
                match event_result {
                    Ok(Event::Incoming(Incoming::ConnAck(ack))) => {
                        info!("MQTT connected (return_code={:?})", ack.code);
                        state.mqtt_connected.store(true, Ordering::Relaxed);
                        on_connected(&state, &client).await;
                    }
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        let topic = p.topic.clone();
                        let payload = p.payload.clone();
                        let s = Arc::clone(&state);
                        tokio::spawn(async move {
                            dispatch_message(s, &topic, payload).await;
                        });
                    }
                    Ok(Event::Incoming(Incoming::Disconnect)) => {
                        state.mqtt_connected.store(false, Ordering::Relaxed);
                        info!("MQTT disconnected by broker");
                        return Ok(());
                    }
                    Ok(_) => {}
                    Err(e) => {
                        state.mqtt_connected.store(false, Ordering::Relaxed);
                        return Err(anyhow::anyhow!("MQTT poll error: {}", e));
                    }
                }
            }

            _ = state.reconnect_signal.notified() => {
                info!("Config changed, reconnecting MQTT");
                // Send graceful offline before disconnecting
                let _ = publish_status(&client, &state, "offline", Some("config_reload")).await;
                state.mqtt_connected.store(false, Ordering::Relaxed);
                return Ok(());
            }

            _ = shutdown.cancelled() => {
                let _ = publish_status(&client, &state, "offline", Some("graceful_shutdown")).await;
                let _ = client.disconnect().await;
                state.mqtt_connected.store(false, Ordering::Relaxed);
                info!("MQTT disconnected (graceful shutdown)");
                return Ok(());
            }
        }
    }
}

// ── Connection established ────────────────────────────────────────────────────

async fn on_connected(state: &AppState, client: &AsyncClient) {
    // Re-subscribe to all command topics
    for (topic, qos) in state.topics.subscriptions() {
        if let Err(e) = client.subscribe(topic, qos).await {
            error!("Subscribe '{}' failed: {}", topic, e);
        }
    }

    // Send online status
    let _ = publish_status(client, state, "online", None).await;
}

// ── Status message helper ─────────────────────────────────────────────────────

pub async fn publish_status(
    client: &AsyncClient,
    state: &AppState,
    msg_type: &str,
    reason: Option<&str>,
) -> anyhow::Result<()> {
    let payload = StatusPayload {
        msg_type: msg_type.to_string(),
        gateway: state.device.device_sn.clone(),
        timestamp: Utc::now().timestamp(),
        reason: reason.map(|s| s.to_string()),
    };
    let json = serde_json::to_string(&payload)?;
    client
        .publish(&state.topics.status, QoS::AtLeastOnce, true, json)
        .await?;
    Ok(())
}

/// Publish any JSON value to a topic.
pub async fn publish_json(
    state: &AppState,
    topic: &str,
    value: &impl serde::Serialize,
) -> anyhow::Result<()> {
    let guard = state.mqtt_client.lock().await;
    let client = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("MQTT not connected"))?;
    let json = serde_json::to_string(value)?;
    client.publish(topic, QoS::AtLeastOnce, false, json).await?;
    Ok(())
}

// ── TLS configuration ─────────────────────────────────────────────────────────

fn build_tls(cert_dir: &str) -> anyhow::Result<TlsConfiguration> {
    let ca = std::fs::read(format!("{}/AmazonRootCA1.pem", cert_dir))
        .or_else(|_| std::fs::read(format!("{}/ca.pem", cert_dir)))
        .map_err(|e| anyhow::anyhow!("CA cert not found in {}: {}", cert_dir, e))?;

    let client_cert = std::fs::read(format!("{}/certificate.pem.crt", cert_dir))
        .or_else(|_| std::fs::read(format!("{}/client.crt", cert_dir)))
        .map_err(|e| anyhow::anyhow!("Client cert not found: {}", e))?;

    let client_key = std::fs::read(format!("{}/private.pem.key", cert_dir))
        .or_else(|_| std::fs::read(format!("{}/client.key", cert_dir)))
        .map_err(|e| anyhow::anyhow!("Client key not found: {}", e))?;

    // rumqttc 0.24 Simple: client_auth is (cert_pem_bytes, key_pem_bytes)
    Ok(TlsConfiguration::Simple {
        ca,
        alpn: None,
        client_auth: Some((client_cert, client_key)),
    })
}

// ── Incoming message dispatch ─────────────────────────────────────────────────

async fn dispatch_message(state: Arc<AppState>, topic: &str, payload: Bytes) {
    let t = &state.topics;

    if topic == t.read {
        handle_read(state, payload).await;
    } else if topic == t.write {
        handle_write(state, payload).await;
    } else if topic == t.call_data {
        handle_call_data(state, payload).await;
    } else if topic == t.call_alarm {
        handle_call_alarm(state, payload).await;
    } else if topic == t.func {
        handle_func(state, payload).await;
    } else if topic == t.inst_sync {
        handle_inst_sync(state, payload).await;
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

async fn handle_read(state: Arc<AppState>, payload: Bytes) {
    let req: ReadRequest = match serde_json::from_slice(&payload) {
        Ok(r) => r,
        Err(e) => {
            warn!("Bad read request: {}", e);
            let err_reply = json!({
                "result": "fail",
                "error": "json_parse_error",
                "message": format!("JSON parse error: {}", e),
                "msgId": "unknown",
                "timestamp": Utc::now().timestamp()
            });
            let _ = publish_json(&state, &state.topics.read_reply, &err_reply).await;
            return;
        },
    };

    // Convert underscores to spaces for Redis key lookup (mirrors Python netsrv behaviour:
    // the cloud sends device names with underscores, Redis stores them with spaces).
    let device_for_redis = req.device.replace('_', " ");
    let redis_key = format!("{}:{}:{}", req.source, device_for_redis, req.data_type);

    let value = match &req.field {
        Some(field) => {
            match state.rtdb.hash_get(&redis_key, field).await {
                Ok(Some(v)) => {
                    let mut map = serde_json::Map::new();
                    map.insert(field.clone(), parse_redis_value(&v));
                    serde_json::Value::Object(map)
                },
                Ok(None) => {
                    warn!("Read: field '{}' not found in '{}'", field, redis_key);
                    return; // Python silently returns without reply when data not found
                },
                Err(e) => {
                    error!("Redis HGET '{}' '{}': {}", redis_key, field, e);
                    return;
                },
            }
        },
        None => match state.rtdb.hash_get_all(&redis_key).await {
            Ok(map) if map.is_empty() => {
                warn!("Read: key '{}' not found or empty", redis_key);
                return;
            },
            Ok(map) => {
                let obj: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .map(|(k, v)| (k, parse_redis_value(&v)))
                    .collect();
                serde_json::Value::Object(obj)
            },
            Err(e) => {
                error!("Redis HGETALL '{}': {}", redis_key, e);
                return;
            },
        },
    };

    let reply = ReadReply {
        timestamp: Utc::now().timestamp(),
        property: vec![ReadReplyProperty {
            source: req.source,
            device: req.device,
            data_type: req.data_type,
            value,
        }],
        msg_id: req.msg_id,
    };

    if let Err(e) = publish_json(&state, &state.topics.read_reply, &reply).await {
        error!("Failed to publish read-reply: {}", e);
    }
}

async fn handle_write(state: Arc<AppState>, payload: Bytes) {
    let fallback_msg_id = extract_msg_id(&payload).unwrap_or_else(|| "unknown".to_string());
    let req: WriteRequest = match serde_json::from_slice::<WriteRequest>(&payload) {
        Ok(req)
            if !req.source.trim().is_empty()
                && !req.device.trim().is_empty()
                && !req.data_type.trim().is_empty()
                && !req.field.trim().is_empty()
                && !req.msg_id.trim().is_empty() =>
        {
            req
        },
        Ok(_) | Err(_) => {
            warn!(msg_id = %fallback_msg_id, "Invalid single-point write request");
            publish_write_reply(&state, "fail", "单点写入下发失败", fallback_msg_id).await;
            return;
        },
    };

    let modsrv_url = state.config.read().await.modsrv_url.clone();
    let url = format!("{}/api/instances/write", modsrv_url.trim_end_matches('/'));
    let response = state.http_client.post(url).json(&req).send().await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            info!(msg_id = %req.msg_id, "Cloud single-point write dispatched");
            publish_write_reply(&state, "success", "单点写入下发成功", req.msg_id).await;
        },
        Ok(resp) => {
            warn!(status = %resp.status(), msg_id = %req.msg_id, "modsrv rejected single-point write");
            publish_write_reply(&state, "fail", "单点写入下发失败", req.msg_id).await;
        },
        Err(e) => {
            warn!(error = %e, msg_id = %req.msg_id, "modsrv single-point write request failed");
            publish_write_reply(&state, "fail", "单点写入下发失败", req.msg_id).await;
        },
    }
}

async fn publish_write_reply(state: &AppState, result: &str, message: &str, msg_id: String) {
    let reply = WriteReply {
        timestamp: Utc::now().timestamp(),
        result: result.to_string(),
        message: message.to_string(),
        msg_id,
    };
    if let Err(e) = publish_json(state, &state.topics.write_reply, &reply).await {
        error!("Failed to publish write-reply: {}", e);
    }
}

async fn handle_call_data(state: Arc<AppState>, payload: Bytes) {
    let msg_id: Option<String> = serde_json::from_slice::<serde_json::Value>(&payload)
        .ok()
        .and_then(|v| {
            v.get("msgId")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        });

    // Reply first, then trigger the upload so the cloud gets an ACK immediately.
    let reply = CommandReply {
        result: "success".to_string(),
        message: "数据总召已启动".to_string(),
        timestamp: Utc::now().timestamp(),
        msg_id,
        error: None,
    };
    if let Err(e) = publish_json(&state, &state.topics.call_data_reply, &reply).await {
        error!("Failed to publish call-data-reply: {}", e);
    }

    crate::forwarder::upload_once(Arc::clone(&state)).await;
}

async fn handle_call_alarm(state: Arc<AppState>, payload: Bytes) {
    let msg_id: Option<String> = serde_json::from_slice::<serde_json::Value>(&payload)
        .ok()
        .and_then(|v| {
            v.get("msgId")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        });

    let alarmsrv_url = state.config.read().await.alarmsrv_url.clone();
    let url = format!("{}/alarmApi/call-data", alarmsrv_url);

    // POST to alarmsrv with msgId + timestamp in body (matches Python netsrv).
    let post_body = json!({
        "msgId": msg_id.as_deref().unwrap_or(""),
        "timestamp": Utc::now().timestamp()
    });

    let (result, message) = match state.http_client.post(&url).json(&post_body).send().await {
        Ok(resp) if resp.status().is_success() => {
            ("success".to_string(), "告警数据请求成功".to_string())
        },
        Ok(resp) => {
            let msg = format!("告警API返回状态码: {}", resp.status());
            warn!("call-alarm: {}", msg);
            ("warning".to_string(), msg)
        },
        Err(e) => {
            let msg = format!("告警API调用失败: {}", e);
            warn!("call-alarm: {}", msg);
            ("fail".to_string(), msg)
        },
    };

    let reply = CommandReply {
        result,
        message,
        timestamp: Utc::now().timestamp(),
        msg_id,
        error: None,
    };
    if let Err(e) = publish_json(&state, &state.topics.call_alarm_reply, &reply).await {
        error!("Failed to publish call-alarm-reply: {}", e);
    }
}

async fn publish_func_reply(state: &AppState, result: &str, message: &str, msg_id: String) {
    let reply = CommandReply {
        result: result.to_string(),
        message: message.to_string(),
        timestamp: Utc::now().timestamp(),
        msg_id: Some(msg_id),
        error: None,
    };
    if let Err(e) = publish_json(state, &state.topics.func_reply, &reply).await {
        error!("Failed to publish func-reply: {}", e);
    }
}

async fn handle_func(state: Arc<AppState>, payload: Bytes) {
    let fallback_msg_id = extract_msg_id(&payload).unwrap_or_else(|| "unknown".to_string());
    let req: FuncRequest = match serde_json::from_slice::<FuncRequest>(&payload) {
        Ok(req) if !req.func.is_empty() && !req.msg_id.is_empty() => req,
        Ok(_) | Err(_) => {
            warn!("Invalid gateway function request");
            publish_func_reply(&state, "fail", "指令格式错误", fallback_msg_id).await;
            return;
        },
    };

    if req.func != "reboot" {
        warn!(func = %req.func, msg_id = %req.msg_id, "Unsupported gateway function");
        publish_func_reply(&state, "fail", "指令不支持", req.msg_id).await;
        return;
    }

    let apigateway_url = state.config.read().await.apigateway_url.clone();
    let url = format!(
        "{}/api/v1/system/reboot",
        apigateway_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(url)
        .json(&json!({ "msgId": req.msg_id }))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            info!(msg_id = %req.msg_id, "Host reboot scheduled");
            publish_func_reply(&state, "success", "指令下发成功", req.msg_id).await;
        },
        Ok(resp) => {
            warn!(status = %resp.status(), msg_id = %req.msg_id, "Host reboot request rejected");
            publish_func_reply(&state, "fail", "指令下发失败", req.msg_id).await;
        },
        Err(e) => {
            warn!(error = %e, msg_id = %req.msg_id, "Host reboot request failed");
            publish_func_reply(&state, "fail", "指令下发失败", req.msg_id).await;
        },
    }
}

async fn handle_inst_sync(state: Arc<AppState>, payload: Bytes) {
    let msg_id: Option<String> = serde_json::from_slice::<serde_json::Value>(&payload)
        .ok()
        .and_then(|v| {
            v.get("msgId")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        });

    if let Err(e) = do_inst_sync(Arc::clone(&state), msg_id).await {
        error!("inst-sync failed: {}", e);
    }
}

/// Fetch the instance list and station topology from modsrv, then publish an
/// `inst-sync-reply`.
/// `msg_id` is echoed back verbatim; pass the ms-timestamp string for
/// HTTP-triggered calls.
pub async fn do_inst_sync(state: Arc<AppState>, msg_id: Option<String>) -> anyhow::Result<()> {
    let modsrv_url = state.config.read().await.modsrv_url.clone();
    let instances_url = format!("{}/api/instances/properties", modsrv_url);

    let list = match state.http_client.get(&instances_url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                let raw_list = body
                    .get("data")
                    .and_then(|d| d.get("list"))
                    .and_then(|l| l.as_array())
                    .cloned()
                    .unwrap_or_default();

                raw_list
                    .into_iter()
                    .filter_map(|item| {
                        let instance_id = item.get("instance_id")?.as_i64()?;
                        let instance_name = item.get("instance_name")?.as_str()?.to_string();
                        let product_name = item.get("product_name")?.as_str()?.to_string();
                        let property = item
                            .get("property")
                            .and_then(|p| p.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|entry| {
                                        Some(InstSyncProperty {
                                            id: entry.get("id")?.as_str()?.to_string(),
                                            value: entry
                                                .get("value")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        Some(InstSyncItem {
                            instance_id,
                            instance_name,
                            product_name,
                            property,
                        })
                    })
                    .collect::<Vec<_>>()
            },
            Err(e) => {
                return Err(anyhow::anyhow!("parse modsrv response: {}", e));
            },
        },
        Ok(resp) => {
            return Err(anyhow::anyhow!("modsrv returned status {}", resp.status()));
        },
        Err(e) => {
            return Err(anyhow::anyhow!("HTTP request to modsrv: {}", e));
        },
    };

    let topology_url = format!("{}/api/station/topology", modsrv_url);
    let flow_json = match state.http_client.get(&topology_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp
                .json::<serde_json::Value>()
                .await
                .map_err(|e| anyhow::anyhow!("parse modsrv topology response: {}", e))?;
            let flow_json = body
                .get("data")
                .and_then(|data| data.get("flow_json"))
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("modsrv topology response missing data.flow_json")
                })?;
            if !flow_json.is_object() {
                return Err(anyhow::anyhow!(
                    "modsrv topology data.flow_json must be an object"
                ));
            }
            flow_json
        },
        Ok(resp) => {
            return Err(anyhow::anyhow!(
                "modsrv topology returned status {}",
                resp.status()
            ));
        },
        Err(e) => {
            return Err(anyhow::anyhow!("HTTP request to modsrv topology: {}", e));
        },
    };

    let reply = InstSyncReply {
        msg_id,
        timestamp: Utc::now().timestamp(),
        list,
        flow_json,
    };

    publish_json(&state, &state.topics.inst_sync_reply, &reply).await
}

fn apply_packet_size_limit(options: &mut MqttOptions) {
    options.set_max_packet_size(MQTT_MAX_PACKET_SIZE_BYTES, MQTT_MAX_PACKET_SIZE_BYTES);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_redis_value(bytes: &Bytes) -> serde_json::Value {
    if let Ok(s) = std::str::from_utf8(bytes) {
        if let Ok(n) = s.parse::<f64>() {
            return serde_json::json!(n);
        }
        return serde_json::Value::String(s.to_string());
    }
    serde_json::Value::Null
}

fn extract_msg_id(payload: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(payload)
        .ok()?
        .get("msgId")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reboot_function_request() {
        let req: FuncRequest =
            serde_json::from_str(r#"{"func":"reboot","msgId":"123456"}"#).unwrap();
        assert_eq!(req.func, "reboot");
        assert_eq!(req.msg_id, "123456");
    }

    #[test]
    fn extracts_message_id_from_invalid_request() {
        assert_eq!(
            extract_msg_id(br#"{"func":7,"msgId":"123456"}"#).as_deref(),
            Some("123456")
        );
        assert_eq!(extract_msg_id(b"not-json"), None);
    }

    #[test]
    fn parses_single_point_write_without_restricting_source() {
        let req: WriteRequest = serde_json::from_str(
            r#"{"source":"future-source","device":"1","data_type":"A","key":"101","value":"123","msgId":"123456"}"#,
        )
        .unwrap();

        assert_eq!(req.source, "future-source");
        assert_eq!(req.msg_id, "123456");
    }

    #[test]
    fn serializes_complete_single_point_write_reply() {
        let reply = WriteReply {
            timestamp: 1_762_395_700,
            result: "success".to_string(),
            message: "单点写入下发成功".to_string(),
            msg_id: "123456".to_string(),
        };
        let value = serde_json::to_value(reply).unwrap();

        assert_eq!(value["timestamp"], 1_762_395_700);
        assert_eq!(value["result"], "success");
        assert_eq!(value["message"], "单点写入下发成功");
        assert_eq!(value["msgId"], "123456");
    }

    #[test]
    fn mqtt_options_allow_large_sync_payloads() {
        let mut options = MqttOptions::new("test", "localhost", 1883);
        apply_packet_size_limit(&mut options);

        assert_eq!(options.max_packet_size(), MQTT_MAX_PACKET_SIZE_BYTES);
    }
}
