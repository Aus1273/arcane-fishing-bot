use anyhow::Result;
use chrono::{Local, Timelike};
use directories::ProjectDirs;
use enigo::{Button, Direction, Enigo, Key, Mouse, Settings};
use image::{DynamicImage, RgbaImage};
use parking_lot::RwLock;
use rusty_tesseract::{image_to_string, Args, Image};
use serde::{Deserialize, Serialize};
use screenshots::Screen;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
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
        let lure = self.rod_lure_value;
        let multiplier = if lure <= 1.0 {
            3.0 - 2.0 * lure
        } else {
            1.25 - lure / 3.0
        };

        let seconds = (multiplier * 60.0 + 5.0).clamp(10.0, 180.0);
        Duration::from_secs_f32(seconds)
    }

    pub fn get_timeout_description(&self) -> String {
        let timeout = self.calculate_max_bite_time();
        format!(
            "Lure {:.1}: ~{:.0}s timeout",
            self.rod_lure_value,
            timeout.as_secs_f32()
        )
    }

    pub fn apply_resolution_preset(&mut self, preset: &str) {
        match preset {
            "3440x1440" => {
                self.red_region = Region {
                    x: 1321,
                    y: 99,
                    width: 768,
                    height: 546,
                };
                self.yellow_region = Region {
                    x: 3097,
                    y: 1234,
                    width: 342,
                    height: 205,
                };
                self.hunger_region = Region {
                    x: 274,
                    y: 1301,
                    width: 43,
                    height: 36,
                };
            }
            "1920x1080" => {
                self.red_region = Region {
                    x: 598,
                    y: 29,
                    width: 901,
                    height: 477,
                };
                self.yellow_region = Region {
                    x: 1649,
                    y: 632,
                    width: 270,
                    height: 447,
                };
                self.hunger_region = Region {
                    x: 212,
                    y: 984,
                    width: 21,
                    height: 18,
                };
            }
            _ => {}
        }
        self.region_preset = preset.to_string();
    }
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
}

impl SharedState {
    pub fn new() -> Result<Self> {
        let config = BotConfig::load()?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(LifetimeStats::default())),
            session: Arc::new(RwLock::new(SessionState::default())),
            running: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const RED_EXCLAMATION: Color = Color {
        r: 241,
        g: 27,
        b: 28,
    };
    const YELLOW_CAUGHT: Color = Color {
        r: 255,
        g: 255,
        b: 0,
    };

    fn distance(&self, other: &[u8]) -> u32 {
        let dr = (self.r as i32 - other[0] as i32).unsigned_abs();
        let dg = (self.g as i32 - other[1] as i32).unsigned_abs();
        let db = (self.b as i32 - other[2] as i32).unsigned_abs();
        dr + dg + db
    }
}

fn capture_region(region: Region) -> Result<RgbaImage> {
    let screens = Screen::all()?;
    if screens.is_empty() {
        anyhow::bail!("No screens found");
    }
    let image = screens[0].capture_area(region.x, region.y, region.width, region.height)?;
    RgbaImage::from_raw(region.width, region.height, image.to_vec())
        .ok_or_else(|| anyhow::anyhow!("Failed to create image"))
}

fn count_matching_pixels(image: &RgbaImage, target: &Color, tolerance: u8) -> usize {
    let max_distance = tolerance as u32 * 3;
    image
        .pixels()
        .filter(|pixel| target.distance(&pixel.0) <= max_distance)
        .count()
}

fn preprocess_hunger_image(image: &RgbaImage) -> DynamicImage {
    let mut grayscale = DynamicImage::ImageRgba8(image.clone()).to_luma8();
    for pixel in grayscale.pixels_mut() {
        let value = if pixel[0] > 160 { 255 } else { 0 };
        *pixel = image::Luma([value]);
    }
    DynamicImage::ImageLuma8(grayscale)
}

fn parse_hunger_value(raw_text: &str) -> Result<u8> {
    let digits: String = raw_text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        anyhow::bail!("OCR returned no digits: {raw_text:?}");
    }
    let value: u8 = digits.parse()?;
    Ok(value.min(100))
}

