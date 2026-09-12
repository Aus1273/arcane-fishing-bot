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

Build output belongs in `target/` and `dist/`; dependencies belong in `node_modules/`. The release app is `target/release/bundle/macos/Arcane Fishing Bot.app`, and Windows installers are under `target/release/bundle/`. Local builds are unsigned. The packaged app contains the production executable; the CLI and benchmark below are developer tools.

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
