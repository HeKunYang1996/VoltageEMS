//! HTTP broadcast to apigateway (6005) and netsrv (6006)

use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use tracing::warn;

use crate::db::AlarmCounts;
use crate::models::AlertRule;

fn point_label(point_name: Option<&str>, point_id: i64) -> String {
    point_name
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Point {}", point_id))
}

fn value_with_unit(value: f64, unit: Option<&str>) -> String {
    format!("{}{}", value, unit.unwrap_or_default())
}

fn format_trigger_message(
    rule: &AlertRule,
    current_value: f64,
    point_name: Option<&str>,
    unit: Option<&str>,
) -> String {
    format!(
        "{}: {} {} {} (Rule: {})",
        point_label(point_name, rule.point_id),
        value_with_unit(current_value, unit),
        rule.operator,
        value_with_unit(rule.value, unit),
        rule.rule_name
    )
}

fn format_recovery_message(
    rule: &AlertRule,
    recovery_value: Option<f64>,
    reason: &str,
    point_name: Option<&str>,
    unit: Option<&str>,
) -> String {
    let point_name = point_label(point_name, rule.point_id);
    match recovery_value {
        Some(value) => format!(
            "{}: {} (No longer satisfies {} {}) (Rule: {})",
            point_name,
            value_with_unit(value, unit),
            rule.operator,
            value_with_unit(rule.value, unit),
            rule.rule_name
        ),
        None => format!(
            "{}: - (Recovered: {}) (Rule: {})",
            point_name, reason, rule.rule_name
        ),
    }
}

pub struct Broadcaster {
    client: Client,
    apigateway_url: String,
    netsrv_url: String,
}

impl Broadcaster {
    pub fn new(client: Client, apigateway_url: String, netsrv_url: String) -> Self {
        Self {
            client,
            apigateway_url,
            netsrv_url,
        }
    }

    /// `device_name`/`point_name` are resolved by the caller (see
    /// `device_names::resolve_for_rule`) and added alongside the existing
    /// `device` field (which stays the raw channel/instance id, unchanged,
    /// so existing consumers keyed off it don't break).
    pub async fn send_alarm_triggered(
        &self,
        alert_id: i64,
        rule: &AlertRule,
        current_value: f64,
        device_name: Option<&str>,
        point_name: Option<&str>,
        unit: Option<&str>,
    ) {
        let ts = Utc::now().timestamp();
        let payload = serde_json::json!({
            "type": "alarm",
            "id": format!("alarm_{:03}", alert_id),
            "timestamp": ts,
            "data": {
                "alarm_id": alert_id.to_string(),
                "service_type": rule.service_type,
                "source": rule.service_type,
                "device": rule.channel_id.to_string(),
                "device_name": device_name,
                "channel_id": rule.channel_id,
                "data_type": rule.data_type,
                "point_id": rule.point_id,
                "point_name": point_name,
                "unit": unit,
                "status": 1,
                "level": rule.warning_level,
                "value": current_value,
                "message": format_trigger_message(rule, current_value, point_name, unit),
            }
        });
        self.broadcast_all(&payload).await;
    }

    pub async fn send_alarm_recovery(
        &self,
        alert_id: i64,
        rule: &AlertRule,
        recovery_value: Option<f64>,
        reason: &str,
        device_name: Option<&str>,
        point_name: Option<&str>,
        unit: Option<&str>,
    ) {
        let ts = Utc::now().timestamp();
        let message = format_recovery_message(rule, recovery_value, reason, point_name, unit);

        let payload = serde_json::json!({
            "type": "alarm",
            "id": format!("alarm_{:03}_recovery", alert_id),
            "timestamp": ts,
            "data": {
                "alarm_id": alert_id.to_string(),
                "service_type": rule.service_type,
                "source": rule.service_type,
                "device": rule.channel_id.to_string(),
                "device_name": device_name,
                "channel_id": rule.channel_id,
                "data_type": rule.data_type,
                "point_id": rule.point_id,
                "point_name": point_name,
                "unit": unit,
                "status": 0,
                "level": rule.warning_level,
                "value": recovery_value,
                "message": message,
            }
        });
        self.broadcast_all(&payload).await;
    }

