# Shared fishing core

This crate contains the controller, colour detectors, configuration/storage and offline replay tools. It does not depend on Tauri, native input, screen capture or a UI framework. The controller accepts observations and returns proposed actions; only the desktop runtime may execute those actions.

Run the isolated tests with `cargo test -p fishing-core`. Build the process bridge with `cargo build -p fishing-core --bin fishing-core-cli`. Run scenarios with `cargo run -p fishing-core --bin fishing-core-cli -- replay tests/replays/normal-cycle.json`; this replaces the redundant desktop replay wrapper.

## JSON process protocol, version 1

`target/debug/fishing-core-cli replay tests/replays/normal-cycle.json` emits one JSON object:

```json
{"protocol_version":1,"kind":"replay","name":"scenario name","frames":[{"at_ms":0,"phase":"selecting_rod","actions":[{"select_slot":4}],"fish":0,"feeds":0,"reason":"Equipping the rod"}]}
```

`fishing-core-cli serve` reads one JSON request per line and writes one JSON response per line. It keeps one controller in memory. All input times are monotonic milliseconds relative to that controller's start; original capture timestamps and observation sequence numbers are preserved.

```json
{"protocol_version":1,"command":"start","config":{"rod_slot":4,"food_slot":5,"startup_delay_ms":0}}
{"protocol_version":1,"command":"step","at_ms":0,"observation":{"sequence":1,"captured_at_ms":0,"rod_selected":false},"focused":true,"stop":false}
{"protocol_version":1,"command":"status"}
```

Responses contain `protocol_version: 1`, `kind: "controller"`, `at_ms`, the public `controller` snapshot and `actions`. Action values are `"click"` or `{"select_slot":4}`. Callers must not execute these automatically: the CLI is intended for offline comparison and read-only prototypes. Controller fields include `phase`, `reason`, `fish_caught`, `feeds`, `errors`, `recovery_attempts` and optional raw `energy`.

Failures return `{"protocol_version":1,"kind":"error","error":"..."}`. Unsupported versions, malformed requests and backwards time do not advance the controller. Each request and replay file is limited to 8 MiB. A future incompatible schema requires a new protocol version.

Optional Energy monitoring defaults to disabled when loading older configurations. Feeding always enables Energy checks. The `energy_failure_policy` setting accepts `"continue"` (default) or `"pause"`; continuing applies only to monitoring, never feeding or screen-capture failures. The feed counter advances after two increased-capacity observations **and** confirmed rod restoration.
