mod backend;

use backend::{start_bot, stop_bot, BotConfig, LifetimeStats, SessionState, SharedState};
use tauri::{Manager, State};

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
fn start_session(window: tauri::Window, state: State<'_, AppState>) {
    start_bot(&state.0, window);
}

#[tauri::command]
fn stop_session(state: State<'_, AppState>) {
    stop_bot(&state.0);
}

fn main() {
    let shared_state = SharedState::new().expect("failed to load config");

    tauri::Builder::default()
        .manage(AppState(shared_state))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_stats,
            start_session,
            stop_session
        ])
        .setup(|app| {
            let window = app.get_window("main").expect("main window");
            window.set_title("Arcane Fishing Bot")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
