use crate::{detection::Calibration, storage::save_json};
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
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
    pub energy_capacity_feed_below: u32,
    pub observation_max_age_ms: u64,
    pub recovery_limit: u32,
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
            energy_capacity_feed_below: 0,
            observation_max_age_ms: 1000,
            recovery_limit: 2,
            calibration: None,
            rod_slot: 2,
            food_slot: 1,
            auto_feed_enabled: false,
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
        if self.auto_feed_enabled
            && (self.calibration.is_none() || self.energy_capacity_feed_below == 0)
        {
            return Err(anyhow!("Automatic feeding requires a calibrated profile and an absolute Energy capacity threshold above zero"));
        }
        if self.energy_capacity_feed_below > 100_000
            || self.observation_max_age_ms < 200
            || self.observation_max_age_ms > 5000
            || self.recovery_limit > 10
            || (self.auto_feed_enabled
                && (self.calibration.is_none() || self.energy_capacity_feed_below == 0))
            || self.rod_slot > 9
            || self.food_slot > 9
            || self.rod_slot == self.food_slot
            || self.feed_below_percent > 100
            || self.fish_per_feed == 0
            || self.detection_interval_ms < 10
            || self.detection_interval_ms > 500
            || self.autoclick_interval_ms > 2000
            || self.max_fishing_timeout_ms > 300_000
            || self.autoclick_interval_ms < 10
            || self.max_fishing_timeout_ms == 0
            || !self.rod_lure_value.is_finite()
            || self.rod_lure_value < 0.0
            || self.startup_delay_ms > 300_000
        {
            return Err(anyhow!("Invalid timing, lure value, or feeding interval"));
        }
        if u64::from(self.hunger_region.width) * u64::from(self.hunger_region.height) > 100_000 {
            return Err(anyhow!("Energy OCR region is too large"));
        }
        for region in [self.red_region, self.yellow_region, self.hunger_region] {
            if region.width == 0
                || region.height == 0
                || region.width > 16384
                || region.height > 16384
                || u64::from(region.width) * u64::from(region.height) > 16_000_000
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
                || u64::from(calibration.frame_width) * u64::from(calibration.frame_height)
                    > 16_000_000
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
                rod_selection_region(calibration, self.food_slot),
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
            let mut config: Self = serde_json::from_str(&contents)?;
            // Old auto-feed was based on the wrong Energy ratio. Require an explicit capacity threshold.
            if config.energy_capacity_feed_below == 0 {
                config.auto_feed_enabled = false;
            }
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

    pub fn bite_timeout_ms(&self) -> u64 {
        calculate_timeout_ms(self.rod_lure_value)
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

pub fn rod_selection_region(calibration: &Calibration, slot: u8) -> Region {
    let [x, y, width, height] = calibration.hotbar_first_slot;
    let index = if slot == 0 { 9 } else { u32::from(slot - 1) };
    Region {
        x: (x + index * calibration.hotbar_slot_stride) as i32,
        y: y as i32,
        width,
        height,
    }
}
