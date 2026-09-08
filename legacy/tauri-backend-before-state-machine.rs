use crate::detection::{self, Calibration};
use anyhow::{anyhow, Result};
use chrono::{Local, Timelike};
use directories::ProjectDirs;
use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use image::{DynamicImage, RgbaImage};
use parking_lot::RwLock;
use rusty_tesseract::{Args, Image as TessImage};
use screenshots::Screen;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
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
    pub settings: Option<BotConfig>,
    pub red_region: Region,
    pub yellow_region: Region,
    pub hunger_region: Region,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BotConfig {
    pub calibration: Option<Calibration>,
    pub rod_slot: u8,
    pub food_slot: u8,
    pub auto_feed_enabled: bool,
    pub feed_below_percent: u8,
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
            calibration: None,
            rod_slot: 2,
            food_slot: 1,
            auto_feed_enabled: true,
            feed_below_percent: 50,
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
    pub fn validate(&self) -> Result<()> {
        if self.rod_slot > 9
            || self.food_slot > 9
            || self.rod_slot == self.food_slot
            || self.feed_below_percent > 100
            || self.fish_per_feed == 0
            || self.detection_interval_ms < 10
            || self.autoclick_interval_ms < 10
            || self.max_fishing_timeout_ms == 0
            || !self.rod_lure_value.is_finite()
            || self.rod_lure_value < 0.0
            || self.startup_delay_ms > 300_000
        {
            return Err(anyhow!("Invalid timing, lure value, or feeding interval"));
        }
        for region in [self.red_region, self.yellow_region, self.hunger_region] {
            if region.width == 0
                || region.height == 0
                || region.width > 16384
                || region.height > 16384
            {
                return Err(anyhow!("Region dimensions must be between 1 and 16384"));
            }
        }
        if let Some(calibration) = &self.calibration {
            if calibration.hotbar_slot_stride > 16384
                || calibration.hotbar_first_slot.iter().any(|v| *v > 16384)
                || calibration.hotbar_first_slot[2] == 0
                || calibration.hotbar_first_slot[3] == 0
                || calibration.frame_width == 0
                || calibration.frame_height == 0
                || calibration.frame_width > 16384
                || calibration.frame_height > 16384
                || calibration.bite_min_pixels == 0
                || calibration.catch_min_pixels == 0
            {
                return Err(anyhow!("Invalid screenshot calibration"));
            }
            for region in [
                self.red_region,
                self.yellow_region,
                self.hunger_region,
                rod_selection_region(calibration, self.rod_slot),
            ] {
                if region.x < 0
                    || region.y < 0
                    || region.x as u64 + region.width as u64 > calibration.frame_width as u64
                    || region.y as u64 + region.height as u64 > calibration.frame_height as u64
                {
                    return Err(anyhow!("Region extends beyond the calibration image"));
                }
            }
        }
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            let config: Self = serde_json::from_str(&contents)?;
            config.validate()?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        self.validate()?;
        save_json(&path, self)
    }

    fn config_path() -> PathBuf {
        ProjectDirs::from("com", "arcane", "fishing-bot")
            .map(|dirs| dirs.config_dir().join("config.json"))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    pub fn calculate_max_bite_time(&self) -> Duration {
        Duration::from_millis(calculate_timeout_ms(self.rod_lure_value))
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
            settings: None,
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
            settings: None,
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
    let profile = macbook_profile();
    presets.insert(
        profile.region_preset.clone(),
        ResolutionPreset {
            red_region: profile.red_region,
            yellow_region: profile.yellow_region,
            hunger_region: profile.hunger_region,
            settings: Some(profile),
        },
    );
    presets
}

pub fn macbook_profile() -> BotConfig {
    serde_json::from_str(include_str!("../../configs/macbook-pro-14-3024x1964.json"))
        .expect("bundled MacBook profile must be valid")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

impl LifetimeStats {
    fn path() -> PathBuf {
        ProjectDirs::from("com", "arcane", "fishing-bot")
            .map(|dirs| dirs.data_dir().join("tauri-stats.json"))
            .unwrap_or_else(|| PathBuf::from("tauri-stats.json"))
    }

    fn load() -> Result<Self> {
        match std::fs::read(Self::path()) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    fn save(&self) -> Result<()> {
        save_json(&Self::path(), self)
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
    pub log_path: Arc<Mutex<Option<PathBuf>>>,
}

impl SharedState {
    pub fn new() -> Result<Self> {
        let config = BotConfig::load()?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(LifetimeStats::load()?)),
            session: Arc::new(RwLock::new(SessionState::default())),
            running: Arc::new(AtomicBool::new(false)),
            worker_handle: Arc::new(Mutex::new(None)),
            log_path: Arc::new(Mutex::new(None)),
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
    let screen = Screen::from_point(region.x, region.y)?;
    let relative_x = region.x - screen.display_info.x;
    let relative_y = region.y - screen.display_info.y;
    if relative_x < 0
        || relative_y < 0
        || relative_x as u64 + region.width as u64 > screen.display_info.width as u64
        || relative_y as u64 + region.height as u64 > screen.display_info.height as u64
    {
        return Err(anyhow!(
            "Capture region extends beyond its display; choose a matching resolution preset"
        ));
    }
    screen.capture_area(relative_x, relative_y, region.width, region.height)
}

fn rod_selection_region(calibration: &Calibration, slot: u8) -> Region {
    let [x, y, width, height] = calibration.hotbar_first_slot;
    let index = if slot == 0 { 9 } else { u32::from(slot - 1) };
    Region {
        x: (x + index * calibration.hotbar_slot_stride) as i32,
        y: y as i32,
        width,
        height,
    }
}

fn capture_config_region(region: Region, config: &BotConfig) -> Result<RgbaImage> {
    let Some(calibration) = &config.calibration else {
        return capture_region(region);
    };
    let screen = Screen::from_point(0, 0)?;
    let display = screen.display_info;
    let reference_aspect = calibration.frame_width as f64 / calibration.frame_height as f64;
    let display_aspect = display.width as f64 / display.height as f64;
    if (reference_aspect - display_aspect).abs() > 0.02 {
        return Err(anyhow!(
            "Display aspect ratio does not match this screenshot profile"
        ));
    }
    let (x, y, width, height) =
        map_reference_region(region, calibration, display.width, display.height);
    let captured = screen.capture_area(x, y, width, height)?;
    Ok(normalize_capture(
        captured,
        region,
        calibration,
        display.width,
        display.height,
    ))
}

// Integer logical capture bounds can add backing pixels around a Retina ROI.
// Remove that padding before resizing, especially for the 22px Energy text.
fn normalize_capture(
    captured: RgbaImage,
    region: Region,
    calibration: &Calibration,
    display_width: u32,
    display_height: u32,
) -> RgbaImage {
    let (x, y, width, height) =
        map_reference_region(region, calibration, display_width, display_height);
    let sx = display_width as f64 / calibration.frame_width as f64;
    let sy = display_height as f64 / calibration.frame_height as f64;
    let px = captured.width() as f64 / width as f64;
    let py = captured.height() as f64 / height as f64;
    let left = (((region.x as f64 * sx - x as f64) * px).round() as u32).min(captured.width() - 1);
    let top = (((region.y as f64 * sy - y as f64) * py).round() as u32).min(captured.height() - 1);
    let right = ((((region.x as f64 + region.width as f64) * sx - x as f64) * px).round() as u32)
        .clamp(left + 1, captured.width());
    let bottom = ((((region.y as f64 + region.height as f64) * sy - y as f64) * py).round() as u32)
        .clamp(top + 1, captured.height());
    let cropped =
        image::imageops::crop_imm(&captured, left, top, right - left, bottom - top).to_image();
    if cropped.dimensions() == (region.width, region.height) {
        return cropped;
    }
    image::imageops::resize(
        &cropped,
        region.width,
        region.height,
        image::imageops::FilterType::Triangle,
    )
}

fn map_reference_region(
    region: Region,
    calibration: &Calibration,
    width: u32,
    height: u32,
) -> (i32, i32, u32, u32) {
    let x = region.x as u64 * width as u64 / calibration.frame_width as u64;
    let y = region.y as u64 * height as u64 / calibration.frame_height as u64;
    let right = ((region.x as u64 + region.width as u64) * width as u64)
        .div_ceil(calibration.frame_width as u64);
    let bottom = ((region.y as u64 + region.height as u64) * height as u64)
        .div_ceil(calibration.frame_height as u64);
    (x as i32, y as i32, (right - x) as u32, (bottom - y) as u32)
}

fn has_bite(image: &RgbaImage, config: &BotConfig) -> bool {
    if let Some(calibration) = &config.calibration {
        return detection::bite_visible(image, calibration);
    }
    let threshold = ((config.red_region.width * config.red_region.height) / 300).max(15);
    count_matching_pixels(image, (241, 27, 28), config.color_tolerance, threshold) >= threshold
}

fn has_catch(image: &RgbaImage, config: &BotConfig) -> bool {
    if let Some(calibration) = &config.calibration {
        return detection::catch_visible(image, calibration);
    }
    let threshold = ((config.yellow_region.width * config.yellow_region.height) / 400).max(10);
    count_matching_pixels(image, (255, 255, 0), config.color_tolerance, threshold) >= threshold
}

fn count_matching_pixels(
    image: &RgbaImage,
    target: (u8, u8, u8),
    tolerance: u8,
    threshold: u32,
) -> u32 {
    let tolerance = tolerance as i32;
    image
        .pixels()
        .filter(|pixel| {
            let dr = (pixel[0] as i32 - target.0 as i32).abs();
            let dg = (pixel[1] as i32 - target.1 as i32).abs();
            let db = (pixel[2] as i32 - target.2 as i32).abs();
            dr + dg + db <= tolerance * 3
        })
        .take(threshold as usize)
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
    cleaned.parse::<u32>().ok().filter(|value| *value <= 100)
}

fn check_hunger_ocr(region: Region, config: &BotConfig) -> Result<u32> {
    let image = capture_config_region(region, config)?;
    read_resource_ocr(image, config.calibration.is_some())
}

fn read_resource_ocr(image: RgbaImage, energy: bool) -> Result<u32> {
    let grayscale = if energy {
        DynamicImage::ImageLuma8(detection::energy_ocr_image(&image))
    } else {
        DynamicImage::ImageRgba8(image).grayscale()
    };
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join("hunger.png");
    grayscale.save(&temp_path)?;

    let mut config_variables = HashMap::new();
    config_variables.insert(
        "tessedit_char_whitelist".to_string(),
        if energy { "0123456789/" } else { "0123456789%" }.to_string(),
    );
    let args = Args {
        lang: "eng".to_string(),
        dpi: Some(150),
        psm: Some(if energy { 7 } else { 8 }),
        oem: Some(3),
        config_variables,
    };

    let result = if let Ok(tess_image) = TessImage::from_path(temp_path.canonicalize()?) {
        rusty_tesseract::image_to_string(&tess_image, &args)
            .ok()
            .and_then(|text| {
                if energy {
                    detection::parse_energy(&text)
                } else {
                    parse_hunger_text(&text)
                }
            })
    } else {
        None
    };

    std::fs::remove_file(&temp_path).ok();
    result.ok_or_else(|| anyhow!("OCR did not return a valid resource reading"))
}

fn update_error_state(state: &SharedState, window: &Window, message: &str) {
    {
        let mut session = state.session.write();
        session.errors_count += 1;
        session.last_action = message.to_string();
    }
    log_event(state, "ERROR", message);
    emit_state_update(window, state);
    wait_running(state, Duration::from_millis(250));
}

fn session_log_path() -> Result<PathBuf> {
    let base_dir = ProjectDirs::from("com", "arcane", "fishing-bot")
        .map(|dirs| dirs.data_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));
    std::fs::create_dir_all(&base_dir)?;
    let filename = format!("session_{}.log", Local::now().format("%Y%m%d_%H%M%S"));
    Ok(base_dir.join(filename))
}

fn append_log(path: &Path, level: &str, message: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}] [{}] {}", timestamp, level, message)?;
    Ok(())
}

fn log_event(state: &SharedState, level: &str, message: &str) {
    let log_path = state.log_path.lock().ok().and_then(|guard| guard.clone());
    if let Some(path) = log_path {
        if let Err(err) = append_log(&path, level, message) {
            eprintln!("Failed to write log: {}", err);
        }
    }
}

fn worker_loop(state: SharedState, window: Window) {
    let start_time = Instant::now();
    let mut last_uptime_minutes = 0;
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(enigo) => enigo,
        Err(error) => {
            update_error_state(
                &state,
                &window,
                &format!("Input initialization failed: {error}"),
            );
            return;
        }
    };
    let previous_runtime = state.stats.read().total_runtime_seconds;

    let startup_delay = state.config.read().startup_delay_ms;
    if !wait_running(&state, Duration::from_millis(startup_delay)) {
        return;
    }
    let initial_config = state.config.read().clone();
    if let Some(calibration) = &initial_config.calibration {
        let region = rod_selection_region(calibration, initial_config.rod_slot);
        let selected = match capture_config_region(region, &initial_config) {
            Ok(image) => detection::rod_selected(&image),
            Err(error) => {
                update_error_state(
                    &state,
                    &window,
                    &format!("Rod slot capture failed: {error}"),
                );
                return;
            }
        };
        // Hotbar keys toggle selection. Do not unequip an already selected rod.
        if !selected {
            if enigo
                .key(
                    Key::Unicode(char::from(b'0' + initial_config.rod_slot)),
                    Direction::Click,
                )
                .is_err()
            {
                update_error_state(&state, &window, "Failed to select fishing rod");
                return;
            }
            if !wait_running(&state, Duration::from_millis(300)) {
                return;
            }
            match capture_config_region(region, &initial_config) {
                Ok(image) if detection::rod_selected(&image) => {}
                _ => {
                    update_error_state(&state, &window, "Rod selection could not be confirmed");
                    return;
                }
            }
        }
    }
    log_event(&state, "INFO", "Worker loop started");

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
                stats.total_runtime_seconds = previous_runtime + elapsed.as_secs();
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
        let mut catch_armed = match capture_config_region(yellow_region, &config) {
            Ok(image) => !has_catch(&image, &config),
            Err(error) => {
                update_error_state(
                    &state,
                    &window,
                    &format!("Catch baseline capture failed: {error}"),
                );
                continue;
            }
        };

        {
            let mut session = state.session.write();
            session.last_action = "Casting line".to_string();
        }
        emit_state_update(&window, &state);

        if enigo.button(Button::Left, Direction::Click).is_err() {
            update_error_state(&state, &window, "Failed to cast");
            wait_running(&state, Duration::from_millis(200));
            continue;
        }
        log_event(&state, "INFO", "Cast line");
        wait_running(&state, reel_interval);

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

            if !catch_armed {
                if let Ok(image) = capture_config_region(yellow_region, &config) {
                    catch_armed = !has_catch(&image, &config);
                }
            }
            match capture_config_region(red_region, &config) {
                Ok(image) => {
                    if has_bite(&image, &config) {
                        bite_detected = true;
                        log_event(&state, "INFO", "Bite detected");
                        break;
                    }
                }
                Err(_) => update_error_state(&state, &window, "Red scan failed"),
            }

            wait_running(&state, detection_interval);
        }

        if !bite_detected {
            continue;
        }

        {
            let mut session = state.session.write();
            session.last_action = "Reeling in".to_string();
        }
        emit_state_update(&window, &state);
        log_event(&state, "INFO", "Reeling started");

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

            if enigo.button(Button::Left, Direction::Click).is_err() {
                update_error_state(&state, &window, "Failed to reel");
            }

            match capture_config_region(yellow_region, &config) {
                Ok(image) => {
                    if !has_catch(&image, &config) {
                        catch_armed = true;
                    }
                    if catch_armed && has_catch(&image, &config) {
                        if !wait_running(&state, detection_interval) {
                            break;
                        }
                        if let Ok(confirm_image) = capture_config_region(yellow_region, &config) {
                            if has_catch(&confirm_image, &config) {
                                caught = true;
                                break;
                            }
                        }
                    }
                }
                Err(_) => update_error_state(&state, &window, "Yellow scan failed"),
            }

            wait_running(&state, reel_interval);
        }

        if !state.running.load(Ordering::Relaxed) {
            break;
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
        log_event(&state, "INFO", &format!("Caught fish #{}", fish_caught));

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
                session.last_action = "Checking Energy / hunger".to_string();
            }
            emit_state_update(&window, &state);
            log_event(&state, "INFO", "Checking Energy / hunger");

            match check_hunger_ocr(hunger_region, &config) {
                Ok(hunger) => {
                    {
                        let mut session = state.session.write();
                        session.hunger_level = hunger.min(100) as u8;
                    }
                    emit_state_update(&window, &state);
                    log_event(&state, "INFO", &format!("Resource level {}%", hunger));

                    if config.auto_feed_enabled
                        && hunger < u32::from(config.feed_below_percent)
                        && state.running.load(Ordering::Relaxed)
                    {
                        if enigo
                            .key(
                                Key::Unicode(char::from(b'0' + config.food_slot)),
                                Direction::Click,
                            )
                            .is_err()
                        {
                            update_error_state(&state, &window, "Failed to select food");
                            continue;
                        }
                        if !wait_running(&state, Duration::from_millis(100)) {
                            break;
                        }
                        let feed_result = enigo.button(Button::Left, Direction::Click);
                        if !wait_running(&state, Duration::from_millis(200)) {
                            break;
                        }
                        let rod_result = enigo.key(
                            Key::Unicode(char::from(b'0' + config.rod_slot)),
                            Direction::Click,
                        );
                        if feed_result.is_err() || rod_result.is_err() {
                            update_error_state(&state, &window, "Failed to feed or restore rod");
                            continue;
                        }

                        {
                            let mut stats = state.stats.write();
                            stats.total_feeds += 1;
                            stats.last_updated =
                                Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        }
                        {
                            let mut session = state.session.write();
                            session.last_action = "Fed character".to_string();
                        }
                        emit_state_update(&window, &state);
                        log_event(&state, "INFO", "Fed character");
                    }
                }
                Err(_) => update_error_state(&state, &window, "Energy / hunger OCR failed"),
            }
        }

        wait_running(&state, Duration::from_millis(50));
    }
}

