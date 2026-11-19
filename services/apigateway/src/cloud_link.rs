//! Cloud registration and the persistent deployment .env configuration.

// Keep the shared, structured HTTP error at this infrequent configuration boundary.
#![allow(clippy::result_large_err)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::http::StatusCode;
use common::api_types::{AppError, ErrorInfo};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub const DEFAULT_URL: &str = "http://192.168.30.210:9100";
const URL_KEY: &str = "CLOUD_LINK_URL";
const ACTIVATION_KEY: &str = "CLOUD_LINK_KEY";
const STATUS_KEY: &str = "CLOUD_LINK_STATUS";

#[derive(Serialize, utoipa::ToSchema)]
pub struct CloudLinkConfig {
    pub url: String,
    /// Empty when unset; otherwise first 4 + 19 asterisks + last 4 characters.
    pub key: String,
    /// Last successfully saved registration, not MQTT connectivity.
    pub status: bool,
    pub device_sn: String,
}

// Do not derive Debug: this request can contain an unmasked activation key.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivateRequest {
    pub url: Option<String>,
    /// Omit, leave empty, or return the current mask to reuse the saved key.
    pub key: Option<String>,
}

#[derive(Deserialize)]
struct RegistrationResponse {
    code: i64,
    #[serde(default)]
    msg: String,
}

pub struct CloudLink {
    env_path: PathBuf,
    serial_path: PathBuf,
    client: reqwest::Client,
    activation: Mutex<()>,
}

impl CloudLink {
    pub fn new(env_path: PathBuf) -> Result<Self, reqwest::Error> {
        Ok(Self {
            env_path,
            serial_path: PathBuf::from("/proc/device-tree/serial-number"),
            // Never forward the key to a redirect target or retry a registration silently.
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()?,
            activation: Mutex::new(()),
        })
    }

    /// Fill missing settings once, retaining existing values and unrelated entries.
    pub fn initialize(&self) -> Result<(), AppError> {
        let mut env = self.read_env()?;
        let mut changed = false;
        for (key, value) in [
            (URL_KEY, DEFAULT_URL),
            (ACTIVATION_KEY, ""),
            (STATUS_KEY, "false"),
        ] {
            if env.get(key)?.is_none() {
                env.set(key, value);
                changed = true;
            }
        }
        if env.get("DEVICE_SN")?.unwrap_or_default().is_empty()
            && let Some(sn) = self.hardware_serial()
        {
            env.set("DEVICE_SN", &sn);
            changed = true;
        }
        if changed {
            self.save_env(&env)?;
        }
        Ok(())
    }

    pub fn config(&self) -> Result<CloudLinkConfig, AppError> {
        self.public_config(&self.read_env()?)
    }

    pub fn is_device_registered(&self, device_sn: &str) -> Result<bool, AppError> {
        let env = self.read_env()?;
        Ok(!device_sn.is_empty()
            && env.registration_confirmed()?
            && self.device_sn(&env)? == device_sn)
    }

    fn public_config(&self, env: &EnvFile) -> Result<CloudLinkConfig, AppError> {
        let key = env.get(ACTIVATION_KEY)?.unwrap_or_default();
        Ok(CloudLinkConfig {
            url: env.get(URL_KEY)?.unwrap_or_else(|| DEFAULT_URL.to_owned()),
            key: mask_key(&key),
            status: env.registration_confirmed()?,
            device_sn: self.device_sn(env)?,
        })
    }

