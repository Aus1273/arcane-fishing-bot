use anyhow::{anyhow, Result};
use chrono::{Local, Timelike, Utc};
use directories::ProjectDirs;
use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use image::{DynamicImage, RgbaImage};
use parking_lot::RwLock;
use screenshots::Screen;
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
use rusty_tesseract::{Args, Image as TessImage};

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

fn capture_region(region: Region) -> Result<RgbaImage> {
    let screens = Screen::all()?;
    let screen = screens.first().ok_or_else(|| anyhow!("No screens found"))?;
    let image = screen.capture_area(region.x, region.y, region.width, region.height)?;
    RgbaImage::from_raw(region.width, region.height, image.to_vec())
        .ok_or_else(|| anyhow!("Failed to build image buffer"))
}

fn count_matching_pixels(image: &RgbaImage, target: (u8, u8, u8), tolerance: u8) -> u32 {
    let tolerance = tolerance as i32;
    image
        .pixels()
        .filter(|pixel| {
            let dr = (pixel[0] as i32 - target.0 as i32).abs();
            let dg = (pixel[1] as i32 - target.1 as i32).abs();
            let db = (pixel[2] as i32 - target.2 as i32).abs();
            dr + dg + db <= tolerance * 3
        })
        .count() as u32
}

fn parse_hunger_text(text: &str) -> Option<u32> {
    let cleaned = text
        .trim()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<u32>().ok()
}

fn check_hunger_ocr(region: Region) -> Result<u32> {
    let image = capture_region(region)?;
    let grayscale = DynamicImage::ImageRgba8(image).grayscale();
    let temp_path = std::env::temp_dir().join(format!(
        "hunger_ocr_{}.png",
        chrono::Utc::now().timestamp_millis()
    ));
    grayscale.save(&temp_path)?;

    let mut config_variables = HashMap::new();
    config_variables.insert(
        "tessedit_char_whitelist".to_string(),
        "0123456789%".to_string(),
    );
    let args = Args {
        lang: "eng".to_string(),
        dpi: Some(150),
        psm: Some(8),
        oem: Some(3),
        config_variables,
    };

    let result = if let Ok(mut tess_image) = TessImage::from_path(&temp_path) {
        #[cfg(windows)]
        {
            tess_image.cmd = "C:\\Program Files\\Tesseract-OCR\\tesseract.exe".to_string();
        }
        rusty_tesseract::image_to_string(&tess_image, &args)
            .ok()
            .and_then(|text| parse_hunger_text(&text))
    } else {
        None
    };

    std::fs::remove_file(&temp_path).ok();
    Ok(result.unwrap_or(100))
}

fn update_error_state(state: &SharedState, window: &Window, message: &str) {
    {
        let mut session = state.session.write();
        session.errors_count += 1;
        session.last_action = message.to_string();
    }
    emit_state_update(window, state);
}

