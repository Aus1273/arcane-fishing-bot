//! Bounded local session evidence. Replays preserve the controller's actual input timing.
//! Stored observations describe detector results, so overrides cannot retest image regions.
use crate::{
    config::BotConfig,
    engine::{Action, Controller, Observation, Phase},
    replay::Trace,
    runtime::SessionMode,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const RECORDING_VERSION: u32 = 1;
pub const MAX_FRAMES: usize = 30_000;
pub const MAX_RECORDING_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionRecording {
    pub version: u32,
    pub name: String,
    pub config: BotConfig,
    pub frames: Vec<RecordingFrame>,
    pub complete: bool,
    pub mode: SessionMode,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingFrame {
    pub at_ms: u64,
    pub observation: Option<Observation>,
    pub focused: bool,
    pub stop: bool,
    pub expected_phase: Phase,
    pub expected_actions: Vec<Action>,
    pub expected_fish: u64,
    pub expected_feeds: u64,
    pub action_error: Option<String>,
}

/// Short alias for callers constructing individual recorded ticks.
pub type Frame = RecordingFrame;

#[derive(Debug, Serialize)]
pub struct ReplayResult {
    pub trace: Vec<Trace>,
    pub errors: Vec<String>,
}

impl SessionRecording {
    pub fn new(config: BotConfig, mode: SessionMode) -> Self {
        Self {
            version: RECORDING_VERSION,
            name: format!(
                "Session {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            ),
            config,
            frames: vec![],
            complete: false,
            mode,
            truncated: false,
        }
    }

    /// Preserve the beginning of the session so replay always has the controller's startup.
    /// Returns false once the bound is reached; later frames cannot displace prior evidence.
    pub fn push(&mut self, frame: RecordingFrame) -> bool {
        if self.frames.len() >= MAX_FRAMES {
            self.truncated = true;
            return false;
        }
        self.frames.push(frame);
        true
    }
}

pub fn replay_json(recording_json: &str, config: Option<BotConfig>) -> Result<ReplayResult> {
    if recording_json.len() > MAX_RECORDING_BYTES {
        bail!("Recording exceeds the 32 MiB limit");
    }
    let recording: SessionRecording =
        serde_json::from_str(recording_json).context("Invalid session recording")?;
    if recording.version != RECORDING_VERSION {
        bail!(
            "Unsupported recording version {}; expected {RECORDING_VERSION}",
            recording.version
        );
    }
    if recording.frames.len() > MAX_FRAMES {
        bail!("Recording exceeds the {MAX_FRAMES} frame limit");
    }
    recording
        .config
        .validate()
        .context("Invalid recorded configuration")?;
    let compare_expected = config.is_none();
    let config = config.unwrap_or(recording.config);
    config.validate().context("Invalid replay configuration")?;
    let mut controller = Controller::new(config);
    let mut trace = Vec::with_capacity(recording.frames.len());
    let mut errors = vec![];
    let mut last_time = 0;
    for (index, frame) in recording.frames.into_iter().enumerate() {
        if frame.at_ms < last_time {
            bail!("Recording time moved backwards at frame {index}");
        }
        last_time = frame.at_ms;
        // Do not renumber observations or make old captures appear fresh during playback.
        let actions = controller.step(
            frame.at_ms,
            frame.observation.as_ref(),
            frame.focused,
            frame.stop,
        );
        if let Some(error) = &frame.action_error {
            controller.action_failed(error);
        }
        if compare_expected
            && (controller.phase != frame.expected_phase
                || actions != frame.expected_actions
                || controller.fish_caught != frame.expected_fish
                || controller.feeds != frame.expected_feeds)
        {
            errors.push(format!(
                "Frame {index} at {}ms: got {:?} {:?}, fish={}, feeds={}; expected {:?} {:?}, fish={}, feeds={}",
                frame.at_ms, controller.phase, actions, controller.fish_caught, controller.feeds,
                frame.expected_phase, frame.expected_actions, frame.expected_fish, frame.expected_feeds
            ));
        }
        trace.push(Trace {
            at_ms: frame.at_ms,
            phase: controller.phase,
            actions,
            fish: controller.fish_caught,
            feeds: controller.feeds,
            reason: controller.reason.clone(),
        });
    }
    Ok(ReplayResult { trace, errors })
}
