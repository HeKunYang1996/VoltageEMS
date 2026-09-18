//! Versioned configuration backup, validation, export, import, and restore.
//!
//! The version repository intentionally lives outside the configured data root so
//! a configuration archive never contains the archive history itself. Network
//! configuration under `/etc` is outside the configured data root and is never
//! read or modified here.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{
    Extension, Json,
    body::Body,
    extract::{Multipart, Path as AxumPath, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Acquire, FromRow, Row, SqlitePool};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::auth::Claims;
use crate::state::AppState;

const DEFAULT_DATA_DIR: &str = "/app/data";
const DEFAULT_HISTORY_DIR: &str = "/app/config-history";
const MAX_ARCHIVE_FILES: usize = 10_000;
const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;
const IMPORT_TTL_MINUTES: i64 = 30;
const PRESERVED_AUTH_TABLES: &[&str] = &["roles", "users"];

static CONFIG_OPERATION: AtomicBool = AtomicBool::new(false);

fn data_dir() -> PathBuf {
    std::env::var_os("VOLTAGE_CONFIG_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

fn history_dir() -> PathBuf {
    std::env::var_os("VOLTAGE_CONFIG_HISTORY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HISTORY_DIR))
}

fn archives_dir() -> PathBuf {
    history_dir().join("archives")
}

fn archive_filename(version: &str, created_at: &str) -> String {
    let timestamp = chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|value| value.format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|_| Utc::now().format("%Y%m%d-%H%M%S").to_string());
    format!("Config_{version}_{timestamp}.zip")
}

fn imports_dir() -> PathBuf {
    history_dir().join("imports")
}

struct OperationGuard;

impl Drop for OperationGuard {
    fn drop(&mut self) {
        CONFIG_OPERATION.store(false, Ordering::Release);
    }
}

struct StagingDir(PathBuf);

impl StagingDir {
    fn create() -> io::Result<Self> {
        let path = imports_dir().join(format!("work-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ConfigFailure {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl IntoResponse for ConfigFailure {
    fn into_response(self) -> Response {
        error_response(self.status, self.code, self.message)
    }
}

fn acquire_operation() -> Result<OperationGuard, ConfigFailure> {
    if CONFIG_OPERATION
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ConfigFailure {
            status: StatusCode::CONFLICT,
            code: "CONFIG_OPERATION_IN_PROGRESS",
            message: "Another configuration operation is already running".to_string(),
        });
    }
    Ok(OperationGuard)
}

fn error_response(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({
            "success": false,
            "error": {
                "code": code,
                "message": message.into()
            }
        })),
    )
        .into_response()
}

#[derive(Debug, Clone, Serialize, FromRow)]
struct VersionRow {
    id: String,
    revision: i64,
    version: String,
    origin: String,
    source_version_id: Option<String>,
    source_filename: Option<String>,
    description: Option<String>,
    content_hash: String,
    archive_path: String,
    archive_size: i64,
    operator_id: Option<i64>,
    operator_name: String,
    created_at: String,
    activated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageManifest {
    package_type: String,
    format_version: u32,
    config_version: String,
    created_at: String,
    created_by: String,
    content_hash: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BackupRequest {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RestoreRequest {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct VersionsQuery {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ValidationCheck {
    code: &'static str,
    passed: bool,
    message: String,
}

#[derive(Debug)]
struct ValidatedArchive {
    checks: Vec<ValidationCheck>,
    valid: bool,
    manifest: Option<PackageManifest>,
}

pub async fn initialize(db: &SqlitePool) -> anyhow::Result<()> {
    let pool = history_pool().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM config_versions")
        .fetch_one(&pool)
        .await?;
    if count == 0 && data_dir().exists() {
        let claims = Claims {
            user_id: 0,
            username: "System".to_string(),
            role: Some("System".to_string()),
            token_id: None,
            auth_version: 0,
            exp: 0,
            iat: 0,
            token_type: "system".to_string(),
        };
        create_version(
            db,
            &pool,
            "initial",
            None,
            None,
            Some("Initial local configuration baseline".to_string()),
            &claims,
            false,
        )
        .await?;
    }
    Ok(())
}

async fn history_pool() -> anyhow::Result<SqlitePool> {
    let root = history_dir();
    std::fs::create_dir_all(root.join("archives"))?;
    std::fs::create_dir_all(root.join("imports"))?;
    let db_path = root.join("metadata.db");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(common::bootstrap_database::sqlite_connect_options(
            db_path.to_str().unwrap_or_default(),
        ))
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS config_versions (
            id TEXT PRIMARY KEY,
            revision INTEGER NOT NULL UNIQUE,
            version TEXT NOT NULL UNIQUE,
            origin TEXT NOT NULL,
            source_version_id TEXT,
            source_filename TEXT,
            description TEXT,
            content_hash TEXT NOT NULL,
            archive_path TEXT NOT NULL,
            archive_size INTEGER NOT NULL,
            operator_id INTEGER,
            operator_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            activated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS config_state (
            singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
            current_version_id TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS config_imports (
            id TEXT PRIMARY KEY,
            original_filename TEXT NOT NULL,
            staged_path TEXT NOT NULL,
            package_hash TEXT NOT NULL,
            status TEXT NOT NULL,
            validation_json TEXT NOT NULL,
            operator_id INTEGER,
            operator_name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}

async fn current_version(pool: &SqlitePool) -> anyhow::Result<Option<VersionRow>> {
    sqlx::query_as::<_, VersionRow>(
        "SELECT v.* FROM config_versions v
         JOIN config_state s ON s.current_version_id = v.id
         WHERE s.singleton_id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

fn validate_description(value: &Option<String>) -> Result<(), ConfigFailure> {
    if value.as_deref().unwrap_or_default().chars().count() > 100 {
        return Err(ConfigFailure {
            status: StatusCode::BAD_REQUEST,
            code: "CONFIG_DESCRIPTION_TOO_LONG",
            message: "Description must not exceed 100 characters".to_string(),
        });
    }
    Ok(())
}

async fn checkpoint_database(db: &SqlitePool) {
    if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(FULL)").execute(db).await {
        warn!("Configuration database WAL checkpoint failed: {}", e);
    }
}

fn collect_paths(root: &Path) -> io::Result<Vec<PathBuf>> {
    fn visit(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_dir() {
                visit(&path, paths)?;
            } else if file_type.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.ends_with("-wal") && !name.ends_with("-shm") {
                    paths.push(path);
                }
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    visit(root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn hash_data_tree(root: &Path) -> io::Result<String> {
    let mut hash = Sha256::new();
    for path in collect_paths(root)? {
        let rel = path
            .strip_prefix(root)
            .map_err(|e| io::Error::other(e.to_string()))?;
        hash.update(rel.to_string_lossy().as_bytes());
        let mut file = File::open(path)?;
        let mut buf = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buf)?;
            if read == 0 {
                break;
            }
            hash.update(&buf[..read]);
        }
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn create_archive(root: &Path, manifest: &PackageManifest) -> io::Result<Vec<u8>> {
    let cursor = io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(manifest).map_err(io::Error::other)?)?;

    for path in collect_paths(root)? {
        let rel = path
            .strip_prefix(root)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let rel_name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(rel_name, options)?;
        let mut input = File::open(path)?;
        io::copy(&mut input, &mut zip)?;
    }
    Ok(zip.finish()?.into_inner())
}

#[allow(clippy::too_many_arguments)]
async fn create_version(
    db: &SqlitePool,
    history: &SqlitePool,
    origin: &str,
    source_version_id: Option<String>,
    source_filename: Option<String>,
    description: Option<String>,
    claims: &Claims,
    deduplicate: bool,
) -> anyhow::Result<(VersionRow, bool)> {
    checkpoint_database(db).await;
    let root = data_dir();
    let content_hash = hash_data_tree(&root)?;

    if deduplicate
        && let Some(current) = current_version(history).await?
        && current.content_hash == content_hash
    {
        return Ok((current, false));
    }

    let revision: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(revision), 0) + 1 FROM config_versions")
            .fetch_one(history)
            .await?;
    let id = Uuid::new_v4().to_string();
    let version = format!("V1.0.{}", revision);
    let now = Utc::now().to_rfc3339();
    let manifest = PackageManifest {
        package_type: "voltageems-config".to_string(),
        format_version: 1,
        config_version: version.clone(),
        created_at: now.clone(),
        created_by: claims.username.clone(),
        content_hash: content_hash.clone(),
    };
    let archive = create_archive(&root, &manifest)?;
    let archive_path = archives_dir().join(archive_filename(&version, &now));
    std::fs::write(&archive_path, &archive)?;

    sqlx::query(
        "INSERT INTO config_versions
         (id, revision, version, origin, source_version_id, source_filename,
          description, content_hash, archive_path, archive_size, operator_id,
          operator_name, created_at, activated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(revision)
    .bind(&version)
    .bind(origin)
    .bind(&source_version_id)
    .bind(&source_filename)
    .bind(&description)
    .bind(&content_hash)
    .bind(archive_path.to_string_lossy().as_ref())
    .bind(i64::try_from(archive.len()).unwrap_or(i64::MAX))
    .bind(claims.user_id)
    .bind(&claims.username)
    .bind(&now)
    .bind(&now)
    .execute(history)
    .await?;
    sqlx::query(
        "INSERT INTO config_state (singleton_id, current_version_id, updated_at)
         VALUES (1, ?, ?)
         ON CONFLICT(singleton_id) DO UPDATE SET
           current_version_id = excluded.current_version_id,
           updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&now)
    .execute(history)
    .await?;

    let row = sqlx::query_as::<_, VersionRow>("SELECT * FROM config_versions WHERE id = ?")
        .bind(&id)
        .fetch_one(history)
        .await?;
    Ok((row, true))
}

#[utoipa::path(get, path = "/api/v1/config/current", tag = "Config",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current configuration version")))]
pub async fn get_current() -> Response {
    match history_pool().await {
        Ok(pool) => match current_version(&pool).await {
            Ok(version) => {
                let initialized = version.is_some();
                Json(json!({
                    "success": true,
                    "data": version.map(|v| json!({
                        "id": v.id,
                        "version": v.version,
                        "revision": v.revision,
                        "content_hash": v.content_hash,
                        "updated_at": v.activated_at,
                        "operator": {"id": v.operator_id, "username": v.operator_name}
                    })),
                    "initialized": initialized
                }))
                .into_response()
            },
            Err(e) => {
                error!("Read current config version error: {}", e);
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_HISTORY_READ_FAILED",
                    e.to_string(),
                )
            },
        },
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_HISTORY_UNAVAILABLE",
            e.to_string(),
        ),
    }
}

#[utoipa::path(post, path = "/api/v1/config/backups", tag = "Config",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "No changes"), (status = 201, description = "Backup created")))]
pub async fn backup_current(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<BackupRequest>,
) -> Response {
    if let Err(error) = validate_description(&request.description) {
        return error.into_response();
    }
    let _guard = match acquire_operation() {
        Ok(guard) => guard,
        Err(error) => return error.into_response(),
    };
    let history = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    match create_version(
        &state.db,
        &history,
        "manual_backup",
        None,
        None,
        request.description,
        &claims,
        true,
    )
    .await
    {
        Ok((version, created)) => (
            if created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            },
            Json(json!({
                "success": true,
                "data": {
                    "created": created,
                    "reason": if created { Value::Null } else { json!("configuration_unchanged") },
                    "version": version
                }
            })),
        )
            .into_response(),
        Err(e) => {
            error!("Create config backup error: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_BACKUP_FAILED",
                e.to_string(),
            )
        },
    }
}

#[utoipa::path(get, path = "/api/v1/config/versions", tag = "Config",
    security(("bearer_auth" = [])), params(VersionsQuery),
    responses((status = 200, description = "Configuration version history")))]
pub async fn list_versions(Query(query): Query<VersionsQuery>) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let pool = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    let current_id: Option<String> =
        sqlx::query_scalar("SELECT current_version_id FROM config_state WHERE singleton_id = 1")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM config_versions")
        .fetch_one(&pool)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_READ_FAILED",
                e.to_string(),
            );
        },
    };
    let rows = match sqlx::query_as::<_, VersionRow>(
        "SELECT * FROM config_versions ORDER BY revision DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_READ_FAILED",
                e.to_string(),
            );
        },
    };
    let items: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            let status = if current_id.as_deref() == Some(row.id.as_str()) {
                "current"
            } else {
                "history"
            };
            json!({
                "id": row.id,
                "version": row.version,
                "status": status,
                "origin": row.origin,
                "source_version_id": row.source_version_id,
                "source_filename": row.source_filename,
                "activated_at": row.activated_at,
                "operator": row.operator_name,
                "description": row.description,
                "archive_size": row.archive_size
            })
        })
        .collect();
    Json(json!({
        "success": true,
        "data": {
            "items": items,
            "pagination": {
                "page": page,
                "page_size": page_size,
                "total": total,
                "total_pages": (total + page_size - 1) / page_size
            }
        }
    }))
    .into_response()
}