// Short waits bound stop latency without changing the configured scan cadence.
fn wait_running(state: &SharedState, duration: Duration) -> bool {
    let started = Instant::now();
    while state.running.load(Ordering::Relaxed) {
        let remaining = duration.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return true;
        }
        thread::sleep(remaining.min(Duration::from_millis(20)));
    }
    false
}

pub fn start_bot(state: &SharedState, window: &Window) {
    let mut handle_guard = state.worker_handle.lock().expect("worker handle lock");
    if let Some(handle) = handle_guard.as_ref() {
        if !handle.is_finished() {
            return;
        }
    }
    if let Some(handle) = handle_guard.take() {
        let _ = handle.join();
    }
    state.running.store(true, Ordering::Relaxed);
    if let Ok(path) = session_log_path() {
        *state.log_path.lock().expect("log path lock") = Some(path);
    }
    *state.session.write() = SessionState {
        running: true,
        last_action: format!(
            "Started at {:02}:{:02}",
            Local::now().hour(),
            Local::now().minute()
        ),
        ..SessionState::default()
    };
    emit_state_update(window, state);
    log_event(state, "INFO", "Session started");
    let thread_state = state.clone();
    let thread_window = window.clone();
    *handle_guard = Some(thread::spawn(move || {
        let started = Instant::now();
        let previous_runtime = thread_state.stats.read().total_runtime_seconds;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_loop(thread_state.clone(), thread_window.clone());
        }));
        thread_state.running.store(false, Ordering::Relaxed);
        {
            let mut session = thread_state.session.write();
            session.running = false;
            session.uptime_minutes = started.elapsed().as_secs() / 60;
            if result.is_err() {
                session.errors_count += 1;
                session.last_action = "Worker failed".into();
            } else if session.errors_count == 0 || session.last_action == "Stopping" {
                session.last_action = "Stopped".into();
            }
        }
        {
            let mut stats = thread_state.stats.write();
            stats.total_runtime_seconds = previous_runtime + started.elapsed().as_secs();
            stats.sessions_completed += 1;
            stats.average_fish_per_hour = if stats.total_runtime_seconds > 0 {
                stats.total_fish_caught as f32 * 3600.0 / stats.total_runtime_seconds as f32
            } else {
                0.0
            };
            stats.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        }
        let stats = thread_state.stats.read().clone();
        if let Err(error) = stats.save() {
            update_error_state(
                &thread_state,
                &thread_window,
                &format!("Failed to save statistics: {error}"),
            );
        }
        emit_state_update(&thread_window, &thread_state);
        log_event(&thread_state, "INFO", "Session ended");
    }));
}

