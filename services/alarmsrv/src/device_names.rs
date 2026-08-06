//! Resolve human-readable device/point names for alert display.
//!
//! Alarm rules bind to a `(service_type, channel_id, data_type, point_id)`
//! Redis polling target, not a name — `channel_id`/`point_id` are the only
//! thing stored on `alert_rule`/`alert`/`alert_event`. This module answers
//! "what is that, in human terms" for the two backing models `service_type`
//! can point at:
//! - `"inst"`: `channel_id` is actually an `instance_id` (see
//!   `AlertRule::redis_key`); point metadata comes from the compile-time
//!   product library (`voltage-model`), keyed by the instance's `product_name`.
//! - `"comsrv"`: `channel_id` is a channel; point metadata comes from the
//!   per-protocol point tables (`telemetry_points`/`signal_points`/
//!   `control_points`/`adjustment_points`), keyed by `(channel_id, point_id)`.

use sqlx::SqlitePool;
use voltage_model::product_lib::get_builtin_product;

use crate::models::AlertRule;

#[derive(Debug, Clone, Default)]
pub struct DeviceNames {
    pub device_name: Option<String>,
    pub point_name: Option<String>,
    pub unit: Option<String>,
}

impl DeviceNames {
    pub fn device_name_ref(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    pub fn point_name_ref(&self) -> Option<&str> {
        self.point_name.as_deref()
    }

    pub fn unit_ref(&self) -> Option<&str> {
        self.unit.as_deref()
    }
}

/// Convenience wrapper for the common case of resolving names for a whole
/// `AlertRule` row (as opposed to a raw tuple).
pub async fn resolve_for_rule(pool: &SqlitePool, rule: &AlertRule) -> DeviceNames {
    resolve(
        pool,
        &rule.service_type,
        rule.channel_id,
        &rule.data_type,
        rule.point_id,
    )
    .await
}

/// Resolve device/point names for a `(service_type, channel_id, data_type,
/// point_id)` tuple. Best-effort: returns `None` fields (never an error) when
/// the device/point can't be found — a rule pointing at a since-deleted
/// device shouldn't break the rule list, just show blank names.
pub async fn resolve(
    pool: &SqlitePool,
    service_type: &str,
    channel_id: i64,
    data_type: &str,
    point_id: i64,
) -> DeviceNames {
    if service_type == "inst" {
        resolve_instance(pool, channel_id, data_type, point_id).await
    } else {
        resolve_channel(pool, channel_id, data_type, point_id).await
    }
}

async fn resolve_instance(
    pool: &SqlitePool,
    instance_id: i64,
    data_type: &str,
    point_id: i64,
) -> DeviceNames {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT instance_name, product_name FROM instances WHERE instance_id = ?")
            .bind(instance_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let Some((instance_name, product_name)) = row else {
        return DeviceNames::default();
    };

    let point = get_builtin_product(&product_name).and_then(|product| {
        let is_action = matches!(data_type, "A" | "action");
        let points = if is_action {
            &product.actions
        } else {
            &product.measurements
        };
        points.iter().find(|p| p.id == point_id as u32)
    });

    DeviceNames {
        device_name: Some(instance_name),
        point_name: point.map(|p| p.name.clone()),
        unit: point.map(|p| p.unit.clone()).filter(|u| !u.is_empty()),
    }
}

async fn resolve_channel(
    pool: &SqlitePool,
    channel_id: i64,
    data_type: &str,
    point_id: i64,
) -> DeviceNames {
    let device_name: Option<String> =
        sqlx::query_scalar("SELECT name FROM channels WHERE channel_id = ?")
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    // Channel-online sentinel rules (see AlertRule::CHANNEL_ONLINE_DATA_TYPE)
    // monitor the channel itself, not a real point — nothing to look up.
    if data_type == AlertRule::CHANNEL_ONLINE_DATA_TYPE {
        return DeviceNames {
            device_name,
            point_name: None,
            unit: None,
        };
    }

    let Some(table) = point_table_for_data_type(data_type) else {
        return DeviceNames {
            device_name,
            point_name: None,
            unit: None,
        };
    };

    let row: Option<(String, Option<String>)> = sqlx::query_as(&format!(
        "SELECT signal_name, unit FROM {table} WHERE channel_id = ? AND point_id = ?"
    ))
    .bind(channel_id)
    .bind(point_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    match row {
        Some((point_name, unit)) => DeviceNames {
            device_name,
            point_name: Some(point_name),
            unit: unit.filter(|u| !u.is_empty()),
        },
        None => DeviceNames {
            device_name,
            point_name: None,
            unit: None,
        },
    }
}

/// Maps a channel point's `data_type` to the table that holds its
/// `signal_name`/`unit`. `table` is only ever one of these four fixed
/// literals — never interpolated from user input — so building the SQL
/// string with `format!` here is safe.
fn point_table_for_data_type(data_type: &str) -> Option<&'static str> {
    match data_type {
        "T" => Some("telemetry_points"),
        "S" => Some("signal_points"),
        "C" => Some("control_points"),
        "A" => Some("adjustment_points"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::test_utils::schema::{init_comsrv_schema, init_modsrv_schema};
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .expect("open in-memory sqlite");
        init_comsrv_schema(&pool).await.expect("comsrv schema");
        init_modsrv_schema(&pool).await.expect("modsrv schema");
        pool
    }

    #[tokio::test]
    async fn resolves_instance_device_and_point_name() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO instances (instance_id, instance_name, product_name) VALUES (?, ?, ?)",
        )
        .bind(42i64)
        .bind("ESS-Battery-1")
        .bind("Battery")
        .execute(&pool)
        .await
        .unwrap();

        // Battery.M[6] = {"id":7,"name":"SOC","unit":"%"}
        let names = resolve(&pool, "inst", 42, "M", 7).await;
        assert_eq!(names.device_name, Some("ESS-Battery-1".to_string()));
        assert_eq!(names.point_name, Some("SOC".to_string()));
        assert_eq!(names.unit, Some("%".to_string()));
    }

    #[tokio::test]
    async fn resolves_channel_device_and_point_name() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO channels (channel_id, name) VALUES (?, ?)")
            .bind(1001i64)
            .bind("PLC-1")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO telemetry_points (channel_id, point_id, signal_name, unit) VALUES (?, ?, ?, ?)",
        )
        .bind(1001i64)
        .bind(5i64)
        .bind("Bus Voltage")
        .bind("V")
        .execute(&pool)
        .await
        .unwrap();

