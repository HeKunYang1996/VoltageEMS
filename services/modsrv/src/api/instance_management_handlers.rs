//! Instance Management API Handlers
//!
//! Handles CRUD operations and synchronization for model instances.

#![allow(clippy::disallowed_methods)] // json! macro used in multiple functions

use crate::config::CreateInstanceRequest;
use axum::{
    extract::{Path, State},
    response::Json,
};
use bytes::Bytes;
use common::SuccessResponse;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::app_state::AppState;
use crate::dto::{ActionRequest, CloudPointWriteRequest, CreateInstanceDto, UpdateInstanceDto};
use crate::error::ModSrvError;
use voltage_model::KeySpaceConfig;
use voltage_rtdb::Rtdb;

/// Create a new model instance
///
/// Creates an instance from a product template with optional property overrides.
#[utoipa::path(
    post,
    path = "/api/instances",
    request_body = crate::dto::CreateInstanceDto,
    responses(
        (status = 200, description = "Instance created", body = serde_json::Value,
            example = json!({
                "instance": {
                    "instance_id": 1,
                    "instance_name": "pv_inverter_01",
                    "product_name": "pv_inverter",
                    "properties": {
                        "rated_power": 5000.0,
                        "manufacturer": "Huawei",
                        "model": "SUN2000-5KTL-L1"
                    },
                    "created_at": "2025-10-15T10:30:00Z",
                    "updated_at": "2025-10-15T10:30:00Z"
                }
            })
        ),
        (status = 409, description = "Instance already exists, including the single Station limit")
    ),
    tag = "modsrv"
)]
pub async fn create_instance(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<CreateInstanceDto>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    let req = CreateInstanceRequest {
        instance_id: dto.instance_id,
        instance_name: dto.instance_name,
        product_name: dto.product_name,
        parent_id: dto.parent_id,
        properties: dto.properties.unwrap_or_default(),
    };

    let instance = state.instance_manager.create_instance(req).await?;
    Ok(Json(SuccessResponse::new(json!({
        "instance": instance
    }))))
}

