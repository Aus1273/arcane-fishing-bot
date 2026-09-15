# Application architecture

There is one production application and one portable engine. The desktop app owns windows, capture and input; the engine decides what to do from timestamped observations. The engine can run in tests and the replay CLI without any desktop permissions.

## Code ownership

| Location | Owns |
| --- | --- |
| `src/App.svelte` | App navigation, shared configuration, session controls, readiness and saving |
| `src/features/session/` | Session summary, diagnostics, recording import/export and replay comparison |
| `src/features/calibration/` | Profiles, screenshot editor, detector tuning, geometry and read-only overlay view |
| `src/features/settings/` | Fishing, Energy/food and runtime timing settings |
| `src/lib/ipc.ts` | Typed frontend/native command boundary and explicitly limited browser preview |
| `src/lib/Button.svelte`, `src/app.css` | Shared button, design tokens and genuinely shared styles |
| `src-tauri/src/main.rs` | Native lifecycle, plugin setup, command registration and shutdown |
| `src-tauri/src/commands.rs` | Frontend command adapters; delegates validation and session ownership |
| `src-tauri/src/runtime.rs` | Worker lifecycle, cancellation, bounded event history and statistics |
| `src-tauri/src/capture.rs`, `input.rs`, `ocr.rs` | OS adapters and permission-sensitive work |
| `src-tauri/src/diagnostics.rs`, `readiness.rs`, `recording.rs` | Inspection, prerequisites and replay evidence |
| `src-tauri/src/overlay.rs`, `overlay_window.rs` | Overlay geometry and native window ownership |
| `crates/fishing-core/` | Controller, detectors, validated configuration, atomic storage and offline replay |
| `configs/`, `tests/`, `src-tauri/tests/`, `crates/fishing-core/tests/` | Profile assets and regression coverage |

Screen styles live inside their Svelte feature components. Calibration settings have one editing surface; they are not duplicated in Settings. Session and calibration components stay mounted across navigation to retain local screenshots, selected evidence and imported recordings. Expensive diagnostic markup is only rendered when expanded.

To change a feature, start in its directory. Add a native command in `commands.rs` and its typed wrapper in `ipc.ts` only when the feature actually needs native work. Put deterministic detection or control logic in `fishing-core`, with an offline regression. There is no plugin system, dependency-injection framework or generic page registry to maintain.

## Session contract

The controller progresses through rod reset/selection, casting, bite wait, reeling, catch confirmation and optional feeding. A catch requires fresh confirmation; feeding requires an absolute capacity increase and rod restoration. Deadlines and bounded recovery prevent indefinite blind input.

- Observe mode never constructs a native input device or updates lifetime fishing statistics.
- Only the control thread owns input. Each action rechecks Roblox foreground identity. Focus loss pauses the session; restart is explicit.
- Cancellation is preserved during preflight and shutdown. Ctrl + Shift + F12 stops the controller and closes the overlay. Failed shortcut registration blocks automation.
- The capture worker keeps only the latest timestamped observation. The controller ticks every 10 ms; UI updates occur on transitions or every 250 ms. Late ticks do not generate catch-up click bursts.
- Old, repeated or future-dated observations cannot authorize new confirmations. Capture failures remain blocking. Optional Energy OCR can fail independently; feeding always requires verified readings.
- OCR is cancellable and bounded to 2.5 seconds. Live OCR uses the smaller remaining frame-freshness budget, reserving 100 ms for analysis and publication so its error can reach the controller before the frame expires. OS capture already in progress may be uninterruptible, so the observer owns no input.
- A 200-entry deque bounds event history without shifting the whole list when an event expires. Optional recordings retain the first 30,000 original controller ticks. JSON replay is limited to 32 MiB and preserves capture sequence/timing.
- Inspection and the wireframe overlay reserve the runtime, preventing overlapping capture, input or configuration writes. The overlay holds its reservation until its native window is destroyed.
- Settings are validated and frozen during a session. Existing serialized compatibility fields and OS data directories are retained. Statistics checkpoint every 30 seconds and at orderly shutdown.

## Build boundary

`npm start` launches Tauri; `npm run package` builds it. Tauri embeds the frontend from `dist/`. Rust output goes to `target/`; no executable is generated into a source directory. Cargo's `src/bin` and `examples` contain developer-tool source and are not additional production apps.

Historical implementations and the native experiment remain recoverable in Git history; see [development](development.md). Builds, deterministic tests and screenshot replay establish only their tested behavior. Live-game acceptance remains a separate supervised check.