fn archive_download(data: Vec<u8>, filename: &str) -> Response {
    match Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
    {
        Ok(response) => response,
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_EXPORT_FAILED",
            e.to_string(),
        ),
    }
}

#[utoipa::path(get, path = "/api/v1/config/versions/{version_id}/export", tag = "Config",
    security(("bearer_auth" = [])),
    params(("version_id" = String, Path, description = "Configuration version ID")),
    responses((status = 200, description = "Version ZIP archive"), (status = 404, description = "Version not found")))]
pub async fn export_version(AxumPath(version_id): AxumPath<String>) -> Response {
    let pool = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    let row = match sqlx::query_as::<_, VersionRow>("SELECT * FROM config_versions WHERE id = ?")
        .bind(&version_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "CONFIG_VERSION_NOT_FOUND",
                "Configuration version not found",
            );
        },
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_READ_FAILED",
                e.to_string(),
            );
        },
    };
    match std::fs::read(&row.archive_path) {
        Ok(data) => archive_download(data, &archive_filename(&row.version, &row.created_at)),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ARCHIVE_MISSING",
            e.to_string(),
        ),
    }
}

#[utoipa::path(get, path = "/api/v1/config/export", tag = "Config",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current configuration ZIP archive")))]
pub async fn export_current(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Response {
    checkpoint_database(&state.db).await;
    let root = data_dir();
    if !root.exists() {
        return error_response(
            StatusCode::NOT_FOUND,
            "CONFIG_DATA_NOT_FOUND",
            "Configuration data directory was not found",
        );
    }
    let history = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    let version = current_version(&history)
        .await
        .ok()
        .flatten()
        .map(|v| v.version)
        .unwrap_or_else(|| "unversioned".to_string());
    let content_hash = match hash_data_tree(&root) {
        Ok(hash) => hash,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_EXPORT_FAILED",
                e.to_string(),
            );
        },
    };
    let created_at = Utc::now().to_rfc3339();
    let manifest = PackageManifest {
        package_type: "voltageems-config".to_string(),
        format_version: 1,
        config_version: version.clone(),
        created_at: created_at.clone(),
        created_by: claims.username,
        content_hash,
    };
    match create_archive(&root, &manifest) {
        Ok(data) => archive_download(data, &archive_filename(&version, &created_at)),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_EXPORT_FAILED",
            e.to_string(),
        ),
    }
}