pub fn stop_bot(state: &SharedState, window: &Window) {
    if !state.running.swap(false, Ordering::Relaxed) {
        return;
    }
    state.session.write().last_action = "Stopping".into();
    emit_state_update(window, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macbook_profile_and_retina_mapping() {
        let profile = macbook_profile();
        profile.validate().unwrap();
        assert_eq!((profile.rod_slot, profile.food_slot), (4, 5));
        let mapped = map_reference_region(
            profile.red_region,
            profile.calibration.as_ref().unwrap(),
            1512,
            982,
        );
        assert_eq!(mapped, (0, 35, 1315, 745));
    }

    // Opt-in: original screenshots stay outside the repository.
    #[test]
    #[ignore = "requires ARCANE_SCREENSHOT_DIR and Tesseract"]
    fn supplied_macbook_screenshots() {
        let directory = std::env::var("ARCANE_SCREENSHOT_DIR").expect("set screenshot directory");
        let config = macbook_profile();
        for (file, expected_bite, expected_catch) in [
            ("1..png", false, false),
            ("2..png", true, false),
            ("3.png", false, false),
            ("4..png", false, true),
        ] {
            let image = image::open(Path::new(&directory).join(file))
                .unwrap()
                .to_rgba8();
            assert_eq!(image.dimensions(), (3024, 1964));
            let crop = |region: Region| {
                image::imageops::crop_imm(
                    &image,
                    region.x as u32,
                    region.y as u32,
                    region.width,
                    region.height,
                )
                .to_image()
            };
            let calibration = config.calibration.as_ref().unwrap();
            for region in [
                config.red_region,
                config.yellow_region,
                config.hunger_region,
            ] {
                let (x, y, w, h) = map_reference_region(region, calibration, 1512, 982);
                let backing =
                    image::imageops::crop_imm(&image, x as u32 * 2, y as u32 * 2, w * 2, h * 2)
                        .to_image();
                assert_eq!(
                    normalize_capture(backing, region, calibration, 1512, 982),
                    crop(region),
                    "Retina crop: {file}"
                );
            }
            let started = Instant::now();
            let bite = has_bite(&crop(config.red_region), &config);
            let elapsed = started.elapsed();
            let caught = has_catch(&crop(config.yellow_region), &config);
            assert_eq!(
                detection::rod_selected(&crop(rod_selection_region(
                    config.calibration.as_ref().unwrap(),
                    config.rod_slot
                ))),
                file != "1..png",
                "rod selection: {file}"
            );
            let energy = read_resource_ocr(crop(config.hunger_region), true).unwrap();
            println!(
                "{file}: bite={bite}, caught={caught}, energy={energy}%, bite scan={elapsed:?}"
            );
            assert_eq!(bite, expected_bite, "bite: {file}");
            assert_eq!(caught, expected_catch, "catch: {file}");
            assert_eq!(energy, 100, "energy: {file}");
            if expected_bite {
                let marker = image::imageops::crop_imm(&image, 1412, 535, 249, 287).to_image();
                for (x, y) in [(20, 20), (2000, 1000)] {
                    let mut moved = RgbaImage::from_pixel(
                        config.red_region.width,
                        config.red_region.height,
                        image::Rgba([30, 30, 30, 255]),
                    );
                    image::imageops::replace(&mut moved, &marker, x, y);
                    assert!(has_bite(&moved, &config), "moving bite at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn rejects_unsafe_configuration() {
        let mut config = BotConfig::default();
        assert!(config.validate().is_ok());
        config.fish_per_feed = 0;
        assert!(config.validate().is_err());
        config.fish_per_feed = 5;
        config.detection_interval_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn hunger_is_bounded() {
        assert_eq!(parse_hunger_text("42%\n"), Some(42));
        assert_eq!(parse_hunger_text("unknown"), None);
        assert_eq!(parse_hunger_text("150%"), None);
    }

    #[test]
    fn detector_stops_at_threshold_and_preserves_decision() {
        let image = RgbaImage::from_pixel(100, 100, image::Rgba([241, 27, 28, 255]));
        assert_eq!(count_matching_pixels(&image, (241, 27, 28), 10, 15), 15);
        assert_eq!(count_matching_pixels(&image, (255, 255, 0), 10, 15), 0);
    }

    #[test]
    fn atomic_json_replaces_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        save_json(&path, &vec![1, 2]).unwrap();
        save_json(&path, &vec![3]).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "[\n  3\n]");
    }

    #[test]
    fn older_config_gets_new_defaults() {
        let config: BotConfig = serde_json::from_str("{\"fish_per_feed\":7}").unwrap();
        assert_eq!(config.fish_per_feed, 7);
        assert_eq!(config.detection_interval_ms, 50);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn timeouts_are_bounded() {
        assert_eq!(calculate_timeout_ms(1.0), 65_000);
        assert_eq!(calculate_timeout_ms(0.0), 180_000);
        assert_eq!(calculate_timeout_ms(10.0), 10_000);
    }
}
