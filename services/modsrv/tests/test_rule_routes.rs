//! Rule Routes API Integration Tests
//!
//! This test suite covers the rule engine API handlers:
//! - GET /api/rules - List rules (paginated)
//! - POST /api/rules - Create rule
//! - GET /api/rules/{id} - Get rule by ID
//! - PUT /api/rules/{id} - Update rule
//! - DELETE /api/rules/{id} - Delete rule
//! - POST /api/rules/{id}/enable - Enable rule
//! - POST /api/rules/{id}/disable - Disable rule
//! - GET /api/scheduler/status - Get scheduler status
//!
//! Test scenarios cover:
//! - Happy path (success cases)
//! - Error handling (not found, validation)
//! - Pagination

#![allow(clippy::disallowed_methods)] // Integration test - unwrap is acceptable

use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use modsrv::rule_routes::{RuleEngineState, create_rule_routes};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;
use voltage_routing::RoutingCache;
use voltage_rtdb::MemoryRtdb;
use voltage_rules::RuleScheduler;

/// Create test SQLite database with rules schema
async fn create_test_database() -> Result<sqlx::SqlitePool> {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    common::test_utils::schema::init_rules_schema(&pool).await?;

    Ok(pool)
}

/// Create test app router with in-memory RTDB
async fn create_test_app() -> Result<axum::Router> {
    Ok(create_test_app_with_pool().await?.0)
}

async fn create_test_app_with_pool() -> Result<(axum::Router, sqlx::SqlitePool)> {
    let pool = create_test_database().await?;
    Ok((create_test_router(pool.clone()), pool))
}

fn create_test_router(pool: sqlx::SqlitePool) -> axum::Router {
    // Create in-memory RTDB (MemoryRtdb implements Rtdb trait)
    let rtdb = Arc::new(MemoryRtdb::new());

    // Create empty routing cache for testing
    let routing_cache = Arc::new(RoutingCache::new());

    // Create rule scheduler with test configuration
    // tick_ms: 1000ms (1 second) - reasonable for tests
    // log_root: temp directory for test logs
    let log_root = PathBuf::from("/tmp/modsrv_test_logs");
    let scheduler = Arc::new(RuleScheduler::new(
        rtdb,
        routing_cache,
        pool.clone(),
        1000, // tick_ms
        log_root,
    ));

    // Create rule engine state
    let state = Arc::new(RuleEngineState::new(pool.clone(), scheduler));

    // Create router
    create_rule_routes(state)
}

fn configured_flow() -> serde_json::Value {
    json!({
        "nodes": [
            {"id":"start", "type":"start", "data":{"wires":{"default":["end"]}}},
            {"id":"end", "type":"end", "data":{}}
        ],
        "edges": [{"id":"start-end", "source":"start", "target":"end"}]
    })
}

/// Helper function to make HTTP requests and extract response
async fn make_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Result<(StatusCode, serde_json::Value)> {
    let mut req_builder = Request::builder().method(method).uri(uri);

    let body_bytes = if let Some(json_body) = body {
        req_builder = req_builder.header("content-type", "application/json");
        serde_json::to_vec(&json_body)?
    } else {
        Vec::new()
    };

    let request = req_builder.body(Body::from(body_bytes))?;

    let response = app.clone().oneshot(request).await?;
    let status = response.status();

    let body_bytes = response.into_body().collect().await?.to_bytes();
    let response_json: serde_json::Value = if body_bytes.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body_bytes) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("JSON parse error on {} {}: {}", method, uri, e);
                let text = String::from_utf8_lossy(&body_bytes);
                eprintln!("Raw response: {}", text);
                json!({ "raw": text.to_string() })
            },
        }
    };

    Ok((status, response_json))
}

// ============================================================================
// GET /api/rules Tests
// ============================================================================