fn validate_archive(data: &[u8]) -> io::Result<ValidatedArchive> {
    let mut archive = zip::ZipArchive::new(io::Cursor::new(data))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut safe = archive.len() <= MAX_ARCHIVE_FILES;
    let mut total = 0_u64;
    let mut has_database = false;
    let mut manifest = None;
    let mut content_entries = Vec::new();

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if file.enclosed_name().is_none() {
            safe = false;
        }
        total = total.saturating_add(file.size());
        if total > MAX_EXTRACTED_BYTES {
            safe = false;
        }
        let name = file.name().replace('\\', "/");
        if name == "manifest.json" {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            manifest = serde_json::from_slice::<PackageManifest>(&bytes).ok();
        } else if !file.is_dir() {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if name == "voltage.db" && bytes.starts_with(b"SQLite format 3\0") {
                has_database = true;
            }
            content_entries.push((name, bytes));
        }
    }

    let compatible = manifest
        .as_ref()
        .is_some_and(|m| m.package_type == "voltageems-config" && m.format_version == 1);
    content_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut content_hasher = Sha256::new();
    for (name, bytes) in &content_entries {
        content_hasher.update(name.as_bytes());
        content_hasher.update(bytes);
    }
    let actual_content_hash = format!("sha256:{:x}", content_hasher.finalize());
    let checksum_matches = manifest
        .as_ref()
        .is_some_and(|value| value.content_hash == actual_content_hash);
    let checks = vec![
        ValidationCheck {
            code: "archive_safety",
            passed: safe,
            message: if safe {
                "File and extraction safety check passed".to_string()
            } else {
                "Archive contains an unsafe path or exceeds extraction limits".to_string()
            },
        },
        ValidationCheck {
            code: "package_structure",
            passed: has_database && manifest.is_some() && checksum_matches,
            message: if has_database && manifest.is_some() && checksum_matches {
                "Package structure, required files, and content checksum check passed".to_string()
            } else {
                "manifest.json or voltage.db is invalid, or the content checksum does not match"
                    .to_string()
            },
        },
        ValidationCheck {
            code: "format_compatibility",
            passed: compatible,
            message: if compatible {
                "Configuration format is compatible".to_string()
            } else {
                "Configuration package format is not supported".to_string()
            },
        },
        ValidationCheck {
            code: "reference_integrity",
            passed: has_database,
            message: "Configuration database integrity has not been checked".to_string(),
        },
    ];
    Ok(ValidatedArchive {
        valid: safe && has_database && compatible && checksum_matches,
        checks,
        manifest,
    })
}