    pub async fn send_alarm_count(&self, counts: &AlarmCounts) {
        let ts = Utc::now().timestamp();
        let payload = serde_json::json!({
            "type": "alarm_num",
            "id": format!("alarm_num_{}", ts),
            "timestamp": ts,
            "data": {
                "current_alarms": counts.total,
                "1": counts.low,
                "2": counts.medium,
                "3": counts.high,
                "update_time": ts,
                "server_id": "alarmsrv",
            }
        });
        self.broadcast_all(&payload).await;
    }

    async fn broadcast_all(&self, payload: &Value) {
        let urls = [
            format!("{}/api/v1/broadcast", self.apigateway_url),
            format!("{}/netApi/alarm/broadcast", self.netsrv_url),
        ];

        let futures = urls.iter().map(|url| {
            let url = url.clone();
            let client = self.client.clone();
            let payload = payload.clone();
            async move {
                match client
                    .post(&url)
                    .header(common::logging::INTERNAL_REQUEST_HEADER, "alarmsrv")
                    .json(&payload)
                    .timeout(std::time::Duration::from_secs(3))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {},
                    Ok(resp) => {
                        warn!("Broadcast failed: {} status={}", url, resp.status());
                    },
                    Err(e) => {
                        warn!("Broadcast error: {} err={}", url, e);
                    },
                }
            }
        });

        futures::future::join_all(futures).await;
    }

    /// Broadcast all currently active alerts (used by /call-data endpoint)
    pub async fn broadcast_active_alerts(
        &self,
        alerts: &[crate::models::Alert],
        rules: &std::collections::HashMap<i64, AlertRule>,
    ) {
        let mut futures_vec = Vec::new();
        for alert in alerts {
            if let Some(rule) = rules.get(&alert.rule_id) {
                let ts = Utc::now().timestamp();
                let payload = serde_json::json!({
                    "type": "alarm",
                    "id": format!("alarm_{:03}", alert.id),
                    "timestamp": ts,
                    "data": {
                        "alarm_id": alert.id.to_string(),
                        "service_type": rule.service_type,
                        "source": rule.service_type,
                        "device": rule.channel_id.to_string(),
                        "device_name": alert.device_name,
                        "channel_id": rule.channel_id,
                        "data_type": rule.data_type,
                        "point_id": rule.point_id,
                        "point_name": alert.point_name,
                        "unit": alert.unit,
                        "status": 1,
                        "level": rule.warning_level,
                        "value": alert.current_value,
                        "message": format_trigger_message(
                            rule,
                            alert.current_value,
                            alert.point_name.as_deref(),
                            alert.unit.as_deref(),
                        ),
                    }
                });
                // Only broadcast to netsrv for manual call-data
                let url = format!("{}/netApi/alarm/broadcast", self.netsrv_url);
                let client = self.client.clone();
                futures_vec.push(async move {
                    let _ = client
                        .post(&url)
                        .header(common::logging::INTERNAL_REQUEST_HEADER, "alarmsrv")
                        .json(&payload)
                        .timeout(std::time::Duration::from_secs(3))
                        .send()
                        .await;
                });
            }
        }
        futures::future::join_all(futures_vec).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> AlertRule {
        AlertRule {
            id: 1,
            service_type: "inst".to_string(),
            channel_id: 1,
            data_type: "M".to_string(),
            point_id: 7,
            rule_name: "Battery Overvoltage".to_string(),
            warning_level: 2,
            operator: ">".to_string(),
            value: 50.0,
            enabled: true,
            description: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn trigger_message_includes_point_unit_threshold_and_rule() {
        assert_eq!(
            format_trigger_message(&rule(), 100.0, Some("Voltage"), Some("V")),
            "Voltage: 100V > 50V (Rule: Battery Overvoltage)"
        );
    }

    #[test]
    fn realtime_recovery_message_uses_recovery_value() {
        assert_eq!(
            format_recovery_message(
                &rule(),
                Some(48.0),
                "Condition cleared",
                Some("Voltage"),
                Some("V")
            ),
            "Voltage: 48V (No longer satisfies > 50V) (Rule: Battery Overvoltage)"
        );
    }

    #[test]
    fn reason_recovery_message_does_not_invent_a_value() {
        assert_eq!(
            format_recovery_message(&rule(), None, "Rule disabled", Some("Voltage"), Some("V")),
            "Voltage: - (Recovered: Rule disabled) (Rule: Battery Overvoltage)"
        );
    }
}
