//! Offline timing baseline: no desktop capture, input or configuration writes.
use anyhow::{bail, Context, Result};
use arcane_fishing_bot_app::{
    capture::crop_frame,
    config::BotConfig,
    diagnostics::{observe, Needs},
    ocr,
};
use serde_json::json;
use std::{sync::atomic::AtomicBool, time::Instant};
fn percentile(values: &mut [u128], percent: usize) -> f64 {
    values.sort_unstable();
    values[((values.len() - 1) * percent).div_ceil(100)] as f64 / 1000.0
}
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let config: BotConfig = serde_json::from_slice(&std::fs::read(
        args.next()
            .context("Usage: benchmark CONFIG.json IMAGE.png [IMAGE.png ...]")?,
    )?)?;
    config.validate()?;
    let paths: Vec<_> = args.collect();
    if paths.is_empty() {
        bail!("Supply at least one PNG");
    }
    let mut reports = vec![];
    for path in paths {
        let image = image::open(&path)?.to_rgba8();
        let cancel = AtomicBool::new(false);
        let needs = Needs {
            energy: false,
            bite: true,
        };
        let observed = observe(&image, &config, needs, &cancel)?;
        let mut detection = vec![];
        for _ in 0..25 {
            let time = Instant::now();
            std::hint::black_box(observe(&image, &config, needs, &cancel)?);
            detection.push(time.elapsed().as_micros());
        }
        let crop = crop_frame(&image, config.hunger_region);
        let mut times = vec![];
        let mut energy = vec![];
        for _ in 0..5 {
            let time = Instant::now();
            energy.push(ocr::read_energy(&crop, &cancel).map_err(|error| error.to_string()));
            times.push(time.elapsed().as_micros());
        }
        reports.push(json!({"file":std::path::Path::new(&path).file_name().unwrap().to_string_lossy(),"dimensions":image.dimensions(),"observation":observed,
            "detection":{"samples":detection.len(),"p50_ms":percentile(&mut detection,50),"p95_ms":percentile(&mut detection,95)},
            "ocr":{"samples":times.len(),"p50_ms":percentile(&mut times,50),"p95_ms":percentile(&mut times,95),"results":energy}}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"version":1,"scope":"Offline normalized pixels; PNG decode and live capture excluded; Tesseract includes preprocess and child startup", "build":if cfg!(debug_assertions){"debug"}else{"release"},"images":reports})
        )?
    );
    Ok(())
}