fn database_from_archive(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(io::Cursor::new(data))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut database = archive
        .by_name("voltage.db")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut bytes = Vec::new();
    database.read_to_end(&mut bytes)?;
    Ok(bytes)
}

async fn validate_database_integrity(data: &[u8]) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!(
        "voltageems-config-validation-{}.db",
        Uuid::new_v4()
    ));
    let bytes = database_from_archive(data).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;

    let result = async {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(common::bootstrap_database::sqlite_connect_options(
                path.to_str().unwrap_or_default(),
            ))
            .await
            .map_err(|e| e.to_string())?;
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string())?;
        if integrity != "ok" {
            pool.close().await;
            return Err(format!("SQLite integrity_check returned: {integrity}"));
        }
        let foreign_key_violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;
        pool.close().await;
        let relevant_violations = foreign_key_violations
            .iter()
            .filter(|row| {
                row.try_get::<String, _>("table")
                    .map(|table| !PRESERVED_AUTH_TABLES.contains(&table.as_str()))
                    .unwrap_or(true)
            })
            .count();
        if relevant_violations > 0 {
            return Err(format!(
                "SQLite foreign_key_check found {} violation(s)",
                relevant_violations
            ));
        }
        Ok(())
    }
    .await;

    let _ = std::fs::remove_file(path);
    result
}