/// Update instance name and/or properties
///
/// Updates the instance_name and/or properties of an existing instance.
/// At least one field (instance_name or properties) must be provided.
#[utoipa::path(
    put,
    path = "/api/instances/{id}",
    params(
        ("id" = u16, Path, description = "Instance ID")
    ),
    request_body = UpdateInstanceDto,
    responses(
        (status = 200, description = "Instance updated successfully", body = serde_json::Value,
            example = json!({
                "instance": {
                    "instance_id": 1,
                    "instance_name": "pv_inverter_renamed",
                    "product_name": "pv_inverter",
                    "properties": {
                        "rated_power": 5000.0,
                        "manufacturer": "Huawei",
                        "model": "SUN2000-5KTL-L1"
                    },
                    "created_at": "2025-10-15T10:30:00Z",
                    "updated_at": "2025-10-20T14:25:00Z"
                }
            })
        ),
        (status = 400, description = "No fields to update"),
        (status = 404, description = "Instance not found"),
        (status = 409, description = "Instance name already exists"),
        (status = 500, description = "Database or Redis error")
    ),
    tag = "modsrv"
)]
pub async fn update_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(dto): Json<UpdateInstanceDto>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    // Validate: at least one field must be provided
    if dto.instance_name.is_none() && dto.properties.is_none() {
        return Err(ModSrvError::InvalidData(
            "At least one field (instance_name or properties) must be provided".to_string(),
        ));
    }

    // Query current instance_name for logging and Redis operations
    let old_instance_name: String =
        match sqlx::query_scalar("SELECT instance_name FROM instances WHERE instance_id = ?")
            .bind(id as i32)
            .fetch_one(&state.instance_manager.pool)
            .await
        {
            Ok(name) => name,
            Err(_) => return Err(ModSrvError::InstanceNotFound(id.to_string())),
        };

    // Determine the final instance name
    let new_instance_name = dto.instance_name.as_deref().unwrap_or(&old_instance_name);
    let is_renaming = dto.instance_name.is_some() && new_instance_name != old_instance_name;

    // Handle renaming
    if is_renaming {
        // Rename in SQLite (includes transaction)
        state
            .instance_manager
            .rename_instance(id, new_instance_name)
            .await?;

        // Rename in Redis (best effort)
        if let Err(e) = crate::redis_state::rename_instance_in_redis(
            state.instance_manager.rtdb.as_ref(),
            id,
            &old_instance_name,
            new_instance_name,
        )
        .await
        {
            warn!(
                "Instance {} renamed in SQLite but Redis sync failed: {}. Will sync on next reload.",
                id, e
            );
        }
    }

    // Handle properties update.
    //
    // Semantics: this endpoint replaces the property map atomically — keys
    // omitted from `dto.properties` are removed. (For single-point edits use
    // PUT /api/instances/{id}/properties/{property_id}, which does not touch
    // sibling properties.)
    if let Some(ref properties) = dto.properties {
        let product_name: String =
            match sqlx::query_scalar("SELECT product_name FROM instances WHERE instance_id = ?")
                .bind(id as i64)
                .fetch_one(&state.instance_manager.pool)
                .await
            {
                Ok(name) => name,
                Err(_) => return Err(ModSrvError::InstanceNotFound(id.to_string())),
            };

        let mut tx = state
            .instance_manager
            .pool
            .begin()
            .await
            .map_err(|e| ModSrvError::DatabaseError(format!("Failed to begin tx: {}", e)))?;

        // Wipe existing rows so omitted keys disappear (replace, not merge).
        if let Err(e) = sqlx::query("DELETE FROM instance_properties WHERE instance_id = ?")
            .bind(id as i64)
            .execute(&mut *tx)
            .await
        {
            error!("Failed to clear properties for instance {}: {}", id, e);
            let _ = tx.rollback().await;
            return Err(ModSrvError::DatabaseError(format!(
                "Database update failed: {}",
                e
            )));
        }

        if let Err(e) = state
            .instance_manager
            .write_properties_tx(&mut tx, id, &product_name, properties)
            .await
        {
            error!("Failed to write properties for instance {}: {}", id, e);
            let _ = tx.rollback().await;
            return Err(ModSrvError::InvalidData(format!(
                "Failed to write properties: {}",
                e
            )));
        }

        if let Err(e) =
            sqlx::query("UPDATE instances SET updated_at = CURRENT_TIMESTAMP WHERE instance_id = ?")
                .bind(id as i64)
                .execute(&mut *tx)
                .await
        {
            error!("Failed to bump updated_at for instance {}: {}", id, e);
            let _ = tx.rollback().await;
            return Err(ModSrvError::DatabaseError(format!(
                "Database update failed: {}",
                e
            )));
        }

        if let Err(e) = tx.commit().await {
            error!("Failed to commit properties tx for instance {}: {}", id, e);
            return Err(ModSrvError::DatabaseError(format!(
                "Database commit failed: {}",
                e
            )));
        }

        // Sync properties to Redis (best effort)
        if let Err(e) = state
            .instance_manager
            .sync_instance_to_redis_internal(new_instance_name, properties)
            .await
        {
            warn!(
                "Instance {} properties updated in SQLite but Redis sync failed: {}. Will sync on next reload.",
                id, e
            );
        }
    }

    info!(
        "Instance {} updated successfully (renamed: {}, properties: {})",
        id,
        is_renaming,
        dto.properties.is_some()
    );

    // Query and return updated instance
    match state.instance_manager.get_instance(id).await {
        Ok(instance) => Ok(Json(SuccessResponse::new(json!({
            "instance": instance
        })))),
        Err(e) => {
            error!("Failed to query updated instance {}: {}", id, e);
            // Update succeeded but query failed - return id as fallback
            Ok(Json(SuccessResponse::new(json!({
                "instance_id": id,
                "instance_name": new_instance_name,
                "message": "Instance updated successfully but failed to retrieve details"
            }))))
        },
    }
}

/// Delete an instance
///
/// Removes an instance from both SQLite and Redis.
#[utoipa::path(
    delete,
    path = "/api/instances/{id}",
    params(
        ("id" = u16, Path, description = "Instance ID")
    ),
    responses(
        (status = 200, description = "Instance deleted", body = serde_json::Value,
            example = json!({
                "message": "Instance 1 deleted"
            })
        )
    ),
    tag = "modsrv"
)]
pub async fn delete_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    state.instance_manager.delete_instance(id).await?;
    Ok(Json(SuccessResponse::new(json!({
        "message": format!("Instance {} deleted", id)
    }))))
}

