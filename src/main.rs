// src/main.rs - Complete Arcane Odyssey Fishing Bot in Rust with All Features

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use chrono::{Local, Timelike};
use std::collections::HashMap;
use std::path::PathBuf;

// ===== CONFIG MODULE =====
mod config {
    use super::*;
    use std::fs;
    
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
    
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Region {
        pub x: i32,
        pub y: i32,
        pub width: u32,
        pub height: u32,
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
                red_region: Region { x: 1321, y: 99, width: 768, height: 546 },
                yellow_region: Region { x: 3097, y: 1234, width: 342, height: 205 },
                hunger_region: Region { x: 274, y: 1301, width: 43, height: 36 },
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
                let contents = fs::read_to_string(path)?;
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
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
            Ok(())
        }
        
        fn config_path() -> PathBuf {
            directories::ProjectDirs::from("com", "arcane", "fishing-bot")
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
            format!("Lure {:.1}: ~{:.0}s timeout", self.rod_lure_value, timeout.as_secs_f32())
        }
        
        pub fn apply_resolution_preset(&mut self, preset: &str) {
            match preset {
                "3440x1440" => {
                    self.red_region = Region { x: 1321, y: 99, width: 768, height: 546 };
                    self.yellow_region = Region { x: 3097, y: 1234, width: 342, height: 205 };
                    self.hunger_region = Region { x: 274, y: 1301, width: 43, height: 36 };
                }
                "1920x1080" => {
                    self.red_region = Region { x: 598, y: 29, width: 901, height: 477 };
                    self.yellow_region = Region { x: 1649, y: 632, width: 270, height: 447 };
                    self.hunger_region = Region { x: 212, y: 984, width: 21, height: 18 };
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
                last_updated: Local::now().to_rfc3339(),
                best_session_fish: 0,
                average_fish_per_hour: 0.0,
                total_feeds: 0,
                uptime_percentage: 100.0,
            }
        }
    }
    
    impl LifetimeStats {
        pub fn load() -> Result<Self> {
            let path = Self::stats_path();
            if path.exists() {
                let contents = fs::read_to_string(path)?;
                Ok(serde_json::from_str(&contents)?)
            } else {
                Ok(Self::default())
            }
        }
        
        pub fn save(&mut self) -> Result<()> {
            self.last_updated = Local::now().to_rfc3339();
            self.update_calculations();
            let path = Self::stats_path();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
            Ok(())
        }
        
        fn stats_path() -> PathBuf {
            directories::ProjectDirs::from("com", "arcane", "fishing-bot")
                .map(|dirs| dirs.data_dir().join("stats.json"))
                .unwrap_or_else(|| PathBuf::from("stats.json"))
        }
        
        pub fn get_formatted_runtime(&self) -> String {
            let hours = self.total_runtime_seconds / 3600;
            let minutes = (self.total_runtime_seconds % 3600) / 60;
            format!("{}h {}m", hours, minutes)
        }
        
        pub fn add_fish(&mut self, count: u64) {
            self.total_fish_caught += count;
            self.save().ok();
        }
        
        pub fn add_runtime(&mut self, seconds: u64) {
            self.total_runtime_seconds += seconds;
            self.save().ok();
        }
        
        pub fn complete_session(&mut self, session_fish: u64) {
            self.sessions_completed += 1;
            if session_fish > self.best_session_fish {
                self.best_session_fish = session_fish;
            }
            self.save().ok();
        }
        
        pub fn add_feed(&mut self) {
            self.total_feeds += 1;
            self.save().ok();
        }
        
        fn update_calculations(&mut self) {
            if self.total_runtime_seconds > 0 {
                self.average_fish_per_hour = (self.total_fish_caught as f32 * 3600.0) / self.total_runtime_seconds as f32;
            }
        }
    }
}

// ===== DETECTION MODULE =====
mod detection {
    use super::*;
    use screenshots::Screen;
    use image::{RgbaImage};
    use config::Region;
    use rayon::prelude::*;
    
    #[derive(Debug, Clone, Copy)]
    pub struct Color {
        pub r: u8,
        pub g: u8,
        pub b: u8,
    }
    
    impl Color {
        pub const RED_EXCLAMATION: Color = Color { r: 241, g: 27, b: 28 };
        pub const YELLOW_CAUGHT: Color = Color { r: 255, g: 255, b: 0 };
        
        pub fn distance(&self, other: &[u8]) -> u32 {
            let dr = (self.r as i32 - other[0] as i32).abs() as u32;
            let dg = (self.g as i32 - other[1] as i32).abs() as u32;
            let db = (self.b as i32 - other[2] as i32).abs() as u32;
            dr + dg + db
        }
        
        pub fn distance_squared(&self, other: &[u8]) -> u32 {
            let dr = (self.r as i32 - other[0] as i32) as u32;
            let dg = (self.g as i32 - other[1] as i32) as u32;
            let db = (self.b as i32 - other[2] as i32) as u32;
            dr * dr + dg * dg + db * db
        }
    }
    
    pub struct AdvancedDetector {
        cache: Arc<RwLock<HashMap<String, (RgbaImage, Instant)>>>,
        cache_duration: Duration,
        tolerance: u8,
        advanced_mode: bool,
    }
    
    impl AdvancedDetector {
        pub fn new(cache_duration_ms: u64, tolerance: u8, advanced_mode: bool) -> Self {
            Self {
                cache: Arc::new(RwLock::new(HashMap::new())),
                cache_duration: Duration::from_millis(cache_duration_ms),
                tolerance,
                advanced_mode,
            }
        }
        
        pub fn detect_color(&self, region: Region, target: &Color) -> Result<bool> {
            let screenshot = self.get_screenshot(region)?;
            
            if self.advanced_mode {
                self.advanced_color_detection(&screenshot, target)
            } else {
                self.basic_color_detection(&screenshot, target)
            }
        }
        
        fn basic_color_detection(&self, image: &RgbaImage, target: &Color) -> Result<bool> {
            let tolerance = self.tolerance as u32 * 3;
            let pixels: Vec<_> = image.pixels().collect();
            
            Ok(pixels.par_iter().any(|pixel| {
                target.distance(&pixel.0) <= tolerance
            }))
        }
        
        fn advanced_color_detection(&self, image: &RgbaImage, target: &Color) -> Result<bool> {
            let tolerance_squared = (self.tolerance as u32 * 3).pow(2);
            let pixels: Vec<_> = image.pixels().collect();
            
            // Use more sophisticated detection with clustering
            let matches: Vec<_> = pixels.par_iter()
                .enumerate()
                .filter(|(_, pixel)| target.distance_squared(&pixel.0) <= tolerance_squared)
                .map(|(i, _)| i)
                .collect();
            
            if matches.is_empty() {
                return Ok(false);
            }
            
            // Check for clustering - reduces false positives
            let cluster_threshold = 5; // pixels
            let mut clusters = 0;
            let width = image.width() as usize;
            
            for &idx in &matches {
                let (x, y) = (idx % width, idx / width);
                let nearby_matches = matches.iter()
                    .filter(|&&other_idx| {
                        let (ox, oy) = (other_idx % width, other_idx / width);
                        let dx = (x as i32 - ox as i32).abs();
                        let dy = (y as i32 - oy as i32).abs();
                        dx <= cluster_threshold && dy <= cluster_threshold
                    })
                    .count();
                
                if nearby_matches >= 3 {
                    clusters += 1;
                    if clusters >= 2 {
                        return Ok(true);
                    }
                }
            }
            
            Ok(clusters > 0)
        }
        
        pub fn get_screenshot(&self, region: Region) -> Result<RgbaImage> {
            let cache_key = format!("{},{},{},{}", region.x, region.y, region.width, region.height);
            let now = Instant::now();
            
            // Check cache first
            {
                let cache = self.cache.read();
                if let Some((img, timestamp)) = cache.get(&cache_key) {
                    if now.duration_since(*timestamp) < self.cache_duration {
                        return Ok(img.clone());
                    }
                }
            }
            
            // Take new screenshot
            let screens = Screen::all()?;
            if screens.is_empty() {
                return Err(anyhow!("No screens found"));
            }
            
            let image = screens[0].capture_area(region.x, region.y, region.width, region.height)?;
            
            let rgba_image = RgbaImage::from_raw(
                region.width,
                region.height,
                image.to_vec()
            ).ok_or_else(|| anyhow!("Failed to create image"))?;
            
            // Update cache
            {
                let mut cache = self.cache.write();
                cache.insert(cache_key, (rgba_image.clone(), now));
                
                // Clean old entries
                cache.retain(|_, (_, timestamp)| {
                    now.duration_since(*timestamp) < Duration::from_secs(10)
                });
            }
            
            Ok(rgba_image)
        }
        