fn check_hunger(region: Region) -> Result<u8> {
    let image = capture_region(region)?;
    let processed = preprocess_hunger_image(&image);
    let input = Image::from_dynamic_image(&processed)?;

    let mut config_variables = HashMap::new();
    config_variables.insert("tessedit_char_whitelist".to_string(), "0123456789".to_string());

    let args = Args {
        lang: "eng".to_string(),
        config_variables,
        dpi: Some(150),
        psm: Some(7),
        oem: Some(3),
    };

    let text = image_to_string(&input, &args)?;
    parse_hunger_value(text.trim())
}

fn emit_session_update(window: &Window, session: &SessionState) {
    let _ = window.emit("state-update", session);
}

pub fn start_bot(state: &SharedState, window: Window) {
    if state.running.swap(true, Ordering::Relaxed) {
        return;
    }

    let started_action = format!(
        "Started at {:02}:{:02}",
        Local::now().hour(),
        Local::now().minute()
    );
    let session_snapshot = {
        let mut session = state.session.write();
        session.running = true;
        session.last_action = started_action;
        session.clone()
    };
    emit_session_update(&window, &session_snapshot);

    let state = state.clone();
    let window = window.clone();
    thread::spawn(move || {
        let mut input = Enigo::new(&Settings::default())
            .expect("failed to initialize input controller");
        let start_time = Instant::now();
        let startup_delay = {
            let config = state.config.read();
            config.startup_delay_ms
        };

        if startup_delay > 0 {
            let session_snapshot = {
                let mut session = state.session.write();
                session.last_action = "Waiting for startup delay...".to_string();
                session.clone()
            };
            emit_session_update(&window, &session_snapshot);
            thread::sleep(Duration::from_millis(startup_delay));
        }

        while state.running.load(Ordering::Relaxed) {
            let (
                red_region,
                yellow_region,
                detection_interval,
                reel_interval,
                bite_timeout,
                reel_timeout,
                color_tolerance,
            ) =
                {
                    let config = state.config.read();
                    (
                        config.red_region,
                        config.yellow_region,
                        Duration::from_millis(config.detection_interval_ms),
                        Duration::from_millis(config.autoclick_interval_ms),
                        config.calculate_max_bite_time(),
                        Duration::from_millis(config.max_fishing_timeout_ms),
                        config.color_tolerance,
                    )
                };

            let session_snapshot = {
                let mut session = state.session.write();
                session.last_action = "Casting fishing line...".to_string();
                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                session.clone()
            };
            emit_session_update(&window, &session_snapshot);
            let _ = input.button(Button::Left, Direction::Click);
            thread::sleep(reel_interval);

            let session_snapshot = {
                let mut session = state.session.write();
                session.last_action = format!(
                    "Scanning red region for bite (x:{} y:{} w:{} h:{})",
                    red_region.x, red_region.y, red_region.width, red_region.height
                );
                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                session.clone()
            };
            emit_session_update(&window, &session_snapshot);

            let mut bite_detected = false;
            let bite_start = Instant::now();
            let mut last_red_count = 0;
            while state.running.load(Ordering::Relaxed) {
                if bite_start.elapsed() > bite_timeout {
                    let session_snapshot = {
                        let mut session = state.session.write();
                        session.last_action = "No bite detected - recasting...".to_string();
                        session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                        session.clone()
                    };
                    emit_session_update(&window, &session_snapshot);
                    break;
                }

                match capture_region(red_region) {
                    Ok(image) => {
                        let red_count =
                            count_matching_pixels(&image, &Color::RED_EXCLAMATION, color_tolerance);
                        if red_count > 0 && red_count >= last_red_count {
                            bite_detected = true;
                            let session_snapshot = {
                                let mut session = state.session.write();
                                session.last_action = "Red bite detected - reeling in...".to_string();
                                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                                session.clone()
                            };
                            emit_session_update(&window, &session_snapshot);
                            break;
                        }
                        last_red_count = red_count;
                    }
                    Err(_) => {
                        let session_snapshot = {
                            let mut session = state.session.write();
                            session.errors_count += 1;
                            session.last_action =
                                "Screen capture failed during bite detection.".to_string();
                            session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                            session.clone()
                        };
                        emit_session_update(&window, &session_snapshot);
                    }
                }

                thread::sleep(detection_interval);
            }

            if !state.running.load(Ordering::Relaxed) {
                break;
            }

            if !bite_detected {
                continue;
            }

            let session_snapshot = {
                let mut session = state.session.write();
                session.last_action = format!(
                    "Reeling in catch (yellow region x:{} y:{} w:{} h:{})",
                    yellow_region.x, yellow_region.y, yellow_region.width, yellow_region.height
                );
                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                session.clone()
            };
            emit_session_update(&window, &session_snapshot);

            let reel_start = Instant::now();
            let mut fish_caught = false;
            while state.running.load(Ordering::Relaxed) {
                if reel_start.elapsed() > reel_timeout {
                    let session_snapshot = {
                        let mut session = state.session.write();
                        session.last_action = "Reeling timeout - fish escaped.".to_string();
                        session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                        session.clone()
                    };
                    emit_session_update(&window, &session_snapshot);
                    break;
                }

                match capture_region(yellow_region) {
                    Ok(image) => {
                        let _ = input.button(Button::Left, Direction::Click);
                        let yellow_count =
                            count_matching_pixels(&image, &Color::YELLOW_CAUGHT, color_tolerance);
                        if yellow_count > 0 {
                            thread::sleep(detection_interval);
                            if let Ok(confirm_image) = capture_region(yellow_region) {
                                let confirm_count = count_matching_pixels(
                                    &confirm_image,
                                    &Color::YELLOW_CAUGHT,
                                    color_tolerance,
                                );
                                if confirm_count > 0 {
                                    fish_caught = true;
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        let session_snapshot = {
                            let mut session = state.session.write();
                            session.errors_count += 1;
                            session.last_action =
                                "Screen capture failed during reeling.".to_string();
                            session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                            session.clone()
                        };
                        emit_session_update(&window, &session_snapshot);
                    }
                }

                thread::sleep(reel_interval);
            }

            if fish_caught {
                let (fish_per_feed, hunger_region) = {
                    let config = state.config.read();
                    (config.fish_per_feed, config.hunger_region)
                };
                let (session_snapshot, fish_count) = {
                    let mut session = state.session.write();
                    session.fish_caught += 1;
                    session.last_action = "Fish caught!".to_string();
                    session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                    (session.clone(), session.fish_caught)
                };
                emit_session_update(&window, &session_snapshot);

                if fish_per_feed > 0 && fish_count % fish_per_feed as u64 == 0 {
                    let hunger_result = check_hunger(hunger_region);
                    match hunger_result {
                        Ok(hunger_level) => {
                            let session_snapshot = {
                                let mut session = state.session.write();
                                session.hunger_level = hunger_level;
                                session.last_action =
                                    format!("Hunger OCR: {hunger_level}%");
                                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                                session.clone()
                            };
                            emit_session_update(&window, &session_snapshot);

                            if hunger_level < 50 {
                                let session_snapshot = {
                                    let mut session = state.session.write();
                                    session.last_action = "Eating food...".to_string();
                                    session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                                    session.clone()
                                };
                                emit_session_update(&window, &session_snapshot);

                                let _ = input.key_click(Key::Layout('1'));
                                thread::sleep(Duration::from_millis(300));
                                let _ = input.key_click(Key::Layout('2'));
                                thread::sleep(Duration::from_millis(300));
                            }
                        }
                        Err(error) => {
                            let session_snapshot = {
                                let mut session = state.session.write();
                                session.errors_count += 1;
                                session.last_action =
                                    format!("Hunger OCR failed: {error}");
                                session.uptime_minutes = start_time.elapsed().as_secs() / 60;
                                session.clone()
                            };
                            emit_session_update(&window, &session_snapshot);
                        }
                    }
                }
            }
        }

        let session_snapshot = {
            let mut session = state.session.write();
            session.running = false;
            session.last_action = "Stopped".to_string();
            session.clone()
        };
        emit_session_update(&window, &session_snapshot);
    });
}

pub fn stop_bot(state: &SharedState) {
    state.running.store(false, Ordering::Relaxed);
    let mut session = state.session.write();
    session.running = false;
    session.last_action = "Stopped".to_string();
}
