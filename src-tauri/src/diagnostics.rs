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
use std::{io::Cursor, sync::atomic::AtomicBool, time::Instant};

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
        energy: if needs.energy {
            Some(ocr::read_energy(
                &crop_frame(image, config.hunger_region),
                cancel,
            )?)
        } else {
            None
        },
        ..Observation::default()
    })
}

#[derive(Serialize)]
pub struct Preview {
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
        Err(error) => observation.error = Some(error.to_string()),
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
    Ok(Preview {
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