    pub async fn activate(&self, request: ActivateRequest) -> Result<CloudLinkConfig, AppError> {
        let _guard = self.activation.try_lock().map_err(|_| {
            AppError::conflict(
                "Device activation is already in progress. Please wait for it to finish.",
            )
        })?;
        let original = self.read_env()?;
        let saved_key = original.get(ACTIVATION_KEY)?.unwrap_or_default();
        let key = resolve_key(request.key.as_deref(), &saved_key)?;
        let url = normalize_url(
            request.url.as_deref().unwrap_or(
                &original
                    .get(URL_KEY)?
                    .unwrap_or_else(|| DEFAULT_URL.to_owned()),
            ),
        )?;
        let sn = self.device_sn(&original)?;
        if sn.is_empty() {
            return Err(AppError::bad_request(
                "Device serial number is missing. Set DEVICE_SN in .env before activating.",
            ));
        }

        let response = self
            .client
            .post(format!("{url}/user-app/common/register"))
            .json(&serde_json::json!({"stationSN": sn, "activation": key}))
            .send()
            .await
            .map_err(cloud_transport_error)?;
        let http_status = response.status();
        let body: RegistrationResponse = response.json().await.map_err(|error| {
            if error.is_timeout() {
                cloud_transport_error(error)
            } else {
                upstream_error("Invalid cloud response. Activation could not be confirmed. Retry with the same device serial number and activation key.")
            }
        })?;
        if !http_status.is_success() || body.code != 200 {
            // Expose only the documented reasons. Other messages may contain
            // credentials or text in a language the client does not support.
            let message = match body.msg.trim() {
                reason @ ("Activation code not found"
                | "Activation code is disabled"
                | "Activation code has no assigned organization"
                | "Activation code has expired") => reason.to_owned(),
                _ => format!(
                    "Cloud rejected the request (HTTP {}, code {})",
                    http_status.as_u16(),
                    body.code
                ),
            };
            return Err(upstream_error(format!(
                "Device activation failed: {message}"
            )));
        }

        // Re-read after the network request so unrelated .env edits are retained.
        // A concurrent configuration change must not be silently overwritten.
        let mut current = self
            .read_env()
            .map_err(|_| save_after_registration_error())?;
        for name in [URL_KEY, ACTIVATION_KEY, STATUS_KEY, "DEVICE_SN"] {
            if current.get(name)? != original.get(name)? {
                return Err(AppError::conflict(
                    "Cloud registration succeeded, but the local configuration changed during activation. Reload it before retrying.",
                ));
            }
        }
        current.set(URL_KEY, &url);
        current.set(ACTIVATION_KEY, &key);
        current.set(STATUS_KEY, "true");
        current.set("DEVICE_SN", &sn);
        self.save_env(&current)
            .map_err(|_| save_after_registration_error())?;
        self.public_config(&current)
    }

    fn hardware_serial(&self) -> Option<String> {
        let serial = fs::read_to_string(&self.serial_path).ok()?;
        let serial = serial.trim_matches(|ch: char| ch == '\0' || ch.is_whitespace());
        (!serial.is_empty() && !serial.chars().any(char::is_control)).then(|| serial.to_owned())
    }

    fn device_sn(&self, env: &EnvFile) -> Result<String, AppError> {
        let sn = env.get("DEVICE_SN")?.unwrap_or_default();
        let sn = sn.trim();
        if sn.chars().any(char::is_control) {
            return Err(AppError::bad_request(
                "DEVICE_SN must not contain control characters.",
            ));
        }
        Ok(if sn.is_empty() {
            self.hardware_serial().unwrap_or_default()
        } else {
            sn.to_owned()
        })
    }

    fn read_env(&self) -> Result<EnvFile, AppError> {
        match fs::read_to_string(&self.env_path) {
            Ok(text) => Ok(EnvFile(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(EnvFile(String::new()))
            },
            Err(_) => Err(AppError::internal_error(
                "Unable to read the device .env configuration.",
            )),
        }
    }

    fn save_env(&self, env: &EnvFile) -> Result<(), AppError> {
        atomic_write(&self.env_path, env.0.as_bytes()).map_err(|_| {
            AppError::internal_error("Unable to save the device .env configuration. Check directory permissions and available disk space.")
        })
    }
}

fn save_after_registration_error() -> AppError {
    AppError::internal_error(
        "Cloud registration succeeded, but the local configuration could not be saved. Check directory permissions and available disk space, then retry with the same device serial number and activation key.",
    )
}

fn upstream_error(message: impl Into<String>) -> AppError {
    AppError::new(
        StatusCode::BAD_GATEWAY,
        ErrorInfo::new(message).with_code(502),
    )
}

fn cloud_transport_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::new(
            StatusCode::GATEWAY_TIMEOUT,
            ErrorInfo::new("The cloud request timed out. Activation could not be confirmed. Retry with the same device serial number and activation key.")
                .with_code(504),
        )
    } else {
        // reqwest errors can include URLs; do not expose credentials in logs/errors.
        upstream_error(
            "Unable to complete the cloud request. Activation could not be confirmed. Check the network and retry with the same device serial number and activation key.",
        )
    }
}

