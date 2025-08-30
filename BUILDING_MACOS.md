# Building on macOS

These steps show how to compile and run **Arcane Fishing Bot** natively on macOS.

## Prerequisites

1. **Rust toolchain** – install via [rustup](https://rustup.rs/).
2. **Xcode Command Line Tools** – run `xcode-select --install` from Terminal.
3. **Tesseract OCR** – `brew install tesseract` (required by `rusty-tesseract`).

## Build

```bash
# clone and enter the project
$ git clone https://github.com/yourusername/arcane-fishing-bot.git
$ cd arcane-fishing-bot

# format and compile
$ cargo fmt
$ cargo build --release
```

The compiled binary will be at `target/release/arcane-fishing-bot`.

## Run

Grant the application **Accessibility** and **Screen Recording** permissions in
System Settings → *Privacy & Security*. Then execute:

```bash
$ ./target/release/arcane-fishing-bot
```

Enjoy automating your fishing sessions on macOS!
