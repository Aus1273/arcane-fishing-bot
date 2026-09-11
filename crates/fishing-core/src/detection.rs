//! Screenshot-calibrated detection; all dimensions are reference-image pixels.
use image::{imageops, GrayImage, Luma, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibration {
    pub hotbar_first_slot: [u32; 4],
    pub hotbar_slot_stride: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub bite_rgb: [u8; 3],
    pub bite_tolerance: u8,
    pub bite_min_pixels: u32,
    pub catch_rgb: [u8; 3],
    pub catch_tolerance: u8,
    pub catch_min_pixels: u32,
}

fn matches(pixel: &[u8], target: [u8; 3], tolerance: u8) -> bool {
    (0..3)
        .map(|i| pixel[i].abs_diff(target[i]) as u32)
        .sum::<u32>()
        <= u32::from(tolerance) * 3
}

pub fn catch_visible(image: &RgbaImage, config: &Calibration) -> bool {
    image
        .pixels()
        .filter(|pixel| matches(&pixel.0, config.catch_rgb, config.catch_tolerance))
        .take(config.catch_min_pixels as usize)
        .count()
        >= config.catch_min_pixels as usize
}

/// Require an upright red component with white space on both sides.
/// This rejects HUD red stars and most world geometry without fixing the marker's position.
pub fn bite_visible(image: &RgbaImage, config: &Calibration) -> bool {
    let (width, height) = image.dimensions();
    let mut mask: Vec<bool> = image
        .pixels()
        .map(|p| matches(&p.0, config.bite_rgb, config.bite_tolerance))
        .collect();
    let mut stack = Vec::new();
    for index in 0..mask.len() {
        if !mask[index] {
            continue;
        }
        mask[index] = false;
        stack.push(index);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
        let mut count = 0;
        while let Some(i) = stack.pop() {
            let (x, y) = (i as u32 % width, i as u32 / width);
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            count += 1;
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < width && ny < height {
                    let next = (ny * width + nx) as usize;
                    if mask[next] {
                        mask[next] = false;
                        stack.push(next);
                    }
                }
            }
        }
        let w = max_x - min_x + 1;
        let h = max_y - min_y + 1;
        if count < config.bite_min_pixels
            || h < 30
            || h * 10 < w * 13
            || h > w * 7
            || min_x < 12
            || max_x + 12 >= width
        {
            continue;
        }
        let mut white = 0;
        for y in min_y..=max_y {
            for x in [min_x - 12, max_x + 12] {
                let p = image.get_pixel(x, y);
                if p[0] > 210 && p[1] > 210 && p[2] > 210 {
                    white += 1;
                }
            }
        }
        if white * 10 >= h * 2 * 6 {
            return true;
        }
    }
    false
}

pub fn rod_selected(image: &RgbaImage) -> bool {
    let selected = image
        .pixels()
        .filter(|p| p[0] > 210 && p[1] > 200 && p[2] > 150)
        .count();
    selected * 2 > (image.width() * image.height()) as usize
}

pub fn energy_ocr_image(image: &RgbaImage) -> GrayImage {
    let mut binary = GrayImage::new(image.width(), image.height());
    for (x, y, pixel) in image.enumerate_pixels() {
        binary.put_pixel(
            x,
            y,
            Luma([if pixel[0] > 165 && pixel[1] > 165 {
                0
            } else {
                255
            }]),
        );
    }
    let scaled = imageops::resize(
        &binary,
        image.width() * 4,
        image.height() * 4,
        imageops::FilterType::Nearest,
    );
    let mut padded = GrayImage::from_pixel(scaled.width() + 40, scaled.height() + 40, Luma([255]));
    imageops::replace(&mut padded, &scaled, 20, 20);
    padded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyReading {
    pub usable: u32,
    pub capacity: u32,
}

pub fn parse_energy(text: &str) -> Option<EnergyReading> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (current, capacity) = cleaned.split_once('/')?;
    let usable: u32 = current.parse().ok()?;
    let capacity: u32 = capacity.parse().ok()?;
    // Unrecognized/saturated readings are not suitable for automated feeding.
    if capacity == 0 || usable > capacity || capacity > 100_000 {
        return None;
    }
    Some(EnergyReading { usable, capacity })
}

use crate::config::BotConfig;
pub fn has_bite(image: &RgbaImage, config: &BotConfig) -> bool {
    if let Some(calibration) = &config.calibration {
        return bite_visible(image, calibration);
    }
    let threshold = ((config.red_region.width * config.red_region.height) / 300).max(15);
    count_matching_pixels(image, (241, 27, 28), config.color_tolerance, threshold) >= threshold
}

pub fn has_catch(image: &RgbaImage, config: &BotConfig) -> bool {
    if let Some(calibration) = &config.calibration {
        return catch_visible(image, calibration);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_ratio_is_not_concatenated() {
        assert_eq!(
            parse_energy("953 / 953\n"),
            Some(EnergyReading {
                usable: 953,
                capacity: 953
            })
        );
        assert_eq!(
            parse_energy("200/1000"),
            Some(EnergyReading {
                usable: 200,
                capacity: 1000
            })
        );
        assert_eq!(parse_energy("9549/9548"), None);
        assert_eq!(parse_energy("956//956"), None);
        assert_eq!(parse_energy("0/0"), None);
        assert_eq!(parse_energy("956"), None);
    }
}
