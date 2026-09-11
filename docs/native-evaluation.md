# Native macOS evaluation

## Decision

Keep Tauri 2 as the production interface. The SwiftUI/AppKit comparison is buildable and uses the real shared Rust controller, so a future native interface is feasible without duplicating fishing behaviour. This experiment does **not** establish that SwiftUI makes the complete application faster or more reliable. A separate production Mac UI would also need calibration, settings, runtime integration, packaging and continuing Windows parity.

The useful near-term result is native OCR: Apple Vision read all four original Energy crops correctly with lower warmed recognition latency. Its first invocation had a substantial delay, so replacing the current OCR backend immediately would be premature.

## What was implemented and verified

- A native SwiftUI sidebar, session state, metrics, selectable decision timeline and evidence panel. The interface explicitly labels the data as recorded replay observations.
- Actual Rust controller execution through the versioned `fishing-core-cli replay` interface. No Swift copy of the state machine or native input exists.
- Native Liquid Glass on the replay badge when macOS 26+ is available. The rest uses standard macOS controls and readable surfaces. Material fallback and Reduced Transparency support are implemented.
- A separate native-services CLI for Vision/Tesseract PNG OCR and explicit window capture with ScreenCaptureKit. The GUI never starts capture.
- Release compilation and locally signed `.app` packaging with Swift 6.2 and the macOS 26 SDK. The target is macOS 14+, but fallback rendering has not been tested on an older OS.
- Protocol validation including unknown action/version rejection and the actual Rust normal-cycle and focus-loss examples. Command Line Tools lacks XCTest, so this uses a standalone Swift check executable.
- Native GUI smoke testing: chose the repository through the file picker, loaded the normal cycle, selected Casting and verified “Would click once,” checked readiness, then loaded Focus loss and verified Paused with no proposed action. The dark interface was visually inspected for readable spacing and labels.

The production Tauri app has considerably more functionality. This prototype deliberately answers whether a native interface can consume the same controller and provide a useful session view; it is not an alternative production download.

## Recorded OCR measurements

Measurements were taken on an Apple M4 Pro, macOS 27.0, using the supplied 3024×1964 PNGs. Each image/engine/preparation group ran one warmup and five measured recognitions. The crop was `x=1988, y=1707, width=101, height=22`; expected usable/capacity values were 956/956, 953/953, 954/954 and 954/954.

| Variant | Correct measured readings | Median | Measured range |
|---|---:|---:|---:|
| Vision, original crop | 20/20 | 16.91 ms | 14.79–33.24 ms |
| Vision, current threshold treatment | 20/20 | 16.87 ms | 15.46–23.86 ms |
| Tesseract, threshold treatment, Swift process runner | 20/20 | 124.50 ms | 120.32–132.10 ms |

The first Vision warmup took **28.34 seconds**. Subsequent Vision process warmups took approximately 90–144 ms. The cause of that first delay was not isolated; it must remain part of the findings. It would be unsafe to advertise the 17 ms warmed result as a first-read guarantee. The complete per-read values, including that warmup, are retained in [ocr-results.json](../prototypes/macos/benchmarks/ocr-results.json).

Timing excludes source PNG decoding, crop extraction and threshold preparation. Vision timing covers request construction and synchronous recognition. Tesseract timing covers temporary PNG writing, process launch, recognition and waiting. Vision uses accurate recognition, English, and no language correction. Thresholding follows the current Rust rule, fourfold nearest-neighbour scaling and a 20-pixel white border.

A separate benchmark of the **production Rust Tesseract path** measured per-image medians of approximately **62.3–63.3 ms**, with all 20 readings correct; see [the production benchmark](benchmarks/macbook-offline-2026-09-10.json). That is a different runner, timing scope and run. The slower Swift subprocess result should not be used to exaggerate the expected improvement over the production application. No end-to-end game capture, CPU, battery or memory improvement has been demonstrated.

Twenty repeated reads of four static crops establish only this small example set. They do not cover partially depleted Energy, saturated values, different HUD scales, animation, missing text or false-positive readings. The 28-second first use also means a fresh one-shot Vision helper per observation would be the wrong integration.

## How native services should proceed

Vision is a promising candidate for a **persistent, warmed macOS OCR worker**, or an in-process native service, behind the existing observation interface. Before production adoption it needs bounded startup/readiness handling, cancellation, stale-result rejection, negative examples, unreadable-value tests and live latency measurements. Keep the existing backend until those checks justify a change. This can benefit Tauri without replacing its UI.

ScreenCaptureKit support compiled successfully. The helper can list windows, then capture an explicitly selected window at native pixel scale. Capture is read-only, excludes audio/cursor and refuses to overwrite an existing output file. **No ScreenCaptureKit live capture or streaming benchmark was run.** Consequently this evaluation makes no capture performance claim. The next supervised check should compare window coordinates, fullscreen behaviour, scaling and capture latency against the production capture backend.

## Repeat the comparison

From the repository root:

```sh
prototypes/macos/build-app.sh --open
swift run --package-path prototypes/macos -c release protocol-checks "$PWD"
python3 prototypes/macos/benchmark-ocr.py /path/to/original-screenshots
```

The `build/` and `.build/` directories are ignored. Source, the small benchmark report and repeatable commands are the only prototype artifacts intended for version control. See [prototype instructions](../prototypes/macos/README.md) for explicit capture commands.

Official API references: [SwiftUI glass effect](https://developer.apple.com/documentation/swiftui/view/glasseffect(_:in:)), [Vision text recognition](https://developer.apple.com/documentation/vision/vnrecognizetextrequest), [ScreenCaptureKit screenshot manager](https://developer.apple.com/documentation/screencapturekit/scscreenshotmanager). These document API capability; the measurements above come from the local experiment.
