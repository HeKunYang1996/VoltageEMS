//! Internal host-management endpoints.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};

const DOCKER_SOCKET: &str = "/var/run/docker.sock";
static REBOOT_SCHEDULED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize)]
pub struct RebootRequest {
    #[serde(rename = "msgId")]
    msg_id: String,
}

fn reboot_command_args() -> [&'static str; 22] {
    [
        "run",
        "--rm",
        "--pid",
        "host",
        "--privileged",
        "alpine:latest",
        "nsenter",
        "--target",
        "1",
        "--mount",
        "--uts",
        "--ipc",
        "--net",
        "--pid",
        "--",
        "systemd-run",
        "--unit",
        "monarch-reboot",
        "--on-active=5s",
        "--collect",
        "systemctl",
        "reboot",
    ]
}

/// Schedule a delayed reboot of the Ubuntu host.
///
/// This endpoint is mounted behind the loopback-only middleware. The delay gives
/// netsrv enough time to publish the MQTT acknowledgement before connectivity is
/// lost. Repeated requests are accepted without scheduling another reboot.
pub async fn schedule_reboot(Json(req): Json<RebootRequest>) -> impl IntoResponse {
    if req.msg_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "msgId is required"})),
        )
            .into_response();
    }

    if !Path::new(DOCKER_SOCKET).exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"success": false, "message": "Docker socket unavailable"})),
        )
            .into_response();
    }

    if REBOOT_SCHEDULED.swap(true, Ordering::SeqCst) {
        return (
            StatusCode::ACCEPTED,
            Json(json!({"success": true, "message": "Host reboot already scheduled"})),
        )
            .into_response();
    }

    let output = std::process::Command::new("docker")
        .args(reboot_command_args())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            info!(msg_id = %req.msg_id, "Host reboot scheduled in 5 seconds");
            (
                StatusCode::ACCEPTED,
                Json(json!({"success": true, "message": "Host reboot scheduled"})),
            )
                .into_response()
        },
        Ok(output) => {
            REBOOT_SCHEDULED.store(false, Ordering::SeqCst);
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(msg_id = %req.msg_id, error = %stderr, "Failed to schedule host reboot");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": "Failed to schedule host reboot"})),
            )
                .into_response()
        },
        Err(e) => {
            REBOOT_SCHEDULED.store(false, Ordering::SeqCst);
            error!(msg_id = %req.msg_id, error = %e, "Failed to run Docker command");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": "Failed to schedule host reboot"})),
            )
                .into_response()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reboot_command_enters_host_and_uses_systemd() {
        let args = reboot_command_args();
        assert!(args.windows(2).any(|pair| pair == ["--pid", "host"]));
        assert!(args.contains(&"--privileged"));
        assert!(args.contains(&"nsenter"));
        assert!(args.contains(&"systemd-run"));
        assert!(args.ends_with(&["systemctl", "reboot"]));
    }
}
