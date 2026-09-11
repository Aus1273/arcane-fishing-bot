use crate::detection::{energy_ocr_image, parse_energy, EnergyReading};
use anyhow::{anyhow, Context, Result};
use image::RgbaImage;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Finder does not inherit an interactive shell's Homebrew PATH.
pub fn executable() -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "tesseract.exe"
    } else {
        "tesseract"
    };
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .map(|directory| directory.join(name))
                .collect()
        })
        .unwrap_or_default();
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/tesseract"),
        PathBuf::from("/usr/local/bin/tesseract"),
    ]);
    #[cfg(windows)]
    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(directory) = std::env::var_os(key) {
            candidates.push(PathBuf::from(directory).join("Tesseract-OCR").join(name));
        }
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow!("Tesseract was not found; install it with English language data"))
}

pub fn check_engine() -> Result<String> {
    let path = executable()?;
    let mut command = Command::new(&path);
    command.arg("--list-langs");
    let output = bounded_output(&mut command, &AtomicBool::new(false))?;
    if !output.lines().any(|line| line.trim() == "eng") {
        return Err(anyhow!(
            "Tesseract is installed but English language data is missing"
        ));
    }
    Ok(path.display().to_string())
}

/// The OCR child is bounded and killed/reaped on cancellation or timeout.
pub fn read_energy(image: &RgbaImage, cancelled: &AtomicBool) -> Result<EnergyReading> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(anyhow!("OCR cancelled"));
    }
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("energy.png");
    energy_ocr_image(image).save(&path)?;
    let mut command = Command::new(executable()?);
    command.arg(path.canonicalize()?).args([
        "stdout",
        "-l",
        "eng",
        "--psm",
        "7",
        "--dpi",
        "150",
        "-c",
        "tessedit_char_whitelist=0123456789/",
    ]);
    let output = bounded_output(&mut command, cancelled)?;
    parse_energy(&output)
        .ok_or_else(|| anyhow!("Energy text was not a valid usable / capacity reading"))
}

fn bounded_output(command: &mut Command, cancelled: &AtomicBool) -> Result<String> {
    // File-backed output avoids a child blocking on a full stdout pipe.
    let output_file = tempfile::tempfile()?;
    let mut child = command
        .stdout(Stdio::from(output_file.try_clone()?))
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
                child.wait()?;
                if !status.success() {
                    return Err(anyhow!("Tesseract failed; check English language data"));
                }
                use std::io::{Read, Seek, SeekFrom};
                let mut output_file = output_file;
                output_file.seek(SeekFrom::Start(0))?;
                let mut output = String::new();
                output_file.take(64 * 1024).read_to_string(&mut output)?;
                return Ok(output);
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