        let names = resolve(&pool, "comsrv", 1001, "T", 5).await;
        assert_eq!(names.device_name, Some("PLC-1".to_string()));
        assert_eq!(names.point_name, Some("Bus Voltage".to_string()));
        assert_eq!(names.unit, Some("V".to_string()));
    }

    #[tokio::test]
    async fn channel_online_rule_has_device_name_but_no_point() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO channels (channel_id, name) VALUES (?, ?)")
            .bind(1001i64)
            .bind("PLC-1")
            .execute(&pool)
            .await
            .unwrap();

        let names = resolve(
            &pool,
            "comsrv",
            1001,
            AlertRule::CHANNEL_ONLINE_DATA_TYPE,
            0,
        )
        .await;
        assert_eq!(names.device_name, Some("PLC-1".to_string()));
        assert_eq!(names.point_name, None);
    }

    #[tokio::test]
    async fn missing_device_resolves_to_none_without_error() {
        let pool = test_pool().await;
        let names = resolve(&pool, "inst", 999, "M", 1).await;
        assert_eq!(names.device_name, None);
        assert_eq!(names.point_name, None);

        let names = resolve(&pool, "comsrv", 999, "T", 1).await;
        assert_eq!(names.device_name, None);
        assert_eq!(names.point_name, None);
    }
}
