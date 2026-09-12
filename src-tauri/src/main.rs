#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod overlay_window;
use arcane_fishing_bot_app::runtime::SharedState;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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
            commands::get_config,
            commands::get_stats,
            commands::get_resolution_presets,
            commands::save_config,
            commands::start_session,
            commands::check_readiness,
            commands::get_session_recording,
            commands::replay_session,
            commands::export_session_recording,
            commands::stop_session,
            commands::inspect_screenshot,
            commands::capture_preview,
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