        pub fn take_full_screenshot(&self) -> Result<RgbaImage> {
            let screens = Screen::all()?;
            if screens.is_empty() {
                return Err(anyhow!("No screens found"));
            }
            
            let screen = &screens[0];
            let image = screen.capture()?;
            
            RgbaImage::from_raw(
                screen.display_info.width as u32,
                screen.display_info.height as u32,
                image.to_vec()
            ).ok_or_else(|| anyhow!("Failed to create full screenshot"))
        }
    }
}

// ===== INPUT MODULE =====
mod input {
    use super::*;
    use enigo::{Enigo, Settings};
    
    #[cfg(windows)]
    use winapi::um::winuser::{
        SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, MOUSEINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MapVirtualKeyW, MAPVK_VK_TO_VSC,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, GetCursorPos,
    };
    
    #[cfg(windows)]
    use winapi::shared::windef::POINT;
    
    pub struct RobloxInputController {
        enigo: Enigo,
        failsafe_enabled: bool,
        last_action_time: Instant,
    }
    
    impl RobloxInputController {
        pub fn new(failsafe_enabled: bool) -> Self {
            Self {
                enigo: Enigo::new(&Settings::default()).expect("Failed to create Enigo instance"),
                failsafe_enabled,
                last_action_time: Instant::now(),
            }
        }
        
        fn check_failsafe(&mut self) -> Result<()> {
            if !self.failsafe_enabled {
                return Ok(());
            }
            
            // Check mouse position failsafe (top-left corner)
            #[cfg(windows)]
            unsafe {
                let mut point = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut point) != 0 {
                    if point.x < 5 && point.y < 5 {
                        return Err(anyhow!("Failsafe triggered: mouse in top-left corner"));
                    }
                }
            }
            
            Ok(())
        }
        
        #[cfg(windows)]
        fn send_key_windows(&self, key_code: u8, key_up: bool) -> Result<()> {
            unsafe {
                let scan_code = MapVirtualKeyW(key_code as u32, MAPVK_VK_TO_VSC) as u16;
                let mut input = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: std::mem::zeroed(),
                };
                
                *input.u.ki_mut() = KEYBDINPUT {
                    wVk: key_code as u16,
                    wScan: scan_code,
                    dwFlags: KEYEVENTF_SCANCODE | if key_up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                };
                
                SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }
        
        #[cfg(windows)]
        fn send_mouse_click_windows(&self) -> Result<()> {
            unsafe {
                // Mouse down
                let mut input_down = INPUT {
                    type_: INPUT_MOUSE,
                    u: std::mem::zeroed(),
                };
                *input_down.u.mi_mut() = MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                };
                
                // Mouse up
                let mut input_up = INPUT {
                    type_: INPUT_MOUSE,
                    u: std::mem::zeroed(),
                };
                *input_up.u.mi_mut() = MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                
                SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);
                thread::sleep(Duration::from_millis(50));
                SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(())
        }
        
        pub fn click(&mut self) -> Result<()> {
            self.check_failsafe()?;
            
            #[cfg(windows)]
            {
                self.send_mouse_click_windows()?;
            }
            
            #[cfg(not(windows))]
            {
                use enigo::{Button, Direction, Mouse};
                self.enigo.button(Button::Left, Direction::Click)?;
            }
            
            self.last_action_time = Instant::now();
            Ok(())
        }
        
        pub fn press_key(&mut self, key: char) -> Result<()> {
            self.check_failsafe()?;
            
            let key_code = match key {
                '5' => 0x35, // VK_5
                '6' => 0x36, // VK_6
                _ => return Err(anyhow!("Unsupported key: {}", key)),
            };
            
            #[cfg(windows)]
            {
                // Use Windows API for better Roblox compatibility
                self.send_key_windows(key_code, false)?; // Key down
                thread::sleep(Duration::from_millis(50));
                self.send_key_windows(key_code, true)?;  // Key up
                thread::sleep(Duration::from_millis(50));
            }
            
            #[cfg(not(windows))]
            {
                // Fallback to enigo for non-Windows systems
                use enigo::{Key, Direction, Keyboard};
                self.enigo.key(Key::Other(key as u32), Direction::Press)?;
                thread::sleep(Duration::from_millis(50));
                self.enigo.key(Key::Other(key as u32), Direction::Release)?;
                thread::sleep(Duration::from_millis(50));
            }
            
            self.last_action_time = Instant::now();
            Ok(())
        }
        
        pub fn reset_rod(&mut self) -> Result<()> {
            self.press_key('5')?;
            thread::sleep(Duration::from_millis(200)); // Longer delay for Roblox
            self.press_key('5')?;
            thread::sleep(Duration::from_millis(200));
            Ok(())
        }
        
        pub fn eat_food(&mut self) -> Result<()> {
            self.click()?;
            thread::sleep(Duration::from_millis(200)); // Longer delays for Roblox
            self.press_key('6')?;
            thread::sleep(Duration::from_millis(200));
            self.click()?;
            thread::sleep(Duration::from_millis(200));
            self.press_key('5')?;
            thread::sleep(Duration::from_millis(200));
            Ok(())
        }
        
        pub fn get_last_action_time(&self) -> Instant {
            self.last_action_time
        }
    }
    
    // Type alias for compatibility
    pub type SafeInputController = RobloxInputController;
}

// ===== WEBHOOK MODULE =====
mod webhook {
    use super::*;
    use reqwest::Client;
    use std::collections::VecDeque;
    
    pub struct WebhookManager {
        client: Client,
        message_queue: Arc<Mutex<VecDeque<WebhookMessage>>>,
        config: Arc<RwLock<config::BotConfig>>,
        running: Arc<std::sync::atomic::AtomicBool>,
        last_screenshot_time: Arc<Mutex<Instant>>,
    }
    
    #[derive(Debug, Clone)]
    pub enum WebhookMessage {
        Text(String),
        Screenshot { message: String, image_data: Vec<u8> },
    }
    
    impl WebhookManager {
        pub fn new(config: Arc<RwLock<config::BotConfig>>) -> Self {
            Self {
                client: Client::new(),
                message_queue: Arc::new(Mutex::new(VecDeque::new())),
                config,
                running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                last_screenshot_time: Arc::new(Mutex::new(Instant::now())),
            }
        }
        
