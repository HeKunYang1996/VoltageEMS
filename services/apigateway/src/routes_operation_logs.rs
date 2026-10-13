use axum::{Json, extract::Query};
use chrono::{DateTime, Duration, Utc};
use common::{api_types::AppError, operation_log::OperationFilter};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub struct OperationQuery {
    start_time: Option<String>,
    end_time: Option<String>,
    username: Option<String>,
    interface: Option<String>,
    service: Option<String>,
    method: Option<String>,
    status: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
}

fn parse_time(value: Option<&str>, fallback: i64) -> Result<i64, &'static str> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|time| time.timestamp_millis())
                .map_err(|_| "Time must be RFC3339, for example 2026-10-08T10:00:00+08:00")
        })
        .unwrap_or(Ok(fallback))
}

/// Query operation records from all six services. Engineer permission is applied by the router.
#[utoipa::path(
    get,
    path = "/api/v1/operation-logs",
    tag = "Operation Logs",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Paginated operation records"),
        (status = 400, description = "Invalid filters"),
        (status = 403, description = "Engineer or Admin role required")
    )
)]
pub async fn list_operation_logs(
    Query(query): Query<OperationQuery>,
) -> Result<Json<Value>, AppError> {
    let now = Utc::now().timestamp_millis();
    let from_ms = parse_time(
        query.start_time.as_deref(),
        now - Duration::days(1).num_milliseconds(),
    )
    .map_err(AppError::bad_request)?;
    let to_ms = parse_time(query.end_time.as_deref(), now + 1).map_err(AppError::bad_request)?;
    if from_ms >= to_ms || to_ms - from_ms > Duration::days(31).num_milliseconds() {
        return Err(AppError::bad_request(
            "Time range must be positive and at most 31 days",
        ));
    }
    for value in [
        query.username.as_deref(),
        query.interface.as_deref(),
        query.service.as_deref(),
    ] {
        if value.is_some_and(|value| value.len() > 128) {
            return Err(AppError::bad_request("Filter value is too long"));
        }
    }
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(50);
    if page < 1 || !(1..=200).contains(&page_size) {
        return Err(AppError::bad_request(
            "page must be positive and page_size must be 1..200",
        ));
    }
    let offset = (page - 1)
        .checked_mul(page_size)
        .ok_or_else(|| AppError::bad_request("Page offset is too large"))?;
    let method = query.method.map(|method| method.to_uppercase());
    let filter = OperationFilter {
        from_ms,
        to_ms,
        username: query.username,
        endpoint: query.interface,
        service: query.service,
        method,
        status: query.status,
        limit: page_size,
        offset,
    };
    let (rows, total) = common::operation_log::search(&filter)
        .await
        .map_err(|error| {
            tracing::error!("Operation log query failed: {}", error);
            AppError::internal_error("Operation log query unavailable")
        })?;
    Ok(Json(json!({
        "success": true,
        "data": rows,
        "metadata": { "total": total, "page": page, "page_size": page_size }
    })))
}
