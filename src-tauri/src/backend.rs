use anyhow::Result;
use chrono::{Local, Timelike};
use directories::ProjectDirs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::Window;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionPreset {
    pub red_region: Region,
    pub yellow_region: Region,
    pub hunger_region: Region,
}

#[derive(Debug)]
pub struct OcrHandler;

impl OcrHandler {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub color_tolerance: u8,
    pub autoclick_interval_ms: u64,
    pub fish_per_feed: u32,
    pub webhook_url: String,
    pub screenshot_interval_mins: u32,
    pub screenshot_enabled: bool,
    pub red_region: Region,
    pub yellow_region: Region,
    pub hunger_region: Region,
    pub region_preset: String,
    pub startup_delay_ms: u64,
    pub detection_interval_ms: u64,
    pub max_fishing_timeout_ms: u64,
    pub rod_lure_value: f32,
    pub always_on_top: bool,
    pub auto_save_enabled: bool,
    pub failsafe_enabled: bool,
    pub advanced_detection: bool,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            color_tolerance: 10,
            autoclick_interval_ms: 70,
            fish_per_feed: 5,
            webhook_url: String::new(),
            screenshot_interval_mins: 60,
            screenshot_enabled: true,
            red_region: Region {
                x: 1321,
                y: 99,
                width: 768,
                height: 546,
            },
            yellow_region: Region {
                x: 3097,
                y: 1234,
                width: 342,
                height: 205,
            },
            hunger_region: Region {
                x: 274,
                y: 1301,
                width: 43,
                height: 36,
            },
            region_preset: "3440x1440".to_string(),
            startup_delay_ms: 3000,
            detection_interval_ms: 50,
            max_fishing_timeout_ms: 25000,
            rod_lure_value: 1.0,
            always_on_top: false,
            auto_save_enabled: true,
            failsafe_enabled: true,
            advanced_detection: false,
        }
    }
}

impl BotConfig {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&contents)?)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        ProjectDirs::from("com", "arcane", "fishing-bot")
            .map(|dirs| dirs.config_dir().join("config.json"))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    pub fn calculate_max_bite_time(&self) -> Duration {
        Duration::from_millis(calculate_timeout_ms(self.rod_lure_value))
    }

    pub fn get_timeout_description(&self) -> String {
        let timeout_ms = calculate_timeout_ms(self.rod_lure_value);
        format!(
            "Lure {:.1}: ~{:.0}s timeout",
            self.rod_lure_value,
            timeout_ms as f32 / 1000.0
        )
    }

    pub fn apply_resolution_preset(&mut self, preset: &str) {
        if let Some(preset_data) = resolution_presets().get(preset) {
            self.red_region = preset_data.red_region;
            self.yellow_region = preset_data.yellow_region;
            self.hunger_region = preset_data.hunger_region;
        }
        self.region_preset = preset.to_string();
    }
}

pub fn calculate_timeout_ms(lure_value: f32) -> u64 {
    let multiplier = if lure_value <= 1.0 {
        3.0 - 2.0 * lure_value
    } else {
        1.25 - lure_value / 3.0
    };

    let seconds = (multiplier * 60.0 + 5.0).clamp(10.0, 180.0);
    (seconds * 1000.0).round() as u64
}

