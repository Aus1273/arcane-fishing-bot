use arcane_fishing_bot_app::{
    config::BotConfig,
    overlay,
    runtime::{InspectionGuard, SharedState},
};
use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const LABEL: &str = "calibration-overlay";
#[derive(Default)]
pub struct OverlayState(Mutex<Option<InspectionGuard>>);

#[tauri::command]
pub fn overlay_visible(state: State<'_, OverlayState>) -> bool {
    state.0.lock().is_some()
}

// Async: WebView2 creation must not run inside a synchronous Windows IPC callback.
#[tauri::command]
pub async fn show_overlay(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, SharedState>,
    overlay_state: State<'_, OverlayState>,
    config: BotConfig,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the control window can open calibration".into());
    }
    let guard = state.reserve_inspection().map_err(|e| e.to_string())?;
    let screen = screenshots::Screen::from_point(0, 0).map_err(|e| e.to_string())?;
    let display = screen.display_info;
    let drawing =
        overlay::layout(&config, display.width, display.height).map_err(|e| e.to_string())?;
    let monitor = window
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|monitor| {
            let p = monitor.position();
            let s = monitor.size();
            p.x <= 0
                && p.y <= 0
                && i64::from(p.x) + i64::from(s.width) > 0
                && i64::from(p.y) + i64::from(s.height) > 0
        })
        .ok_or_else(|| "Could not locate the capture display".to_string())?;
    let aspect = monitor.size().width as f64 / monitor.size().height as f64;
    if (aspect - display.width as f64 / display.height as f64).abs() > 0.02 {
        return Err("Window and capture display geometry do not match".into());
    }
    let payload = serde_json::to_string(&drawing).map_err(|e| e.to_string())?;
    *overlay_state.0.lock() = Some(guard);
    let result = (|| -> tauri::Result<()> {
        let overlay =
            WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html#overlay".into()))
                .title("Detection areas - calibration only")
                .initialization_script(format!("window.__ARCANE_OVERLAY__ = {payload};"))
                .transparent(true)
                .decorations(false)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible_on_all_workspaces(true)
                .focused(false)
                .focusable(false)
                .visible(false)
                .build()?;
        overlay.set_ignore_cursor_events(true)?;
        overlay.set_position(*monitor.position())?;
        overlay.set_size(*monitor.size())?;
        #[cfg(target_os = "macos")]
        support_fullscreen_spaces(&overlay)?;
        overlay.show()?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(window) = app.get_webview_window(LABEL) {
            // Keep the guard if cleanup fails: never permit input under an overlay.
            if let Err(cleanup) = window.destroy() {
                return Err(format!(
                    "Overlay failed: {error}; hide it before continuing: {cleanup}"
                ));
            }
        } else {
            overlay_state.0.lock().take();
        }
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_overlay(app: AppHandle) -> Result<(), String> {
    close(&app).map_err(|e| e.to_string())
}
pub fn close(app: &AppHandle) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.destroy()?;
    } else if app.state::<OverlayState>().0.lock().is_some() {
        return Err(anyhow::anyhow!(
            "Overlay is opening or closing; try again in a moment"
        ));
    }
    Ok(())
}
pub fn destroyed(app: &AppHandle, label: &str) {
    if label == LABEL {
        app.state::<OverlayState>().0.lock().take();
    }
}

// Join the game's fullscreen Space as an auxiliary window. All AppKit messages
// run on the main thread, and the captured window keeps the pointer alive.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn support_fullscreen_spaces(window: &WebviewWindow) -> tauri::Result<()> {
    let address = window.ns_window()? as usize;
    let keep_alive = window.clone();
    window.run_on_main_thread(move || {
        use objc::{msg_send, sel, sel_impl};
        let _keep_alive = keep_alive;
        unsafe {
            let native = address as *mut objc::runtime::Object;
            let flags: usize = msg_send![native, collectionBehavior];
            // NSWindowCollectionBehaviorFullScreenAuxiliary (1 << 8),
            // clearing FullScreenPrimary (1 << 7), preserving CanJoinAllSpaces.
            let _: () = msg_send![native, setCollectionBehavior: (flags & !(1 << 7)) | (1 << 8)];
        }
    })
}