#[tokio::test]
async fn test_list_rules_empty() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = make_request(&app, "GET", "/api/rules", None).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    assert!(
        body.get("data").is_some(),
        "Response should have data field"
    );

    let data = &body["data"];
    assert_eq!(data["total"], 0);
    assert!(data["list"].as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_list_rules_pagination() -> Result<()> {
    let app = create_test_app().await?;

    // Create multiple rules first
    for i in 1..=5 {
        let create_req = json!({
            "name": format!("Test Rule {}", i),
            "description": format!("Rule {} for pagination test", i)
        });
        let (status, _) = make_request(&app, "POST", "/api/rules", Some(create_req)).await?;
        assert_eq!(status, StatusCode::OK);
    }

    // Test page 1 with page_size=2
    let (status, body) = make_request(&app, "GET", "/api/rules?page=1&page_size=2", None).await?;

    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    assert_eq!(data["total"], 5);
    assert_eq!(data["page"], 1);
    assert_eq!(data["page_size"], 2);
    assert!(data["has_next"].as_bool().unwrap());
    // Note: PaginatedResponse uses 0-indexed pages internally, so page=1 results in has_previous=true
    // This is a known semantic mismatch between 1-indexed API and 0-indexed PaginatedResponse
    assert!(data["has_previous"].as_bool().unwrap()); // page > 0 (1 > 0 = true)
    assert_eq!(data["list"].as_array().unwrap().len(), 2);

    Ok(())
}

// ============================================================================
// POST /api/rules Tests
// ============================================================================

#[tokio::test]
async fn test_create_rule_success() -> Result<()> {
    let app = create_test_app().await?;

    let create_req = json!({
        "name": "Battery SOC Protection",
        "description": "Protect battery when SOC is too low"
    });

    let (status, body) = make_request(&app, "POST", "/api/rules", Some(create_req)).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    let data = &body["data"];
    assert_eq!(data["name"], "Battery SOC Protection");
    assert_eq!(data["status"], "created");
    assert!(data["id"].as_i64().is_some());

    Ok(())
}

#[tokio::test]
async fn test_create_rule_minimal() -> Result<()> {
    let app = create_test_app().await?;

    // Only name is required
    let create_req = json!({
        "name": "Minimal Rule"
    });

    let (status, body) = make_request(&app, "POST", "/api/rules", Some(create_req)).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);

    Ok(())
}

#[tokio::test]
async fn test_create_rule_sequential_ids() -> Result<()> {
    let app = create_test_app().await?;

    // Create first rule
    let (_, body1) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name": "Rule 1"}))).await?;
    let id1 = body1["data"]["id"].as_i64().unwrap();

    // Create second rule
    let (_, body2) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name": "Rule 2"}))).await?;
    let id2 = body2["data"]["id"].as_i64().unwrap();

    // IDs should be sequential
    assert_eq!(id2, id1 + 1);

    Ok(())
}

// ============================================================================
// GET /api/rules/{id} Tests
// ============================================================================

#[tokio::test]
async fn test_get_rule_success() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule first
    let create_req = json!({
        "name": "Test Rule",
        "description": "A test rule"
    });
    let (_, create_body) = make_request(&app, "POST", "/api/rules", Some(create_req)).await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // Get the rule
    let (status, body) =
        make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    let data = &body["data"];
    assert_eq!(data["id"], rule_id);
    assert_eq!(data["name"], "Test Rule");
    assert_eq!(data["description"], "A test rule");

    Ok(())
}

#[tokio::test]
async fn test_get_rule_not_found() -> Result<()> {
    let app = create_test_app().await?;

    let (status, _) = make_request(&app, "GET", "/api/rules/99999", None).await?;

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

// ============================================================================
// PUT /api/rules/{id} Tests
// ============================================================================

#[tokio::test]
async fn test_update_rule_name() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule
    let (_, create_body) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name": "Original Name"})),
    )
    .await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // Update the rule
    let update_req = json!({
        "name": "Updated Name"
    });
    let (status, body) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{}", rule_id),
        Some(update_req),
    )
    .await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    assert_eq!(body["data"]["status"], "updated");

    // Verify the update
    let (_, get_body) = make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;
    assert_eq!(get_body["data"]["name"], "Updated Name");

    Ok(())
}

#[tokio::test]
async fn test_update_rule_priority() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule
    let (_, create_body) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name": "Test"}))).await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // Update priority
    let update_req = json!({
        "priority": 50
    });
    let (status, _) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{}", rule_id),
        Some(update_req),
    )
    .await?;

    assert_eq!(status, StatusCode::OK);

    // Verify
    let (_, get_body) = make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;
    assert_eq!(get_body["data"]["priority"], 50);

    Ok(())
}

#[tokio::test]
async fn test_update_rule_not_found() -> Result<()> {
    let app = create_test_app().await?;

    let update_req = json!({
        "name": "Updated"
    });
    let (status, _) = make_request(&app, "PUT", "/api/rules/99999", Some(update_req)).await?;

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_update_rule_empty_body() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule
    let (_, create_body) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name": "Test"}))).await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // Update with empty object (should fail - no fields to update)
    // API returns 422 UNPROCESSABLE_ENTITY for validation errors (more semantically correct)
    let (status, _) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{}", rule_id),
        Some(json!({})),
    )
    .await?;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    Ok(())
}

