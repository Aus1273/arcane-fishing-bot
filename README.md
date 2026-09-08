# Arcane Fishing Bot

Tauri **2**, Rust, Svelte 5, TypeScript and Tailwind. The active application is built from the root Cargo workspace. Older egui and Tauri implementations are preserved in `legacy/` for reference.

## Run

Install Rust, Node/npm, platform Tauri prerequisites, and Tesseract with English language data. See [macOS setup](BUILDING_MACOS.md).

```sh
npm ci
npm run tauri dev
```

`npm run dev` is a browser settings preview: it does not simulate a successful automation session or send game input.

## Configure and inspect

1. Select **MacBook Pro 14 - 3024x1964** for the supplied fullscreen HUD layout. Rod slot is 4; food slot is 5.
2. Use **Calibration check → Load a PNG** to inspect the detector results and region crops without sending input. Alternatively, use the five-second screen capture and focus Roblox during the countdown.
3. Save settings. Start a session and focus Roblox during the startup delay.
4. The controller resets and verifies the rod, casts, waits for a bite, reels, and confirms a new catch. Failed fishing cycles have bounded recovery attempts; focus loss or unsafe observations pause the session. Restart explicitly after resolving the displayed reason.

Automatic feeding defaults **off**. Energy is parsed as usable Energy / current capacity. Feeding uses an explicitly configured **absolute capacity threshold**, not their ratio. A feed counts only after two fresh readings show increased capacity and rod selection is restored. Verify the food and threshold in game before enabling it.

The profile uses screenshot pixels, including the top black strip, and the display containing desktop origin. Different HUD layouts need calibration. See [profile details](docs/calibration/macbook-pro-14.md).

## Verify and build

```sh
npm run check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --bin replay -- tests/replays/normal-cycle.json
npm run tauri build
```

Release output is in `target/release/`. `npm run compile` installs missing frontend dependencies, checks the frontend, and builds Tauri.

The optional original-screenshot regression requires the supplied PNGs and Tesseract:

```sh
ARCANE_SCREENSHOT_DIR='/path/to/original/screenshots' cargo test --release \
  --test screenshots -- --ignored --nocapture
```

## Structure and limits

[Architecture](docs/architecture.md) explains the controller, capture, input, configuration and replay modules. Settings are frozen during a session. Statistics are saved every 30 seconds and at orderly session exit; force-quitting can lose the latest interval. Settings and data use the existing OS directories for `com / arcane / fishing-bot`.

Screenshot and controller tests establish offline behavior. They do not establish live game input acceptance, animation timing, or unattended reliability. Windows code still needs a Windows build and runtime check. macOS needs Screen Recording and Accessibility permissions. OS capture already in progress cannot be interrupted; input runs separately, and OCR has a cancellation/timeout boundary.

Historical webhook and other unimplemented compatibility settings are not exposed as working features. This project is not affiliated with Roblox or Arcane Odyssey.
