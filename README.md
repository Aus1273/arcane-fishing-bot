# Arcane Fishing Bot

Arcane Fishing Bot is a Rust-powered automation tool for the Roblox game **Arcane Odyssey**.
It uses screen capture, OCR (Tesseract), and simulated input to automatically fish in-game with a Tauri/Svelte desktop shell.

## Features
- Cross-platform desktop application powered by Tauri with a Svelte + Tailwind UI.
- OCR-based bite detection using `rusty-tesseract`.
- Statistics and settings GUI for tracking performance.
- Configurable failsafe and adjustable resolution presets.

## Requirements
1. [Rust toolchain](https://rustup.rs/)
2. [Tesseract OCR](https://github.com/UB-Mannheim/tesseract)
3. Platform dependencies for window access and screen capture (e.g. X11, Accessibility permissions on macOS)

## Building
Follow these steps to produce a distributable Tauri application:

1. **Install prerequisites**
   - Rust toolchain with `cargo` and `rustup`
   - Node.js 18+ with `npm`
   - Tesseract OCR binaries
   - Tauri system libraries
     - **Linux (Debian/Ubuntu):** `sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev pkg-config libssl-dev`
     - **macOS:** see [BUILDING_MACOS.md](BUILDING_MACOS.md) for a walkthrough and signing notes
     - **Windows:** install the WebView2 runtime (via Edge) and build tools noted in the [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
2. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/arcane-fishing-bot.git
   cd arcane-fishing-bot
   ```
3. **Install frontend dependencies**
   ```bash
   npm install
   ```
4. **Build the Svelte UI** (outputs to `dist/`)
   ```bash
   npm run build
   ```
5. **Build the Tauri shell and Rust backend**
   ```bash
   cd src-tauri
   cargo tauri build
   ```

The bundled application will be created in `src-tauri/target/release/`. During development, you can use `npm run dev` and `cargo tauri dev` in separate terminals to hot-reload the UI.

## Running
During development, run the Vite dev server and Tauri shell in separate terminals:

```bash
$ npm run dev
$ cd src-tauri && cargo tauri dev
```

For production builds, run the bundle produced by `cargo tauri build`.

## Disclaimer
Use responsibly and at your own risk. This project is provided for educational purposes and is not affiliated with
Roblox or Arcane Odyssey.