async fn validate_package(data: &[u8]) -> io::Result<ValidatedArchive> {
    let mut validation = validate_archive(data)?;
    let database_result = if validation.valid {
        validate_database_integrity(data).await
    } else {
        Err("Earlier package checks failed".to_string())
    };
    if let Some(check) = validation
        .checks
        .iter_mut()
        .find(|check| check.code == "reference_integrity")
    {
        check.passed = database_result.is_ok();
        check.message = match database_result {
            Ok(()) => "SQLite integrity and business foreign-key checks passed".to_string(),
            Err(message) => format!("Configuration database integrity check failed: {message}"),
        };
    }
    validation.valid = validation.checks.iter().all(|check| check.passed);
    Ok(validation)
}

#[utoipa::path(post, path = "/api/v1/config/imports/validate", tag = "Config",
    security(("bearer_auth" = [])),
    request_body(content_type = "multipart/form-data", description = "Configuration ZIP package"),
    responses((status = 200, description = "Package validation result"), (status = 422, description = "Invalid package")))]
pub async fn validate_import(
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Response {
    let mut filename = "configuration.zip".to_string();
    let mut data = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") || data.is_none() {
            if let Some(name) = field.file_name() {
                filename = name.to_string();
            }
            match field.bytes().await {
                Ok(bytes) => data = Some(bytes.to_vec()),
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CONFIG_UPLOAD_FAILED",
                        e.to_string(),
                    );
                },
            }
        }
    }
    let Some(data) = data.filter(|d| !d.is_empty()) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "CONFIG_PACKAGE_MISSING",
            "No configuration package received",
        );
    };
    let validation = match validate_package(&data).await {
        Ok(validation) => validation,
        Err(e) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "CONFIG_PACKAGE_STRUCTURE_INVALID",
                e.to_string(),
            );
        },
    };
    let pool = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    let id = Uuid::new_v4().to_string();
    let path = imports_dir().join(format!("{}.zip", id));
    if let Err(e) = std::fs::write(&path, &data) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_IMPORT_STAGE_FAILED",
            e.to_string(),
        );
    }
    let package_hash = format!("sha256:{:x}", Sha256::digest(&data));
    let created_at = Utc::now();
    let expires_at = created_at + Duration::minutes(IMPORT_TTL_MINUTES);
    let validation_json =
        serde_json::to_string(&validation.checks).unwrap_or_else(|_| "[]".to_string());
    let status = if validation.valid {
        "validated"
    } else {
        "invalid"
    };
    if let Err(e) = sqlx::query(
        "INSERT INTO config_imports
         (id, original_filename, staged_path, package_hash, status, validation_json,
          operator_id, operator_name, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&filename)
    .bind(path.to_string_lossy().as_ref())
    .bind(&package_hash)
    .bind(status)
    .bind(&validation_json)
    .bind(claims.user_id)
    .bind(&claims.username)
    .bind(created_at.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .execute(&pool)
    .await
    {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_IMPORT_STAGE_FAILED",
            e.to_string(),
        );
    }

    let package = validation.manifest.as_ref().map(|m| {
        json!({
            "config_version": m.config_version,
            "format_version": m.format_version,
            "created_at": m.created_at,
            "created_by": m.created_by
        })
    });
    (
        if validation.valid {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        },
        Json(json!({
            "success": validation.valid,
            "data": {
                "import_id": id,
                "filename": filename,
                "size": data.len(),
                "expires_at": expires_at.to_rfc3339(),
                "valid": validation.valid,
                "checks": validation.checks,
                "package": package
            }
        })),
    )
        .into_response()
}

/// Compatibility endpoint for clients that still use the original one-step
/// import API. It uses the same validation, snapshot, and versioning pipeline
/// as the two-step import flow.
#[utoipa::path(post, path = "/api/v1/config/import", tag = "Config",
    security(("bearer_auth" = [])),
    request_body(content_type = "multipart/form-data", description = "Configuration ZIP package"),
    responses((status = 200, description = "Configuration imported"), (status = 422, description = "Invalid package")))]
pub async fn import_compat(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Response {
    let _guard = match acquire_operation() {
        Ok(guard) => guard,
        Err(error) => return error.into_response(),
    };
    let mut filename = "configuration.zip".to_string();
    let mut data = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") || data.is_none() {
            if let Some(name) = field.file_name() {
                filename = name.to_string();
            }
            match field.bytes().await {
                Ok(bytes) => data = Some(bytes.to_vec()),
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CONFIG_UPLOAD_FAILED",
                        e.to_string(),
                    );
                },
            }
        }
    }
    let Some(data) = data.filter(|data| !data.is_empty()) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "CONFIG_PACKAGE_MISSING",
            "No configuration package received",
        );
    };
    let history = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    match apply_archive(
        &state,
        &history,
        &data,
        "import",
        None,
        Some(filename),
        None,
        &claims,
    )
    .await
    {
        Ok(result) => Json(json!({"success": true, "data": result})).into_response(),
        Err(response) => response,
    }
}

