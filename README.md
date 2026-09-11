# Arcane Fishing Bot

Tauri **2**, Rust, Svelte 5, TypeScript and Tailwind. The active application is built from the root Cargo workspace. Older egui and Tauri implementations are preserved in `legacy/` for reference.

## Run

Install Rust, Node/npm and platform Tauri prerequisites. Tesseract with English language data is required for Energy inspection or monitoring, and automatic feeding. Basic fishing with Energy monitoring disabled does not require it. See [macOS setup](BUILDING_MACOS.md).

```sh
npm ci
npm run tauri dev
```

`npm run dev` is a browser settings preview: it does not simulate a successful automation session or send game input.

## Configure and inspect

1. Select **MacBook Pro 14 - 3024x1964** for the supplied fullscreen HUD layout. Rod slot is 4; food slot is 5.
2. In **Calibration**, load a PNG and draw, move or resize the coloured detection boxes. Zoom and use the pixel inspector to sample colours. Run inspection again after edits. Alternatively, use the five-second screen capture and focus Roblox during the countdown.
3. Save settings and run **Check readiness**. Start with **Observe only**, then focus Roblox during the startup delay and fish manually. Suggested actions and detector evidence appear in the session timeline. This mode never constructs a native input device and never increases lifetime statistics.
4. Once a supervised rehearsal confirms the profile, select automation. **Control + Shift + F12** stops input system-wide and closes the calibration overlay. Automation requires successful shortcut registration. A second app launch focuses the existing instance and stops any active session.
5. The controller resets and verifies the rod, casts, waits for a bite, reels, and confirms a new catch. Failed fishing cycles have bounded recovery attempts; focus loss or unsafe observations pause the session. Restart explicitly after resolving the displayed reason.

Automatic feeding defaults **off**. Energy is parsed as usable Energy / current capacity. Feeding uses an explicitly configured **absolute capacity threshold**, not their ratio. A feed counts only after two fresh readings show increased capacity and rod selection is restored. Verify the food and threshold in game before enabling it.

The profile uses screenshot pixels, including the top black strip, and the display containing desktop origin. Different HUD layouts need calibration. See [profile details](docs/calibration/macbook-pro-14.md).

**Record replay data** is opt-in. It retains the first 30,000 controller ticks (about five minutes at the usual rate) in memory, including observation values, timing and requested actions; it does not automatically save screenshots. Export JSON to retain it after closing the app. Offline replay preserves original timestamps and can rerun controller settings. Changing regions or colour thresholds still requires image inspection. Exported recordings include the saved configuration; review that configuration before sharing them.

Energy monitoring is separately optional. When feeding is off, its failure policy can continue fishing or pause; feeding always requires verified readings. Screen capture errors remain blocking regardless of the OCR policy.

## Verify and build

```sh
npm run check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p fishing-core --bin fishing-core-cli -- replay tests/replays/normal-cycle.json
cargo test -p fishing-core
npm run tauri build
```

Release output is in `target/release/`. After `npm ci`, `npm run compile` checks the frontend and builds Tauri using npm scripts. Developer tools are separate: the shared-core CLI runs replay scenarios, and `cargo run --release --example benchmark -- CONFIG.json IMAGE.png` measures offline detection/OCR. Neither tool is included in the production app bundle.

The optional original-screenshot regression requires the supplied PNGs and Tesseract:

```sh
ARCANE_SCREENSHOT_DIR='/path/to/original/screenshots' cargo test --release \
  --test screenshots -- --ignored --nocapture
```

## Structure and limits

[Architecture](docs/architecture.md) explains the controller, capture, input, configuration and replay modules. Settings are frozen during a session. Statistics are saved every 30 seconds and at orderly session exit; force-quitting can lose the latest interval. Settings and data use the existing OS directories for `com / arcane / fishing-bot`.

The portable engine lives in `crates/fishing-core`. The separate [native macOS evaluation](docs/native-evaluation.md) uses that same engine through a versioned JSON bridge. It is a prototype, not an alternate automation app. See the [implementation record](docs/implementation-2026-09-10.md) for measurements and remaining live validation.

Screenshot and controller tests establish offline behavior. They do not establish live game input acceptance, animation timing, or unattended reliability. Windows code still needs a Windows build and runtime check. macOS needs Screen Recording and Accessibility permissions. OS capture already in progress cannot be interrupted; input runs separately, and OCR has a cancellation/timeout boundary.

Historical webhook and other unimplemented compatibility settings are not exposed as working features. This project is not affiliated with Roblox or Arcane Odyssey.