        pub fn start(&self) {
            self.running.store(true, std::sync::atomic::Ordering::Relaxed);
            let queue = self.message_queue.clone();
            let config = self.config.clone();
            let client = self.client.clone();
            let running = self.running.clone();
            let last_screenshot = self.last_screenshot_time.clone();
            
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    Self::webhook_worker(queue, config, client, running, last_screenshot).await;
                });
            });
        }
        
        pub fn stop(&self) {
            self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        
        pub fn send_message(&self, message: String) {
            if let Ok(mut queue) = self.message_queue.lock() {
                queue.push_back(WebhookMessage::Text(message));
                
                // Limit queue size
                while queue.len() > 50 {
                    queue.pop_front();
                }
            }
        }
        
        pub fn send_screenshot(&self, message: String, image_data: Vec<u8>) {
            if let Ok(mut queue) = self.message_queue.lock() {
                queue.push_back(WebhookMessage::Screenshot { message, image_data });
                
                // Limit queue size
                while queue.len() > 10 {
                    queue.pop_front();
                }
            }
        }
        
        pub fn check_periodic_screenshot(&self, detector: &detection::AdvancedDetector) {
            let config = self.config.read();
            if !config.screenshot_enabled || config.webhook_url.is_empty() {
                return;
            }
            
            let should_take = {
                let mut last_time = self.last_screenshot_time.lock().unwrap();
                let interval = Duration::from_secs(config.screenshot_interval_mins as u64 * 60);
                
                if last_time.elapsed() >= interval {
                    *last_time = Instant::now();
                    true
                } else {
                    false
                }
            };
            
            if should_take {
                if let Ok(screenshot) = detector.take_full_screenshot() {
                    let mut image_data = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut image_data);
                    if image::DynamicImage::ImageRgba8(screenshot).write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                        self.send_screenshot("📸 Periodic Screenshot".to_string(), image_data);
                    }
                }
            }
        }
        
        async fn webhook_worker(
            queue: Arc<Mutex<VecDeque<WebhookMessage>>>,
            config: Arc<RwLock<config::BotConfig>>,
            client: Client,
            running: Arc<std::sync::atomic::AtomicBool>,
            _last_screenshot: Arc<Mutex<Instant>>,
        ) {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let webhook_url = {
                    let cfg = config.read();
                    if cfg.webhook_url.is_empty() {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    cfg.webhook_url.clone()
                };
                
                let messages = {
                    let mut q = queue.lock().unwrap();
                    let mut batch = Vec::new();
                    for _ in 0..5 {  // Process up to 5 messages at once
                        if let Some(msg) = q.pop_front() {
                            batch.push(msg);
                        } else {
                            break;
                        }
                    }
                    batch
                };
                
                for message in messages {
                    match message {
                        WebhookMessage::Text(text) => {
                            let payload = serde_json::json!({ "content": text });
                            let _ = client.post(&webhook_url)
                                .json(&payload)
                                .send()
                                .await;
                        }
                        WebhookMessage::Screenshot { message, image_data } => {
                            let form = reqwest::multipart::Form::new()
                                .text("content", message)
                                .part("file", reqwest::multipart::Part::bytes(image_data)
                                    .file_name("screenshot.jpg")
                                    .mime_str("image/jpeg").unwrap());
                            
                            let _ = client.post(&webhook_url)
                                .multipart(form)
                                .send()
                                .await;
                        }
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

// ===== OCR MODULE =====
mod ocr {
    use super::*;
    use rusty_tesseract::{Image as TessImage, Args};
    use image::{RgbaImage, GrayImage, Luma};
    
    pub struct EnhancedOCRHandler {
        cache: HashMap<String, (Option<u32>, Instant)>,
    }
    
    impl EnhancedOCRHandler {
        pub fn new() -> Result<Self> {
            Ok(Self {
                cache: HashMap::new(),
            })
        }
        
        pub fn read_hunger(&mut self, image: &RgbaImage) -> Result<Option<u32>> {
            // Create cache key from image hash
            let cache_key = format!("{:?}", image.pixels().take(10).collect::<Vec<_>>());
            
            // Check cache first
            if let Some((cached_result, timestamp)) = self.cache.get(&cache_key) {
                if timestamp.elapsed() < Duration::from_secs(2) {
                    return Ok(*cached_result);
                }
            }
            
            let result = self.perform_ocr(image)?;
            
            // Cache the result
            self.cache.insert(cache_key, (result, Instant::now()));
            
            // Clean old cache entries
            let now = Instant::now();
            self.cache.retain(|_, (_, timestamp)| now.duration_since(*timestamp) < Duration::from_secs(10));
            
            Ok(result)
        }
        
        fn perform_ocr(&self, image: &RgbaImage) -> Result<Option<u32>> {
            // Simple preprocessing - just convert to grayscale and threshold
            let gray = GrayImage::from_fn(image.width(), image.height(), |x, y| {
                let pixel = image.get_pixel(x, y);
                let gray_value = ((pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3) as u8;
                Luma([gray_value])
            });
            
            let binary = GrayImage::from_fn(gray.width(), gray.height(), |x, y| {
                let pixel = gray.get_pixel(x, y);
                if pixel[0] > 128 { Luma([255]) } else { Luma([0]) }
            });
            
            // Save to temporary file for rusty-tesseract
            let temp_path = std::env::temp_dir().join(format!("hunger_ocr_{}.png", chrono::Utc::now().timestamp_millis()));
            binary.save(&temp_path)?;
            
            // Simple Tesseract config
            let mut config_variables = HashMap::new();
            config_variables.insert("tessedit_char_whitelist".to_string(), "0123456789%".to_string());
            
            let args = Args {
                lang: "eng".to_string(),
                dpi: Some(150),
                psm: Some(8),
                oem: Some(3),
                config_variables,
            };
            
            // Run OCR once
            let result = if let Ok(image_tess) = TessImage::from_path(&temp_path) {
                if let Ok(output) = rusty_tesseract::image_to_string(&image_tess, &args) {
                    self.parse_hunger_text(&output)
                } else {
                    None
                }
            } else {
                None
            };
            
            // Clean up temp file
            std::fs::remove_file(&temp_path).ok();
            
            Ok(result)
        }
        
        fn to_grayscale_enhanced(&self, image: &RgbaImage) -> GrayImage {
            GrayImage::from_fn(image.width(), image.height(), |x, y| {
                let pixel = image.get_pixel(x, y);
                // Weighted grayscale conversion for better text recognition
                let gray_value = (0.299 * pixel[0] as f32 + 
                                 0.587 * pixel[1] as f32 + 
                                 0.114 * pixel[2] as f32) as u8;
                Luma([gray_value])
            })
        }
        
        fn apply_adaptive_threshold(&self, gray: &GrayImage) -> GrayImage {
            let threshold = self.calculate_otsu_threshold(gray);
            
            GrayImage::from_fn(gray.width(), gray.height(), |x, y| {
                let pixel = gray.get_pixel(x, y);
                if pixel[0] > threshold { Luma([255]) } else { Luma([0]) }
            })
        }
        
        fn calculate_otsu_threshold(&self, image: &GrayImage) -> u8 {
            let mut histogram = [0u32; 256];
            
            // Build histogram
            for pixel in image.pixels() {
                histogram[pixel[0] as usize] += 1;
            }
            
            let total_pixels = image.width() * image.height();
            let mut sum = 0u64;
            
            for i in 0..256 {
                sum += i as u64 * histogram[i] as u64;
            }
            
            let mut sum_background = 0u64;
            let mut weight_background = 0u32;
            let mut max_variance = 0.0;
            let mut best_threshold = 0u8;
            
            for threshold in 0..256 {
                weight_background += histogram[threshold];
                if weight_background == 0 { continue; }
                
                let weight_foreground = total_pixels - weight_background;
                if weight_foreground == 0 { break; }
                
                sum_background += threshold as u64 * histogram[threshold] as u64;
                
                let mean_background = sum_background as f64 / weight_background as f64;
                let mean_foreground = (sum - sum_background) as f64 / weight_foreground as f64;
                
                let variance = weight_background as f64 * weight_foreground as f64 * 
                              (mean_background - mean_foreground).powi(2);
                
                if variance > max_variance {
                    max_variance = variance;
                    best_threshold = threshold as u8;
                }
            }
            
            best_threshold
        }
        
        fn noise_reduction(&self, image: &GrayImage) -> GrayImage {
            // Simple median filter for noise reduction
            let width = image.width();
            let height = image.height();
            
            GrayImage::from_fn(width, height, |x, y| {
                let mut neighbors = Vec::new();
                
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = (x as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let ny = (y as i32 + dy).max(0).min(height as i32 - 1) as u32;
                        neighbors.push(image.get_pixel(nx, ny)[0]);
                    }
                }
                
                neighbors.sort_unstable();
                Luma([neighbors[4]]) // Median of 9 values
            })
        }
        
        fn parse_hunger_text(&self, text: &str) -> Option<u32> {
            // Simple parsing - just find the first number
            let cleaned = text.trim().replace('%', "");
            
            // Try direct parsing
            if let Ok(value) = cleaned.parse::<u32>() {
                if value <= 999 {  // Reasonable upper limit
                    return Some(value);
                }
            }
            
            // Find any numbers in the text
            let numbers: Vec<u32> = cleaned
                .split_whitespace()
                .filter_map(|s| {
                    s.chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .ok()
                })
                .filter(|&n| n <= 999)
                .collect();
            
            numbers.first().copied()
        }
    }
}

// ===== BOT MODULE =====
mod bot {
    use super::*;
    use detection::{AdvancedDetector, Color};
    use input::RobloxInputController;
    use ocr::EnhancedOCRHandler;
    use webhook::WebhookManager;
    use config::{BotConfig, LifetimeStats};
    
    #[derive(Debug, Clone)]
    pub struct BotState {
        pub running: bool,
        pub paused: bool,
        pub fish_count: u64,
        pub last_hunger: Option<u32>,
        pub start_time: Option<Instant>,
        pub status: String,
        pub current_phase: FishingPhase,
        pub errors_count: u32,
        pub uptime_percentage: f32,
        pub fish_per_hour: f32,
        pub session_best_streak: u32,
        pub current_streak: u32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum FishingPhase {
        Idle,
        Casting,
        WaitingForBite,
        Reeling,
        Caught,
        Feeding,
        Error,
    }
    
    impl Default for BotState {
        fn default() -> Self {
            Self {
                running: false,
                paused: false,
                fish_count: 0,
                last_hunger: None,
                start_time: None,
                status: "Ready to start fishing! 🎣".to_string(),
                current_phase: FishingPhase::Idle,
                errors_count: 0,
                uptime_percentage: 100.0,
                fish_per_hour: 0.0,
                session_best_streak: 0,
                current_streak: 0,
            }
        }
    }
    
    pub struct AdvancedFishingBot {
        config: Arc<RwLock<BotConfig>>,
        state: Arc<RwLock<BotState>>,
        lifetime_stats: Arc<RwLock<LifetimeStats>>,
        detector: Arc<AdvancedDetector>,
        input: Arc<Mutex<RobloxInputController>>,
        webhook: Arc<WebhookManager>,
        ocr: Arc<Mutex<EnhancedOCRHandler>>,
        performance_monitor: Arc<Mutex<PerformanceMonitor>>,
    }
    
    #[derive(Debug)]
    struct PerformanceMonitor {
        total_operations: u64,
        successful_operations: u64,
        error_count: u32,
        last_error_time: Option<Instant>,
        operation_times: std::collections::VecDeque<Duration>,
    }
    
    impl PerformanceMonitor {
        fn new() -> Self {
            Self {
                total_operations: 0,
                successful_operations: 0,
                error_count: 0,
                last_error_time: None,
                operation_times: std::collections::VecDeque::new(),
            }
        }
        
        fn record_operation(&mut self, duration: Duration, success: bool) {
            self.total_operations += 1;
            if success {
                self.successful_operations += 1;
            } else {
                self.error_count += 1;
                self.last_error_time = Some(Instant::now());
            }
            
            self.operation_times.push_back(duration);
            while self.operation_times.len() > 100 {
                self.operation_times.pop_front();
            }
        }
        
        fn get_success_rate(&self) -> f32 {
            if self.total_operations == 0 {
                return 100.0;
            }
            (self.successful_operations as f32 / self.total_operations as f32) * 100.0
        }
        
        fn get_average_operation_time(&self) -> Duration {
            if self.operation_times.is_empty() {
                return Duration::from_secs(0);
            }
            
            let total: Duration = self.operation_times.iter().sum();
            total / self.operation_times.len() as u32
        }
    }
    
    impl AdvancedFishingBot {
        pub fn new(config: BotConfig, lifetime_stats: LifetimeStats) -> Self {
            let config_arc = Arc::new(RwLock::new(config.clone()));
            let detector = Arc::new(AdvancedDetector::new(
                config.detection_interval_ms,
                config.color_tolerance,
                config.advanced_detection
            ));
            let webhook = Arc::new(WebhookManager::new(config_arc.clone()));
            
            Self {
                config: config_arc,
                state: Arc::new(RwLock::new(BotState::default())),
                lifetime_stats: Arc::new(RwLock::new(lifetime_stats)),
                detector,
                input: Arc::new(Mutex::new(RobloxInputController::new(config.failsafe_enabled))),
                webhook,
                ocr: Arc::new(Mutex::new(EnhancedOCRHandler::new().unwrap_or_else(|_| 
                    EnhancedOCRHandler::new().unwrap()))),
                performance_monitor: Arc::new(Mutex::new(PerformanceMonitor::new())),
            }
        }
        
        pub fn start(&self) {
            let mut state = self.state.write();
            if state.running {
                return;
            }
            
            state.running = true;
            state.paused = false;
            state.fish_count = 0;
            state.start_time = Some(Instant::now());
            state.status = "🚀 Starting advanced fishing bot...".to_string();
            state.current_phase = FishingPhase::Idle;
            state.errors_count = 0;
            state.current_streak = 0;
            drop(state);
            
            // Start webhook manager
            self.webhook.start();
            
            // Send startup notification
            self.webhook.send_message("🎣 Advanced Fishing Bot Started! Beginning automated fishing session...".to_string());
            
            // Run bot in separate thread
            let bot_clone = self.clone();
            thread::spawn(move || {
                bot_clone.run_loop();
            });
        }
        
        pub fn stop(&self) {
            let mut state = self.state.write();
            state.running = false;
            state.current_phase = FishingPhase::Idle;
            state.status = "🛑 Bot stopped".to_string();
            
            if let Some(start_time) = state.start_time {
                let runtime = start_time.elapsed().as_secs();
                let session_fish = state.fish_count;
                drop(state);
                
                let mut stats = self.lifetime_stats.write();
                stats.add_runtime(runtime);
                stats.complete_session(session_fish);
                drop(stats);
                
                // Send session summary
                self.webhook.send_message(format!(
                    "📊 Session Complete!\n🐟 Fish Caught: {}\n⏱️ Runtime: {}h {}m\n🎯 Best Streak: {}",
                    session_fish,
                    runtime / 3600,
                    (runtime % 3600) / 60,
                    self.state.read().session_best_streak
                ));
            }
            
            self.webhook.stop();
        }
        
        pub fn pause(&self) {
            let mut state = self.state.write();
            state.paused = !state.paused;
            state.status = if state.paused { 
                "⏸️ Bot paused".to_string() 
            } else { 
                "▶️ Bot resumed".to_string() 
            };
            
            let message = if state.paused { "⏸️ Bot Paused" } else { "▶️ Bot Resumed" };
            self.webhook.send_message(message.to_string());
        }
        
        pub fn get_state(&self) -> BotState {
            self.state.read().clone()
        }
        
        pub fn get_lifetime_stats(&self) -> LifetimeStats {
            self.lifetime_stats.read().clone()
        }
        
        pub fn get_performance_stats(&self) -> (f32, Duration, u32) {
            let monitor = self.performance_monitor.lock().unwrap();
            (
                monitor.get_success_rate(),
                monitor.get_average_operation_time(),
                monitor.error_count,
            )
        }
        
        fn run_loop(&self) {
            self.update_status("🔧 Initializing bot systems...");
            self.update_phase(FishingPhase::Idle);
            
            thread::sleep(Duration::from_millis(self.config.read().startup_delay_ms));
            
            // Initialize rod state
            self.update_status("🎣 Preparing fishing rod...");
            if let Ok(mut input) = self.input.lock() {
                input.reset_rod().ok();
            }
            
            // Send startup screenshot
            if self.config.read().screenshot_enabled {
                if let Ok(screenshot) = self.detector.take_full_screenshot() {
                    let mut image_data = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut image_data);
                    if image::DynamicImage::ImageRgba8(screenshot)
                        .write_to(&mut cursor, image::ImageFormat::Jpeg)
                        .is_ok() {
                        self.webhook.send_screenshot("🚀 Bot Started - Ready to Fish!".to_string(), image_data);
                    }
                }
            }
            
            self.update_status("🌊 Bot active! Starting fishing sequence...");
            
            let mut consecutive_errors = 0;
            let max_consecutive_errors = 5;
            
            while self.state.read().running {
                if self.state.read().paused {
                    self.update_status("⏸️ Bot paused - Waiting for resume...");
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
                
                let operation_start = Instant::now();
                let success = match self.fish_once() {
                    Ok(caught) => {
                        consecutive_errors = 0;
                        if caught {
                            self.handle_successful_catch();
                        }
                        true
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        self.handle_error(&e, consecutive_errors);
                        
                        if consecutive_errors >= max_consecutive_errors {
                            self.update_status("❌ Too many consecutive errors - Stopping for safety");
                            break;
                        }
                        false
                    }
                };
                
                // Record performance
                let mut monitor = self.performance_monitor.lock().unwrap();
                monitor.record_operation(operation_start.elapsed(), success);
                drop(monitor);
                
                // Update statistics
                self.update_runtime_stats();
                
                // Check for periodic screenshot
                self.webhook.check_periodic_screenshot(&self.detector);
                
                // Brief pause between cycles
                thread::sleep(Duration::from_millis(50));
            }
            
            self.webhook.stop();
            self.update_status("🏁 Fishing session completed");
        }
        
        fn fish_once(&self) -> Result<bool> {
            // Cast rod
            self.update_phase(FishingPhase::Casting);
            self.update_status("🎯 Casting fishing line...");
            
            if let Ok(mut input) = self.input.lock() {
                input.click()?;
            }
            thread::sleep(Duration::from_millis(100));
            
            // Wait for bite
            self.update_phase(FishingPhase::WaitingForBite);
            let bite_detected = self.wait_for_bite()?;
            
            if !bite_detected {
                return Ok(false); // Timeout, try again
            }
            
            // Reel in fish
            self.update_phase(FishingPhase::Reeling);
            let caught = self.reel_in_fish()?;
            
            if caught {
                self.update_phase(FishingPhase::Caught);
                return Ok(true);
            }
            
            Ok(false)
        }
        
        fn wait_for_bite(&self) -> Result<bool> {
            let timeout = self.config.read().calculate_max_bite_time();
            let start_time = Instant::now();
            
            self.update_status(&format!("🎣 Waiting for fish bite... (Timeout: {:.0}s)", timeout.as_secs_f32()));
            
            while self.state.read().running && !self.state.read().paused {
                if start_time.elapsed() > timeout {
                    self.update_status("⏱️ No bite detected - Recasting...");
                    return Ok(false);
                }
                
                if self.detector.detect_color(
                    self.config.read().red_region,
                    &Color::RED_EXCLAMATION
                )? {
                    self.update_status("🎯 Fish bite detected! Reeling in...");
                    return Ok(true);
                }
                
                thread::sleep(Duration::from_millis(self.config.read().detection_interval_ms));
            }
            
            Ok(false)
        }
        
        fn reel_in_fish(&self) -> Result<bool> {
            let start_time = Instant::now();
            let max_duration = Duration::from_millis(self.config.read().max_fishing_timeout_ms);
            
            while self.state.read().running && !self.state.read().paused {
                if start_time.elapsed() > max_duration {
                    self.update_status("⏱️ Reeling timeout - Fish got away...");
                    return Ok(false);
                }
                
                // Auto-click
                if let Ok(mut input) = self.input.lock() {
                    input.click()?;
                }
                
                // Check if fish is caught
                if self.detector.detect_color(
                    self.config.read().yellow_region,
                    &Color::YELLOW_CAUGHT
                )? {
                    self.update_status("🎉 Fish successfully caught!");
                    return Ok(true);
                }
                
                thread::sleep(Duration::from_millis(self.config.read().autoclick_interval_ms));
            }
            
            Ok(false)
        }
        
        fn handle_successful_catch(&self) {
            // Reset rod
            if let Ok(mut input) = self.input.lock() {
                input.reset_rod().ok();
            }
            
            // Update counts
            let mut state = self.state.write();
            state.fish_count += 1;
            state.current_streak += 1;
            
            if state.current_streak > state.session_best_streak {
                state.session_best_streak = state.current_streak;
            }
            
            let fish_count = state.fish_count;
            drop(state);
            
            // Update lifetime stats
            let mut stats = self.lifetime_stats.write();
            stats.add_fish(1);
            drop(stats);
            
            self.update_status(&format!("🐟 Fish #{} caught! Current streak: {}", 
                fish_count, self.state.read().current_streak));
            
            // Send milestone notifications
            if fish_count % 10 == 0 {
                self.webhook.send_message(format!("🎉 Milestone Reached! {} fish caught this session!", fish_count));
            }
            
            // Check if need to feed
            if fish_count % self.config.read().fish_per_feed as u64 == 0 {
                self.check_and_feed();
            }
        }
        
        fn check_and_feed(&self) {
            self.update_phase(FishingPhase::Feeding);
            self.update_status("🍖 Checking hunger level...");
            
            let hunger_region = self.config.read().hunger_region;
            if let Ok(screenshot) = self.detector.get_screenshot(hunger_region) {
                let mut ocr = self.ocr.lock().unwrap();
                let hunger = ocr.read_hunger(&screenshot).unwrap_or(None);
                
                let mut state = self.state.write();
                state.last_hunger = hunger;
                drop(state);
                
                if let Some(h) = hunger {
                    if h < 100 {
                        self.update_status(&format!("🍖 Hunger at {}% - Feeding character...", h));
                        
                        if let Ok(mut input) = self.input.lock() {
                            input.eat_food().ok();
                        }
                        
                        // Update feed count
                        let mut stats = self.lifetime_stats.write();
                        stats.add_feed();
                        drop(stats);
                        
                        self.webhook.send_message(format!("🍖 Fed character (Hunger was {}%)", h));
                        self.update_status("✅ Successfully fed character!");
                    } else {
                        self.update_status(&format!("✅ Hunger at {}% - No feeding needed", h));
                    }
                } else {
                    // OCR failed, feed anyway to be safe
                    self.update_status("⚠️ Could not read hunger - Feeding to be safe...");
                    if let Ok(mut input) = self.input.lock() {
                        input.eat_food().ok();
                    }
                    self.webhook.send_message("⚠️ OCR failed - Fed character as safety measure".to_string());
                }
            }
        }
        
        fn handle_error(&self, error: &anyhow::Error, consecutive_count: u32) {
            self.update_phase(FishingPhase::Error);
            
            let mut state = self.state.write();
            state.errors_count += 1;
            state.current_streak = 0; // Reset streak on error
            drop(state);
            
            let error_msg = format!("⚠️ Error #{}: {} (Consecutive: {})", 
                self.state.read().errors_count, error, consecutive_count);
            
            self.update_status(&error_msg);
            
            // Send error notification for critical errors
            if consecutive_count >= 3 {
                self.webhook.send_message(format!("🚨 Critical Error Alert: {}", error_msg));
            }
            
            // Recovery delay
            let delay = std::cmp::min(1000 * consecutive_count as u64, 5000);
            thread::sleep(Duration::from_millis(delay));
        }
        
        fn update_runtime_stats(&self) {
            let mut state = self.state.write();
            
            if let Some(start_time) = state.start_time {
                let elapsed = start_time.elapsed();
                let hours = elapsed.as_secs_f32() / 3600.0;
                
                if hours > 0.0 {
                    state.fish_per_hour = state.fish_count as f32 / hours;
                }
                
                // Calculate uptime percentage (simplified)
                let total_time = elapsed.as_secs_f32();
                let error_time = state.errors_count as f32 * 2.0; // Assume 2 seconds per error
                state.uptime_percentage = ((total_time - error_time) / total_time * 100.0).max(0.0);
            }
        }
        
        fn update_status(&self, status: &str) {
            let mut state = self.state.write();
            state.status = status.to_string();
        }
        
        fn update_phase(&self, phase: FishingPhase) {
            let mut state = self.state.write();
            state.current_phase = phase;
        }
    }
    
    impl Clone for AdvancedFishingBot {
        fn clone(&self) -> Self {
            Self {
                config: self.config.clone(),
                state: self.state.clone(),
                lifetime_stats: self.lifetime_stats.clone(),
                detector: self.detector.clone(),
                input: Arc::new(Mutex::new(RobloxInputController::new(
                    self.config.read().failsafe_enabled
                ))),
                webhook: self.webhook.clone(),
                ocr: Arc::new(Mutex::new(EnhancedOCRHandler::new().unwrap_or_else(|_| 
                    EnhancedOCRHandler::new().unwrap()))),
                performance_monitor: self.performance_monitor.clone(),
            }
        }
    }
}

// ===== UI MODULE =====
mod ui {
    use super::*;
    use egui::*;
    use config::{BotConfig, LifetimeStats, Region};
    use bot::AdvancedFishingBot;
    
    pub struct AdvancedFishingBotApp {
        bot: AdvancedFishingBot,
        config: BotConfig,
        show_settings: bool,
        show_advanced_stats: bool,
        status_messages: Vec<(chrono::DateTime<chrono::Local>, String)>,
        last_update: Instant,
        last_status: String,
        resolution_presets: HashMap<String, (String, Region, Region, Region)>,
        window_size: egui::Vec2,
        scale_factor: f32,
    }
    
    impl AdvancedFishingBotApp {
        fn add_scaled_space(&self, ui: &mut Ui, base_space: f32) {
            ui.add_space(base_space * self.scale_factor);
        }
        
        fn scaled_font_size(&self, base_size: f32) -> f32 {
            (base_size * self.scale_factor).max(8.0).min(32.0)
        }
        
        fn scaled_button_size(&self, base_width: f32, base_height: f32) -> egui::Vec2 {
            egui::vec2(
                (base_width * self.scale_factor).max(80.0),
                (base_height * self.scale_factor).max(30.0)
            )
        }
        
        fn render_header(&mut self, ui: &mut Ui) {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("🎣 Arcane Odyssey Fish Bot")
                    .size(self.scaled_font_size(24.0))
                    .strong()
                    .color(Color32::from_rgb(100, 200, 255)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut self.config.always_on_top, "📌 Always on Top");
                });
            });
            
            self.add_scaled_space(ui, 4.0);
            ui.label(RichText::new("⚡ Rust Edition")
                .size(self.scaled_font_size(12.0))
                .color(Color32::from_rgb(150, 255, 150)));
            ui.separator();
        }
        
        fn render_bottom_buttons(&mut self, ui: &mut Ui) {
            // Calculate if buttons can fit horizontally
            let available_width = ui.available_width();
            let min_button_width = 180.0 * self.scale_factor; // Minimum readable width
            let button_height = self.scaled_button_size(40.0, 40.0).y;
            let spacing = 10.0 * self.scale_factor;
            
            let total_width_needed = (min_button_width * 2.0) + spacing;
            
            if available_width >= total_width_needed {
                // Horizontal layout when there's enough space
                let button_width = (available_width - spacing) / 2.0;
                ui.horizontal(|ui| {
                    if ui.add_sized([button_width, button_height], 
                        Button::new(RichText::new("⚙️ Advanced Settings")
                            .size(self.scaled_font_size(12.0)))
                    ).clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    
                    if ui.add_sized([button_width, button_height], 
                        Button::new(RichText::new("📊 Advanced Statistics")
                            .size(self.scaled_font_size(12.0)))
                    ).clicked() {
                        self.show_advanced_stats = !self.show_advanced_stats;
                    }
                });
            } else {
                // Vertical layout when window is too narrow
                let button_width = available_width;
                
                if ui.add_sized([button_width, button_height], 
                    Button::new(RichText::new("⚙️ Advanced Settings")
                        .size(self.scaled_font_size(12.0)))
                ).clicked() {
                    self.show_settings = !self.show_settings;
                }
                
                ui.add_space(spacing);
                
                if ui.add_sized([button_width, button_height], 
                    Button::new(RichText::new("📊 Advanced Statistics")
                        .size(self.scaled_font_size(12.0)))
                ).clicked() {
                    self.show_advanced_stats = !self.show_advanced_stats;
                }
            }
        }
        pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
            // Enhanced styling
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = vec2(8.0, 6.0);
            style.spacing.window_margin = egui::style::Margin::same(16.0);
            style.spacing.button_padding = vec2(12.0, 8.0);
            style.spacing.indent = 20.0;
            
            // Enhanced dark theme
            style.visuals = Visuals::dark();
            style.visuals.window_fill = Color32::from_rgb(15, 15, 20);
            style.visuals.panel_fill = Color32::from_rgb(25, 25, 35);
            style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 40, 55);
            style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 130, 180);
            style.visuals.widgets.active.bg_fill = Color32::from_rgb(100, 150, 200);
            style.visuals.selection.bg_fill = Color32::from_rgb(100, 150, 200);
            
            cc.egui_ctx.set_style(style);
            
            // Load configuration and statistics
            let config = BotConfig::load().unwrap_or_default();
            let lifetime_stats = LifetimeStats::load().unwrap_or_default();
            
            // Initialize resolution presets
            let mut presets = HashMap::new();
            presets.insert("3440x1440".to_string(), (
                "3440x1440 Ultrawide".to_string(),
                Region { x: 1321, y: 99, width: 768, height: 546 },
                Region { x: 3097, y: 1234, width: 342, height: 205 },
                Region { x: 274, y: 1301, width: 43, height: 36 },
            ));
            presets.insert("1920x1080".to_string(), (
                "1920x1080 Standard".to_string(),
                Region { x: 598, y: 29, width: 901, height: 477 },
                Region { x: 1649, y: 632, width: 270, height: 447 },
                Region { x: 212, y: 984, width: 21, height: 18 },
            ));
            
            Self {
                bot: AdvancedFishingBot::new(config.clone(), lifetime_stats),
                config,
                show_settings: false,
                show_advanced_stats: false,
                status_messages: vec![],
                last_update: Instant::now(),
                last_status: String::new(),
                resolution_presets: presets,
                window_size: egui::Vec2::new(900.0, 800.0),
                scale_factor: 1.0,
            }
        }
        
        fn update_status(&mut self, message: String) {
            let now = Local::now();
            let timestamped_message = format!("[{:02}:{:02}:{:02}] {}", 
                now.hour(), now.minute(), now.second(), message);
            self.status_messages.push((now, timestamped_message));
            
            if self.status_messages.len() > 100 {
                self.status_messages.remove(0);
            }
        }
    }
    
    impl eframe::App for AdvancedFishingBotApp {
        fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
            // Update window size and scale factor
            let current_size = ctx.screen_rect().size();
            if (current_size - self.window_size).length() > 10.0 {
                self.window_size = current_size;
                self.scale_factor = (current_size.x / 900.0).min(current_size.y / 800.0).max(0.5).min(2.0);
            }
            
            // Window properties
            if self.config.always_on_top {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            }
            
            // Update status from bot
            if self.last_update.elapsed() > Duration::from_millis(100) {
                let state = self.bot.get_state();
                if !state.status.is_empty() && state.status != self.last_status {
                    self.update_status(state.status.clone());
                    self.last_status = state.status;
                }
                self.last_update = Instant::now();
            }
            
            CentralPanel::default().show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        // Set dynamic spacing based on scale
                        ui.spacing_mut().item_spacing = egui::vec2(8.0 * self.scale_factor, 6.0 * self.scale_factor);
                        ui.spacing_mut().button_padding = egui::vec2(8.0 * self.scale_factor, 6.0 * self.scale_factor);
                        
                        // Enhanced Header
                        self.render_header(ui);
                        self.add_scaled_space(ui, 8.0);
                        
                        // Enhanced Control Panel
                        self.render_control_panel(ui);
                        self.add_scaled_space(ui, 12.0);
                        
                        // Enhanced Statistics Panel
                        self.render_statistics_panel(ui);
                        self.add_scaled_space(ui, 12.0);
                        
                        // Performance Monitor
                        self.render_performance_panel(ui);
                        self.add_scaled_space(ui, 12.0);
                        
                        // Activity Monitor
                        self.render_activity_monitor(ui);
                        self.add_scaled_space(ui, 12.0);
                        
                        // Settings Panel - Now responsive
                        self.render_bottom_buttons(ui);
                    });
            });
            
            // Settings Window
            if self.show_settings {
                self.render_settings_window(ctx);
            }
            
            // Advanced Statistics Window
            if self.show_advanced_stats {
                self.render_advanced_stats_window(ctx);
            }
            
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        
        fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
            self.bot.stop();
            self.config.save().ok();
        }
    }
    
    impl AdvancedFishingBotApp {
        fn render_control_panel(&mut self, ui: &mut Ui) {
            Frame::none()
                .fill(Color32::from_rgb(25, 25, 35))
                .inner_margin(16.0)
                .rounding(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let state = self.bot.get_state();
                        
                        // Start Button
                        let start_enabled = !state.running;
                        if ui.add_enabled(start_enabled, 
                            Button::new("▶ Start Bot")
                                .fill(if start_enabled { Color32::from_rgb(50, 150, 50) } else { Color32::DARK_GRAY })
                                .min_size(vec2(120.0, 40.0))
                        ).clicked() {
                            self.bot.start();
                        }
                        
                        // Pause/Resume Button
                        if state.running {
                            let pause_text = if state.paused { "▶ Resume" } else { "⏸ Pause" };
                            let pause_color = if state.paused { Color32::from_rgb(50, 150, 50) } else { Color32::from_rgb(200, 150, 50) };
                            
                            if ui.add(Button::new(pause_text)
                                .fill(pause_color)
                                .min_size(vec2(120.0, 40.0))
                            ).clicked() {
                                self.bot.pause();
                            }
                            
                            // Stop Button
                            if ui.add(Button::new("⏹ Stop")
                                .fill(Color32::from_rgb(200, 50, 50))
                                .min_size(vec2(120.0, 40.0))
                            ).clicked() {
                                self.bot.stop();
                            }
                        } else {
                            // Disabled buttons when not running
                            ui.add_enabled(false, Button::new("⏸ Pause").fill(Color32::DARK_GRAY).min_size(vec2(120.0, 40.0)));
                            ui.add_enabled(false, Button::new("⏹ Stop").fill(Color32::DARK_GRAY).min_size(vec2(120.0, 40.0)));
                        }
                    });
                    
                    ui.add_space(8.0);
                    
                    // Current Phase Indicator
                    let state = self.bot.get_state();
                    let phase_text = match state.current_phase {
                        bot::FishingPhase::Idle => "🏠 Idle",
                        bot::FishingPhase::Casting => "🎯 Casting",
                        bot::FishingPhase::WaitingForBite => "⏳ Waiting for Bite",
                        bot::FishingPhase::Reeling => "🎣 Reeling In",
                        bot::FishingPhase::Caught => "🎉 Fish Caught!",
                        bot::FishingPhase::Feeding => "🍖 Feeding",
                        bot::FishingPhase::Error => "❌ Error State",
                    };
                    
                    let phase_color = match state.current_phase {
                        bot::FishingPhase::Idle => Color32::GRAY,
                        bot::FishingPhase::Casting => Color32::from_rgb(100, 150, 255),
                        bot::FishingPhase::WaitingForBite => Color32::from_rgb(255, 200, 100),
                        bot::FishingPhase::Reeling => Color32::from_rgb(255, 150, 100),
                        bot::FishingPhase::Caught => Color32::from_rgb(100, 255, 100),
                        bot::FishingPhase::Feeding => Color32::from_rgb(255, 100, 255),
                        bot::FishingPhase::Error => Color32::from_rgb(255, 100, 100),
                    };
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Current Phase:").strong());
                        ui.label(RichText::new(phase_text).color(phase_color).strong());
                    });
                });
        }
        
        fn render_statistics_panel(&mut self, ui: &mut Ui) {
            Frame::none()
                .fill(Color32::from_rgb(25, 25, 35))
                .inner_margin(16.0 * self.scale_factor)
                .rounding(8.0)
                .show(ui, |ui| {
                    ui.heading(RichText::new("📊 Session & Lifetime Statistics")
                        .size(self.scaled_font_size(16.0)));
                    ui.separator();
                    
                    Grid::new("stats_grid")
                        .num_columns(4)
                        .spacing([20.0 * self.scale_factor, 8.0 * self.scale_factor])
                        .show(ui, |ui| {
                            let state = self.bot.get_state();
                            let lifetime = self.bot.get_lifetime_stats();
                            
                            // Session Stats Row 1
                            ui.label(RichText::new("Session Fish:").strong());
                            ui.label(RichText::new(format!("🐟 {}", state.fish_count)).size(16.0).color(Color32::from_rgb(100, 200, 255)));
                            
                            ui.label(RichText::new("Current Streak:").strong());
                            ui.label(RichText::new(format!("🔥 {}", state.current_streak)).size(16.0).color(Color32::from_rgb(255, 150, 50)));
                            ui.end_row();
                            
                            // Session Stats Row 2
                            ui.label(RichText::new("Session Time:").strong());
                            let runtime = if let Some(start) = state.start_time {
                                let elapsed = start.elapsed();
                                format!("⏱️ {:02}:{:02}:{:02}", 
                                    elapsed.as_secs() / 3600,
                                    (elapsed.as_secs() % 3600) / 60,
                                    elapsed.as_secs() % 60
                                )
                            } else {
                                "⏱️ 00:00:00".to_string()
                            };
                            ui.label(RichText::new(runtime).size(16.0).color(Color32::from_rgb(150, 255, 150)));
                            
                            ui.label(RichText::new("Fish/Hour:").strong());
                            ui.label(RichText::new(format!("📈 {:.1}", state.fish_per_hour)).size(16.0).color(Color32::from_rgb(255, 200, 100)));
                            ui.end_row();
                            
                            // Separator
                            ui.separator();
                            ui.separator();
                            ui.separator();
                            ui.separator();
                            ui.end_row();
                            
                            // Lifetime Stats Row 1
                            ui.label(RichText::new("Total Fish Ever:").strong());
                            ui.label(RichText::new(format!("🏆 {}", lifetime.total_fish_caught)).size(16.0).color(Color32::from_rgb(255, 215, 0)));
                            
                            ui.label(RichText::new("Best Session:").strong());
                            ui.label(RichText::new(format!("🎯 {}", lifetime.best_session_fish)).size(16.0).color(Color32::from_rgb(255, 100, 255)));
                            ui.end_row();
                            
                            // Lifetime Stats Row 2
                            ui.label(RichText::new("Total Runtime:").strong());
                            ui.label(RichText::new(format!("📅 {}", lifetime.get_formatted_runtime())).size(16.0).color(Color32::from_rgb(100, 255, 255)));
                            
                            ui.label(RichText::new("Total Sessions:").strong());
                            ui.label(RichText::new(format!("🔄 {}", lifetime.sessions_completed)).size(16.0).color(Color32::from_rgb(200, 200, 255)));
                            ui.end_row();
                        });
                });
        }
        
        fn render_performance_panel(&mut self, ui: &mut Ui) {
            Frame::none()
                .fill(Color32::from_rgb(25, 25, 35))
                .inner_margin(16.0)
                .rounding(8.0)
                .show(ui, |ui| {
                    ui.heading("⚡ Performance Monitor");
                    ui.separator();
                    
                    let (success_rate, avg_time, error_count) = self.bot.get_performance_stats();
                    let state = self.bot.get_state();
                    
                    Grid::new("perf_grid")
                        .num_columns(4)
                        .spacing([20.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Success Rate:").strong());
                            let success_color = if success_rate > 95.0 {
                                Color32::from_rgb(100, 255, 100)
                            } else if success_rate > 85.0 {
                                Color32::from_rgb(255, 200, 100)
                            } else {
                                Color32::from_rgb(255, 100, 100)
                            };
                            ui.label(RichText::new(format!("✅ {:.1}%", success_rate)).color(success_color));
                            
                            ui.label(RichText::new("Uptime:").strong());
                            let uptime_color = if state.uptime_percentage > 95.0 {
                                Color32::from_rgb(100, 255, 100)
                            } else {
                                Color32::from_rgb(255, 200, 100)
                            };
                            ui.label(RichText::new(format!("📈 {:.1}%", state.uptime_percentage)).color(uptime_color));
                            ui.end_row();
                            
                            ui.label(RichText::new("Avg Operation:").strong());
                            ui.label(RichText::new(format!("⏱️ {:.1}ms", avg_time.as_millis())).color(Color32::from_rgb(150, 200, 255)));
                            
                            ui.label(RichText::new("Total Errors:").strong());
                            let error_color = if error_count == 0 {
                                Color32::from_rgb(100, 255, 100)
                            } else if error_count < 5 {
                                Color32::from_rgb(255, 200, 100)
                            } else {
                                Color32::from_rgb(255, 100, 100)
                            };
                            ui.label(RichText::new(format!("❌ {}", error_count)).color(error_color));
                            ui.end_row();
                        });
                });
        }
        
        fn render_activity_monitor(&mut self, ui: &mut Ui) {
            Frame::none()
                .fill(Color32::from_rgb(25, 25, 35))
                .inner_margin(16.0)
                .rounding(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("📋 Activity Monitor");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("🗑️ Clear").clicked() {
                                self.status_messages.clear();
                            }
                        });
                    });
                    ui.separator();
                    
                    ScrollArea::vertical()
                        .max_height(150.0)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for (_timestamp, message) in self.status_messages.iter().rev().take(20) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(message).family(FontFamily::Proportional));
                                });
                            }
                        });
                });
        }
        
        fn render_settings_window(&mut self, ctx: &Context) {
            Window::new("⚙️ Advanced Settings")
                .default_size([700.0, 600.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        // Basic Settings
                        CollapsingHeader::new("🎯 Detection Settings")
                            .default_open(true)
                            .show(ui, |ui| {
                                Grid::new("detection_settings")
                                    .num_columns(2)
                                    .spacing([20.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label("Color Tolerance:");
                                        ui.add(Slider::new(&mut self.config.color_tolerance, 1..=50).text("pixels"));
                                        ui.end_row();
                                        
                                        ui.label("Detection Interval:");
                                        ui.add(Slider::new(&mut self.config.detection_interval_ms, 10..=200).text("ms"));
                                        ui.end_row();
                                        
                                        ui.checkbox(&mut self.config.advanced_detection, "Advanced Detection (Reduces false positives)");
                                        ui.label("Uses clustering algorithms for better accuracy");
                                        ui.end_row();
                                    });
                            });
                        
                        // Fishing Settings
                        CollapsingHeader::new("🎣 Fishing Settings")
                            .default_open(true)
                            .show(ui, |ui| {
                                Grid::new("fishing_settings")
                                    .num_columns(2)
                                    .spacing([20.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label("Autoclick Interval:");
                                        ui.add(Slider::new(&mut self.config.autoclick_interval_ms, 10..=200).text("ms"));
                                        ui.end_row();
                                        
                                        ui.label("Fish Per Feed:");
                                        ui.add(Slider::new(&mut self.config.fish_per_feed, 1..=20));
                                        ui.end_row();
                                        
                                        ui.label("Rod Lure Value:");
                                        ui.add(Slider::new(&mut self.config.rod_lure_value, 0.1..=5.0).step_by(0.1));
                                        ui.end_row();
                                        
                                        ui.label("Bite Timeout:");
                                        ui.label(self.config.get_timeout_description());
                                        ui.end_row();
                                        
                                        ui.label("Max Fishing Timeout:");
                                        ui.add(Slider::new(&mut self.config.max_fishing_timeout_ms, 5000..=60000).text("ms"));
                                        ui.end_row();
                                    });
                            });
                        
                        // Safety Settings
                        CollapsingHeader::new("🛡️ Safety Settings")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.checkbox(&mut self.config.failsafe_enabled, "Enable Failsafe (Stop on mouse corner)");
                                ui.checkbox(&mut self.config.auto_save_enabled, "Auto-save Configuration");
                                
                                ui.horizontal(|ui| {
                                    ui.label("Startup Delay:");
                                    ui.add(Slider::new(&mut self.config.startup_delay_ms, 1000..=10000).text("ms"));
                                });
                            });
                        
                        // Discord Webhook
                        CollapsingHeader::new("📢 Discord Integration")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Webhook URL:");
                                    ui.add(TextEdit::singleline(&mut self.config.webhook_url).desired_width(400.0));
                                });
                                
                                ui.checkbox(&mut self.config.screenshot_enabled, "Enable Screenshots");
                                
                                ui.horizontal(|ui| {
                                    ui.label("Screenshot Interval:");
                                    ui.add(Slider::new(&mut self.config.screenshot_interval_mins, 1..=120).text("minutes"));
                                });
                            });
                        
                        // Resolution Presets
                        CollapsingHeader::new("🖥️ Resolution Presets")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Preset:");
                                    ComboBox::from_label("")
                                        .selected_text(&self.config.region_preset)
                                        .show_ui(ui, |ui| {
                                            for (key, (name, _, _, _)) in &self.resolution_presets {
                                                ui.selectable_value(&mut self.config.region_preset, key.clone(), name);
                                            }
                                        });
                                    
                                    if ui.button("Apply").clicked() {
                                        if let Some((_, red, yellow, hunger)) = self.resolution_presets.get(&self.config.region_preset) {
                                            self.config.red_region = *red;
                                            self.config.yellow_region = *yellow;
                                            self.config.hunger_region = *hunger;
                                        }
                                    }
                                });
                                
                                ui.label(format!("Red Region: ({}, {}) {}x{}", 
                                    self.config.red_region.x, self.config.red_region.y,
                                    self.config.red_region.width, self.config.red_region.height));
                                ui.label(format!("Yellow Region: ({}, {}) {}x{}", 
                                    self.config.yellow_region.x, self.config.yellow_region.y,
                                    self.config.yellow_region.width, self.config.yellow_region.height));
                                ui.label(format!("Hunger Region: ({}, {}) {}x{}", 
                                    self.config.hunger_region.x, self.config.hunger_region.y,
                                    self.config.hunger_region.width, self.config.hunger_region.height));
                            });
                        
                        ui.add_space(20.0);
                        
                        // Action Buttons
                        ui.horizontal(|ui| {
                            if ui.button("💾 Save Settings").clicked() {
                                if let Err(e) = self.config.save() {
                                    self.update_status(format!("❌ Failed to save settings: {}", e));
                                } else {
                                    self.update_status("✅ Settings saved successfully!".to_string());
                                    self.show_settings = false;
                                }
                            }
                            
                            if ui.button("🔄 Reset to Defaults").clicked() {
                                self.config = BotConfig::default();
                                self.update_status("🔄 Settings reset to defaults".to_string());
                            }
                            
                            if ui.button("❌ Cancel").clicked() {
                                self.config = BotConfig::load().unwrap_or_default();
                                self.show_settings = false;
                            }
                        });
                    });
                });
        }
        
        fn render_advanced_stats_window(&mut self, ctx: &Context) {
            Window::new("📊 Advanced Statistics")
                .default_size([600.0, 500.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    let lifetime = self.bot.get_lifetime_stats();
                    let state = self.bot.get_state();
                    
                    ui.heading("📈 Detailed Analytics");
                    ui.separator();
                    
                    Grid::new("advanced_stats")
                        .num_columns(2)
                        .spacing([40.0, 12.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Average Fish/Hour:").strong());
                            ui.label(format!("{:.2}", lifetime.average_fish_per_hour));
                            ui.end_row();
                            
                            ui.label(RichText::new("Total Feeds:").strong());
                            ui.label(format!("{}", lifetime.total_feeds));
                            ui.end_row();
                            
                            ui.label(RichText::new("Current Session Best:").strong());
                            ui.label(format!("{}", state.session_best_streak));
                            ui.end_row();
                            
                            ui.label(RichText::new("All-time Best Session:").strong());
                            ui.label(format!("{} fish", lifetime.best_session_fish));
                            ui.end_row();
                            
                            ui.label(RichText::new("System Uptime:").strong());
                            ui.label(format!("{:.1}%", state.uptime_percentage));
                            ui.end_row();
                            
                            ui.label(RichText::new("Error Count:").strong());
                            ui.label(format!("{}", state.errors_count));
                            ui.end_row();
                        });
                    
                    ui.add_space(20.0);
                    
                    if ui.button("🗑️ Reset All Statistics").clicked() {
                        // Note: This would require implementing a reset method
                        self.update_status("⚠️ Statistics reset not implemented yet".to_string());
                    }
                    
                    if ui.button("❌ Close").clicked() {
                        self.show_advanced_stats = false;
                    }
                });
        }
    }
}

