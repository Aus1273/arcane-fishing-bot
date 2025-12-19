mod backend;

use backend::{
    calculate_timeout_ms, resolution_presets, start_bot, stop_bot, BotConfig, LifetimeStats,
    OcrHandler, ResolutionPreset, SessionState, SharedState,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, Window};

struct AppState(SharedState);

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> BotConfig {
    state.0.config.read().clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: BotConfig) -> Result<(), String> {
    {
        let mut stored = state.0.config.write();
        *stored = config.clone();
    }
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_stats(state: State<'_, AppState>) -> (LifetimeStats, SessionState) {
    (state.0.stats.read().clone(), state.0.session.read().clone())
}

#[tauri::command]
fn start_session(state: State<'_, AppState>, window: Window) {
    start_bot(&state.0, &window);
}

#[tauri::command]
fn stop_session(state: State<'_, AppState>, window: Window) {
    stop_bot(&state.0, &window);
}

#[tauri::command]
fn calculate_timeout(lure_value: f32) -> u64 {
    calculate_timeout_ms(lure_value)
}

#[tauri::command]
fn get_resolution_presets() -> HashMap<String, ResolutionPreset> {
    resolution_presets()
}

fn main() {
    let ocr = Arc::new(Mutex::new(OcrHandler::new()));
    let shared_state = SharedState::new(ocr).expect("failed to load config");

    tauri::Builder::default()
        .manage(AppState(shared_state))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_stats,
            start_session,
            stop_session,
            calculate_timeout,
            get_resolution_presets
        ])
        .setup(|app| {
            let window = app.get_window("main").expect("main window");
            window.set_title("Arcane Fishing Bot")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