fn valid_key(key: &str) -> bool {
    key.len() == 27
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn mask_key(key: &str) -> String {
    if valid_key(key) {
        format!("{}{}{}", &key[..4], "*".repeat(19), &key[23..])
    } else {
        // Never expose an invalid or legacy saved key either.
        "*".repeat(key.chars().count())
    }
}

fn resolve_key(requested: Option<&str>, saved: &str) -> Result<String, AppError> {
    let requested = requested.unwrap_or_default().trim();
    let key = if requested.is_empty() || (valid_key(saved) && requested == mask_key(saved)) {
        saved
    } else {
        requested
    };
    if !valid_key(key) {
        return Err(AppError::bad_request(
            "The activation key must be 27 characters long and contain only letters, digits, hyphens, and underscores. To reuse the saved key, omit it or send its current masked value exactly as returned.",
        ));
    }
    Ok(key.to_owned())
}

fn normalize_url(input: &str) -> Result<String, AppError> {
    let invalid = || {
        AppError::bad_request(
            "URL must be a valid HTTP or HTTPS base URL without credentials, a query string, or a fragment.",
        )
    };
    let input = input.trim();
    if input.chars().any(char::is_control) {
        return Err(invalid());
    }
    let url = reqwest::Url::parse(input).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// Only edit owned assignments; retain other settings and comments verbatim.
struct EnvFile(String);

fn assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
    let (name, value) = line.split_once('=')?;
    Some((name.trim(), value.trim()))
}

impl EnvFile {
    fn registration_confirmed(&self) -> Result<bool, AppError> {
        Ok(self.get(STATUS_KEY)?.as_deref() == Some("true"))
    }

    fn get(&self, name: &str) -> Result<Option<String>, AppError> {
        let Some((_, raw)) = self
            .0
            .lines()
            .filter_map(assignment)
            .rfind(|(key, _)| *key == name)
        else {
            return Ok(None);
        };
        let value = if raw.starts_with('"') {
            // Our writer uses JSON-compatible double quotes, with literal Compose dollars.
            let mut values = serde_json::Deserializer::from_str(raw).into_iter::<String>();
            values
                .next()
                .transpose()
                .map_err(|_| AppError::internal_error("Invalid device .env configuration format."))?
                .ok_or_else(|| {
                    AppError::internal_error("Invalid device .env configuration format.")
                })?
                .replace("$$", "$")
        } else if let Some(quoted) = raw.strip_prefix('\'') {
            quoted
                .split_once('\'')
                .map(|(value, _)| value.to_owned())
                .ok_or_else(|| {
                    AppError::internal_error("Invalid device .env configuration format.")
                })?
        } else {
            raw.split_once(" #")
                .map_or(raw, |(value, _)| value)
                .trim()
                .to_owned()
        };
        Ok(Some(value))
    }

    fn set(&mut self, name: &str, value: &str) {
        let mut text = String::new();
        for line in self.0.lines() {
            if assignment(line).is_none_or(|(key, _)| key != name) {
                text.push_str(line);
                text.push('\n');
            }
        }
        text.push_str(name);
        text.push('=');
        text.push_str(
            &serde_json::Value::String(value.to_owned())
                .to_string()
                .replace('$', "$$"),
        );
        text.push('\n');
        self.0 = text;
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(".cloud-link-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
#[path = "cloud_link_tests.rs"]
mod tests;
