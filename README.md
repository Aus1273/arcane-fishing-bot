# Arcane Fishing Bot

Arcane Fishing Bot is a Rust-powered automation tool for the Roblox game **Arcane Odyssey**. 
It uses screen capture, OCR (Tesseract), and simulated input to automatically fish in-game with a simple desktop GUI.

## Features
- Cross-platform desktop application built with `egui`.
- OCR-based bite detection using `rusty-tesseract`.
- Statistics and settings GUI for tracking performance.
- Configurable failsafe and adjustable resolution presets.

## Requirements
1. [Rust toolchain](https://rustup.rs/)
2. [Tesseract OCR](https://github.com/UB-Mannheim/tesseract)
3. Platform dependencies for window access and screen capture (e.g. X11, Accessibility permissions on macOS)

## Building
Clone the repository and build in release mode:

```bash
$ git clone https://github.com/yourusername/arcane-fishing-bot.git
$ cd arcane-fishing-bot
$ cargo build --release
```

See [BUILDING_MACOS.md](BUILDING_MACOS.md) for a macOS-specific walkthrough.

## Running
The compiled binary will be located at `target/release/arcane-fishing-bot`.
Execute it and grant any required screen-access permissions:

```bash
$ ./target/release/arcane-fishing-bot
```

The bot window provides controls for starting/stopping automation and adjusting settings.

## Disclaimer
Use responsibly and at your own risk. This project is provided for educational purposes and is not affiliated with
Roblox or Arcane Odyssey.