/// Sync measurement data to an instance
///
/// Updates measurement point values in Redis for the instance.
#[utoipa::path(
    post,
    path = "/api/instances/{id}/sync",
    params(
        ("id" = u16, Path, description = "Instance ID")
    ),
    request_body = std::collections::HashMap<String, serde_json::Value>,
    responses(
        (status = 200, description = "Measurement synced", body = serde_json::Value,
            example = json!({
                "message": "Measurement synced"
            })
        )
    ),
    tag = "modsrv"
)]
pub async fn sync_instance_measurement(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(data): Json<HashMap<String, serde_json::Value>>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    state
        .instance_manager
        .sync_measurement(id, data)
        .await
        .map_err(|e| ModSrvError::InternalError(format!("Failed to sync measurement: {}", e)))?;
    Ok(Json(SuccessResponse::new(json!({
        "message": "Measurement synced"
    }))))
}

/// Sync all instances to Redis
///
/// Reloads all instance configurations from SQLite to Redis.
///
pub async fn sync_all_instances(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    match state.instance_manager.sync_instances_to_redis().await {
        Ok(_) => Ok(Json(SuccessResponse::new(json!({
            "message": "All instances synced to Redis"
        })))),
        Err(e) => Err(ModSrvError::InternalError(format!(
            "Failed to sync instances: {}",
            e
        ))),
    }
}

/// Reload instances from database
///
/// Forces a reload of all instances from SQLite to Redis.
/// Useful after manual database updates.
///
pub async fn reload_instances_from_db(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    // Use unified ReloadableService interface for incremental sync
    use common::ReloadableService;
    match ReloadableService::reload_from_database(
        &*state.instance_manager,
        &state.instance_manager.pool,
    )
    .await
    {
        Ok(result) => {
            info!(
                "Instances reloaded: {} added, {} updated, {} removed, {} errors",
                result.added.len(),
                result.updated.len(),
                result.removed.len(),
                result.errors.len()
            );
            Ok(Json(SuccessResponse::new(json!({
                "message": "Instances reloaded successfully",
                "result": result
            }))))
        },
        Err(e) => {
            error!("Failed to reload instances: {}", e);
            Err(ModSrvError::InternalError(format!(
                "Failed to reload instances: {}",
                e
            )))
        },
    }
}

// ============================================================================
// Action Execution
// ============================================================================

/// Execute an action on an instance
///
/// Triggers an action point with the specified value.
#[utoipa::path(
    post,
    path = "/api/instances/{id}/action",
    params(
        ("id" = u16, Path, description = "Instance ID")
    ),
    request_body = crate::dto::ActionRequest,
    responses(
        (status = 200, description = "Action executed", body = serde_json::Value,
            example = json!({
                "message": "Action executed"
            })
        )
    ),
    tag = "modsrv"
)]
pub async fn execute_instance_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    state
        .instance_manager
        .execute_action(id, &req.point_id, req.value)
        .await?;
    Ok(Json(SuccessResponse::new(json!({
        "message": "Action executed",
        "instance_id": id,
        "point_id": req.point_id,
        "value": req.value
    }))))
}