fn extract_archive(data: &[u8], target: &Path) -> io::Result<usize> {
    let mut archive = zip::ZipArchive::new(io::Cursor::new(data))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut count = 0;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let Some(name) = file.enclosed_name() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsafe ZIP entry path",
            ));
        };
        if name == Path::new("manifest.json") {
            continue;
        }
        let out_path = target.join(name);
        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(out_path)?;
        io::copy(&mut file, &mut out)?;
        count += 1;
    }
    Ok(count)
}

fn copy_staged_data(source: &Path, target: &Path) -> io::Result<()> {
    std::fs::create_dir_all(target)?;
    for path in collect_paths(source)? {
        let relative = path
            .strip_prefix(source)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(path, destination)?;
    }
    Ok(())
}

async fn preserve_local_auth(
    imported_database: &Path,
    local_database: &Path,
) -> anyhow::Result<()> {
    let imported_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(common::bootstrap_database::sqlite_connect_options(
            imported_database.to_str().unwrap_or_default(),
        ))
        .await?;
    crate::db::create_tables(&imported_pool).await?;

    let mut connection = imported_pool.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS local_auth")
        .bind(local_database.to_string_lossy().as_ref())
        .execute(&mut *connection)
        .await?;

    let copy_result: Result<(), sqlx::Error> = async {
        let mut transaction = connection.begin().await?;
        sqlx::query("DELETE FROM users")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM roles")
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "INSERT INTO roles (id, name_en, name_zh, description, created_at, updated_at)
             SELECT id, name_en, name_zh, description, created_at, updated_at
             FROM local_auth.roles",
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO users
             (id, username, password_hash, role_id, is_active, auth_version,
              last_login, created_at, updated_at)
             SELECT id, username, password_hash, role_id, is_active, auth_version,
                    last_login, created_at, updated_at
             FROM local_auth.users",
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
    .await;

    let detach_result = sqlx::query("DETACH DATABASE local_auth")
        .execute(&mut *connection)
        .await;
    drop(connection);
    if copy_result.is_ok() {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&imported_pool)
            .await?;
    }
    imported_pool.close().await;
    copy_result?;
    detach_result?;
    Ok(())
}

fn restart_core_services() -> Vec<Value> {
    let services = [
        "voltageems-comsrv",
        "voltageems-modsrv",
        "voltageems-hissrv",
        "voltageems-netsrv",
        "voltageems-alarmsrv",
    ];
    services
        .iter()
        .map(|service| {
            match std::process::Command::new("docker")
                .args(["restart", service])
                .output()
            {
                Ok(output) if output.status.success() => {
                    json!({"service": service, "success": true})
                },
                Ok(output) => json!({
                    "service": service,
                    "success": false,
                    "error": String::from_utf8_lossy(&output.stderr)
                }),
                Err(e) => json!({"service": service, "success": false, "error": e.to_string()}),
            }
        })
        .collect()
}

fn rollback_snapshot(snapshot: &VersionRow) -> Value {
    match std::fs::read(&snapshot.archive_path)
        .and_then(|archive| extract_archive(&archive, &data_dir()))
    {
        Ok(_) => {
            let restart_results = restart_core_services();
            let succeeded = restart_results
                .iter()
                .all(|result| result["success"].as_bool().unwrap_or(false));
            json!({
                "succeeded": succeeded,
                "restart_results": restart_results
            })
        },
        Err(e) => json!({
            "succeeded": false,
            "error": e.to_string()
        }),
    }
}

async fn apply_archive(
    state: &AppState,
    history: &SqlitePool,
    data: &[u8],
    origin: &str,
    source_version_id: Option<String>,
    source_filename: Option<String>,
    description: Option<String>,
    claims: &Claims,
) -> Result<Value, Response> {
    let validation = validate_package(data).await.map_err(|e| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIG_PACKAGE_STRUCTURE_INVALID",
            e.to_string(),
        )
    })?;
    if !validation.valid {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIG_PACKAGE_VALIDATION_FAILED",
            "Configuration package validation failed",
        ));
    }

    let (rollback_version, _) = create_version(
        &state.db,
        history,
        if origin == "import" {
            "pre_import_snapshot"
        } else {
            "pre_restore_snapshot"
        },
        None,
        None,
        Some(if origin == "import" {
            "Automatic snapshot before configuration import".to_string()
        } else {
            "Automatic snapshot before configuration restore".to_string()
        }),
        claims,
        false,
    )
    .await
    .map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_SNAPSHOT_FAILED",
            e.to_string(),
        )
    })?;

    let staging = StagingDir::create().map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_IMPORT_STAGE_FAILED",
            e.to_string(),
        )
    })?;
    let files_extracted = extract_archive(data, staging.path()).map_err(|e| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIG_PACKAGE_STRUCTURE_INVALID",
            e.to_string(),
        )
    })?;
    preserve_local_auth(
        &staging.path().join("voltage.db"),
        Path::new(&state.config.db_path),
    )
    .await
    .map_err(|e| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIG_AUTH_PRESERVATION_FAILED",
            format!("Failed to preserve local users and roles: {e}"),
        )
    })?;

    match copy_staged_data(staging.path(), &data_dir()) {
        Ok(()) => {},
        Err(e) => {
            let rollback = rollback_snapshot(&rollback_version);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": {
                        "code": "CONFIG_APPLY_FAILED",
                        "message": e.to_string(),
                        "details": {"rollback": rollback}
                    }
                })),
            )
                .into_response());
        },
    }
    let restart_results = restart_core_services();
    let all_restarted = restart_results
        .iter()
        .all(|result| result["success"].as_bool().unwrap_or(false));
    if !all_restarted {
        let rollback = rollback_snapshot(&rollback_version);
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "success": false,
                "error": {
                    "code": "CONFIG_SERVICE_RESTART_FAILED",
                    "message": "Configuration was applied, but one or more services failed to restart",
                    "details": {
                        "restart_results": restart_results,
                        "rollback": rollback
                    }
                }
            })),
        )
            .into_response());
    }

    let (version, _) = create_version(
        &state.db,
        history,
        origin,
        source_version_id,
        source_filename,
        description,
        claims,
        false,
    )
    .await
    .map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_VERSION_CREATE_FAILED",
            e.to_string(),
        )
    })?;
    Ok(json!({
        "version": version,
        "files_extracted": files_extracted,
        "preserved_local_tables": PRESERVED_AUTH_TABLES,
        "restart_results": restart_results
    }))
}

