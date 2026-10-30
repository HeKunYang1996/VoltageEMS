//! Container and local API readiness shared by config import, restore, and restart.

use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::{Instant, sleep_until, timeout_at};

const CORE_SERVICES: &[&str] = &[
    "voltageems-comsrv",
    "voltageems-modsrv",
    "voltageems-hissrv",
    "voltageems-netsrv",
    "voltageems-alarmsrv",
];

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerState {
    running: bool,
    restarting: bool,
    paused: bool,
    status: String,
}

struct ReadinessProbe {
    url: String,
    json: bool,
}

fn local_probes() -> Vec<ReadinessProbe> {
    // These endpoints check local startup only. In particular, netsrv /ping does
    // not require a remote MQTT broker and hissrv /ping does not require cloud storage.
    [
        (common::comsrv_url(), "/health?local_only=true", true),
        (common::modsrv_url(), "/health", true),
        (
            std::env::var("HISSRV_URL").unwrap_or_else(|_| "http://localhost:6004".into()),
            "/ping",
            false,
        ),
        (
            std::env::var("NETSRV_URL").unwrap_or_else(|_| "http://localhost:6006".into()),
            "/ping",
            false,
        ),
        (
            std::env::var("ALARMSRV_URL").unwrap_or_else(|_| "http://localhost:6007".into()),
            "/health",
            true,
        ),
    ]
    .into_iter()
    .map(|(base, path, json)| ReadinessProbe {
        url: format!("{}{path}", base.trim_end_matches('/')),
        json,
    })
    .collect()
}

/// Confirm Docker Running state and required local APIs/components.
/// Remote MQTT/device connectivity is irrelevant.
/// One attempt, including all five restarts, is bounded to 60 seconds.
pub async fn restart_core_services() -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut results = restart_with_docker(
        Path::new("docker"),
        CORE_SERVICES,
        deadline.saturating_duration_since(Instant::now()),
        Duration::from_secs(1),
    )
    .await;
    wait_for_local_apis(
        &mut results,
        &local_probes(),
        deadline,
        Duration::from_secs(1),
    )
    .await;
    results
}

async fn probe_local_api(client: &reqwest::Client, probe: &ReadinessProbe) -> Result<(), String> {
    let response = client
        .get(&probe.url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "HTTP {status}: {}",
            body.chars().take(512).collect::<String>()
        ));
    }
    if probe.json {
        let body: Value =
            serde_json::from_str(&body).map_err(|e| format!("Invalid health response: {e}"))?;
        if body["success"] != true {
            return Err(format!("Health check did not succeed: {body}"));
        }
    } else if body.trim() != "pong" {
        return Err("Expected pong from local API".into());
    }
    Ok(())
}

async fn wait_for_local_apis(
    results: &mut [Value],
    probes: &[ReadinessProbe],
    deadline: Instant,
    poll_interval: Duration,
) {
    assert_eq!(results.len(), probes.len());
    let running: Vec<usize> = results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| (result["success"] == true).then_some(index))
        .collect();
    if running.is_empty() {
        return;
    }
    // A Docker success is provisional until its API has also passed the probe.
    for &index in &running {
        results[index]["success"] = json!(false);
        results[index]["error"] = json!(format!(
            "Local API readiness timed out: {}",
            probes[index].url
        ));
    }
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            for &index in &running {
                results[index]["error"] =
                    json!(format!("Could not create readiness client: {error}"));
            }
            return;
        },
    };
    while Instant::now() < deadline {
        let checks = futures::future::join_all(running.iter().map(|&index| {
            let client = &client;
            async move {
                timeout_at(deadline, probe_local_api(client, &probes[index]))
                    .await
                    .unwrap_or_else(|_| Err("Local API probe timed out".into()))
            }
        }))
        .await;
        let all_ready = checks.iter().all(Result::is_ok);
        for (&index, check) in running.iter().zip(checks) {
            match check {
                Ok(()) => {
                    results[index]["success"] = json!(true);
                    if let Some(result) = results[index].as_object_mut() {
                        result.remove("error");
                    }
                },
                Err(error) => {
                    results[index]["success"] = json!(false);
                    results[index]["error"] = json!(format!(
                        "Local API not ready ({}): {error}",
                        probes[index].url
                    ));
                },
            }
        }
        if all_ready {
            return;
        }
        sleep_until(deadline.min(Instant::now() + poll_interval)).await;
    }
}

