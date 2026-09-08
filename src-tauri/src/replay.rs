//! Replays observations with simulated time; never captures the screen or creates input devices.
use crate::{
    config::{macbook_profile, BotConfig},
    engine::{Action, Controller, Observation, Phase},
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Scenario {
    pub name: String,
    #[serde(default = "macbook_profile")]
    pub config: BotConfig,
    pub frames: Vec<Frame>,
}
#[derive(Deserialize)]
pub struct Frame {
    pub at_ms: u64,
    pub observation: Option<Observation>,
    #[serde(default = "yes")]
    pub focused: bool,
    #[serde(default)]
    pub stop: bool,
    #[serde(default)]
    pub reuse_observation: bool,
    pub expected_phase: Phase,
    #[serde(default)]
    pub expected_actions: Vec<Action>,
    pub expected_fish: Option<u64>,
    pub expected_feeds: Option<u64>,
}
fn yes() -> bool {
    true
}
#[derive(Serialize)]
pub struct Trace {
    pub at_ms: u64,
    pub phase: Phase,
    pub actions: Vec<Action>,
    pub fish: u64,
    pub feeds: u64,
    pub reason: String,
}

pub fn run(scenario: Scenario) -> Result<Vec<Trace>> {
    scenario.config.validate()?;
    let mut controller = Controller::new(scenario.config);
    let mut previous = None;
    let mut traces = vec![];
    let mut last_time = 0;
    for (index, frame) in scenario.frames.into_iter().enumerate() {
        if frame.at_ms < last_time {
            bail!("Replay time moved backwards");
        }
        last_time = frame.at_ms;
        if !frame.reuse_observation {
            previous = frame.observation.map(|mut obs| {
                obs.sequence = index as u64 + 1;
                obs.captured_at_ms = frame.at_ms;
                obs
            });
        }
        let actions = controller.step(frame.at_ms, previous.as_ref(), frame.focused, frame.stop);
        if controller.phase != frame.expected_phase
            || actions != frame.expected_actions
            || frame
                .expected_fish
                .is_some_and(|value| value != controller.fish_caught)
            || frame
                .expected_feeds
                .is_some_and(|value| value != controller.feeds)
        {
            bail!(
                "{} frame {} at {}ms: got {:?} {:?}, fish={}, feeds={}; expected {:?} {:?}",
                scenario.name,
                index,
                frame.at_ms,
                controller.phase,
                actions,
                controller.fish_caught,
                controller.feeds,
                frame.expected_phase,
                frame.expected_actions
            );
        }
        traces.push(Trace {
            at_ms: frame.at_ms,
            phase: controller.phase,
            actions,
            fish: controller.fish_caught,
            feeds: controller.feeds,
            reason: controller.reason.clone(),
        });
    }
    Ok(traces)
}