#[utoipa::path(post, path = "/api/v1/config/imports/{import_id}/apply", tag = "Config",
    security(("bearer_auth" = [])),
    params(("import_id" = String, Path, description = "Validated import ID")),
    responses((status = 200, description = "Configuration imported")))]
pub async fn apply_import(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(import_id): AxumPath<String>,
    Json(request): Json<BackupRequest>,
) -> Response {
    if let Err(error) = validate_description(&request.description) {
        return error.into_response();
    }
    let _guard = match acquire_operation() {
        Ok(guard) => guard,
        Err(error) => return error.into_response(),
    };
    let history = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    let row = match sqlx::query(
        "SELECT original_filename, staged_path, package_hash, status, expires_at
         FROM config_imports WHERE id = ?",
    )
    .bind(&import_id)
    .fetch_optional(&history)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "CONFIG_IMPORT_NOT_FOUND",
                "Validated import was not found",
            );
        },
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_READ_FAILED",
                e.to_string(),
            );
        },
    };
    let status: String = row.get("status");
    if status != "validated" {
        return error_response(
            StatusCode::CONFLICT,
            "CONFIG_IMPORT_NOT_VALIDATED",
            "Import is not in validated state",
        );
    }
    let expires_at: String = row.get("expires_at");
    if chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|value| value.with_timezone(&Utc) < Utc::now())
        .unwrap_or(true)
    {
        return error_response(
            StatusCode::GONE,
            "CONFIG_IMPORT_EXPIRED",
            "Validated import has expired",
        );
    }
    let path: String = row.get("staged_path");
    let filename: String = row.get("original_filename");
    let expected_hash: String = row.get("package_hash");
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ARCHIVE_MISSING",
                e.to_string(),
            );
        },
    };
    let actual_hash = format!("sha256:{:x}", Sha256::digest(&data));
    if actual_hash != expected_hash {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "CONFIG_PACKAGE_CHECKSUM_MISMATCH",
            "Staged import package has changed",
        );
    }
    match apply_archive(
        &state,
        &history,
        &data,
        "import",
        None,
        Some(filename),
        request.description,
        &claims,
    )
    .await
    {
        Ok(result) => {
            let _ = sqlx::query("UPDATE config_imports SET status = 'applied' WHERE id = ?")
                .bind(&import_id)
                .execute(&history)
                .await;
            info!("Configuration import applied: {}", import_id);
            Json(json!({"success": true, "data": result})).into_response()
        },
        Err(response) => response,
    }
}

#[utoipa::path(post, path = "/api/v1/config/versions/{version_id}/restore", tag = "Config",
    security(("bearer_auth" = [])),
    params(("version_id" = String, Path, description = "Configuration version ID")),
    responses((status = 200, description = "Configuration restored")))]
