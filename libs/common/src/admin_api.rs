//! Admin API handlers for service management
//!
//! Provides shared endpoints for all Rust services:
//! - Dynamic log level adjustment
//! - Log file listing and viewing
//!
//! Usage in services:
//! ```ignore
//! use common::admin_api::{set_log_level, get_log_level, list_log_files, view_log_file};
//!
//! // In routes:
//! .route("/api/admin/logs/level", post(set_log_level).get(get_log_level))
//! .route("/api/admin/logs/files", get(list_log_files))
//! .route("/api/admin/logs/view", get(view_log_file))
//! ```

use axum::extract::Query;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};

/// Request to set log level
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SetLogLevelRequest {
    /// Log level string (e.g., "debug", "info", "warn", "error", "trace")
    /// or full filter spec (e.g., "info,service=debug")
    pub level: String,
}

/// Response for log level operations
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LogLevelResponse {
    /// Current log level
    pub level: String,
    /// Operation status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Set log level dynamically
///
/// POST /api/admin/logs/level
/// Body: {"level": "debug"}
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/api/admin/logs/level",
    request_body = SetLogLevelRequest,
    responses(
        (status = 200, description = "Log level updated successfully", body = LogLevelResponse),
        (status = 400, description = "Invalid log level", body = LogLevelResponse)
    ),
    tag = "admin"
))]
pub async fn set_log_level(Json(req): Json<SetLogLevelRequest>) -> impl IntoResponse {
    match crate::logging::set_log_level(&req.level) {
        Ok(_) => (
            StatusCode::OK,
            Json(LogLevelResponse {
                level: req.level,
                status: Some("ok".to_string()),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(LogLevelResponse {
                level: crate::logging::get_log_level(),
                status: None,
                error: Some(e),
            }),
        ),
    }
}

/// Get current log level
///
/// GET /api/admin/logs/level
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/admin/logs/level",
    responses(
        (status = 200, description = "Current log level", body = LogLevelResponse)
    ),
    tag = "admin"
))]
pub async fn get_log_level() -> impl IntoResponse {
    Json(LogLevelResponse {
        level: crate::logging::get_log_level(),
        status: None,
        error: None,
    })
}

// ── Log file access endpoints ───────────────────────────────────────

/// Query params for listing log files
#[derive(Debug, Deserialize)]
pub struct ListLogFilesQuery {
    /// Date prefix filter (YYYYMMDD). Default: today.
    pub date: Option<String>,
    /// Service name filter (optional).
    pub service: Option<String>,
}

/// A single log file entry
#[derive(Debug, Serialize)]
pub struct LogFileEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    pub name: String,
    pub size: u64,
}

fn is_safe_path_component(value: &str) -> bool {
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn append_log_files(
    files: &mut Vec<LogFileEntry>,
    dir: &Path,
    service: Option<&str>,
    date_prefix: &str,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(date_prefix) || !name.ends_with(".log") {
            continue;
        }
        if let Some(svc) = service
            && !name[date_prefix.len()..].starts_with(&format!("_{svc}"))
        {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(LogFileEntry {
            service: service.map(str::to_string),
            name,
            size,
        });
    }

    Ok(())
}

fn resolve_log_file(log_root: &Path, service: Option<&str>, file: &str) -> Option<PathBuf> {
    if !is_safe_path_component(file) {
        return None;
    }

    if let Some(service) = service {
        if !is_safe_path_component(service) {
            return None;
        }
        let service_file = log_root.join(service).join(file);
        if service_file.is_file() {
            return Some(service_file);
        }
    }

    // Backward compatibility for logs written before per-service directories.
    let legacy_file = log_root.join(file);
    if legacy_file.is_file() {
        return Some(legacy_file);
    }

    // Preserve the old API shape where callers supplied only `file`.
    // Search exactly one directory level: {root}/{service}/{file}.
    if service.is_none()
        && let Ok(entries) = std::fs::read_dir(log_root)
    {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|ty| ty.is_dir()) {
                let candidate = entry.path().join(file);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// List log files in the log directory
///
/// GET /api/admin/logs/files?date=YYYYMMDD&service=comsrv
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/admin/logs/files",
    params(
        ("date" = Option<String>, Query, description = "Date filter (YYYYMMDD, default: today)"),
        ("service" = Option<String>, Query, description = "Service name filter"),
    ),
    responses(
        (status = 200, description = "List of log files"),
        (status = 500, description = "Failed to read log directory")
    ),
    tag = "admin"
))]
pub async fn list_log_files(Query(q): Query<ListLogFilesQuery>) -> impl IntoResponse {
    let log_root = crate::logging::get_log_root();
    let date_prefix = q
        .date
        .unwrap_or_else(|| chrono::Local::now().format("%Y%m%d").to_string());

    let mut files: Vec<LogFileEntry> = Vec::new();
    let result = if let Some(service) = q.service.as_deref() {
        if !is_safe_path_component(service) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid service name"})),
            );
        }

        let service_dir = log_root.join(service);
        if service_dir.is_dir() {
            append_log_files(&mut files, &service_dir, Some(service), &date_prefix)
        } else {
            // Fall back to legacy flat files during migration.
            append_log_files(&mut files, &log_root, Some(service), &date_prefix)
        }
    } else {
        // Include legacy flat files, then scan each service directory once.
        let mut result = append_log_files(&mut files, &log_root, None, &date_prefix);
        if result.is_ok() {
            result = std::fs::read_dir(&log_root).and_then(|entries| {
                for entry in entries {
                    let entry = entry?;
                    if !entry.file_type()?.is_dir() {
                        continue;
                    }
                    let service = entry.file_name().to_string_lossy().to_string();
                    if !is_safe_path_component(&service) {
                        continue;
                    }
                    append_log_files(&mut files, &entry.path(), Some(&service), &date_prefix)?;
                }
                Ok(())
            });
        }
        result
    };

    if let Err(e) = result {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Cannot read log dir: {}", e)})),
        );
    }
    files.sort_by(|a, b| (&a.service, &a.name).cmp(&(&b.service, &b.name)));

    (StatusCode::OK, Json(serde_json::json!({"files": files})))
}