// ============================================================================
// DELETE /api/rules/{id} Tests
// ============================================================================

#[tokio::test]
async fn test_delete_rule_success() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule
    let (_, create_body) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name": "To Delete"})),
    )
    .await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // Delete the rule
    let (status, body) =
        make_request(&app, "DELETE", &format!("/api/rules/{}", rule_id), None).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    assert_eq!(body["data"]["status"], "OK");

    // Verify deletion
    let (get_status, _) =
        make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;
    assert_eq!(get_status, StatusCode::NOT_FOUND);

    Ok(())
}

// ============================================================================
// POST /api/rules/{id}/enable & /disable Tests
// ============================================================================

#[tokio::test]
async fn test_empty_or_invalid_flow_cannot_be_enabled() -> Result<()> {
    let (app, pool) = create_test_app_with_pool().await?;
    let (_, created) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name":"Draft"}))).await?;
    let id = created["data"]["id"].as_i64().unwrap();
    // Include malformed legacy records, empty topologies and invalid entry nodes.
    for nodes in [
        "{}",
        "null",
        "not json",
        r#"{"start_node":"start","nodes":{}}"#,
        r#"{"start_node":"start","nodes":{"start":{"type":"end"}}}"#,
        r#"{"start_node":"missing","nodes":{"start":{"type":"start","wires":{}}}}"#,
    ] {
        sqlx::query("UPDATE rules SET nodes_json = ?, updated_at = '2000-01-01' WHERE id = ?")
            .bind(nodes)
            .bind(id)
            .execute(&pool)
            .await?;
        let (status, body) =
            make_request(&app, "POST", &format!("/api/rules/{id}/enable"), None).await?;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{nodes}: {body:?}"
        );
        assert_eq!(body["success"], false);
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap()
                .contains("请先配置")
        );
        let stored: (bool, String) =
            sqlx::query_as("SELECT enabled, updated_at FROM rules WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(stored, (false, "2000-01-01".into()));
    }
    Ok(())
}

#[tokio::test]
async fn test_update_cannot_enable_a_draft_or_partially_save_other_fields() -> Result<()> {
    let (app, pool) = create_test_app_with_pool().await?;
    let (_, created) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name":"Original draft"})),
    )
    .await?;
    let id = created["data"]["id"].as_i64().unwrap();
    let (status, body) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{id}"),
        Some(json!({"enabled":true,"name":"Changed name","priority":42})),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body:?}");
    assert_eq!(body["success"], false);
    let stored: (bool, String, i64, String, Option<String>) = sqlx::query_as(
        "SELECT enabled, name, priority, nodes_json, flow_json FROM rules WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        stored,
        (false, "Original draft".into(), 0, "{}".into(), None)
    );
    Ok(())
}