pub fn resolution_presets() -> HashMap<String, ResolutionPreset> {
    let mut presets = HashMap::new();
    presets.insert(
        "3440x1440".to_string(),
        ResolutionPreset {
            red_region: Region {
                x: 1321,
                y: 99,
                width: 768,
                height: 546,
            },
            yellow_region: Region {
                x: 3097,
                y: 1234,
                width: 342,
                height: 205,
            },
            hunger_region: Region {
                x: 274,
                y: 1301,
                width: 43,
                height: 36,
            },
        },
    );
    presets.insert(
        "1920x1080".to_string(),
        ResolutionPreset {
            red_region: Region {
                x: 598,
                y: 29,
                width: 901,
                height: 477,
            },
            yellow_region: Region {
                x: 1649,
                y: 632,
                width: 270,
                height: 447,
            },
            hunger_region: Region {
                x: 212,
                y: 984,
                width: 21,
                height: 18,
            },
        },
    );
    presets
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeStats {
    pub total_fish_caught: u64,
    pub total_runtime_seconds: u64,
    pub sessions_completed: u64,
    pub last_updated: String,
    pub best_session_fish: u64,
    pub average_fish_per_hour: f32,
    pub total_feeds: u64,
    pub uptime_percentage: f32,
}

impl Default for LifetimeStats {
    fn default() -> Self {
        Self {
            total_fish_caught: 0,
            total_runtime_seconds: 0,
            sessions_completed: 0,
            last_updated: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            best_session_fish: 0,
            average_fish_per_hour: 0.0,
            total_feeds: 0,
            uptime_percentage: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub running: bool,
    pub last_action: String,
    pub fish_caught: u64,
    pub hunger_level: u8,
    pub errors_count: u32,
    pub uptime_minutes: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            running: false,
            last_action: "Idle".to_string(),
            fish_caught: 0,
            hunger_level: 100,
            errors_count: 0,
            uptime_minutes: 0,
        }
    }
}

#[derive(Clone)]
pub struct SharedState {
    pub config: Arc<RwLock<BotConfig>>,
    pub stats: Arc<RwLock<LifetimeStats>>,
    pub session: Arc<RwLock<SessionState>>,
    pub running: Arc<AtomicBool>,
    pub worker_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub ocr: Arc<Mutex<OcrHandler>>,
}

impl SharedState {
    pub fn new(ocr: Arc<Mutex<OcrHandler>>) -> Result<Self> {
        let config = BotConfig::load()?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(LifetimeStats::default())),
            session: Arc::new(RwLock::new(SessionState::default())),
            running: Arc::new(AtomicBool::new(false)),
            worker_handle: Arc::new(Mutex::new(None)),
            ocr,
        })
    }
}

#[derive(Clone, Serialize)]
struct StateUpdate {
    stats: LifetimeStats,
    session: SessionState,
}

fn emit_state_update(window: &Window, state: &SharedState) {
    let payload = StateUpdate {
        stats: state.stats.read().clone(),
        session: state.session.read().clone(),
    };
    let _ = window.emit("state-update", payload);
}

fn worker_loop(state: SharedState, window: Window) {
    let start_time = Instant::now();
    let mut last_uptime_minutes = 0;

    loop {
        if !state.running.load(Ordering::Relaxed) {
            break;
        }

        let elapsed = start_time.elapsed();
        let uptime_minutes = elapsed.as_secs() / 60;

        if uptime_minutes != last_uptime_minutes {
            {
                let mut session = state.session.write();
                session.uptime_minutes = uptime_minutes;
            }
            {
                let mut stats = state.stats.write();
                stats.total_runtime_seconds = elapsed.as_secs();
                stats.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            }
            emit_state_update(&window, &state);
            last_uptime_minutes = uptime_minutes;
        }

        thread::sleep(Duration::from_millis(500));
    }
}

pub fn start_bot(state: &SharedState, window: &Window) {
    state.running.store(true, Ordering::Relaxed);
    {
        let mut session = state.session.write();
        session.running = true;
        session.uptime_minutes = 0;
        session.last_action = format!(
            "Started at {:02}:{:02}",
            Local::now().hour(),
            Local::now().minute()
        );
    }
    emit_state_update(window, state);

    let mut handle_guard = state.worker_handle.lock().expect("worker handle lock");
    if handle_guard.is_none() {
        let thread_state = state.clone();
        let thread_window = window.clone();
        *handle_guard = Some(thread::spawn(move || worker_loop(thread_state, thread_window)));
    }
}

pub fn stop_bot(state: &SharedState, window: &Window) {
    state.running.store(false, Ordering::Relaxed);
    {
        let mut session = state.session.write();
        session.running = false;
        session.last_action = "Stopped".to_string();
    }
    emit_state_update(window, state);

    if let Some(handle) = state.worker_handle.lock().expect("worker handle lock").take() {
        let _ = handle.join();
    }
}
