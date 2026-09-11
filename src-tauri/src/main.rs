mod overlay_window;
use arcane_fishing_bot_app::{
    config::{resolution_presets, BotConfig},
    diagnostics, readiness, recording,
    runtime::{SessionMode, SharedState, Snapshot},
};
use std::sync::Arc;
use tauri::{Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[tauri::command]
fn get_config(state: State<'_, SharedState>) -> BotConfig {
    state.config.read().clone()
}
#[tauri::command]
fn get_stats(state: State<'_, SharedState>) -> Snapshot {
    state.snapshot()
}
#[tauri::command]
fn get_resolution_presets(
) -> std::collections::HashMap<String, arcane_fishing_bot_app::config::ResolutionPreset> {
    resolution_presets()
}
#[tauri::command]
fn save_config(
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
async fn start_session(
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
async fn check_readiness(
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
fn get_session_recording(
    state: State<'_, SharedState>,
) -> Result<recording::SessionRecording, String> {
    state
        .recording
        .read()
        .clone()
        .ok_or_else(|| "Enable recording before starting a session".into())
}
#[tauri::command]
async fn replay_session(
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
async fn export_session_recording(
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
fn stop_session(state: State<'_, SharedState>) {
    state.stop();
}
#[tauri::command]
fn calculate_timeout(lure_value: f32) -> u64 {
    arcane_fishing_bot_app::config::calculate_timeout_ms(lure_value)
}
#[tauri::command]
async fn inspect_screenshot(
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
async fn capture_preview(
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
fn main() {
    let state = SharedState::new().expect("Could not load application settings/statistics");
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(state) = app.try_state::<SharedState>() {
                state.stop();
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        app.state::<SharedState>().stop();
                        let _ = overlay_window::close(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .manage(overlay_window::OverlayState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_stats,
            get_resolution_presets,
            save_config,
            start_session,
            check_readiness,
            get_session_recording,
            replay_session,
            export_session_recording,
            stop_session,
            calculate_timeout,
            inspect_screenshot,
            capture_preview,
            overlay_window::show_overlay,
            overlay_window::hide_overlay,
            overlay_window::overlay_visible
        ])
        .setup(|app| {
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F12);
            let registered = app.global_shortcut().register(shortcut).is_ok();
            app.state::<SharedState>()
                .shortcut_ready
                .store(registered, std::sync::atomic::Ordering::SeqCst);
            let window = app.get_webview_window("main").expect("main window");
            window.set_always_on_top(app.state::<SharedState>().config.read().always_on_top)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                overlay_window::destroyed(window.app_handle(), window.label());
            }
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if overlay_window::close(window.app_handle()).is_err() {
                    api.prevent_close();
                    return;
                }
                let state = window.state::<SharedState>();
                if state.session.read().running {
                    api.prevent_close();
                    state.stop();
                    let shared = state.inner().clone();
                    let window = window.clone();
                    std::thread::spawn(move || {
                        shared.shutdown();
                        let _ = window.close();
                    });
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not initialize Tauri")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<SharedState>().shutdown();
            }
        });
}