/// Write one instance point from the cloud protocol.
///
/// The source name is carried as metadata and deliberately not restricted.
/// M points update the instance measurement mirror; A points use the normal
/// SHM/UDS action path and therefore reach comsrv and the target device.
#[utoipa::path(
    post,
    path = "/api/instances/write",
    request_body = crate::dto::CloudPointWriteRequest,
    responses(
        (status = 200, description = "Point written", body = serde_json::Value),
        (status = 400, description = "Invalid point write request"),
        (status = 404, description = "Instance not found"),
        (status = 502, description = "Device dispatch failed"),
        (status = 503, description = "Target channel is offline")
    ),
    tag = "modsrv"
)]
pub async fn write_instance_point(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CloudPointWriteRequest>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    validate_cloud_write_fields(&req)?;

    let instance_id = resolve_cloud_device(&state, &req.device).await?;
    let point_id = req
        .point_id
        .parse::<u32>()
        .map_err(|_| ModSrvError::InvalidData("key must be a numeric point ID".to_string()))?;
    let value = parse_cloud_numeric_value(&req.value)?;
    let data_type = req.data_type.trim().to_ascii_uppercase();

    match data_type.as_str() {
        "M" => {
            state
                .instance_manager
                .load_single_measurement_point(instance_id, point_id)
                .await
                .map_err(|e| ModSrvError::InvalidData(e.to_string()))?;

            let keyspace = KeySpaceConfig::production_cached();
            let value_key = keyspace.instance_measurement_key(instance_id);
            let timestamp_key = keyspace.instance_measurement_ts_key(instance_id);
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as i64)
                .unwrap_or(0);

            state
                .instance_manager
                .rtdb
                .hash_set(&value_key, &req.point_id, Bytes::from(value.to_string()))
                .await
                .map_err(|e| ModSrvError::RedisError(e.to_string()))?;
            state
                .instance_manager
                .rtdb
                .hash_set(
                    &timestamp_key,
                    &req.point_id,
                    Bytes::from(timestamp_ms.to_string()),
                )
                .await
                .map_err(|e| ModSrvError::RedisError(e.to_string()))?;
        },
        "A" => {
            let action_point = state
                .instance_manager
                .load_single_action_point(instance_id, point_id)
                .await
                .map_err(|e| ModSrvError::InvalidData(e.to_string()))?;
            let routing = action_point.routing.ok_or_else(|| {
                ModSrvError::InvalidRouting(format!(
                    "Action point {} of instance {} is not routed",
                    point_id, instance_id
                ))
            })?;
            if !routing.enabled
                || routing.channel_id.is_none()
                || routing.channel_point_id.is_none()
                || routing.channel_type.is_none()
            {
                return Err(ModSrvError::InvalidRouting(format!(
                    "Action point {} of instance {} has no enabled target",
                    point_id, instance_id
                )));
            }

            state
                .instance_manager
                .execute_action(instance_id, &req.point_id, value)
                .await?;
        },
        _ => {
            return Err(ModSrvError::InvalidData(format!(
                "Unsupported data_type '{}'; expected M or A",
                req.data_type
            )));
        },
    }

    info!(
        source = %req.source,
        device = %req.device,
        instance_id,
        data_type = %data_type,
        point_id,
        msg_id = %req.msg_id,
        "Cloud single-point write completed"
    );

    Ok(Json(SuccessResponse::new(json!({
        "source": req.source,
        "device": req.device,
        "data_type": data_type,
        "key": req.point_id,
        "value": value,
        "msgId": req.msg_id
    }))))
}

fn validate_cloud_write_fields(req: &CloudPointWriteRequest) -> Result<(), ModSrvError> {
    if req.source.trim().is_empty()
        || req.device.trim().is_empty()
        || req.data_type.trim().is_empty()
        || req.point_id.trim().is_empty()
        || req.msg_id.trim().is_empty()
    {
        return Err(ModSrvError::InvalidData(
            "source, device, data_type, key and msgId must not be empty".to_string(),
        ));
    }
    Ok(())
}

async fn resolve_cloud_device(state: &AppState, device: &str) -> Result<u32, ModSrvError> {
    if let Ok(instance_id) = device.parse::<u32>() {
        return Ok(instance_id);
    }

    match state.get_instance_id(device).await {
        Ok(instance_id) => Ok(u32::from(instance_id)),
        Err(exact_error) if device.contains('_') => state
            .get_instance_id(&device.replace('_', " "))
            .await
            .map(u32::from)
            .map_err(|_| exact_error),
        Err(error) => Err(error),
    }
}

fn parse_cloud_numeric_value(value: &serde_json::Value) -> Result<f64, ModSrvError> {
    let parsed = match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|value| value.is_finite())
    .ok_or_else(|| ModSrvError::InvalidData("value must be a finite number".to_string()))?;

    Ok(parsed)
}

#[cfg(test)]
mod cloud_write_tests {
    use super::parse_cloud_numeric_value;

    #[test]
    fn accepts_number_and_numeric_string_values() {
        assert_eq!(
            parse_cloud_numeric_value(&serde_json::json!(123)).unwrap(),
            123.0
        );
        assert_eq!(
            parse_cloud_numeric_value(&serde_json::json!("12.5")).unwrap(),
            12.5
        );
    }

    #[test]
    fn rejects_non_numeric_values() {
        assert!(parse_cloud_numeric_value(&serde_json::json!("on")).is_err());
        assert!(parse_cloud_numeric_value(&serde_json::json!(true)).is_err());
    }
}
