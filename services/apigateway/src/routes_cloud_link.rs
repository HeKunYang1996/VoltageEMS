use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State, rejection::JsonRejection},
    http::StatusCode,
};
use common::api_types::{AppError, SuccessResponse};
use serde::Deserialize;

use crate::{
    cloud_link::{ActivateRequest, CloudLinkConfig},
    state::AppState,
};

/// Read cloud registration settings. Available to every authenticated role.
#[utoipa::path(get, path = "/api/v1/cloud-link", tag = "CloudLink",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current configuration with a masked activation key"),
        (status = 401, description = "Unauthorized"), (status = 500, description = "Unable to read local configuration")))]
pub async fn get_cloud_link(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SuccessResponse<CloudLinkConfig>>, AppError> {
    Ok(Json(SuccessResponse::new(state.cloud_link.config()?)))
}

/// Register with the cloud, then save the confirmed configuration locally.
#[utoipa::path(post, path = "/api/v1/cloud-link/activate", tag = "CloudLink",
    security(("bearer_auth" = [])), request_body = ActivateRequest,
    responses((status = 200, description = "Cloud registration and local save succeeded, with a masked activation key"),
        (status = 400, description = "Invalid request or device serial number"), (status = 401, description = "Unauthorized"),
        (status = 409, description = "Activation in progress or concurrent configuration change"), (status = 500, description = "Unable to read or save local configuration"),
        (status = 502, description = "Cloud rejected the request or returned an invalid response"), (status = 504, description = "Cloud request timed out; activation could not be confirmed")))]
pub async fn activate_cloud_link(
    State(state): State<Arc<AppState>>,
    request: Result<Json<ActivateRequest>, JsonRejection>,
) -> Result<Json<SuccessResponse<CloudLinkConfig>>, AppError> {
    let Json(request) = request.map_err(|_| {
        AppError::bad_request(
            "Request body must be a JSON object containing only the url and key fields.",
        )
    })?;
    Ok(Json(SuccessResponse::new(
        state.cloud_link.activate(request).await?,
    )))
}

#[derive(Deserialize)]
pub struct RegistrationStatusQuery {
    device_sn: String,
}

/// Loopback-only preflight for the device identity that netsrv will publish under.
pub async fn check_registration_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RegistrationStatusQuery>,
) -> Result<StatusCode, AppError> {
    if state.cloud_link.is_device_registered(&query.device_sn)? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::conflict("Device is not registered."))
    }
}
