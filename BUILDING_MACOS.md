# Building on macOS

The root Cargo workspace builds the Tauri 2 application. Historical code is under `legacy/`.

## Prerequisites

- Rust (`cargo` and `rustc`).
- Node.js and npm.
- Xcode Command Line Tools: `xcode-select --install`.
- Tesseract with English data: `brew install tesseract`.

From the project root:

```sh
npm ci
npm run tauri dev
```

For automation, grant Accessibility and Screen Recording access in System Settings → Privacy & Security. Select and inspect the correct profile before starting. Focus Roblox during the startup delay. Losing focus pauses the controller.

```sh
npm run check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run tauri build
```

Release output is in `target/release/`. Browser preview (`npm run dev`) only edits settings in memory. PNG inspection, screen capture and automation require the desktop app. The screenshot inspector does not send game input.

To build only the macOS application bundle:

```sh
npm run tauri build -- --bundles app
```

Open `target/release/bundle/macos/Arcane Fishing Bot.app`. This is a local unsigned build.
