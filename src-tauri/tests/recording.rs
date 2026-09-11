use arcane_fishing_bot_app::{
    config::macbook_profile,
    engine::{Action, Controller, Observation, Phase},
    recording::{replay_json, RecordingFrame, SessionRecording, MAX_FRAMES, MAX_RECORDING_BYTES},
    runtime::SessionMode,
};

fn observation(at_ms: u64, rod_selected: bool) -> Observation {
    Observation {
        sequence: at_ms + 1,
        captured_at_ms: at_ms,
        rod_selected,
        bite_checked: true,
        ..Default::default()
    }
}

fn record_tick(
    recording: &mut SessionRecording,
    controller: &mut Controller,
    at_ms: u64,
    observation: Option<Observation>,
    action_error: Option<&str>,
) {
    let actions = controller.step(at_ms, observation.as_ref(), true, false);
    if let Some(error) = action_error {
        controller.action_failed(error);
    }
    recording.push(RecordingFrame {
        at_ms,
        observation,
        focused: true,
        stop: false,
        expected_phase: controller.phase,
        expected_actions: actions,
        expected_fish: controller.fish_caught,
        expected_feeds: controller.feeds,
        action_error: action_error.map(String::from),
    });
}

fn sample_recording() -> SessionRecording {
    let mut config = macbook_profile();
    config.startup_delay_ms = 0;
    let mut controller = Controller::new(config.clone());
    let mut recording = SessionRecording::new(config, SessionMode::Observe);
    record_tick(
        &mut recording,
        &mut controller,
        0,
        Some(observation(0, false)),
        None,
    );
    record_tick(
        &mut recording,
        &mut controller,
        100,
        Some(observation(100, true)),
        None,
    );
    // The same observation must not authorize a new cast.
    record_tick(
        &mut recording,
        &mut controller,
        200,
        Some(observation(100, true)),
        None,
    );
    record_tick(
        &mut recording,
        &mut controller,
        300,
        Some(observation(300, true)),
        None,
    );
    // This capture becomes stale; replay must not update its timestamp to 2000.
    record_tick(
        &mut recording,
        &mut controller,
        2000,
        Some(observation(300, true)),
        None,
    );
    recording.complete = true;
    recording
}

#[test]
fn serialized_recording_replays_duplicate_and_stale_observations_exactly() {
    let recording = sample_recording();
    let json = serde_json::to_string(&recording).unwrap();
    let decoded: SessionRecording = serde_json::from_str(&json).unwrap();
    assert_eq!(
        decoded.frames[2]
            .observation
            .as_ref()
            .unwrap()
            .captured_at_ms,
        100
    );
    assert_eq!(
        decoded.frames[2].observation.as_ref().unwrap().sequence,
        101
    );
    let replay = replay_json(&json, None).unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(replay.trace[2].phase, Phase::Ready);
    assert!(replay.trace[2].actions.is_empty());
    assert_eq!(replay.trace[3].actions, vec![Action::Click]);
    assert_eq!(replay.trace[4].phase, Phase::Paused);
}

#[test]
fn recorded_action_failure_is_applied_after_proposed_action() {
    let mut config = macbook_profile();
    config.startup_delay_ms = 0;
    let mut controller = Controller::new(config.clone());
    let mut recording = SessionRecording::new(config, SessionMode::Automate);
    record_tick(
        &mut recording,
        &mut controller,
        0,
        Some(observation(0, false)),
        Some("Native input unavailable"),
    );
    let replay = replay_json(&serde_json::to_string(&recording).unwrap(), None).unwrap();
    assert!(replay.errors.is_empty());
    assert_eq!(replay.trace[0].actions, vec![Action::SelectSlot(4)]);
    assert_eq!(replay.trace[0].phase, Phase::Paused);
    assert_eq!(replay.trace[0].reason, "Native input unavailable");
}

#[test]
fn expected_mismatches_are_reported_but_overrides_compare_different_settings() {
    let mut recording = sample_recording();
    recording.frames[0].expected_fish = 99;
    let json = serde_json::to_string(&recording).unwrap();
    let replay = replay_json(&json, None).unwrap();
    assert_eq!(replay.errors.len(), 1);
    let mut config = recording.config;
    config.startup_delay_ms = 3000;
    let replay = replay_json(&json, Some(config)).unwrap();
    assert!(replay.errors.is_empty());
    assert!(replay
        .trace
        .iter()
        .all(|frame| frame.phase == Phase::Startup && frame.actions.is_empty()));
}

#[test]
fn recordings_validate_version_clock_configuration_and_frame_limits() {
    let original = sample_recording();
    let mut invalid = original.clone();
    invalid.version = 2;
    assert!(replay_json(&serde_json::to_string(&invalid).unwrap(), None)
        .unwrap_err()
        .to_string()
        .contains("version"));
    invalid = original.clone();
    invalid.frames[2].at_ms = 50;
    assert!(replay_json(&serde_json::to_string(&invalid).unwrap(), None)
        .unwrap_err()
        .to_string()
        .contains("backwards"));
    invalid = original.clone();
    invalid.config.rod_slot = invalid.config.food_slot;
    assert!(replay_json(&serde_json::to_string(&invalid).unwrap(), None).is_err());
    invalid = original;
    invalid.frames = vec![invalid.frames[0].clone(); MAX_FRAMES + 1];
    assert!(replay_json(&serde_json::to_string(&invalid).unwrap(), None)
        .unwrap_err()
        .to_string()
        .contains("frame limit"));
    assert!(replay_json(&" ".repeat(MAX_RECORDING_BYTES + 1), None)
        .unwrap_err()
        .to_string()
        .contains("32 MiB"));
}

#[test]
fn recording_cap_keeps_startup_evidence_and_marks_truncation() {
    let mut recording = sample_recording();
    let first = recording.frames[0].clone();
    recording.frames = vec![first.clone(); MAX_FRAMES];
    let mut late = first;
    late.at_ms = 900_000;
    assert!(!recording.push(late));
    assert_eq!(recording.frames.len(), MAX_FRAMES);
    assert_eq!(recording.frames.first().unwrap().at_ms, 0);
    assert!(!recording.complete || recording.truncated);
    assert!(recording.truncated);
}