fn worker_loop(state: SharedState, window: Window) {
    let start_time = Instant::now();
    let mut last_uptime_minutes = 0;
    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to init Enigo");

    thread::sleep(Duration::from_millis(state.config.read().startup_delay_ms));

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

        let config = state.config.read().clone();
        let red_region = config.red_region;
        let yellow_region = config.yellow_region;
        let hunger_region = config.hunger_region;
        let detection_interval = Duration::from_millis(config.detection_interval_ms);
        let reel_interval = Duration::from_millis(config.autoclick_interval_ms);
        let reel_timeout = Duration::from_millis(config.max_fishing_timeout_ms);
        let bite_timeout = config.calculate_max_bite_time();
        let red_threshold = ((red_region.width * red_region.height) / 300).max(15) as u32;
        let yellow_threshold = ((yellow_region.width * yellow_region.height) / 400).max(10) as u32;

        {
            let mut session = state.session.write();
            session.last_action = "Casting line".to_string();
        }
        emit_state_update(&window, &state);

        if enigo
            .button(Button::Left, Direction::Click)
            .is_err()
        {
            update_error_state(&state, &window, "Failed to cast");
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        thread::sleep(reel_interval);

        {
            let mut session = state.session.write();
            session.last_action = "Scanning for bite".to_string();
        }
        emit_state_update(&window, &state);

        let bite_start = Instant::now();
        let mut bite_detected = false;
        while state.running.load(Ordering::Relaxed) {
            if bite_start.elapsed() > bite_timeout {
                {
                    let mut session = state.session.write();
                    session.last_action = "Bite timeout - recasting".to_string();
                }
                emit_state_update(&window, &state);
                break;
            }

            match capture_region(red_region) {
                Ok(image) => {
                    let count = count_matching_pixels(
                        &image,
                        (241, 27, 28),
                        config.color_tolerance,
                    );
                    if count >= red_threshold {
                        bite_detected = true;
                        break;
                    }
                }
                Err(_) => update_error_state(&state, &window, "Red scan failed"),
            }

            thread::sleep(detection_interval);
        }

        if !bite_detected {
            continue;
        }

        {
            let mut session = state.session.write();
            session.last_action = "Reeling in".to_string();
        }
        emit_state_update(&window, &state);

        let reel_start = Instant::now();
        let mut caught = false;
        while state.running.load(Ordering::Relaxed) {
            if reel_start.elapsed() > reel_timeout {
                {
                    let mut session = state.session.write();
                    session.last_action = "Reel timeout".to_string();
                }
                emit_state_update(&window, &state);
                break;
            }

            if enigo
                .button(Button::Left, Direction::Click)
                .is_err()
            {
                update_error_state(&state, &window, "Failed to reel" );
            }

            match capture_region(yellow_region) {
                Ok(image) => {
                    let count = count_matching_pixels(
                        &image,
                        (255, 255, 0),
                        config.color_tolerance,
                    );
                    if count >= yellow_threshold {
                        let _ = enigo.button(Button::Left, Direction::Click);
                        thread::sleep(detection_interval);
                        if let Ok(confirm_image) = capture_region(yellow_region) {
                            let confirm_count = count_matching_pixels(
                                &confirm_image,
                                (255, 255, 0),
                                config.color_tolerance,
                            );
                            if confirm_count >= yellow_threshold {
                                caught = true;
                                break;
                            }
                        }
                    }
                }
                Err(_) => update_error_state(&state, &window, "Yellow scan failed"),
            }

            thread::sleep(reel_interval);
        }

        if !caught {
            continue;
        }

        let fish_caught = {
            let mut session = state.session.write();
            session.fish_caught += 1;
            session.last_action = format!("Caught fish #{}", session.fish_caught);
            session.fish_caught
        };

        {
            let mut stats = state.stats.write();
            stats.total_fish_caught += 1;
            stats.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            stats.best_session_fish = stats.best_session_fish.max(fish_caught);
        }
        emit_state_update(&window, &state);

        if fish_caught % config.fish_per_feed as u64 == 0 {
            {
                let mut session = state.session.write();
                session.last_action = "Checking hunger".to_string();
            }
            emit_state_update(&window, &state);

            match check_hunger_ocr(hunger_region) {
                Ok(hunger) => {
                    {
                        let mut session = state.session.write();
                        session.hunger_level = hunger.min(100) as u8;
                    }
                    emit_state_update(&window, &state);

                    if hunger < 50 {
                        let _ = enigo.key(Key::Layout('1'), Direction::Click);
                        thread::sleep(Duration::from_millis(100));
                        let _ = enigo.button(Button::Left, Direction::Click);
                        thread::sleep(Duration::from_millis(200));
                        let _ = enigo.key(Key::Layout('2'), Direction::Click);

                        {
                            let mut stats = state.stats.write();
                            stats.total_feeds += 1;
                            stats.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        }
                        {
                            let mut session = state.session.write();
                            session.last_action = "Fed character".to_string();
                        }
                        emit_state_update(&window, &state);
                    }
                }
                Err(_) => update_error_state(&state, &window, "OCR hunger check failed"),
            }
        }

        thread::sleep(Duration::from_millis(50));
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
    {
        let mut stats = state.stats.write();
        stats.sessions_completed += 1;
        stats.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }
    emit_state_update(window, state);

    if let Some(handle) = state.worker_handle.lock().expect("worker handle lock").take() {
        let _ = handle.join();
    }
}
