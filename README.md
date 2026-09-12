# Arcane Fishing Bot

A desktop fishing assistant for Arcane Odyssey, built with Tauri 2, Rust and Svelte. One production app, with three screens:

- **Session** — observe or automate, start/stop, session status and optional recording diagnostics.
- **Calibration** — select a screen profile, edit detection areas on a screenshot, inspect detector results and show the live wireframe overlay.
- **Settings** — rod, food, Energy monitoring and advanced timing.

## Open the app

Install the [development prerequisites](docs/development.md), then run:

```sh
npm ci
npm start
```

To create the installable application:

```sh
npm run package
```

On macOS, open `target/release/bundle/macos/Arcane Fishing Bot.app`. Windows installers are under `target/release/bundle/`. `npm run dev` is only the browser preview used by the desktop development server.

## First session

1. In **Calibration**, select the screen profile and load a screenshot. The supplied 14-inch MacBook profile uses 3024×1964 screenshot pixels, rod slot 4 and food slot 5. [Profile details](docs/calibration/macbook-pro-14.md).
2. Adjust detection boxes if needed, inspect the image and save changes. Keep Roblox fullscreen on the display containing desktop origin.
3. In **Session**, check setup and start **Observe only** with feeding disabled. Fish manually while reviewing the proposed actions. Observe mode sends no input and does not change lifetime statistics.
4. Enable automation only after a supervised test confirms the calibration and timing. **Ctrl + Shift + F12** stops the session and closes the overlay. Losing Roblox focus pauses input.

macOS requires Screen Recording access; automation also requires Accessibility. Energy OCR requires Tesseract with English data. Monitoring and feeding are optional and off by default. Feeding uses **absolute Energy capacity**, not a percentage, and requires a verified increase before it counts.

Recordings are optional, in memory, and limited to the first 30,000 controller ticks. Open **Session diagnostics & recordings** to export JSON or replay it. Image calibration changes require fresh screenshot inspection; replay only reruns the recorded observations.

## Development

See [development and verification](docs/development.md) for builds and tests, and [architecture](docs/architecture.md) for code ownership. Saved settings retain the existing `com / arcane / fishing-bot` application directories and compatibility fields.

Offline tests and macOS/Windows builds have passed on the preceding version. Live game acceptance, timing under load, physical emergency-shortcut delivery and unattended reliability still require supervised testing. This app is not affiliated with Roblox or Arcane Odyssey.