#[tokio::test]
async fn test_concurrent_writer_does_not_turn_flow_validation_into_database_locked() -> Result<()> {
    use std::time::Duration;
    let directory = tempfile::tempdir()?;
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(directory.path().join("rules.db"))
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(2));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    common::test_utils::schema::init_rules_schema(&pool).await?;
    let app = create_test_router(pool.clone());
    let (_, created) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name":"Concurrent draft"})),
    )
    .await?;
    let id = created["data"]["id"].as_i64().unwrap();

    // Another service owns SQLite's writer lock while this request starts.
    let mut writer = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("UPDATE rules SET description = 'Background update' WHERE id = ?")
        .bind(id)
        .execute(&mut *writer)
        .await?;
    let mut update = tokio::spawn(async move {
        make_request(
            &app,
            "PUT",
            &format!("/api/rules/{id}"),
            Some(json!({"enabled":true,"priority":25})),
        )
        .await
    });
    let early_response = tokio::time::timeout(Duration::from_millis(200), &mut update).await;
    writer.commit().await?;
    let (status, body) = match early_response {
        Ok(result) => result??,
        Err(_) => update.await??,
    };
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body:?}");
    assert_eq!(body["success"], false);
    let stored: (bool, i64, Option<String>) =
        sqlx::query_as("SELECT enabled, priority, description FROM rules WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored, (false, 0, Some("Background update".into())));
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn test_save_flow_and_enable_together_and_preserve_it_on_invalid_edit() -> Result<()> {
    let (app, pool) = create_test_app_with_pool().await?;
    let (_, created) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name":"Configured rule"})),
    )
    .await?;
    let id = created["data"]["id"].as_i64().unwrap();
    let flow = configured_flow();
    let (status, body) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{id}"),
        Some(json!({"enabled":true,"flow_json":flow})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let before: (bool, String, String, Option<String>) =
        sqlx::query_as("SELECT enabled, name, nodes_json, flow_json FROM rules WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    let columns = voltage_rules::flow_column_values(&flow)?;
    assert!(before.0);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&before.2)?,
        serde_json::from_str::<serde_json::Value>(&columns.nodes_json)?
    );
    assert_eq!(before.3.as_deref(), Some(columns.flow_json.as_str()));

    // The parser accepts this document, but it has no usable execution entry.
    let unusable_flow = json!({"nodes":[{"id":" ","type":"start","data":{}}]});
    let (status, body) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{id}"),
        Some(json!({"name":"Changed name","flow_json":unusable_flow})),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body:?}");
    let after: (bool, String, String, Option<String>) =
        sqlx::query_as("SELECT enabled, name, nodes_json, flow_json FROM rules WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn test_legacy_execution_flow_can_be_enabled_without_editor_document() -> Result<()> {
    let (app, pool) = create_test_app_with_pool().await?;
    let (_, created) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name":"Legacy rule"})),
    )
    .await?;
    let id = created["data"]["id"].as_i64().unwrap();
    let columns = voltage_rules::flow_column_values(&configured_flow())?;
    // Historical compact-only rules intentionally have no editor document.
    sqlx::query("UPDATE rules SET nodes_json = ?, flow_json = NULL WHERE id = ?")
        .bind(columns.nodes_json)
        .bind(id)
        .execute(&pool)
        .await?;
    let (status, body) =
        make_request(&app, "POST", &format!("/api/rules/{id}/enable"), None).await?;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let enabled: bool = sqlx::query_scalar("SELECT enabled FROM rules WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert!(enabled);
    Ok(())
}

#[tokio::test]
async fn test_draft_can_be_disabled_and_missing_rule_returns_404() -> Result<()> {
    let app = create_test_app().await?;
    let (_, created) =
        make_request(&app, "POST", "/api/rules", Some(json!({"name":"Draft"}))).await?;
    let id = created["data"]["id"].as_i64().unwrap();
    let (status, body) =
        make_request(&app, "POST", &format!("/api/rules/{id}/disable"), None).await?;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let (status, body) = make_request(&app, "POST", "/api/rules/99999/enable", None).await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body:?}");
    Ok(())
}

#[tokio::test]
async fn test_enable_disable_rule() -> Result<()> {
    let app = create_test_app().await?;

    // Create a rule (enabled by default is false in create_rule)
    let (_, create_body) = make_request(
        &app,
        "POST",
        "/api/rules",
        Some(json!({"name": "Toggle Test"})),
    )
    .await?;
    let rule_id = create_body["data"]["id"].as_i64().unwrap();

    // A draft must have its execution flow configured before enabling.
    let (status, body) = make_request(
        &app,
        "PUT",
        &format!("/api/rules/{rule_id}"),
        Some(json!({"flow_json": configured_flow()})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    // Enable the rule
    let (status, body) = make_request(
        &app,
        "POST",
        &format!("/api/rules/{}/enable", rule_id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);

    // Verify enabled
    let (_, get_body) = make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;
    assert_eq!(get_body["data"]["enabled"], true);

    // Disable the rule
    let (status, _) = make_request(
        &app,
        "POST",
        &format!("/api/rules/{}/disable", rule_id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);

    // Verify disabled
    let (_, get_body) = make_request(&app, "GET", &format!("/api/rules/{}", rule_id), None).await?;
    assert_eq!(get_body["data"]["enabled"], false);

    Ok(())
}

// ============================================================================
// GET /api/scheduler/status Tests
// ============================================================================

#[tokio::test]
async fn test_scheduler_status() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = make_request(&app, "GET", "/api/scheduler/status", None).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);
    // Scheduler should return some status information
    assert!(body.get("data").is_some());

    Ok(())
}

// ============================================================================
// POST /api/scheduler/reload Tests
// ============================================================================

#[tokio::test]
async fn test_scheduler_reload() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = make_request(&app, "POST", "/api/scheduler/reload", None).await?;

    assert_eq!(status, StatusCode::OK, "Response: {:?}", body);

    Ok(())
}
