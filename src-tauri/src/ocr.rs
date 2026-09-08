use crate::detection::{energy_ocr_image, parse_energy, EnergyReading};
use anyhow::{anyhow, Context, Result};
use image::RgbaImage;
use std::{
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// The OCR child is bounded and killed/reaped on cancellation or timeout.
pub fn read_energy(image: &RgbaImage, cancelled: &AtomicBool) -> Result<EnergyReading> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(anyhow!("OCR cancelled"));
    }
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("energy.png");
    energy_ocr_image(image).save(&path)?;
    let mut child = Command::new("tesseract")
        .arg(path.canonicalize()?)
        .args([
            "stdout",
            "-l",
            "eng",
            "--psm",
            "7",
            "--dpi",
            "150",
            "-c",
            "tessedit_char_whitelist=0123456789/",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Tesseract could not be started; install it with English language data")?;
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Relaxed) || started.elapsed() > Duration::from_millis(2500) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!("OCR cancelled or exceeded 2.5 seconds"));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output()?;
                if !status.success() {
                    return Err(anyhow!("Tesseract failed; check English language data"));
                }
                return parse_energy(&String::from_utf8_lossy(&output.stdout)).ok_or_else(|| {
                    anyhow!("Energy text was not a valid usable / capacity reading")
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        }
    }
}
