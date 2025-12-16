# Arcane Fishing Bot

Arcane Fishing Bot is a Rust-powered automation tool for the Roblox game **Arcane Odyssey**.
It uses screen capture, OCR (Tesseract), and simulated input to automatically fish in-game with a Tauri + Svelte desktop shell.

## Features
- Cross-platform desktop application powered by Tauri with a Svelte + Tailwind UI.
- OCR-based bite detection using `rusty-tesseract`.
- Statistics and settings GUI for tracking performance.
- Configurable failsafe and adjustable resolution presets.

## Requirements
1. [Rust toolchain](https://rustup.rs/)
2. [Tesseract OCR](https://github.com/UB-Mannheim/tesseract)
3. Platform dependencies for window access and screen capture (e.g. X11, Accessibility permissions on macOS)

## Building (beginner-friendly, step-by-step)
Follow the steps below in order. Every command is meant to be run from a terminal/command prompt.

1. **Install the required tools**
   - **Rust toolchain**: Install from [rustup.rs](https://rustup.rs/) (adds `cargo` and `rustc`).
   - **Node.js 18+**: Install from [nodejs.org](https://nodejs.org/) (choose the LTS installer if unsure; npm comes with Node).
   - **Tesseract OCR**: Install the Tesseract binaries for your OS (e.g., `sudo apt install tesseract-ocr` on Ubuntu or the Windows installer from UB Mannheim).
2. **Verify the tools are on your PATH**
   ```bash
   cargo --version
   node --version
   npm --version
   tesseract --version
   ```
   Each command should print a version number. If any command fails, re-run the installer or restart your terminal so PATH changes take effect.
3. **Clone the project source code**
   ```bash
   git clone https://github.com/yourusername/arcane-fishing-bot.git
   cd arcane-fishing-bot
   ```
4. **Install JavaScript dependencies** (downloads the Tauri/Svelte packages)
   ```bash
   npm install
   ```
   The first run can take a few minutes while npm downloads packages into `node_modules/`.
5. **Build the Svelte UI** (emits a production-ready `dist/` folder)
   ```bash
   npm run build
   ```
   This step verifies the front-end compiles successfully.
6. **Compile the Rust core in release mode**
   ```bash
   cargo build --release
   ```
   The optimized binary is written to `target/release/arcane-fishing-bot`.
7. **Start the Tauri shell with the freshly built UI**
   ```bash
   npm run tauri dev
   ```
   The desktop window should open using the contents of `dist/`.

During development you can also run the Vite dev server with `npm run dev` to iterate on the UI in the browser, then restart the Tauri shell to pick up changes.

## Automatic compile helper
If you prefer a single command that runs all build steps in sequence, use the provided helper:

```bash
npm run compile
```

This script will:
- Install/update npm packages
- Build the Svelte UI
- Compile the Rust project in release mode

After it completes, run `npm run tauri dev` to launch the desktop app with the compiled assets.

## Running
To start the Tauri shell with the latest UI build:

```bash
$ npm run tauri dev
```

## Loop review and future ideas

See [docs/suggestions.md](docs/suggestions.md) for a review of the current control loops plus suggestions on where to add new capabilities or trim redundant work.

## Disclaimer
Use responsibly and at your own risk. This project is provided for educational purposes and is not affiliated with
Roblox or Arcane Odyssey.
