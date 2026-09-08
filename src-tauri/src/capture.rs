use crate::{
    config::{BotConfig, Region},
    detection::Calibration,
};
use anyhow::{anyhow, Result};
use image::RgbaImage;
use screenshots::Screen;
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

pub fn capture_config_region(region: Region, config: &BotConfig) -> Result<RgbaImage> {
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
pub fn normalize_capture(
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
    if left == 0
        && top == 0
        && right == captured.width()
        && bottom == captured.height()
        && captured.dimensions() == (region.width, region.height)
    {
        return captured;
    }
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

pub fn map_reference_region(
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

pub fn capture_frame(config: &BotConfig) -> Result<RgbaImage> {
    let calibration = config
        .calibration
        .as_ref()
        .ok_or_else(|| anyhow!("Select a calibrated profile before capturing"))?;
    capture_config_region(
        Region {
            x: 0,
            y: 0,
            width: calibration.frame_width,
            height: calibration.frame_height,
        },
        config,
    )
}

pub fn crop_frame(image: &RgbaImage, region: Region) -> RgbaImage {
    image::imageops::crop_imm(
        image,
        region.x as u32,
        region.y as u32,
        region.width,
        region.height,
    )
    .to_image()
}
