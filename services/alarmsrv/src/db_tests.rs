use super::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .foreign_keys(true),
        )
        .await
        .unwrap();
    create_tables(&pool).await.unwrap();
    pool
}

async fn rule(pool: &SqlitePool) -> AlertRule {
    let id = insert_rule(
        pool,
        "inst",
        1,
        "M",
        1,
        "Temperature",
        2,
        ">",
        30.0,
        true,
        None,
    )
    .await
    .unwrap();
    get_rule_by_id(pool, id).await.unwrap().unwrap()
}

async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn delete_rule_preserves_history_and_original_rule_id() {
    let pool = pool().await;
    let rule = rule(&pool).await;
    let alert_id = insert_alert(&pool, &rule, 35.0).await.unwrap();
    let alert = get_alert_by_id(&pool, alert_id).await.unwrap().unwrap();
    resolve_alert(&pool, &alert, 20.0).await.unwrap();
    let before: AlertEvent = sqlx::query_as("SELECT * FROM alert_event")
        .fetch_one(&pool)
        .await
        .unwrap();

    let resolved = delete_rule(&pool, rule.id).await.unwrap().unwrap();
    assert!(resolved.is_empty());
    assert_eq!(count(&pool, "alert_rule").await, 0);
    let after: AlertEvent = sqlx::query_as("SELECT * FROM alert_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after.rule_id, rule.id);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    assert!(delete_rule(&pool, rule.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_rule_resolves_active_alerts_atomically() {
    let pool = pool().await;
    let rule = rule(&pool).await;
    let alert_id = insert_alert(&pool, &rule, 35.0).await.unwrap();

    let resolved = delete_rule(&pool, rule.id).await.unwrap().unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].id, alert_id);
    assert_eq!(count(&pool, "alert_rule").await, 0);
    assert_eq!(count(&pool, "alert").await, 0);
    assert_eq!(count(&pool, "alert_event").await, 1);
    let event: AlertEvent = sqlx::query_as("SELECT * FROM alert_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(event.rule_id, rule.id);
    assert_eq!(event.trigger_value, Some(35.0));
    assert_eq!(event.event_type, "recovery");
}

#[tokio::test]
async fn failed_delete_rolls_back_alert_recovery() {
    let pool = pool().await;
    let rule = rule(&pool).await;
    let alert_id = insert_alert(&pool, &rule, 35.0).await.unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_delete BEFORE DELETE ON alert_rule \
         BEGIN SELECT RAISE(ABORT, 'test delete failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(delete_rule(&pool, rule.id).await.is_err());
    assert!(get_rule_by_id(&pool, rule.id).await.unwrap().is_some());
    assert!(get_alert_by_id(&pool, alert_id).await.unwrap().is_some());
    assert_eq!(count(&pool, "alert_event").await, 0);
}

async fn legacy_history(pool: &SqlitePool, empty: bool) {
    sqlx::query("DROP TABLE alert_event")
        .execute(pool)
        .await
        .unwrap();
    // Older tables gained name columns through ALTER TABLE, so their column
    // order differs from the current CREATE TABLE definition.
    let old_schema = ALERT_EVENT_SCHEMA
        .replace("            device_name     TEXT,\n", "")
        .replace("            point_name      TEXT,\n", "")
        .replace("            unit            TEXT,\n", "")
        .replace(
            "duration        INTEGER",
            "duration        INTEGER, FOREIGN KEY (rule_id) REFERENCES alert_rule(id)",
        );
    sqlx::query(&old_schema).execute(pool).await.unwrap();
    let rule = rule(pool).await;
    sqlx::query(
        "INSERT INTO alert_event (id, rule_id, rule_snapshot, event_type, triggered_at) \
         VALUES (42, ?, '{\"rule_name\":\"original\"}', 'recovery', 123)",
    )
    .bind(rule.id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO alert_event (id, event_type) VALUES (90, 'recovery')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM alert_event WHERE id = 90 OR ?")
        .bind(empty)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("CREATE INDEX idx_history_rule ON alert_event(rule_id)")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn startup_migrates_legacy_history_without_losing_records_or_indexes() {
    let pool = pool().await;
    legacy_history(&pool, false).await;
    create_tables(&pool).await.unwrap();
    create_tables(&pool).await.unwrap(); // Migration is safe on every startup.

    let record: (i64, i64, String, i64, Option<String>) = sqlx::query_as(
        "SELECT id, rule_id, rule_snapshot, triggered_at, device_name FROM alert_event",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        record,
        (42, 1, "{\"rule_name\":\"original\"}".into(), 123, None)
    );
    let index_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE name = 'idx_history_rule'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(index_count, 1);
    assert!(delete_rule(&pool, 1).await.unwrap().is_some());
    assert_eq!(count(&pool, "alert_event").await, 1);
    let id: i64 =
        sqlx::query_scalar("INSERT INTO alert_event (event_type) VALUES ('recovery') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(id > 90);
}

#[tokio::test]
async fn migration_preserves_sequence_even_when_history_is_empty() {
    let pool = pool().await;
    legacy_history(&pool, true).await;
    create_tables(&pool).await.unwrap();
    let id: i64 =
        sqlx::query_scalar("INSERT INTO alert_event (event_type) VALUES ('recovery') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(id > 90);
}
