use arcane_fishing_bot_app::{
    config::{resolution_presets, BotConfig},
    diagnostics,
    runtime::{SharedState, Snapshot},
};
use std::sync::Arc;
use tauri::{Emitter, Manager, State, WebviewWindow};

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
fn start_session(state: State<'_, SharedState>, window: WebviewWindow) -> Result<(), String> {
    state
        .start(Arc::new(move |snapshot| {
            let _ = window.emit("state-update", snapshot);
        }))
        .map_err(|e| e.to_string())
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
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_stats,
            get_resolution_presets,
            save_config,
            start_session,
            stop_session,
            calculate_timeout,
            inspect_screenshot,
            capture_preview
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");
            window.set_always_on_top(app.state::<SharedState>().config.read().always_on_top)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
