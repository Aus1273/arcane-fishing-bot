use crate::{
    capture::{capture_frame, crop_frame},
    config::{rod_selection_region, BotConfig},
    detection,
    engine::Observation,
    ocr,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, ImageOutputFormat, RgbaImage};
use serde::Serialize;
use std::{
    io::Cursor,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Default)]
pub struct Needs {
    pub energy: bool,
    pub bite: bool,
}

pub fn observe(
    image: &RgbaImage,
    config: &BotConfig,
    needs: Needs,
    cancel: &AtomicBool,
) -> Result<Observation> {
    observe_before(
        image,
        config,
        needs,
        cancel,
        Instant::now() + Duration::from_millis(2500),
    )
}

pub fn observe_before(
    image: &RgbaImage,
    config: &BotConfig,
    needs: Needs,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<Observation> {
    config.validate()?;
    let calibration = config
        .calibration
        .as_ref()
        .ok_or_else(|| anyhow!("Select a calibrated profile"))?;
    if image.dimensions() != (calibration.frame_width, calibration.frame_height) {
        return Err(anyhow!(
            "Screenshot dimensions do not match the selected profile"
        ));
    }
    let energy = needs.energy.then(|| {
        ocr::read_energy_before(&crop_frame(image, config.hunger_region), cancel, deadline)
    });
    let (energy, energy_error) = match energy {
        Some(Ok(value)) => (Some(value), None),
        Some(Err(error)) => (None, Some(error.to_string())),
        None => (None, None),
    };
    Ok(Observation {
        bite_checked: needs.bite,
        bite: needs.bite && detection::has_bite(&crop_frame(image, config.red_region), config),
        caught: detection::has_catch(&crop_frame(image, config.yellow_region), config),
        rod_selected: detection::rod_selected(&crop_frame(
            image,
            rod_selection_region(calibration, config.rod_slot),
        )),
        food_selected: detection::rod_selected(&crop_frame(
            image,
            rod_selection_region(calibration, config.food_slot),
        )),
        energy,
        energy_error,
        ..Observation::default()
    })
}

#[derive(Serialize)]
pub struct Preview {
    pub frame_data_url: String,
    pub observation: Observation,
    pub elapsed_ms: u64,
    pub regions: Vec<RegionPreview>,
}
#[derive(Serialize)]
pub struct RegionPreview {
    pub name: String,
    pub data_url: String,
}

pub fn inspect_frame(image: RgbaImage, config: &BotConfig) -> Result<Preview> {
    config.validate()?;
    let calibration = config
        .calibration
        .as_ref()
        .ok_or_else(|| anyhow!("Select a calibrated profile"))?;
    if image.dimensions() != (calibration.frame_width, calibration.frame_height) {
        return Err(anyhow!(
            "Expected {}×{} image; received {}×{}",
            calibration.frame_width,
            calibration.frame_height,
            image.width(),
            image.height()
        ));
    }
    let started = Instant::now();
    // Still show colors/regions if OCR is unavailable.
    let cancel = AtomicBool::new(false);
    let mut observation = observe(
        &image,
        config,
        Needs {
            energy: false,
            bite: true,
        },
        &cancel,
    )?;
    match ocr::read_energy(&crop_frame(&image, config.hunger_region), &cancel) {
        Ok(value) => observation.energy = Some(value),
        Err(error) => observation.energy_error = Some(error.to_string()),
    }
    let mut regions = vec![];
    for (name, region) in [
        ("Bite search", config.red_region),
        ("Catch heading", config.yellow_region),
        ("Energy numbers", config.hunger_region),
        (
            "Rod selection",
            rod_selection_region(calibration, config.rod_slot),
        ),
    ] {
        let crop = DynamicImage::ImageRgba8(crop_frame(&image, region)).resize(
            800,
            500,
            image::imageops::FilterType::Triangle,
        );
        let mut bytes = Cursor::new(Vec::new());
        crop.write_to(&mut bytes, ImageOutputFormat::Png)?;
        regions.push(RegionPreview {
            name: name.into(),
            data_url: format!(
                "data:image/png;base64,{}",
                STANDARD.encode(bytes.into_inner())
            ),
        });
    }
    let mut full_image = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut full_image, ImageOutputFormat::Png)?;
    Ok(Preview {
        frame_data_url: format!(
            "data:image/png;base64,{}",
            STANDARD.encode(full_image.into_inner())
        ),
        observation,
        regions,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

pub fn inspect_png(encoded: &str, config: &BotConfig) -> Result<Preview> {
    if encoded.len() > 32_000_000 {
        return Err(anyhow!("Screenshot exceeds 24 MB"));
    }
    let bytes = STANDARD.decode(encoded)?;
    // Check dimensions before allocating decoded pixels.
    let decoder = image::codecs::png::PngDecoder::new(Cursor::new(&bytes))?;
    use image::ImageDecoder;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > 16_000_000 {
        return Err(anyhow!("Screenshot exceeds 16 megapixels"));
    }
    inspect_frame(DynamicImage::from_decoder(decoder)?.to_rgba8(), config)
}
pub fn inspect_live(config: &BotConfig) -> Result<Preview> {
    inspect_frame(capture_frame(config)?, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expired_ocr_is_an_energy_error_not_a_capture_failure() {
        let config = crate::config::macbook_profile();
        let calibration = config.calibration.as_ref().unwrap();
        let image = RgbaImage::new(calibration.frame_width, calibration.frame_height);
        let observation = observe_before(
            &image,
            &config,
            Needs {
                energy: true,
                bite: false,
            },
            &AtomicBool::new(false),
            Instant::now(),
        )
        .unwrap();
        assert!(observation.error.is_none());
        assert!(observation.energy.is_none());
        assert!(observation.energy_error.unwrap().contains("time budget"));
    }
}
