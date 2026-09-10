//! Read-only calibration geometry. Overlay and capture share rounding and bounds.
use crate::{
    capture::map_reference_region,
    config::{rod_selection_region, BotConfig, Region},
};
use anyhow::{anyhow, Result};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct OverlayRegion {
    pub name: String,
    pub color: &'static str,
    pub bounds: Region,
}
#[derive(Clone, Serialize)]
pub struct OverlayLayout {
    pub width: u32,
    pub height: u32,
    pub regions: Vec<OverlayRegion>,
}

pub fn layout(config: &BotConfig, width: u32, height: u32) -> Result<OverlayLayout> {
    config.validate()?;
    let calibration = config
        .calibration
        .as_ref()
        .ok_or_else(|| anyhow!("Select a calibrated screen profile first"))?;
    if width == 0
        || height == 0
        || ((width as f64 / height as f64)
            - (calibration.frame_width as f64 / calibration.frame_height as f64))
            .abs()
            > 0.02
    {
        return Err(anyhow!(
            "Display aspect ratio does not match this screenshot profile"
        ));
    }
    let regions = [
        ("Bite search".into(), "#fb7185", config.red_region),
        ("Catch heading".into(), "#facc15", config.yellow_region),
        ("Energy numbers".into(), "#c084fc", config.hunger_region),
        (
            format!("Rod slot {}", config.rod_slot),
            "#22d3ee",
            rod_selection_region(calibration, config.rod_slot),
        ),
        (
            format!("Food slot {}", config.food_slot),
            "#4ade80",
            rod_selection_region(calibration, config.food_slot),
        ),
    ]
    .into_iter()
    .map(|(name, color, region)| {
        let (x, y, width, height) = map_reference_region(region, calibration, width, height);
        OverlayRegion {
            name,
            color,
            bounds: Region {
                x,
                y,
                width,
                height,
            },
        }
    })
    .collect();
    Ok(OverlayLayout {
        width,
        height,
        regions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::macbook_profile;
    #[test]
    fn overlay_uses_capture_rounding_for_all_regions() {
        let config = macbook_profile();
        let calibration = config.calibration.as_ref().unwrap();
        for (width, height) in [(1512, 982), (3024, 1964)] {
            let drawing = layout(&config, width, height).unwrap();
            let source = [
                config.red_region,
                config.yellow_region,
                config.hunger_region,
                rod_selection_region(calibration, 4),
                rod_selection_region(calibration, 5),
            ];
            assert_eq!(drawing.regions.len(), source.len());
            for (item, source) in drawing.regions.iter().zip(source) {
                let bounds = item.bounds;
                assert_eq!(
                    (bounds.x, bounds.y, bounds.width, bounds.height),
                    map_reference_region(source, calibration, width, height)
                );
                assert!(
                    bounds.x as u32 + bounds.width <= width
                        && bounds.y as u32 + bounds.height <= height
                );
            }
        }
    }
    #[test]
    fn overlay_rejects_wrong_display_or_uncalibrated_profile() {
        assert!(layout(&macbook_profile(), 1920, 1080).is_err());
        assert!(layout(&macbook_profile(), 0, 0).is_err());
        assert!(layout(&BotConfig::default(), 3024, 1964).is_err());
    }
}
