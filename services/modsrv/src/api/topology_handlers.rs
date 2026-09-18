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
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::app_state::AppState;
use crate::error::ModSrvError;

// ============================================================================
// DTOs
// ============================================================================

/// Request body for PUT /api/station/topology
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveTopologyRequest {
    pub station_id: String,
    pub station_name: String,
    pub flow_json: TopologyFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TopologyFlow {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    #[serde(rename = "fixedBindings")]
    pub fixed_bindings: FixedBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TopologyNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub position: NodePosition,
    pub data: TopologyNodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyNodeData {
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub product_name: String,
    pub instance_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyEdge {
    pub id: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub source: String,
    pub target: String,
    pub source_handle: String,
    pub target_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixedBindings {
    pub station_instance_id: Option<i64>,
    pub environment_instance_id: Option<i64>,
}

async fn validate_final_topology(state: &AppState, flow: &TopologyFlow) -> Result<(), ModSrvError> {
    let mut node_ids = HashSet::new();
    let mut bound_instance_ids = HashSet::new();
    let mut node_products = HashMap::new();

    for node in &flow.nodes {
        if node.id.is_empty() || !node_ids.insert(node.id.as_str()) {
            return Err(ModSrvError::InvalidData(format!(
                "topology node id '{}' is empty or duplicated",
                node.id
            )));
        }
        if node.node_type != "product" {
            return Err(ModSrvError::InvalidData(format!(
                "node '{}' type must be 'product'",
                node.id
            )));
        }
        if !node.position.x.is_finite() || !node.position.y.is_finite() {
            return Err(ModSrvError::InvalidData(format!(
                "node '{}' position must be finite",
                node.id
            )));
        }
        if node.data.instance_id <= 0 || !bound_instance_ids.insert(node.data.instance_id) {
            return Err(ModSrvError::InvalidData(format!(
                "node '{}' instanceId must be positive and unique",
                node.id
            )));
        }
        let product = state
            .instance_manager
            .product_loader()
            .get_product(&node.data.product_name)
            .map_err(|_| {
                ModSrvError::InvalidData(format!(
                    "node '{}' references unknown product '{}'",
                    node.id, node.data.product_name
                ))
            })?;
        if product.topology.is_none() {
            return Err(ModSrvError::InvalidData(format!(
                "product '{}' does not participate in the energy topology",
                node.data.product_name
            )));
        }
        node_products.insert(node.id.as_str(), product);
    }

    let fixed = [
        (flow.fixed_bindings.station_instance_id, "Station"),
        (flow.fixed_bindings.environment_instance_id, "Env"),
    ];
    for (instance_id, product_name) in fixed {
        if let Some(instance_id) = instance_id
            && (instance_id <= 0 || !bound_instance_ids.insert(instance_id))
        {
            return Err(ModSrvError::InvalidData(format!(
                "fixed binding for '{}' must be positive and unique",
                product_name
            )));
        }
    }

    if !bound_instance_ids.is_empty() {
        let id_list = bound_instance_ids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let rows: Vec<(i64, String)> = sqlx::query_as(&format!(
            "SELECT instance_id, product_name FROM instances WHERE instance_id IN ({id_list})"
        ))
        .fetch_all(&state.instance_manager.pool)
        .await
        .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;
        let actual: HashMap<i64, String> = rows.into_iter().collect();

        for node in &flow.nodes {
            match actual.get(&node.data.instance_id) {
                Some(product_name) if product_name == &node.data.product_name => {},
                Some(product_name) => {
                    return Err(ModSrvError::InvalidData(format!(
                        "instance {} is product '{}', not '{}'",
                        node.data.instance_id, product_name, node.data.product_name
                    )));
                },
                None => {
                    return Err(ModSrvError::InvalidData(format!(
                        "instance {} does not exist",
                        node.data.instance_id
                    )));
                },
            }
        }
        for (instance_id, expected_product) in fixed {
            if let Some(instance_id) = instance_id {
                match actual.get(&instance_id) {
                    Some(product_name) if product_name == expected_product => {},
                    Some(product_name) => {
                        return Err(ModSrvError::InvalidData(format!(
                            "fixed binding {} is product '{}', not '{}'",
                            instance_id, product_name, expected_product
                        )));
                    },
                    None => {
                        return Err(ModSrvError::InvalidData(format!(
                            "fixed binding instance {} does not exist",
                            instance_id
                        )));
                    },
                }
            }
        }
    }

    let mut edge_ids = HashSet::new();
    let mut node_pairs = HashSet::new();
    let mut rule_counts: HashMap<(&str, usize), u32> = HashMap::new();

    for edge in &flow.edges {
        if edge.id.is_empty() || !edge_ids.insert(edge.id.as_str()) {
            return Err(ModSrvError::InvalidData(format!(
                "topology edge id '{}' is empty or duplicated",
                edge.id
            )));
        }
        if edge.edge_type != "deletable-smoothstep"
            || !is_valid_handle(&edge.source_handle)
            || !is_valid_handle(&edge.target_handle)
        {
            return Err(ModSrvError::InvalidData(format!(
                "edge '{}' has an invalid type or handle",
                edge.id
            )));
        }
        if edge.source == edge.target {
            return Err(ModSrvError::InvalidData(format!(
                "edge '{}' cannot be a self-loop",
                edge.id
            )));
        }
        let source_product = node_products.get(edge.source.as_str()).ok_or_else(|| {
            ModSrvError::InvalidData(format!(
                "edge '{}' references missing source node '{}'",
                edge.id, edge.source
            ))
        })?;
        let target_product = node_products.get(edge.target.as_str()).ok_or_else(|| {
            ModSrvError::InvalidData(format!(
                "edge '{}' references missing target node '{}'",
                edge.id, edge.target
            ))
        })?;
        let pair = if edge.source < edge.target {
            (edge.source.as_str(), edge.target.as_str())
        } else {
            (edge.target.as_str(), edge.source.as_str())
        };
        if !node_pairs.insert(pair) {
            return Err(ModSrvError::InvalidData(format!(
                "edge '{}' duplicates an existing node pair",
                edge.id
            )));
        }

        let source_rule =
            matching_rule(source_product, &target_product.product_name).ok_or_else(|| {
                ModSrvError::InvalidData(format!(
                    "product '{}' does not allow connection to '{}'",
                    source_product.product_name, target_product.product_name
                ))
            })?;
        let target_rule =
            matching_rule(target_product, &source_product.product_name).ok_or_else(|| {
                ModSrvError::InvalidData(format!(
                    "product '{}' does not allow connection to '{}'",
                    target_product.product_name, source_product.product_name
                ))
            })?;
        *rule_counts
            .entry((edge.source.as_str(), source_rule))
            .or_default() += 1;
        *rule_counts
            .entry((edge.target.as_str(), target_rule))
            .or_default() += 1;
    }

    for node in &flow.nodes {
        let product = &node_products[node.id.as_str()];
        let Some(topology) = product.topology.as_ref() else {
            return Err(ModSrvError::InvalidData(format!(
                "product '{}' does not participate in the energy topology",
                product.product_name
            )));
        };
        for (index, rule) in topology.connections.iter().enumerate() {
            let count = rule_counts
                .get(&(node.id.as_str(), index))
                .copied()
                .unwrap_or(0);
            if count < rule.min || rule.max.is_some_and(|max| count > max) {
                return Err(ModSrvError::InvalidData(format!(
                    "node '{}' connection group {:?} requires min {} and max {:?}, got {}",
                    node.id, rule.products, rule.min, rule.max, count
                )));
            }
        }
    }

    Ok(())
}

fn is_valid_handle(handle: &str) -> bool {
    matches!(handle, "top" | "bottom" | "left" | "right")
}

fn matching_rule(product: &crate::config::Product, peer_name: &str) -> Option<usize> {
    product
        .topology
        .as_ref()?
        .connections
        .iter()
        .position(|rule| {
            rule.products
                .iter()
                .any(|product_name| product_name == peer_name)
        })
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
                "station_name": "Station",
                "flow_json": {
                    "nodes": [],
                    "edges": [],
                    "fixedBindings": {
                        "stationInstanceId": null,
                        "environmentInstanceId": null
                    }
                }
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

    let row = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT station_id, station_name, flow_json
           FROM station_topology WHERE station_id = 'station'"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;

    let resp = match row {
        Some((station_id, station_name, flow_json_str)) => {
            let flow_json: TopologyFlow = serde_json::from_str(&flow_json_str).map_err(|e| {
                ModSrvError::SerializationError(format!("Invalid flow_json in DB: {}", e))
            })?;
            json!({
                "station_id": station_id,
                "station_name": station_name,
                "flow_json": flow_json
            })
        },
        None => json!({
            "station_id": "station",
            "station_name": "Station",
            "flow_json": {
                "nodes": [],
                "edges": [],
                "fixedBindings": {
                    "stationInstanceId": null,
                    "environmentInstanceId": null
                }
            }
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
/// one topology record. Drafts remain frontend-only, so all canvas nodes must
/// have valid unique instance bindings and every connection group must satisfy
/// its `min`/`max` constraints before persistence.
#[utoipa::path(
    put,
    path = "/api/station/topology",
    request_body = SaveTopologyRequest,
    responses(
        (status = 200, description = "Saved topology (same shape as GET response)", body = serde_json::Value),
        (status = 400, description = "Invalid final topology"),
        (status = 500, description = "Database error")
    ),
    tag = "topology"
)]
pub async fn put_station_topology(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveTopologyRequest>,
) -> Result<Json<SuccessResponse<Value>>, ModSrvError> {
    if req.station_id != "station" {
        return Err(ModSrvError::InvalidData(
            "station_id must be 'station'".to_string(),
        ));
    }
    if req.station_name != "Station" {
        return Err(ModSrvError::InvalidData(
            "station_name must be 'Station'".to_string(),
        ));
    }
    validate_final_topology(&state, &req.flow_json).await?;

    let flow_json_str = serde_json::to_string(&req.flow_json)
        .map_err(|e| ModSrvError::SerializationError(e.to_string()))?;

    let pool = &state.instance_manager.pool;

    sqlx::query(
        r#"INSERT INTO station_topology (station_id, station_name, description, gateway_id, flow_json, updated_at)
           VALUES ('station', 'Station', NULL, NULL, ?, datetime('now'))
           ON CONFLICT(station_id) DO UPDATE SET
               station_name = 'Station',
               description  = NULL,
               gateway_id   = NULL,
               flow_json    = excluded.flow_json,
               updated_at   = excluded.updated_at"#,
    )
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
/// Every persisted node has exactly one instance. Instance names and channel
/// IDs are resolved live and are never duplicated into `flow_json`.
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

    let flow: TopologyFlow = serde_json::from_str(&flow_json_str)
        .map_err(|e| ModSrvError::SerializationError(format!("Invalid flow_json in DB: {}", e)))?;

    // 2. Extract the final one-instance-per-node bindings.
    struct NodeMeta {
        node_id: String,
        product_name: String,
        instance_id: i64,
    }

    let mut node_metas: Vec<NodeMeta> = Vec::new();
    let mut all_instance_ids: Vec<i64> = Vec::new();

    for node in flow.nodes {
        all_instance_ids.push(node.data.instance_id);
        node_metas.push(NodeMeta {
            node_id: node.id,
            product_name: node.data.product_name,
            instance_id: node.data.instance_id,
        });
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
    let instance_rows: Vec<(i64, String)> = sqlx::query_as(&format!(
        "SELECT instance_id, instance_name FROM instances WHERE instance_id IN ({})",
        id_list
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| ModSrvError::DatabaseError(e.to_string()))?;
    let instance_names: HashMap<i64, String> = instance_rows.into_iter().collect();

    // 4. Build response
    let bindings: Vec<Value> = node_metas
        .into_iter()
        .map(|meta| {
            let channel_ids = channel_map
                .get(&meta.instance_id)
                .cloned()
                .unwrap_or_default();
            let instance_name = instance_names
                .get(&meta.instance_id)
                .cloned()
                .unwrap_or_default();
            json!({
                "nodeId": meta.node_id,
                "productName": meta.product_name,
                "instances": [{
                    "instanceId": meta.instance_id,
                    "instanceName": instance_name,
                    "channelIds": channel_ids
                }]
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

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_FLOW: &str = r#"{
        "nodes": [{
            "id": "vm_node_meter_01",
            "type": "product",
            "position": {"x": 10, "y": 20},
            "data": {
                "label": "Meter",
                "description": "",
                "productName": "Meter",
                "instanceId": 201
            }
        }],
        "edges": [],
        "fixedBindings": {
            "stationInstanceId": 1,
            "environmentInstanceId": null
        }
    }"#;

    #[test]
    fn final_topology_contract_deserializes() {
        let flow: TopologyFlow = serde_json::from_str(VALID_FLOW).unwrap();
        assert_eq!(flow.nodes[0].data.instance_id, 201);
        assert_eq!(flow.fixed_bindings.station_instance_id, Some(1));
        let encoded = serde_json::to_value(flow).unwrap();
        assert_eq!(encoded["nodes"][0]["data"]["productName"], "Meter");
        assert_eq!(
            encoded["fixedBindings"]["environmentInstanceId"],
            Value::Null
        );
    }

    #[test]
    fn final_topology_rejects_null_instance_and_legacy_fields() {
        let null_instance = VALID_FLOW.replace("\"instanceId\": 201", "\"instanceId\": null");
        assert!(serde_json::from_str::<TopologyFlow>(&null_instance).is_err());

        let legacy = VALID_FLOW.replace(
            "\"instanceId\": 201",
            "\"instanceId\": 201, \"instances\": []",
        );
        assert!(serde_json::from_str::<TopologyFlow>(&legacy).is_err());
    }

    #[test]
    fn topology_handles_use_plain_directions() {
        for handle in ["top", "bottom", "left", "right"] {
            assert!(is_valid_handle(handle));
        }
        for legacy in ["top-source", "bottom-source", "left-target", "right-target"] {
            assert!(!is_valid_handle(legacy));
        }
    }
}
