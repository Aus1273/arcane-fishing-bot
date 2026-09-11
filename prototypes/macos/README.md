# Native macOS comparison

A bounded SwiftUI/AppKit comparison of the session interface. The production application remains Tauri. This prototype displays results from the **same Rust controller**, invoked through `fishing-core-cli` protocol v1. It does not implement fishing logic, screen capture or native input in the GUI.

The prototype has a replay timeline, selectable decision evidence, actual controller/build readiness and a native Liquid Glass badge on macOS 26+. Earlier systems use a material fallback; Reduced Transparency uses an opaque surface. It targets macOS 14+ and requires Swift 6.2 / the macOS 26 SDK to build the glass API.

## Build and open

Run from the repository root:

```sh
prototypes/macos/build-app.sh --open
```

This builds the core CLI and a locally signed app under `prototypes/macos/build/Arcane Native Comparison.app`. Build products are ignored by Git. Opening the app directly from Finder may require choosing the repository with the **Project** toolbar button. **Load replay** provides normal-cycle and focus-loss examples, or a file picker for another scenario.

Validate the Swift protocol reader against actual Rust scenarios:

```sh
swift run --package-path prototypes/macos -c release protocol-checks "$PWD"
```

The standalone check runner works with Command Line Tools, which do not include XCTest. It rejects incompatible protocol versions, unexpected response kinds, empty replays and unsupported actions; it verifies actual normal-cycle and focus-loss output from the Rust executable.

## Native service experiments

The separate `native-services` executable measures Vision OCR on local PNGs and provides an explicit ScreenCaptureKit capture experiment. Nothing captures the display unless a `capture` command is requested. It never sends mouse or keyboard input.

```sh
swift build --package-path prototypes/macos -c release
prototypes/macos/.build/release/native-services --help
python3 prototypes/macos/benchmark-ocr.py /path/to/original-screenshots
```

The benchmark expects `1..png`, `2..png`, `3.png`, `4..png` and reports raw OCR text, parsed values, warmups and measured samples. It compares Vision with and without thresholding against Tesseract through the same Swift runner. Tesseract must already be installed. The helper checks Homebrew locations and PATH, or accepts `--tesseract /absolute/path`.

Optional manual capture experiment (not run during implementation):

```sh
prototypes/macos/.build/release/native-services capture --list-windows
prototypes/macos/.build/release/native-services capture --window-id WINDOW_ID --output /absolute/new-window.png
```

The commands can trigger macOS Screen Recording permission. Pick the game's window ID from the first command. Capture excludes audio and the cursor, uses the window's native pixel scale, and refuses an existing output path. The helper reports screenshot capture time; it does not test streaming performance.

See [the measured evaluation](../../docs/native-evaluation.md) before considering a production replacement. In particular, warm Vision performance omits a substantial first-use delay observed in this run.
