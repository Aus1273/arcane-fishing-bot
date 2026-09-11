use fishing_core::{
    config::macbook_profile,
    engine::Controller,
    protocol::{Payload, Session, PROTOCOL_VERSION},
};
use serde_json::json;

fn request(session: &mut Session, value: serde_json::Value) -> serde_json::Value {
    serde_json::to_value(session.handle_json(&value.to_string())).unwrap()
}

#[test]
fn process_protocol_uses_the_same_controller_and_preserves_observation_age() {
    let mut session = Session::default();
    let mut config = macbook_profile();
    config.startup_delay_ms = 0;
    let mut controller = Controller::new(config.clone());
    let response = request(
        &mut session,
        json!({"protocol_version": 1, "command": "start", "config": config}),
    );
    assert_eq!(response["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(
        response["controller"],
        serde_json::to_value(&controller).unwrap()
    );
    let observation = fishing_core::engine::Observation {
        sequence: 1,
        captured_at_ms: 0,
        ..Default::default()
    };
    let actions = controller.step(0, Some(&observation), true, false);
    let response = request(
        &mut session,
        json!({"protocol_version": 1, "command": "step", "at_ms":0,"observation":observation,"focused":true}),
    );
    assert_eq!(response["actions"], serde_json::to_value(actions).unwrap());
    assert_eq!(
        response["controller"],
        serde_json::to_value(controller).unwrap()
    );
    let response = request(
        &mut session,
        json!({"protocol_version": 1, "command": "step", "at_ms":2000,"observation":observation,"focused":true}),
    );
    assert_eq!(response["controller"]["phase"], "paused");
    assert_eq!(response["actions"], json!([]));
}

#[test]
fn invalid_requests_do_not_mutate_session() {
    let mut session = Session::default();
    assert!(matches!(
        session.handle_json("not-json").payload,
        Payload::Error { .. }
    ));
    request(
        &mut session,
        json!({"protocol_version":1,"command":"start","config":macbook_profile()}),
    );
    let before = request(
        &mut session,
        json!({"protocol_version":1,"command":"status"}),
    );
    let unsupported = request(
        &mut session,
        json!({"protocol_version":99,"command":"start","config":{}}),
    );
    assert_eq!(unsupported["kind"], "error");
    let invalid_config = request(
        &mut session,
        json!({"protocol_version":1,"command":"start","config":{"rod_slot":5,"food_slot":5}}),
    );
    assert_eq!(invalid_config["kind"], "error");
    assert_eq!(
        before,
        request(
            &mut session,
            json!({"protocol_version":1,"command":"status"})
        )
    );
    request(
        &mut session,
        json!({"protocol_version":1,"command":"step","at_ms":20,"focused":true,"observation":null}),
    );
    let backwards = request(
        &mut session,
        json!({"protocol_version":1,"command":"step","at_ms":10,"focused":true,"observation":null}),
    );
    assert_eq!(backwards["kind"], "error");
    assert_eq!(
        request(
            &mut session,
            json!({"protocol_version":1,"command":"status"})
        )["at_ms"],
        20
    );
}

#[test]
fn cli_replay_emits_a_versioned_native_ui_payload() {
    let replay = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/replays/normal-cycle.json"
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fishing-core-cli"))
        .args(["replay", replay])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["protocol_version"], 1);
    assert_eq!(payload["kind"], "replay");
    assert!(!payload["frames"].as_array().unwrap().is_empty());
    assert_eq!(
        payload["frames"].as_array().unwrap().last().unwrap()["fish"],
        1
    );
}
