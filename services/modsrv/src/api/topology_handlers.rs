//! Station Topology API Handlers
//!
//! Provides endpoints for saving and querying the visual station topology
//! configured in PCManagement. The topology stores a Vue-Flow canvas JSON
//! that maps product nodes to device instances.
//!
//! Endpoints:
//! - `GET  /api/station/topology`                  — load saved topology
//! - `PUT  /api/station/topology`                  — upsert topology
//! - `GET  /api/station/topology/channel-bindings` — live channel mapping per product
//! - `GET  /api/instances/{id}/channel-summary`    — channels bound to one instance

#![allow(clippy::disallowed_methods)] // json! macro used in this module

use axum::{
    extract::{Path, State},
    response::Json,
};
use common::SuccessResponse;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::app_state::AppState;
use crate::error::ModSrvError;

// ============================================================================
// DTOs
// ============================================================================

/// Request body for PUT /api/station/topology
#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveTopologyRequest {
    pub station_name: Option<String>,
    pub description: Option<String>,
    pub gateway_id: Option<String>,
    /// Full Vue-Flow canvas JSON. Must contain at least `nodes` and `edges` array keys.
    #[schema(value_type = Object)]
    pub flow_json: Value,
}

/// A single channel binding for one product node returned by channel-bindings.
#[derive(Debug, Serialize)]
pub struct NodeBinding {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub instances: Vec<InstanceChannelInfo>,
}

#[derive(Debug, Serialize)]
pub struct InstanceChannelInfo {
    #[serde(rename = "instanceId")]
    pub instance_id: i64,
    #[serde(rename = "instanceName")]
    pub instance_name: String,
    #[serde(rename = "channelIds")]
    pub channel_ids: Vec<i64>,
}

// ============================================================================
// GET /api/station/topology
// ============================================================================

/// Get station topology
///
/// Returns the saved Vue-Flow canvas JSON. If no topology has been configured yet,
/// returns 200 with empty `nodes` / `edges` so callers can render a blank canvas
/// without special-casing a 404.
#[utoipa::path(
    get,
    path = "/api/station/topology",
    responses(
        (status = 200, description = "Station topology (empty nodes/edges if not configured yet)", body = serde_json::Value,
            example = json!({
                "station_id": "station",
                "station_name": "Edge Station #1",
                "description": null,
                "gateway_id": null,
                "flow_json": { "nodes": [], "edges": [] },
                "created_at": "2026-06-01 08:00:00",
                "updated_at": "2026-06-09 10:30:00"
            })
        ),
        (status = 500, description = "Database error")
    ),
    tag = "topology"
)]
pub async fn get_station_topology(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<Value>>, ModSrvError> {
    let pool = &state.instance_manager.pool;

    let row = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<String>, String, String, String)>(
        r#"SELECT id, station_id, station_name, description, gateway_id, flow_json, created_at, updated_at
           FROM station_topology WHERE station_id = 'station'"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let resp = match row {
        Some((
            _,
            station_id,
            station_name,
            description,
            gateway_id,
            flow_json_str,
            created_at,
            updated_at,
        )) => {
            let flow_json: Value = serde_json::from_str(&flow_json_str).map_err(|e| {
                ModSrvError::SerializationError(format!("Invalid flow_json in DB: {}", e))
            })?;
            json!({
                "station_id": station_id,
                "station_name": station_name,
                "description": description,
                "gateway_id": gateway_id,
                "flow_json": flow_json,
                "created_at": created_at,
                "updated_at": updated_at
            })
        },
        None => json!({
            "station_id": "station",
            "station_name": "Edge Station",
            "description": null,
            "gateway_id": null,
            "flow_json": { "nodes": [], "edges": [] },
            "created_at": null,
            "updated_at": null
        }),
    };

    Ok(Json(SuccessResponse::new(resp)))
}

// ============================================================================
// PUT /api/station/topology
// ============================================================================