// ===== MAIN FUNCTION =====
fn main() -> Result<()> {
    env_logger::init();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Arcane Odyssey Advanced Fishing Bot")
            .with_inner_size([900.0, 800.0])
            .with_min_inner_size([700.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    
    eframe::run_native(
        "Arcane Odyssey Advanced Fishing Bot",
        options,
        Box::new(|cc| Box::new(ui::AdvancedFishingBotApp::new(cc))),
    ).map_err(|e| anyhow!("Failed to run app: {}", e))
}

fn load_icon() -> egui::IconData {
    let size = 64;
    let mut pixels = vec![0u8; size * size * 4];
    
    // Create a more detailed fish icon
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            
            // Create fish shape with gradient
            let center_x = size as f32 / 2.0;
            let center_y = size as f32 / 2.0;
            let dist = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            
            if dist < 25.0 {
                // Fish body - blue gradient
                let intensity = (1.0 - dist / 25.0) * 255.0;
                pixels[idx] = (intensity * 0.2) as u8;     // R
                pixels[idx + 1] = (intensity * 0.6) as u8; // G
                pixels[idx + 2] = (intensity * 1.0) as u8; // B
                pixels[idx + 3] = 255;                     // A
            } else {
                // Transparent background
                pixels[idx + 3] = 0;
            }
        }
    }
    
    egui::IconData {
        rgba: pixels,
        width: size as u32,
        height: size as u32,
    }
}
                                