pub async fn restore_version(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(version_id): AxumPath<String>,
    Json(request): Json<RestoreRequest>,
) -> Response {
    if let Err(error) = validate_description(&request.description) {
        return error.into_response();
    }
    let _guard = match acquire_operation() {
        Ok(guard) => guard,
        Err(error) => return error.into_response(),
    };
    let history = match history_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_UNAVAILABLE",
                e.to_string(),
            );
        },
    };
    if current_version(&history)
        .await
        .ok()
        .flatten()
        .is_some_and(|current| current.id == version_id)
    {
        return error_response(
            StatusCode::CONFLICT,
            "CONFIG_VERSION_ALREADY_CURRENT",
            "The selected version is already current",
        );
    }
    let source = match sqlx::query_as::<_, VersionRow>("SELECT * FROM config_versions WHERE id = ?")
        .bind(&version_id)
        .fetch_optional(&history)
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "CONFIG_VERSION_NOT_FOUND",
                "Configuration version not found",
            );
        },
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_HISTORY_READ_FAILED",
                e.to_string(),
            );
        },
    };
    let data = match std::fs::read(&source.archive_path) {
        Ok(data) => data,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ARCHIVE_MISSING",
                e.to_string(),
            );
        },
    };
    match apply_archive(
        &state,
        &history,
        &data,
        "restore",
        Some(source.id),
        None,
        request.description,
        &claims,
    )
    .await
    {
        Ok(result) => Json(json!({"success": true, "data": result})).into_response(),
        Err(response) => response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_readable_archive_filename() {
        assert_eq!(
            archive_filename("V1.0.5", "2026-09-18T10:30:00+08:00"),
            "Config_V1.0.5_20260918-103000.zip"
        );
    }

    #[cfg(unix)]
    #[test]
    fn archive_collection_skips_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let outside = temp.path().join("host-secret");
        std::fs::write(&outside, b"must not be exported").unwrap();
        symlink(&outside, data.join("linked-secret")).unwrap();

        assert!(collect_paths(&data).unwrap().is_empty());
    }

    #[test]
    fn rejects_path_traversal() {
        let cursor = io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("../outside", options).unwrap();
        writer.write_all(b"bad").unwrap();
        let data = writer.finish().unwrap().into_inner();
        let result = validate_archive(&data).unwrap();
        assert!(!result.valid);
        assert!(!result.checks[0].passed);
    }

    #[tokio::test]
    async fn validates_generated_archive() {
        let temp = tempfile::tempdir().unwrap();
        let database_path = temp.path().join("voltage.db");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(common::bootstrap_database::sqlite_connect_options(
                database_path.to_str().unwrap(),
            ))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE config_test (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO config_test (value) VALUES (?)")
            .bind("ok")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let content_hash = hash_data_tree(temp.path()).unwrap();
        let manifest = PackageManifest {
            package_type: "voltageems-config".to_string(),
            format_version: 1,
            config_version: "V1.0.1".to_string(),
            created_at: Utc::now().to_rfc3339(),
            created_by: "test".to_string(),
            content_hash,
        };
        let data = create_archive(temp.path(), &manifest).unwrap();
        let result = validate_package(&data).await.unwrap();
        assert!(result.valid);
        assert!(result.checks.iter().all(|check| check.passed));
    }

    #[tokio::test]
    async fn imported_database_keeps_local_users_and_roles() {
        let temp = tempfile::tempdir().unwrap();
        let local_path = temp.path().join("local.db");
        let imported_path = temp.path().join("imported.db");

        let local = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(common::bootstrap_database::sqlite_connect_options(
                local_path.to_str().unwrap(),
            ))
            .await
            .unwrap();
        crate::db::create_tables(&local).await.unwrap();
        crate::db::init_roles(&local).await.unwrap();
        sqlx::query("UPDATE roles SET description = 'local-role' WHERE id = 1")
            .execute(&local)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, role_id) VALUES ('local-admin', 'local-hash', 1)",
        )
        .execute(&local)
        .await
        .unwrap();
        sqlx::query("INSERT INTO calculated_points (name) VALUES ('local-point')")
            .execute(&local)
            .await
            .unwrap();

        let imported = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(common::bootstrap_database::sqlite_connect_options(
                imported_path.to_str().unwrap(),
            ))
            .await
            .unwrap();
        crate::db::create_tables(&imported).await.unwrap();
        crate::db::init_roles(&imported).await.unwrap();
        sqlx::query("UPDATE roles SET description = 'imported-role' WHERE id = 1")
            .execute(&imported)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, role_id) VALUES ('imported-admin', 'imported-hash', 1)",
        )
        .execute(&imported)
        .await
        .unwrap();
        sqlx::query("INSERT INTO calculated_points (name) VALUES ('imported-point')")
            .execute(&imported)
            .await
            .unwrap();
        imported.close().await;

        preserve_local_auth(&imported_path, &local_path)
            .await
            .unwrap();

        let result = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(common::bootstrap_database::sqlite_connect_options(
                imported_path.to_str().unwrap(),
            ))
            .await
            .unwrap();
        let usernames: Vec<String> = sqlx::query_scalar("SELECT username FROM users ORDER BY id")
            .fetch_all(&result)
            .await
            .unwrap();
        let role_description: String =
            sqlx::query_scalar("SELECT description FROM roles WHERE id = 1")
                .fetch_one(&result)
                .await
                .unwrap();
        let point_name: String =
            sqlx::query_scalar("SELECT name FROM calculated_points ORDER BY id LIMIT 1")
                .fetch_one(&result)
                .await
                .unwrap();
        assert_eq!(usernames, vec!["local-admin"]);
        assert_eq!(role_description, "local-role");
        assert_eq!(point_name, "imported-point");

        result.close().await;
        local.close().await;
    }
}
