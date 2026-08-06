//! Cloud Sync API Handlers
//!
//! Endpoints for cloud-edge synchronization:
//! - GET /api/instances/export   - Export instance topology to cloud
//! - GET /api/instances/properties - Instance list with full property values (for MQTT inst-sync)

#![allow(clippy::disallowed_methods)] // json! macro used in multiple functions

use axum::{extract::State, response::Json};
use common::SuccessResponse;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::error::ModSrvError;

/// Instance export item (edge → cloud sync)
#[derive(Debug, Serialize)]
pub struct InstanceExport {
    pub id: u32,
    pub name: String,
    pub product: String,
    pub parent_id: Option<u32>,
    pub properties: serde_json::Value,
}

/// Instance topology export response
#[derive(Debug, Serialize)]
pub struct InstanceTopology {
    pub version: String,
    pub instances: Vec<InstanceExport>,
}

/// Export instance topology for cloud sync
///
/// Returns all instances with their topology (parent_id) and properties.
/// Used for edge → cloud synchronization.
///
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/instances/export",
    tag = "instances",
    responses(
        (status = 200, description = "Instance topology exported",
            body = inline(Object),
            example = json!({
                "success": true,
                "data": {
                    "version": "1.0.0",
                    "instances": [
                        {"id": 1, "name": "pv_001", "product": "pv_inverter", "parent_id": null, "properties": {}}
                    ]
                }
            })
        ),
        (status = 500, description = "Database error")
    )
))]
pub async fn export_instances(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<InstanceTopology>>, ModSrvError> {
    let pool = &state.instance_manager.pool;

    // Query all instances with parent_id
    #[allow(clippy::type_complexity)]
    let rows: Vec<(u32, String, String, Option<u32>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT instance_id, instance_name, product_name, parent_id, properties
        FROM instances
        ORDER BY instance_id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ModSrvError::InternalError(format!("Failed to query instances: {}", e)))?;

    let instances: Vec<InstanceExport> = rows
        .into_iter()
        .map(|(id, name, product, parent_id, props_json)| {
            let properties = props_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(json!({}));

            InstanceExport {
                id,
                name,
                product,
                parent_id,
                properties,
            }
        })
        .collect();

    // Use a fixed version for edge export
    let version = "1.0.0".to_string();

    Ok(Json(SuccessResponse::new(InstanceTopology {
        version,
        instances,
    })))
}

// ── GET /api/instances/properties ────────────────────────────────────────────

/// One property entry in an `inst-sync-reply` property list.
#[derive(Debug, Serialize)]
pub struct InstPropertyItem {
    /// Property name from the product template (e.g. `"Longitude"`).
    pub id: String,
    /// Current value as a string; `""` when the property has not been set.
    pub value: String,
}

/// One instance entry in the `/api/instances/properties` response.
#[derive(Debug, Serialize)]
pub struct InstPropertiesEntry {
    pub instance_id: u32,
    pub instance_name: String,
    pub product_name: String,
    /// All properties defined in the product template.
    /// Each entry carries the current value or an empty string if not set.
    pub property: Vec<InstPropertyItem>,
}

/// List all instances with their full property values.
///
/// Returns every instance together with **all** property names defined in its
/// product template. If a property has never been set on a specific instance
/// its `value` is `""`. Intended for the MQTT `inst-sync-reply` payload.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/instances/properties",
    tag = "instances",
    responses(
        (status = 200, description = "Instance property list",
            body = inline(Object),
            example = json!({
                "success": true,
                "data": {
                    "list": [
                        {
                            "instance_id": 1,
                            "instance_name": "battery_01",
                            "product_name": "Battery",
                            "property": [
                                {"id": "Longitude", "value": "12.21"},
                                {"id": "Latitude",  "value": ""}
                            ]
                        }
                    ]
                }
            })
        ),
        (status = 500, description = "Database error")
    )
))]
pub async fn list_instances_properties(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    let pool = &state.instance_manager.pool;

    // Step 1: fetch all instances (id, name, product_name)
    let rows: Vec<(u32, String, String)> = sqlx::query_as(
        "SELECT instance_id, instance_name, product_name FROM instances ORDER BY instance_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ModSrvError::InternalError(format!("Failed to query instances: {}", e)))?;

    if rows.is_empty() {
        return Ok(Json(SuccessResponse::new(json!({ "list": [] }))));
    }

    // Step 2: batch-load all set property values in a single query (avoids N+1)
    let id_product_pairs: Vec<(u32, String)> = rows
        .iter()
        .map(|(id, _, product)| (*id, product.clone()))
        .collect();
    let mut props_batch = state
        .instance_manager
        .fetch_properties_batch(&id_product_pairs)
        .await
        .unwrap_or_default();

    // Step 3: for each instance, merge template definitions with actual values
    let mut product_cache: HashMap<String, Arc<crate::product_loader::Product>> = HashMap::new();
    let mut list: Vec<InstPropertiesEntry> = Vec::with_capacity(rows.len());

    for (instance_id, instance_name, product_name) in rows {
        let product = if let Some(cached) = product_cache.get(&product_name) {
            Arc::clone(cached)
        } else {
            let p = Arc::new(
                state
                    .instance_manager
                    .product_loader()
                    .get_product(&product_name)
                    .map_err(|e| {
                        ModSrvError::InternalError(format!(
                            "Failed to load product {}: {}",
                            product_name, e
                        ))
                    })?,
            );
            product_cache.insert(product_name.clone(), Arc::clone(&p));
            p
        };

        // Values set for this instance (name → JSON value); absent = not set
        let instance_props = props_batch.remove(&instance_id).unwrap_or_default();

        let property = product
            .properties
            .iter()
            .map(|tpl| {
                let value = match instance_props.get(&tpl.name) {
                    Some(v) => json_value_to_string(v),
                    None => String::new(),
                };
                InstPropertyItem {
                    id: tpl.name.clone(),
                    value,
                }
            })
            .collect();

        list.push(InstPropertiesEntry {
            instance_id,
            instance_name,
            product_name,
            property,
        });
    }

    Ok(Json(SuccessResponse::new(json!({ "list": list }))))
}

/// Convert a JSON value to its string representation for cloud sync payloads.
/// JSON strings are returned as-is; numbers/booleans are rendered as strings;
/// null and arrays/objects fall back to the JSON serialisation.
fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
