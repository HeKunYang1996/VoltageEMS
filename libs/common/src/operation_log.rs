//! Structured HTTP operation records, stored separately from voltage.db.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::Method;
use serde::Serialize;
use sqlx::{
    FromRow, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous},
};
use tokio::sync::OnceCell;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

#[derive(Clone, Debug)]
pub struct AuditActor {
    pub user_id: i64,
    pub username: String,
}

#[derive(Clone, Debug)]
pub struct AuditSubmittedUsername(pub String);

#[derive(Debug)]
pub struct NewOperation<'a> {
    pub occurred_at_ms: i64,
    pub service: &'a str,
    pub actor: Option<&'a AuditActor>,
    pub submitted_username: Option<&'a str>,
    pub actor_type: &'a str,
    pub source_service: Option<&'a str>,
    pub method: &'a str,
    pub route: &'a str,
    pub path: &'a str,
    pub status: i64,
    pub business_success: Option<bool>,
    pub duration_ms: i64,
    pub request_id: Option<&'a str>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct OperationRow {
    pub event_id: String,
    pub occurred_at_ms: i64,
    pub service: String,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub actor_type: String,
    pub source_service: Option<String>,
    pub method: String,
    pub route: String,
    pub path: String,
    pub status: i64,
    pub business_success: Option<bool>,
    pub duration_ms: i64,
    pub request_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct OperationFilter {
    pub from_ms: i64,
    pub to_ms: i64,
    pub username: Option<String>,
    pub endpoint: Option<String>,
    pub service: Option<String>,
    pub method: Option<String>,
    pub status: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Decode the ASCII-only value sent in the authenticated Nginx response header.
pub fn decode_header_value(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hi = (bytes[index + 1] as char).to_digit(16);
            let lo = (bytes[index + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                decoded.push(((hi << 4) | lo) as u8);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

pub fn should_record(method: &Method, path: &str) -> bool {
    if matches!(path, "/health" | "/ping")
        || path.contains("/auth/validate")
        || path == "/api/v1/broadcast"
        || path == "/netApi/alarm/broadcast"
    {
        return false;
    }
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    ) || (method == Method::GET
        && (path.starts_with("/api/v1/auth/users")
            || path.contains("/export")
            || path == "/api/v1/operation-logs"))
}

pub fn database_path() -> PathBuf {
    std::env::var_os("VOLTAGE_OPERATION_LOG_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::logging::get_log_root().join("operation_logs.db"))
}

#[cfg(unix)]
fn prepare_database_file(path: &std::path::Path) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, chown};

    let file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o664)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(error),
    };
    drop(file);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let owner = std::fs::metadata(parent)?;
    // In local Compose, a root service may create the file before apigateway.
    // Match the mounted directory owner so the unprivileged gateway can write.
    let _ = chown(path, Some(owner.uid()), Some(owner.gid()));
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o664))
}

fn audit_connect_options(path: &std::path::Path) -> SqliteConnectOptions {
    // Leave the new database in SQLite's default DELETE journal mode. The
    // bundled SQLite is 3.46.0, predating the WAL reset race fix.
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(std::time::Duration::from_secs(5))
}

pub async fn pool() -> anyhow::Result<&'static SqlitePool> {
    POOL.get_or_try_init(|| async {
        let path = database_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        #[cfg(unix)]
        prepare_database_file(&path)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(audit_connect_options(&path))
            .await?;
        create_schema(&pool).await?;
        Ok(pool)
    })
    .await
}