async fn docker_output(docker: &Path, args: &[&str], deadline: Instant) -> Result<String, String> {
    if Instant::now() >= deadline {
        return Err("Docker restart confirmation timed out".to_string());
    }
    let output = timeout_at(
        deadline,
        Command::new(docker).args(args).kill_on_drop(true).output(),
    )
    .await
    .map_err(|_| "Docker command timed out".to_string())?
    .map_err(|e| format!("Failed to run Docker: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Docker command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn restart_with_docker(
    docker: &Path,
    services: &[&str],
    timeout: Duration,
    poll_interval: Duration,
) -> Vec<Value> {
    let deadline = Instant::now() + timeout;
    let mut results = Vec::with_capacity(services.len());
    let mut restarted = Vec::new();

    for service in services {
        let index = results.len();
        match docker_output(docker, &["restart", service], deadline).await {
            Ok(_) => {
                restarted.push((index, *service));
                results.push(json!({"service": service, "success": false}));
            },
            Err(error) => {
                results.push(json!({"service": service, "success": false, "error": error}));
            },
        }
    }
    if restarted.is_empty() {
        return results;
    }

    // Inspect together AFTER all restarts, so an earlier container that has
    // already exited is not reported as successfully running.
    let mut args = vec!["inspect", "--format", "{{json .State}}"];
    args.extend(restarted.iter().map(|(_, service)| *service));
    loop {
        let states = match docker_output(docker, &args, deadline).await {
            Ok(output) => output
                .lines()
                .map(serde_json::from_str::<ContainerState>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Invalid Docker state response: {e}")),
            Err(error) => Err(error),
        };
        let states = match states {
            Ok(states) if states.len() == restarted.len() => states,
            result => {
                let error = result.err().unwrap_or_else(|| {
                    "Docker did not return a state for every container".to_string()
                });
                for (index, service) in &restarted {
                    results[*index] = json!({"service": service, "success": false, "error": error});
                }
                return results;
            },
        };

        // A stopped/paused container has already failed startup. Only transient
        // states such as created/restarting should keep the caller waiting.
        let startup_failed = states.iter().any(|state| {
            state.paused || matches!(state.status.as_str(), "exited" | "dead" | "removing")
        });
        let mut all_running = true;
        for ((index, service), state) in restarted.iter().zip(states) {
            let running = state.running && !state.restarting && !state.paused;
            all_running &= running;
            results[*index] = if running {
                json!({"service": service, "success": true, "state": state.status})
            } else {
                json!({
                    "service": service,
                    "success": false,
                    "state": state.status,
                    "error": format!("Container did not reach Running state: {}", state.status)
                })
            };
        }
        if all_running || startup_failed || Instant::now() >= deadline {
            return results;
        }
        sleep_until(deadline.min(Instant::now() + poll_interval)).await;
        if Instant::now() >= deadline {
            return results;
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    async fn serve_probe(
        app: axum::Router,
        json: bool,
    ) -> (ReadinessProbe, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (ReadinessProbe { url, json }, server)
    }

    #[tokio::test]
    async fn running_container_waits_for_api_success() {
        use axum::{http::StatusCode, routing::get};
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let app = axum::Router::new().route(
            "/",
            get(move || {
                let call = observed.fetch_add(1, Ordering::SeqCst);
                async move {
                    match call {
                        0 => (
                            StatusCode::SERVICE_UNAVAILABLE,
                            r#"{"success":false,"error":{"message":"initializing"}}"#,
                        ),
                        1 => (StatusCode::OK, r#"{"success":false}"#),
                        _ => (StatusCode::OK, r#"{"success":true}"#),
                    }
                }
            }),
        );
        let (probe, server) = serve_probe(app, true).await;
        let mut results = vec![json!({"service":"modsrv","success":true,"state":"running"})];
        wait_for_local_apis(
            &mut results,
            &[probe],
            Instant::now() + Duration::from_secs(5),
            Duration::from_millis(10),
        )
        .await;
        server.abort();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(results[0]["success"], true, "{results:?}");
        assert!(results[0].get("error").is_none());
    }

    #[tokio::test]
    async fn running_container_with_missing_action_writer_fails_with_reason() {
        use axum::{http::StatusCode, routing::get};
        let app = axum::Router::new().route(
            "/",
            get(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    r#"{"success":false,"error":{"message":"shm_dispatch: writer unavailable"}}"#,
                )
            }),
        );
        let (probe, server) = serve_probe(app, true).await;
        let mut results = vec![json!({"service":"modsrv","success":true,"state":"running"})];
        wait_for_local_apis(
            &mut results,
            &[probe],
            Instant::now() + Duration::from_millis(300),
            Duration::from_millis(10),
        )
        .await;
        server.abort();
        assert_eq!(results[0]["success"], false, "{results:?}");
        assert!(
            results[0]["error"]
                .as_str()
                .unwrap()
                .contains("writer unavailable")
        );
    }

    #[tokio::test]
    async fn mqtt_disconnection_does_not_block_local_readiness() {
        use axum::routing::get;
        assert!(local_probes()[3].url.ends_with("/ping"));
        assert!(!local_probes()[3].json);
        let app = axum::Router::new()
            .route("/ping", get(|| async { "pong" }))
            .route(
                "/netApi/health",
                get(|| async {
                    axum::Json(json!({"success":false,"data":{"mqtt_connected":false}}))
                }),
            );
        let (mut probe, server) = serve_probe(app, false).await;
        probe.url.push_str("/ping");
        let mut results = vec![json!({"service":"netsrv","success":true,"state":"running"})];
        wait_for_local_apis(
            &mut results,
            &[probe],
            Instant::now() + Duration::from_secs(5),
            Duration::from_millis(10),
        )
        .await;
        server.abort();
        assert_eq!(results[0]["success"], true, "{results:?}");
    }

    #[tokio::test]
    async fn stalled_local_api_is_bounded_and_docker_failure_is_preserved() {
        use axum::routing::get;
        let app = axum::Router::new().route(
            "/",
            get(|| async { std::future::pending::<&'static str>().await }),
        );
        let (probe, server) = serve_probe(app, true).await;
        let mut results = vec![
            json!({"service":"missing","success":false,"error":"Docker restart failed"}),
            json!({"service":"modsrv","success":true,"state":"running"}),
        ];
        let probes = [
            ReadinessProbe {
                url: probe.url.clone(),
                json: true,
            },
            probe,
        ];
        let start = Instant::now();
        wait_for_local_apis(
            &mut results,
            &probes,
            start + Duration::from_millis(150),
            Duration::from_millis(10),
        )
        .await;
        server.abort();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert_eq!(results[0]["error"], "Docker restart failed");
        assert_eq!(results[1]["success"], false);
        assert!(results[1]["error"].as_str().unwrap().contains("timed out"));
    }

    fn fake_docker(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let docker = root.path().join("docker");
        std::fs::write(&docker, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
        std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).unwrap();
        (root, docker)
    }

    #[tokio::test]
    async fn confirms_all_containers_after_restart_without_checking_business_health() {
        let (root, docker) = fake_docker(
            r#"
if [ "$1" = restart ]; then
    echo "$2" >> "$0.restarted"
else
    test "$(wc -l < "$0.restarted")" -eq 2
    echo '{"Running":true,"Restarting":false,"Paused":false,"Status":"running","Health":{"Status":"unhealthy"}}'
    echo '{"Running":true,"Restarting":false,"Paused":false,"Status":"running"}'
fi
"#,
        );
        let results = restart_with_docker(
            &docker,
            &["comsrv", "netsrv"],
            Duration::from_secs(20),
            Duration::from_millis(10),
        )
        .await;
        assert!(
            results.iter().all(|result| result["success"] == true),
            "{results:?}"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("docker.restarted")).unwrap(),
            "comsrv\nnetsrv\n"
        );
    }

    #[tokio::test]
    async fn waits_for_restarting_container_to_be_running() {
        let (_root, docker) = fake_docker(
            r#"
if [ "$1" = inspect ]; then
    if [ -f "$0.checked" ]; then
        echo '{"Running":true,"Restarting":false,"Paused":false,"Status":"running"}'
    else
        touch "$0.checked"
        echo '{"Running":true,"Restarting":true,"Paused":false,"Status":"restarting"}'
    fi
fi
"#,
        );
        let results = restart_with_docker(
            &docker,
            &["comsrv"],
            Duration::from_secs(20),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(results[0]["success"], true, "{results:?}");
        assert_eq!(results[0]["state"], "running");
    }

    #[tokio::test]
    async fn reports_container_that_exited_after_successful_restart() {
        let (_root, docker) = fake_docker(
            r#"
if [ "$1" = inspect ]; then
    echo '{"Running":false,"Restarting":false,"Paused":false,"Status":"exited"}'
fi
"#,
        );
        let results = restart_with_docker(
            &docker,
            &["modsrv"],
            Duration::from_secs(20),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(results[0]["service"], "modsrv");
        assert_eq!(results[0]["success"], false);
        assert_eq!(results[0]["state"], "exited", "{results:?}");
    }

    #[tokio::test]
    async fn keeps_restart_failure_while_confirming_other_containers() {
        let (_root, docker) = fake_docker(
            r#"
if [ "$1" = restart ] && [ "$2" = missing ]; then
    echo 'No such container: missing' >&2
    exit 1
fi
if [ "$1" = inspect ]; then
    test "$4" = comsrv
    test "$#" -eq 4
    echo '{"Running":true,"Restarting":false,"Paused":false,"Status":"running"}'
fi
"#,
        );
        let results = restart_with_docker(
            &docker,
            &["missing", "comsrv"],
            Duration::from_secs(20),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(results[0]["success"], false);
        assert!(results[0]["error"].as_str().unwrap().contains("missing"));
        assert_eq!(results[1]["success"], true, "{results:?}");
    }

    #[tokio::test]
    async fn inspect_failure_is_not_reported_as_success() {
        let (_root, docker) = fake_docker(
            r#"
if [ "$1" = inspect ]; then
    echo 'Cannot connect to the Docker daemon' >&2
    exit 1
fi
"#,
        );
        let results = restart_with_docker(
            &docker,
            &["comsrv"],
            Duration::from_secs(20),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(results[0]["success"], false);
        assert!(
            results[0]["error"]
                .as_str()
                .unwrap()
                .contains("Docker daemon")
        );
    }

    #[tokio::test]
    async fn hung_docker_command_is_bounded() {
        let (_root, docker) = fake_docker("exec sleep 10");
        let started = Instant::now();
        let results = restart_with_docker(
            &docker,
            &["comsrv"],
            Duration::from_millis(50),
            Duration::from_millis(10),
        )
        .await;
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(results[0]["success"], false);
        assert!(results[0]["error"].as_str().unwrap().contains("timed out"));
    }
}
