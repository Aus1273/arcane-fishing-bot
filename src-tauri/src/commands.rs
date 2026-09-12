//! Typed frontend adapters. Session ownership and validation remain in the runtime.
use arcane_fishing_bot_app::{
    config::{resolution_presets, BotConfig},
    diagnostics, readiness, recording,
    runtime::{SessionMode, SharedState, Snapshot},
};
use std::sync::Arc;
use tauri::{Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn get_config(state: State<'_, SharedState>) -> BotConfig {
    state.config.read().clone()
}
#[tauri::command]
pub fn get_stats(state: State<'_, SharedState>) -> Snapshot {
    state.snapshot()
}
#[tauri::command]
pub fn get_resolution_presets(
) -> std::collections::HashMap<String, arcane_fishing_bot_app::config::ResolutionPreset> {
    resolution_presets()
}
#[tauri::command]
pub fn save_config(
    window: WebviewWindow,
    state: State<'_, SharedState>,
    config: BotConfig,
) -> Result<(), String> {
    state
        .save_config(config.clone())
        .map_err(|e| e.to_string())?;
    window
        .set_always_on_top(config.always_on_top)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn start_session(
    state: State<'_, SharedState>,
    window: WebviewWindow,
    mode: Option<SessionMode>,
    record: Option<bool>,
) -> Result<(), String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        shared
            .start_mode(
                mode.unwrap_or_default(),
                record.unwrap_or(false),
                Arc::new(move |snapshot| {
                    let _ = window.emit("state-update", snapshot);
                }),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn check_readiness(
    state: State<'_, SharedState>,
    mode: SessionMode,
) -> Result<readiness::Readiness, String> {
    let guard = state.reserve_inspection().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let shortcut_ready = state
        .shortcut_ready
        .load(std::sync::atomic::Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        readiness::check(&config, mode, shortcut_ready)
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn get_session_recording(
    state: State<'_, SharedState>,
) -> Result<recording::SessionRecording, String> {
    state
        .recording
        .read()
        .clone()
        .ok_or_else(|| "Enable recording before starting a session".into())
}
#[tauri::command]
pub async fn replay_session(
    recording_json: String,
    config: Option<BotConfig>,
) -> Result<recording::ReplayResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        recording::replay_json(&recording_json, config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn export_session_recording(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    window: WebviewWindow,
) -> Result<Option<String>, String> {
    if window.label() != "main" {
        return Err("Only the control window can export a recording".into());
    }
    let recording = state
        .recording
        .read()
        .clone()
        .ok_or_else(|| "Enable recording before starting a session".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(selected) = app
            .dialog()
            .file()
            .add_filter("Session recording", &["json"])
            .set_file_name(format!(
                "arcane-session-{}.json",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ))
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|error| error.to_string())?;
        let bytes = serde_json::to_vec(&recording).map_err(|error| error.to_string())?;
        if bytes.len() > recording::MAX_RECORDING_BYTES {
            return Err("Recording exceeds the 32 MiB export limit".into());
        }
        use std::io::Write;
        let parent = path
            .parent()
            .ok_or_else(|| "Choose a valid destination folder".to_string())?;
        let mut output =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        output
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        output
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        output.persist(&path).map_err(|error| error.to_string())?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub fn stop_session(state: State<'_, SharedState>) {
    state.stop();
}
#[tauri::command]
pub async fn inspect_screenshot(
    state: State<'_, SharedState>,
    config: BotConfig,
    image_base64: String,
) -> Result<diagnostics::Preview, String> {
    let guard = state.reserve_inspection().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        diagnostics::inspect_png(&image_base64, &config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn capture_preview(
    state: State<'_, SharedState>,
    config: BotConfig,
) -> Result<diagnostics::Preview, String> {
    let guard = state.reserve_inspection().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        std::thread::sleep(std::time::Duration::from_secs(5));
        if !arcane_fishing_bot_app::input::roblox_focused().map_err(|e| e.to_string())? {
            return Err("Focus Roblox during the five-second countdown".into());
        }
        diagnostics::inspect_live(&config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
