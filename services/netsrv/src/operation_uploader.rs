//! Publish persisted operation records through a dedicated QoS 1 MQTT client.
//! The dedicated connection lets us associate each PubAck with exactly one log row.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Result, anyhow};
use rumqttc::{
    AsyncClient, ConnectReturnCode, Event, Incoming, MqttOptions, NetworkOptions, Outgoing, QoS,
    Transport,
};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::models::{PropertyEntry, PropertyPayload};
use crate::state::AppState;

const BATCH_SIZE: i64 = 100;
const ACK_TIMEOUT: Duration = Duration::from_secs(30);

fn audit_client_id(base: &str) -> String {
    // MQTT 3.1.1 brokers may limit client IDs to 23 bytes.
    let hash = base
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("audit-{hash:016x}")
}

fn operation_payload(
    row: &common::operation_log::OperationRow,
    device_sn: &str,
) -> Result<PropertyPayload> {
    let mut value = HashMap::new();
    value.insert(
        "log".to_owned(),
        serde_json::Value::String(serde_json::to_string(row)?),
    );
    Ok(PropertyPayload {
        timestamp: row.occurred_at_ms.div_euclid(1000),
        property: vec![PropertyEntry {
            source: "gateway".to_owned(),
            device: device_sn.to_owned(),
            data_type: "L".to_owned(),
            value,
        }],
    })
}

pub async fn run(state: Arc<AppState>, shutdown: CancellationToken) {
    let mut last_prune = tokio::time::Instant::now() - Duration::from_secs(6 * 3600);
    loop {
        if shutdown.is_cancelled() {
            return;
        }

        if last_prune.elapsed() >= Duration::from_secs(6 * 3600) {
            let cutoff = common::operation_log::now_ms() - 30 * 86400 * 1000;
            if let Err(error) = common::operation_log::prune_uploaded(cutoff).await {
                error!("Operation log cleanup failed: {}", error);
            }
            if let Ok(pending) = common::operation_log::count_pending().await
                && pending > 100_000
            {
                warn!(pending, "Operation log upload backlog is large");
            }
            if let Ok(metadata) = tokio::fs::metadata(common::operation_log::database_path()).await
                && metadata.len() > 512 * 1024 * 1024
            {
                warn!(bytes = metadata.len(), "Operation log database is large");
            }
            last_prune = tokio::time::Instant::now();
        }

        if !state.disconnect_requested.load(Ordering::Relaxed) {
            match common::operation_log::pending(BATCH_SIZE).await {
                Ok(rows) if !rows.is_empty() => {
                    if let Err(error) = upload_batch(&state, rows).await {
                        warn!(
                            "Operation log MQTT upload failed, retaining rows: {}",
                            error
                        );
                        tokio::select! {
                            _ = sleep(Duration::from_secs(10)) => {}
                            _ = shutdown.cancelled() => return,
                        }
                        continue;
                    }
                    continue;
                },
                Err(error) => error!("Operation log pending query failed: {}", error),
                _ => {},
            }
        }
        tokio::select! {
            _ = sleep(Duration::from_secs(2)) => {}
            _ = shutdown.cancelled() => return,
        }
    }
}

async fn upload_batch(
    state: &AppState,
    rows: Vec<common::operation_log::OperationRow>,
) -> Result<()> {
    let cfg = state.config.read().await.clone();
    let base_client_id = if cfg.client_id == "auto" {
        &state.device.device_sn
    } else {
        &cfg.client_id
    };
    let mut options = MqttOptions::new(
        audit_client_id(base_client_id),
        &cfg.broker_host,
        cfg.broker_port,
    );
    options.set_keep_alive(Duration::from_secs(cfg.broker_keepalive_secs));
    options.set_clean_session(true);
    if let (Some(user), Some(password)) = (&cfg.username, &cfg.password)
        && !user.is_empty()
    {
        options.set_credentials(user, password);
    }
    if cfg.ssl_enabled {
        options.set_transport(Transport::tls_with_config(crate::mqtt::build_tls(
            &state.env.cert_dir,
        )?));
    }

    let (client, mut events) = AsyncClient::new(options, 8);
    let mut network = NetworkOptions::new();
    network.set_connection_timeout(30);
    events.set_network_options(network);
    loop {
        match timeout(ACK_TIMEOUT, events.poll()).await?? {
            Event::Incoming(Incoming::ConnAck(ack)) if ack.code == ConnectReturnCode::Success => {
                break;
            },
            Event::Incoming(Incoming::ConnAck(ack)) => {
                return Err(anyhow!("MQTT connection refused: {:?}", ack.code));
            },
            _ => {},
        }
    }

    for row in rows {
        let payload = serde_json::to_vec(&operation_payload(&row, &state.device.device_sn)?)?;
        client
            .publish(&state.topics.property, QoS::AtLeastOnce, false, payload)
            .await?;
        let mut packet_id = None;
        loop {
            let event = timeout(ACK_TIMEOUT, events.poll()).await??;
            match event {
                Event::Outgoing(Outgoing::Publish(id)) => packet_id = Some(id),
                Event::Incoming(Incoming::PubAck(ack)) if packet_id == Some(ack.pkid) => break,
                Event::Incoming(Incoming::Disconnect) => {
                    return Err(anyhow!("MQTT disconnected before PubAck"));
                },
                _ => {},
            }
        }
        common::operation_log::mark_uploaded(&row.event_id).await?;
    }
    let _ = client.disconnect().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_uses_existing_property_protocol() {
        let row = common::operation_log::OperationRow {
            event_id: "event-1".to_owned(),
            occurred_at_ms: 1_756_967_417_000,
            service: "comsrv".to_owned(),
            user_id: Some(7),
            username: Some("admin".to_owned()),
            actor_type: "user".to_owned(),
            source_service: None,
            method: "POST".to_owned(),
            route: "/api/channels".to_owned(),
            path: "/api/channels".to_owned(),
            status: 200,
            business_success: None,
            duration_ms: 12,
            request_id: None,
            request_body: Some("{\"name\":\"test\"}".to_owned()),
            response_body: Some("{\"success\":true}".to_owned()),
        };
        let payload =
            serde_json::to_value(operation_payload(&row, "20f0bcc6c1a8cddf").unwrap()).unwrap();
        assert_eq!(payload["timestamp"], 1_756_967_417_i64);
        assert_eq!(payload["property"][0]["source"], "gateway");
        assert_eq!(payload["property"][0]["device"], "20f0bcc6c1a8cddf");
        assert_eq!(payload["property"][0]["data_type"], "L");
        let log: serde_json::Value =
            serde_json::from_str(payload["property"][0]["value"]["log"].as_str().unwrap()).unwrap();
        assert_eq!(log["event_id"], "event-1");
        assert_eq!(log["username"], "admin");
        assert_eq!(log["request_body"], "{\"name\":\"test\"}");
        assert_eq!(log["response_body"], "{\"success\":true}");
        assert_eq!(
            crate::device::Topics::new("MonarchHub", "20f0bcc6c1a8cddf").property,
            "property/MonarchHub/20f0bcc6c1a8cddf"
        );
    }
}
