# Development

## Prerequisites

- Rust with Cargo, Clippy and rustfmt.
- Node 20.19+ or 22.12+ and npm. Both lockfiles are committed.
- macOS: Xcode Command Line Tools (`xcode-select --install`).
- Windows: Microsoft C++ Build Tools with the desktop C++ workload and WebView2.
- Optional Energy OCR: Tesseract with English data (`brew install tesseract` on macOS).

## Commands

| Command | Purpose |
| --- | --- |
| `npm ci` | Install the locked frontend dependencies |
| `npm start` | Launch the desktop app in development mode |
| `npm run package` | Check the frontend and build the desktop app/installers |
| `npm run dev` | Internal Vite server/browser preview; native actions are unavailable |
| `npm run check` | Svelte and TypeScript checks |
| `npm test` | Calibration geometry regressions |
| `npm run format:check` | Check frontend formatting |
| `cargo test --locked --workspace` | Core and desktop regressions |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Rust static checks |
| `cargo fmt --all --check` | Rust formatting |

Tauri uses `npm run dev` and `npm run build` internally. No custom build wrapper or generated source step is required. For only a macOS app bundle, run `npm run package -- --bundles app`.

Build output belongs in `target/` and `dist/`; dependencies belong in `node_modules/`. The release app is `target/release/bundle/macos/Arcane Fishing Bot.app`, and Windows installers are under `target/release/bundle/`. Local macOS bundles are ad-hoc signed and are not notarized. Windows local builds are unsigned. The packaged app contains the production executable; the CLI and benchmark below are developer tools.

### macOS signing and permissions

The macOS bundle explicitly uses `signingIdentity: "-"`. This seals the complete app bundle; the Rust linker's executable-only signature is not sufficient after packaging. Verify a local build with:

```sh
codesign --verify --deep --strict --verbose=2 "target/release/bundle/macos/Arcane Fishing Bot.app"
```

Ad-hoc signing does not preserve privacy approvals across code changes. After rebuilding, remove stale entries and add the current bundle in both Accessibility and Screen & System Audio Recording, then reopen it. System Audio Recording Only does not grant screen capture. Historical native prototypes used a different bundle ID (`com.aus1273.arcane.native-comparison`); the production app uses `com.arcane.fishing-bot`. A consistent developer signing identity is required for durable identity across updates.

## Offline tools

```sh
cargo run --locked -p fishing-core --bin fishing-core-cli -- replay tests/replays/normal-cycle.json
cargo run --release --example benchmark -- configs/macbook-pro-14-3024x1964.json IMAGE.png
```

The [core CLI](../crates/fishing-core/README.md) exposes the actual controller without native input. The optional original-screenshot regression needs the supplied PNGs and Tesseract:

```sh
ARCANE_SCREENSHOT_DIR='/path/to/original/screenshots' cargo test --release \
  --test screenshots -- --ignored --nocapture
```

[Recorded offline measurements](benchmarks/macbook-offline-2026-09-10.json) cover the supplied screenshots, not live capture speed.

## Live validation

Grant Screen Recording access in macOS System Settings → Privacy & Security; automation also needs Accessibility. Reopen the app after changing permissions. Check setup in Session, start with Observe only and feeding off, and switch to Roblox during the countdown.

Verify camera/fullscreen changes, bite and catch animations, rod reset/cast acceptance, focus loss, physical Ctrl + Shift + F12 delivery, click timing and low-Energy/food behavior before longer sessions. Neither CI nor screenshot fixtures prove live game reliability.

## Historical code

Old egui/Tauri implementations and the SwiftUI experiment were removed from the production checkout. Their source and evaluation reports remain in Git history at commit `c0d9c7972e94f5add51af89bc45551fb05d87d9e`. Recover an experiment in a separate checkout if needed; do not mix alternative app entry points into this one.


## Code audit — 2026-09-15

Scope: controller and replay, capture and Retina mapping, detection and OCR, native input, session lifecycle, configuration/storage, IPC, calibration/overlay, frontend state and packaging. This is a code and offline regression audit, not certification of live automation.

### Fixed

| Finding | Effect before the fix | Change |
| --- | --- | --- |
| Stop while startup is queued | A queued worker could clear the Stop flag and start anyway | Capture a cancellation generation before dispatch; reject invalidated starts before platform checks |
| OCR timeout exceeds frame freshness | Slow OCR could trigger a stale-frame pause before optional monitoring's Continue policy | Bound live OCR by the remaining freshness budget; retain strict stale-frame input blocking |
| Live capture validates settings too late | Invalid edited calibration could reach display capture and coordinate arithmetic | Validate before capture; reject empty native capture images |
| Ambiguous hotbar calibration | Zero/overlapping slot spacing could report rod and food selected from the same pixels | Reject overlapping selection regions |
| Capture interval exceeds allowed frame age | Accepted settings could repeatedly pause between scheduled captures | Reject that timing combination |
| Empty recordings look successful | Zero frames returned zero differences | Reject empty replay input |
| Async frontend results replace newer state | A delayed snapshot could overwrite a newer event; replay imports/comparisons could race or leave old results visible after failure | Preserve newer session events, serialize imports, snapshot comparison settings and clear old results |
| Timeline selection uses object identity | Published snapshots replace objects, preventing a second click from collapsing the selected event | Match the event's timestamp and phase |

The preceding macOS hotbar crash fix remains in place: number-row raw keys avoid Enigo's off-main-thread Carbon keyboard-layout lookup.

### Remaining findings and validation gaps

1. **P2 — damaged saved files prevent launch.** `main.rs` still uses `expect` on `SharedState::new`; invalid JSON, invalid settings or unreadable statistics return an error before the UI opens. Writes are atomic, but there is no recovery screen. A future recovery flow should preserve the original files, explain the error, and let the user repair settings or reset statistics separately. This audit does not discard or replace saved data.
2. **P2 — catch detection is colour-based.** It counts matching yellow pixels inside the configured heading region rather than recognizing the text. Yellow scene/UI content in that region can satisfy the detector. Controller confirmation and notification rearming reduce repeated counts but do not prove the heading is a catch. A labelled live capture set is needed before selecting a more discriminating detector and claiming an accuracy improvement.
3. **Live acceptance remains incomplete.** The four supplied screenshots exercise idle, bite, reeling and catch, including shifted bite markers and Retina crop normalization. They do not cover food consumption, missed catches, arbitrary camera angles/lighting, other resolutions or long sessions. The rebuilt app's actual post-fix automated input, emergency shortcut delivery and Windows runtime still need supervised testing.
4. **Recording comparison has limited evidence.** Saved booleans cannot retest detector thresholds/crops or predict how the game would respond to changed actions. A successful replay demonstrates deterministic agreement only within the recorded segment.

Local validation: 51 Rust tests, the separately enabled original-screenshot test (all four images), four frontend geometry tests, Svelte/TypeScript checks, Clippy with warnings denied and formatting checks passed. `npm audit` reported zero known vulnerabilities; this does not constitute a Rust dependency security audit. A macOS production bundle was built. No live automation was started during this audit.