/// Query params for viewing log file content
#[derive(Debug, Deserialize)]
pub struct ViewLogFileQuery {
    /// Service directory containing the log file.
    pub service: Option<String>,
    /// Log file name (e.g., "20260325_comsrv.log"). No path separators allowed.
    pub file: String,
    /// Number of lines from end (default: 50)
    pub lines: Option<usize>,
    /// Case-insensitive grep filter
    pub grep: Option<String>,
}

/// View last N lines of a log file
///
/// GET /api/admin/logs/view?service=comsrv&file=20260325_comsrv.log&lines=50&grep=ERROR
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/admin/logs/view",
    params(
        ("service" = Option<String>, Query, description = "Service directory containing the log file"),
        ("file" = String, Query, description = "Log file name (no path separators)"),
        ("lines" = Option<usize>, Query, description = "Lines from end (default: 50)"),
        ("grep" = Option<String>, Query, description = "Case-insensitive filter"),
    ),
    responses(
        (status = 200, description = "Log file content"),
        (status = 400, description = "Invalid file name"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Read error")
    ),
    tag = "admin"
))]
pub async fn view_log_file(Query(q): Query<ViewLogFileQuery>) -> impl IntoResponse {
    let log_root = crate::logging::get_log_root();

    if !is_safe_path_component(&q.file)
        || q.service
            .as_deref()
            .is_some_and(|service| !is_safe_path_component(service))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid service or file name"})),
        );
    }

    // Security: canonicalize resolves symlinks and `..` components at the OS level,
    // then assert the resolved path is still inside the log root.
    // This replaces character-based denylist checks and also handles the TOCTOU
    // race between exists() and open() — canonicalize() fails if the file is absent.
    let canonical_root = match log_root.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Log directory unavailable"})),
            );
        },
    };
    let Some(log_file) = resolve_log_file(&log_root, q.service.as_deref(), &q.file) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("File not found: {}", q.file)})),
        );
    };
    let canonical = match log_file.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("File not found: {}", q.file)})),
            );
        },
    };
    if !canonical.starts_with(&canonical_root) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid file name"})),
        );
    }

    // Cap lines to avoid excessive memory use on embedded ARM64 target.
    let n = q.lines.unwrap_or(50).min(10_000);
    let grep_pattern = q.grep.as_ref().map(|s| s.to_lowercase());

    // Offload blocking file I/O to the blocking thread pool so we don't
    // starve the tokio async executor.
    // Use a ring buffer (VecDeque<N>) so memory is O(n) regardless of file size.
    let read_result =
        tokio::task::spawn_blocking(move || -> std::io::Result<(usize, Vec<String>)> {
            let file = std::fs::File::open(&canonical)?;
            let reader = BufReader::new(file);
            let mut total = 0usize;
            let mut ring: std::collections::VecDeque<String> =
                std::collections::VecDeque::with_capacity(n);
            for line in reader.lines().map_while(Result::ok) {
                let matches = match &grep_pattern {
                    Some(pat) => line.to_lowercase().contains(pat.as_str()),
                    None => true,
                };
                if matches {
                    total += 1;
                    if n > 0 {
                        if ring.len() == n {
                            ring.pop_front();
                        }
                        ring.push_back(line);
                    }
                }
            }
            Ok((total, ring.into_iter().collect()))
        })
        .await;

    let (total, tail) = match read_result {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Cannot read file: {}", e)})),
            );
        },
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error reading file"})),
            );
        },
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "service": q.service,
            "file": q.file,
            "total": total,
            "lines": tail,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "voltage-admin-api-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_service_directory_and_legacy_file() {
        let temp = TestDir::new("resolve");
        let service_dir = temp.path().join("comsrv");
        std::fs::create_dir_all(&service_dir).unwrap();
        let nested = service_dir.join("20260831_comsrv.log");
        std::fs::write(&nested, "nested").unwrap();
        let legacy = temp.path().join("20260831_modsrv.log");
        std::fs::write(&legacy, "legacy").unwrap();

        assert_eq!(
            resolve_log_file(temp.path(), Some("comsrv"), "20260831_comsrv.log"),
            Some(nested)
        );
        assert_eq!(
            resolve_log_file(temp.path(), Some("modsrv"), "20260831_modsrv.log"),
            Some(legacy)
        );
    }

    #[test]
    fn rejects_path_components() {
        assert!(!is_safe_path_component("../comsrv"));
        assert!(!is_safe_path_component("comsrv/file.log"));
        assert!(is_safe_path_component("comsrv"));
    }

    #[test]
    fn lists_files_from_service_directory() {
        let temp = TestDir::new("list");
        let service_dir = temp.path().join("comsrv");
        std::fs::create_dir_all(&service_dir).unwrap();
        std::fs::write(service_dir.join("20260831_comsrv.log"), "hello").unwrap();
        std::fs::write(service_dir.join("20260830_comsrv.log"), "old").unwrap();

        let mut files = Vec::new();
        append_log_files(&mut files, &service_dir, Some("comsrv"), "20260831").unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].service.as_deref(), Some("comsrv"));
        assert_eq!(files[0].name, "20260831_comsrv.log");
    }
}
