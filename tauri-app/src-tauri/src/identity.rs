use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const IDENTITY_FILE: &str = "identity.toml";
const MAX_USERNAME_LEN: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
struct PersistedIdentity {
    username: Option<String>,
}

fn identity_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("no app data dir")
        .join(IDENTITY_FILE)
}

fn load_raw(app: &AppHandle) -> PersistedIdentity {
    let path = identity_path(app);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str::<PersistedIdentity>(&s).ok())
        .unwrap_or(PersistedIdentity { username: None })
}

fn persist(app: &AppHandle, identity: &PersistedIdentity) -> std::io::Result<()> {
    let path = identity_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_file_name(".identity.toml.tmp");
    fs::write(
        &tmp,
        toml::to_string_pretty(identity).expect("serialize failed"),
    )?;
    fs::rename(tmp, path)
}

pub fn sanitize_username(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_USERNAME_LEN).collect())
}

#[derive(serde::Serialize)]
pub struct IdentityDto {
    pub username: Option<String>,
}

#[tauri::command]
pub fn get_identity(app: AppHandle) -> IdentityDto {
    let id = load_raw(&app);
    IdentityDto {
        username: id.username,
    }
}

#[tauri::command]
pub fn set_username(app: AppHandle, username: String) -> Result<(), String> {
    let clean = sanitize_username(&username).ok_or("username must not be empty")?;
    persist(
        &app,
        &PersistedIdentity {
            username: Some(clean),
        },
    )
    .map_err(|e| e.to_string())
}