pub async fn create_schema(pool: &SqlitePool) -> anyhow::Result<()> {
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await?;
    anyhow::ensure!(
        journal_mode.eq_ignore_ascii_case("delete"),
        "operation log database must use DELETE journal mode (found {journal_mode}); migrate it offline"
    );
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS operation_logs (
            event_id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
            occurred_at_ms INTEGER NOT NULL,
            service TEXT NOT NULL,
            user_id INTEGER,
            username TEXT,
            actor_type TEXT NOT NULL,
            source_service TEXT,
            method TEXT NOT NULL,
            route TEXT NOT NULL,
            path TEXT NOT NULL,
            status INTEGER NOT NULL,
            business_success BOOLEAN,
            duration_ms INTEGER NOT NULL,
            request_id TEXT,
            upload_state INTEGER NOT NULL DEFAULT 0,
            uploaded_at_ms INTEGER
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_operation_time ON operation_logs(occurred_at_ms DESC, event_id DESC)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_operation_upload ON operation_logs(upload_state, occurred_at_ms)")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert(record: NewOperation<'_>) -> anyhow::Result<()> {
    let pool = pool().await?;
    sqlx::query(
        "INSERT INTO operation_logs
         (occurred_at_ms, service, user_id, username, actor_type, source_service, method, route, path, status, business_success, duration_ms, request_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.occurred_at_ms)
    .bind(record.service)
    .bind(record.actor.map(|a| a.user_id))
    .bind(record.actor.map(|a| a.username.as_str()).or(record.submitted_username))
    .bind(record.actor_type)
    .bind(record.source_service)
    .bind(record.method)
    .bind(record.route)
    .bind(record.path)
    .bind(record.status)
    .bind(record.business_success)
    .bind(record.duration_ms)
    .bind(record.request_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn like_pattern(raw: Option<&str>) -> Option<String> {
    raw.map(|s| {
        format!(
            "%{}%",
            s.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        )
    })
}

pub async fn search(filter: &OperationFilter) -> anyhow::Result<(Vec<OperationRow>, i64)> {
    search_with_pool(pool().await?, filter).await
}

async fn search_with_pool(
    pool: &SqlitePool,
    filter: &OperationFilter,
) -> anyhow::Result<(Vec<OperationRow>, i64)> {
    let username = like_pattern(filter.username.as_deref());
    let endpoint = like_pattern(filter.endpoint.as_deref());
    let where_sql = "occurred_at_ms >= ? AND occurred_at_ms < ?
        AND (? IS NULL OR username LIKE ? ESCAPE '\\')
        AND (? IS NULL OR route LIKE ? ESCAPE '\\' OR path LIKE ? ESCAPE '\\')
        AND (? IS NULL OR service = ?)
        AND (? IS NULL OR method = ?)
        AND (? IS NULL OR status = ?)";
    let count_sql = format!("SELECT COUNT(*) FROM operation_logs WHERE {where_sql}");
    let count: i64 = sqlx::query_scalar(&count_sql)
        .bind(filter.from_ms)
        .bind(filter.to_ms)
        .bind(&username)
        .bind(&username)
        .bind(&endpoint)
        .bind(&endpoint)
        .bind(&endpoint)
        .bind(&filter.service)
        .bind(&filter.service)
        .bind(&filter.method)
        .bind(&filter.method)
        .bind(filter.status)
        .bind(filter.status)
        .fetch_one(pool)
        .await?;
    let rows_sql = format!(
        "SELECT event_id, occurred_at_ms, service, user_id, username, actor_type, source_service, method, route, path,
                status, business_success, duration_ms, request_id FROM operation_logs WHERE {where_sql}
         ORDER BY occurred_at_ms DESC, event_id DESC LIMIT ? OFFSET ?"
    );
    let rows = sqlx::query_as::<_, OperationRow>(&rows_sql)
        .bind(filter.from_ms)
        .bind(filter.to_ms)
        .bind(&username)
        .bind(&username)
        .bind(&endpoint)
        .bind(&endpoint)
        .bind(&endpoint)
        .bind(&filter.service)
        .bind(&filter.service)
        .bind(&filter.method)
        .bind(&filter.method)
        .bind(filter.status)
        .bind(filter.status)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(pool)
        .await?;
    Ok((rows, count))
}

pub async fn pending(limit: i64) -> anyhow::Result<Vec<OperationRow>> {
    pending_with_pool(pool().await?, limit).await
}

async fn pending_with_pool(pool: &SqlitePool, limit: i64) -> anyhow::Result<Vec<OperationRow>> {
    Ok(sqlx::query_as::<_, OperationRow>(
        "SELECT event_id, occurred_at_ms, service, user_id, username, actor_type, source_service, method, route, path,
                status, business_success, duration_ms, request_id FROM operation_logs WHERE upload_state = 0
         ORDER BY occurred_at_ms, event_id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn mark_uploaded(event_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE operation_logs SET upload_state = 1, uploaded_at_ms = ? WHERE event_id = ?",
    )
    .bind(now_ms())
    .bind(event_id)
    .execute(pool().await?)
    .await?;
    Ok(())
}

pub async fn count_pending() -> anyhow::Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM operation_logs WHERE upload_state = 0")
            .fetch_one(pool().await?)
            .await?,
    )
}

pub async fn prune_uploaded(before_ms: i64) -> anyhow::Result<u64> {
    Ok(
        sqlx::query("DELETE FROM operation_logs WHERE upload_state = 1 AND occurred_at_ms < ?")
            .bind(before_ms)
            .execute(pool().await?)
            .await?
            .rows_affected(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_and_header_decoding() {
        assert!(should_record(&Method::POST, "/api/channels"));
        assert!(!should_record(&Method::GET, "/health"));
        assert!(!should_record(&Method::GET, "/api/v1/auth/validate"));
        assert_eq!(decode_header_value("%E7%AE%A1%E7%90%86%E5%91%98"), "管理员");
        assert_eq!(
            like_pattern(Some("a%b_c\\d")).as_deref(),
            Some("%a\\%b\\_c\\\\d%")
        );
    }

    #[tokio::test]
    async fn schema_supports_filtered_query_and_pending_order() {
        let db = std::env::temp_dir().join(format!(
            "operation-log-test-{}-{}.db",
            std::process::id(),
            now_ms()
        ));
        let pool = SqlitePoolOptions::new()
            .connect_with(audit_connect_options(&db))
            .await
            .unwrap();
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal_mode, "delete");
        create_schema(&pool).await.unwrap();
        sqlx::query("INSERT INTO operation_logs (occurred_at_ms, service, user_id, username, actor_type, method, route, path, status, duration_ms) VALUES (100, 'comsrv', 4, 'alice', 'user', 'POST', '/api/channels/{id}', '/api/channels/1', 200, 3)")
            .execute(&pool).await.unwrap();
        let filter = OperationFilter {
            from_ms: 0,
            to_ms: 200,
            username: Some("ali".to_owned()),
            endpoint: Some("channels".to_owned()),
            limit: 10,
            ..Default::default()
        };
        let (rows, total) = search_with_pool(&pool, &filter).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows[0].username.as_deref(), Some("alice"));
        let escaped_filter = OperationFilter {
            username: Some("a%".to_owned()),
            ..filter
        };
        assert_eq!(search_with_pool(&pool, &escaped_filter).await.unwrap().1, 0);
        assert_eq!(pending_with_pool(&pool, 10).await.unwrap().len(), 1);
        pool.close().await;
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{}-wal", db.display()));
        let _ = std::fs::remove_file(format!("{}-shm", db.display()));
    }
}