/// Save (upsert) station topology
///
/// Overwrites the current topology. Uses upsert semantics — there is exactly
/// one topology record per station. Validates that `flow_json` contains
/// `nodes` and `edges` arrays before persisting.
#[utoipa::path(
    put,
    path = "/api/station/topology",
    request_body = SaveTopologyRequest,
    responses(
        (status = 200, description = "Saved topology (same shape as GET response)", body = serde_json::Value),
        (status = 400, description = "flow_json missing nodes/edges keys"),
        (status = 500, description = "Database error")
    ),
    tag = "topology"
)]
pub async fn put_station_topology(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveTopologyRequest>,
) -> Result<Json<SuccessResponse<Value>>, ModSrvError> {
    // Validate flow_json structure
    if req.flow_json.get("nodes").is_none() || req.flow_json.get("edges").is_none() {
        return Err(ModSrvError::InvalidData(
            "flow_json must contain 'nodes' and 'edges' keys".to_string(),
        ));
    }
    let nodes = req.flow_json["nodes"]
        .as_array()
        .ok_or_else(|| ModSrvError::InvalidData("flow_json.nodes must be an array".to_string()))?;
    let edges = req.flow_json["edges"]
        .as_array()
        .ok_or_else(|| ModSrvError::InvalidData("flow_json.edges must be an array".to_string()))?;
    let _ = (nodes, edges); // validation only

    let flow_json_str = serde_json::to_string(&req.flow_json)
        .map_err(|e| ModSrvError::SerializationError(e.to_string()))?;

    let station_name = req.station_name.as_deref().unwrap_or("Edge Station");
    let pool = &state.instance_manager.pool;

    sqlx::query(
        r#"INSERT INTO station_topology (station_id, station_name, description, gateway_id, flow_json, updated_at)
           VALUES ('station', ?, ?, ?, ?, datetime('now'))
           ON CONFLICT(station_id) DO UPDATE SET
               station_name = excluded.station_name,
               description  = excluded.description,
               gateway_id   = excluded.gateway_id,
               flow_json    = excluded.flow_json,
               updated_at   = excluded.updated_at"#,
    )
    .bind(station_name)
    .bind(&req.description)
    .bind(&req.gateway_id)
    .bind(&flow_json_str)
    .execute(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    // Return the freshly saved record
    get_station_topology(State(state)).await
}

// ============================================================================
// GET /api/station/topology/channel-bindings
// ============================================================================

/// Get channel bindings (live join)
///
/// Returns the channel IDs currently bound to each product node in the topology.
/// Channel IDs are resolved by a live join against `measurement_routing` — not
/// from stale values stored in `flow_json` — so the result always reflects the
/// current routing configuration even if channels changed after the last save.
///
/// Nodes without any bound instances are omitted from the response.
#[utoipa::path(
    get,
    path = "/api/station/topology/channel-bindings",
    responses(
        (status = 200, description = "Channel bindings per product node", body = serde_json::Value,
            example = json!({
                "bindings": [
                    {
                        "nodeId": "vm_node_battery",
                        "productName": "Battery",
                        "instances": [{ "instanceId": 1, "instanceName": "battery_01", "channelIds": [2] }]
                    },
                    {
                        "nodeId": "vm_node_diesel",
                        "productName": "Diesel",
                        "instances": [{ "instanceId": 2, "instanceName": "diesel_gen_01", "channelIds": [3] }]
                    }
                ]
            })
        ),
        (status = 500, description = "Database error")
    ),
    tag = "topology"
)]
pub async fn get_channel_bindings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<Value>>, ModSrvError> {
    let pool = &state.instance_manager.pool;

    // 1. Load flow_json
    let flow_json_str: Option<String> =
        sqlx::query_scalar("SELECT flow_json FROM station_topology WHERE station_id = 'station'")
            .fetch_optional(pool)
            .await
            .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let flow_json_str = match flow_json_str {
        Some(s) => s,
        None => return Ok(Json(SuccessResponse::new(json!({ "bindings": [] })))),
    };

    let flow: Value = serde_json::from_str(&flow_json_str)
        .map_err(|e| ModSrvError::SerializationError(format!("Invalid flow_json in DB: {}", e)))?;

    let nodes = match flow.get("nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => return Ok(Json(SuccessResponse::new(json!({ "bindings": [] })))),
    };

    // 2. Extract (nodeId, productName, [(instanceId, instanceName)]) for nodes that have instances
    struct NodeMeta {
        node_id: String,
        product_name: String,
        instances: Vec<(i64, String)>,
    }

    let mut node_metas: Vec<NodeMeta> = Vec::new();
    let mut all_instance_ids: Vec<i64> = Vec::new();

    for node in nodes {
        let node_id = match node.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let data = match node.get("data") {
            Some(d) => d,
            None => continue,
        };
        let product_name = match data.get("productName").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        let instances_arr = match data.get("instances").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => continue,
        };

        let mut inst_pairs: Vec<(i64, String)> = Vec::new();
        for inst in instances_arr {
            let iid = match inst.get("instanceId").and_then(|v| v.as_i64()) {
                Some(id) => id,
                None => continue,
            };
            let iname = inst
                .get("instanceName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            inst_pairs.push((iid, iname));
            all_instance_ids.push(iid);
        }

        if !inst_pairs.is_empty() {
            node_metas.push(NodeMeta {
                node_id,
                product_name,
                instances: inst_pairs,
            });
        }
    }

    if node_metas.is_empty() {
        return Ok(Json(SuccessResponse::new(json!({ "bindings": [] }))));
    }

    // 3. Batch-query measurement_routing for all relevant instance IDs (single query)
    //
    // SQLite doesn't support array binding, so we build the IN list manually.
    // instance IDs are integer database primary keys — no SQL injection risk.
    let id_list = all_instance_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let routing_rows: Vec<(i64, i64)> = sqlx::query_as(&format!(
        "SELECT DISTINCT instance_id, channel_id FROM measurement_routing \
             WHERE instance_id IN ({}) AND enabled = 1 AND channel_id IS NOT NULL",
        id_list
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    // instance_id → [channel_id] map
    let mut channel_map: HashMap<i64, Vec<i64>> = HashMap::new();
    for (inst_id, ch_id) in routing_rows {
        channel_map.entry(inst_id).or_default().push(ch_id);
    }

    // 4. Build response
    let bindings: Vec<Value> = node_metas
        .into_iter()
        .map(|meta| {
            let instances: Vec<Value> = meta
                .instances
                .into_iter()
                .map(|(iid, iname)| {
                    let channel_ids = channel_map.get(&iid).cloned().unwrap_or_default();
                    json!({
                        "instanceId": iid,
                        "instanceName": iname,
                        "channelIds": channel_ids
                    })
                })
                .collect();
            json!({
                "nodeId": meta.node_id,
                "productName": meta.product_name,
                "instances": instances
            })
        })
        .collect();

    Ok(Json(SuccessResponse::new(json!({ "bindings": bindings }))))
}

// ============================================================================
// GET /api/instances/{id}/channel-summary
// ============================================================================

/// Get instance channel summary
///
/// Returns the distinct channels currently routed to a given instance.
/// Intended for PCManagement: call after the user selects an instance to
/// auto-fill `channelIds` in the topology node panel. Data is read from
/// the SQLite routing table, not Redis.
#[utoipa::path(
    get,
    path = "/api/instances/{id}/channel-summary",
    params(
        ("id" = u32, Path, description = "Instance ID")
    ),
    responses(
        (status = 200, description = "Channel summary for the instance", body = serde_json::Value,
            example = json!({
                "instanceId": 1,
                "instanceName": "battery_01",
                "channelIds": [2],
                "channelNames": ["Battery BMS Channel"],
                "routingCount": 24
            })
        ),
        (status = 404, description = "Instance not found"),
        (status = 500, description = "Database error")
    ),
    tag = "topology"
)]
pub async fn get_instance_channel_summary(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<SuccessResponse<Value>>, ModSrvError> {
    let pool = &state.instance_manager.pool;

    // Verify instance exists and fetch name
    let instance_name: Option<String> =
        sqlx::query_scalar("SELECT instance_name FROM instances WHERE instance_id = ?")
            .bind(id as i64)
            .fetch_optional(pool)
            .await
            .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let instance_name = instance_name
        .ok_or_else(|| ModSrvError::InstanceNotFound(format!("Instance {} not found", id)))?;

    // Distinct channels with name lookup and total routing point count
    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        r#"SELECT DISTINCT mr.channel_id, c.name
           FROM measurement_routing mr
           LEFT JOIN channels c ON mr.channel_id = c.channel_id
           WHERE mr.instance_id = ? AND mr.enabled = 1 AND mr.channel_id IS NOT NULL
           ORDER BY mr.channel_id"#,
    )
    .bind(id as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let routing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM measurement_routing WHERE instance_id = ? AND enabled = 1",
    )
    .bind(id as i64)
    .fetch_one(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let channel_ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
    let channel_names: Vec<Option<String>> = rows.into_iter().map(|(_, name)| name).collect();

    Ok(Json(SuccessResponse::new(json!({
        "instanceId": id,
        "instanceName": instance_name,
        "channelIds": channel_ids,
        "channelNames": channel_names,
        "routingCount": routing_count
    }))))
}
