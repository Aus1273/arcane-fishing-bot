//! Non-prompting checks. Readiness does not certify live detector accuracy.
use crate::{
    config::{BotConfig, EnergyFailurePolicy},
    input, ocr,
    runtime::SessionMode,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct Check {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}
#[derive(Serialize)]
pub struct Readiness {
    pub ready: bool,
    pub checks: Vec<Check>,
}
impl Readiness {
    fn add(&mut self, id: &str, label: &str, status: &str, detail: impl Into<String>) {
        self.ready &= status != "fail";
        self.checks.push(Check {
            id: id.into(),
            label: label.into(),
            status: status.into(),
            detail: detail.into(),
        });
    }
    pub fn failure_message(&self) -> String {
        self.checks
            .iter()
            .filter(|check| check.status == "fail")
            .map(|check| check.detail.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}
pub fn check(config: &BotConfig, mode: SessionMode, shortcut_ready: bool) -> Readiness {
    let mut report = Readiness {
        ready: true,
        checks: vec![],
    };
    match config.validate() {
        Ok(()) => report.add("config", "Saved settings", "pass", "Settings are valid"),
        Err(error) => report.add("config", "Saved settings", "fail", error.to_string()),
    }
    if let Some(calibration) = &config.calibration {
        match screenshots::Screen::from_point(0, 0) {
            Ok(screen) => {
                let display = screen.display_info;
                let expected = calibration.frame_width as f64 / calibration.frame_height as f64;
                let actual = display.width as f64 / display.height as f64;
                let matches = (expected - actual).abs() <= 0.02;
                report.add("display", "Capture display", if matches {"pass"} else {"fail"},
                    format!("Display at desktop origin: {}×{} logical pixels; profile {}×{}. {}", display.width, display.height, calibration.frame_width, calibration.frame_height,
                        if matches {"Run Roblox fullscreen on this display; windowed layouts require recalibration."} else {"Aspect ratio does not match the profile."}));
            }
            Err(error) => report.add("display", "Capture display", "fail", error.to_string()),
        }
    } else {
        report.add(
            "profile",
            "Screen profile",
            "fail",
            "Select and save a calibrated screen profile first",
        );
    }
    #[cfg(target_os = "macos")]
    {
        let capture = capture_allowed();
        report.add("capture_permission", "Screen recording permission", if capture {"pass"} else {"fail"},
            if capture {"macOS screen recording permission is available"} else {"Enable Arcane Fishing Bot in System Settings → Privacy & Security → Screen & System Audio Recording, then reopen the app"});
        if mode == SessionMode::Automate {
            let input = input_allowed();
            report.add("input_permission", "Accessibility permission", if input {"pass"} else {"fail"},
                if input {"macOS Accessibility permission is available"} else {"Enable Arcane Fishing Bot in System Settings → Privacy & Security → Accessibility, then reopen the app"});
        } else {
            report.add(
                "input_permission",
                "Observe-only input",
                "pass",
                "No native input device is created in this mode",
            );
        }
    }
    #[cfg(not(target_os = "macos"))]
    report.add(
        "permissions",
        "Desktop access",
        "warn",
        "Capture and input still need a supervised test on this Windows account",
    );
    report.add("stop_shortcut", "Emergency stop", if shortcut_ready {"pass"} else if mode == SessionMode::Observe {"warn"} else {"fail"},
        if shortcut_ready {"Control + Shift + F12 is registered system-wide"} else {"The emergency shortcut could not be registered. Close any app using Control + Shift + F12 and reopen this app"});
    if config.monitors_energy() {
        match ocr::check_engine() {
            Ok(path) => report.add(
                "ocr",
                "Energy OCR",
                "pass",
                format!("English data available: {path}"),
            ),
            Err(error) => report.add(
                "ocr",
                "Energy OCR",
                if config.auto_feed_enabled
                    || config.energy_failure_policy == EnergyFailurePolicy::Pause
                {
                    "fail"
                } else {
                    "warn"
                },
                error.to_string(),
            ),
        }
    } else {
        report.add(
            "ocr",
            "Energy OCR",
            "pass",
            "Energy monitoring and feeding are disabled; OCR is not required",
        );
    }
    match input::roblox_focused() {
        Ok(true) => report.add("focus", "Roblox focus", "pass", "Roblox is foreground"),
        _ => report.add(
            "focus",
            "Roblox focus",
            "warn",
            "Switch to Roblox during the startup countdown. Input stops when Roblox loses focus",
        ),
    }
    report.add("validation", "Live validation", "warn", "Passing checks confirms prerequisites, not detection accuracy. Use observe-only before automation");
    report
}

#[cfg(target_os = "macos")]
pub fn capture_allowed() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }
    unsafe { CGPreflightScreenCaptureAccess() }
}
#[cfg(target_os = "macos")]
pub fn input_allowed() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}
