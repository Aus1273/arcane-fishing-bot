> Historical review before the controller restructure. See [current architecture](architecture.md) and [README](../README.md) for active behavior and remaining limits.

# Code review and optimization pass — 2026-09-07

A subsequent [MacBook screenshot calibration](calibration/macbook-pro-14.md) adds Energy ratio OCR, configurable slots, Retina coordinate mapping and a specialized red-bubble detector. The findings below describe the preceding general repair pass.

## Architecture and scope

The repository contains two independent applications. Tauri uses `src-tauri/` and the Svelte frontend; the root Cargo package builds 2,797 lines of legacy egui code. The text source snapshot is not compiled. The old documentation mixed these applications and described legacy behavior as though it existed in Tauri.

This pass repairs and optimizes the active Tauri application and reviews the legacy implementation for migration risks. The legacy source is retained unchanged as reference; it has not been certified or optimized by this pass. No Tauri v2 or Electron migration was made. The downloaded directory has no `.git` history, so no commit or Git diff was available.

## Findings addressed

| Priority | Finding | Resolution |
| --- | --- | --- |
| High | Shared Button did not forward click events, so controls could silently do nothing | Forward native click events |
| High | Tabs subscribed to the context object rather than its writable store | Subscribe to the actual value store |
| High | Select dispatched a CustomEvent but handlers expected a native event target | Forward the native change event |
| High | Desktop command failures silently activated fake browser state | Propagate desktop errors and display them in the UI; label browser preview |
| High | Repeated Start reset live session state; Stop counted sessions repeatedly and joined on the UI thread | Serialize worker creation; ignore duplicate starts/stops; finalize once in the worker; stop asynchronously |
| High | Zero feeding interval caused modulo-by-zero; zero scan interval allowed tight loops | Validate config on load/save, including finite lure values and capture dimensions |
| High | Lifetime runtime was overwritten each session, catch counts were not reset, and lifetime stats were never persisted | Reset per-session counters; accumulate runtime; save separate Tauri statistics at worker exit |
| High | Build helper compiled legacy egui; Tauri features mismatched the allowlist; PNG icon was missing | Build via Tauri CLI, remove unused permissions, derive PNG from existing fallback icon bytes |
| Medium | Color scans counted all matching pixels even after reaching the decision threshold | Stop at the threshold, preserving the color-distance rule and detection decision |
| Medium | Timeout IPC used snake_case arguments instead of Tauri's default camelCase | Send `lureValue` |
| Medium | Any config edit could re-trigger timeout IPC and overwrite the independent reel timeout | React only to lure changes and show bite timeout separately |
| Medium | Sleeps delayed cancellation and held the configuration read guard during startup | Snapshot startup delay; check cancellation during short sleep slices |
| Medium | Input initialization panic could leave session marked running | Report initialization failure and finalize worker state; catch worker panics |
| Medium | Feeding counted success even when input calls failed | Report failures and count only sequences whose input calls succeed |
| Medium | Invalid/failed OCR was reported as 100% hunger | Return an error for missing or out-of-range OCR results |
| Medium | Temporary OCR filenames could collide | Use a unique temporary directory with automatic cleanup |
| Medium | Config writes could truncate the existing file | Write and sync a temporary file, then replace the destination; test replacement |
| Medium | Invalid capture point silently fell back to another monitor | Require the selected display and validate region bounds |
| Medium | Browser preview editing mutated saved config by reference, making Reset ineffective | Clone preview state and saved config |
| Medium | Async event registration could leak after component destruction | Immediately unregister if mount has already been destroyed |
| Medium | Unimplemented compatibility settings were presented as working features | Disable them and explain the limitation |
| Low | Switch thumb CSS relied on a peer selector across a nested element | Drive thumb position directly from checked state |
| Low | Missing Svelte type checks allowed multiple wiring/type errors to survive builds | Add `npm run check` and fix reported errors |
| Low | Vite could move off Tauri's expected port | Require port 5173 |
| Low | Empty OCR handler and unused dependency/methods added dead scaffolding | Remove them from the active backend |

## Performance effects

The detector now scans only until it finds enough matching pixels. For a uniformly matching default red region, this means 1,397 matching pixels instead of 419,328; real savings depend on pixel placement. Negative scans still examine the whole image. This is an algorithmic work reduction, not a measured end-to-end speedup. Screen capture and external Tesseract execution may still dominate runtime.

Error reporting applies a cancellable 250 ms backoff to reduce failure-loop CPU/log traffic. Configuration changes no longer issue repeated timeout commands unrelated to lure changes. Release builds use thin LTO, one codegen unit and stripped symbols. No capture cache was added because stale screenshots could invalidate catch confirmation.

## Legacy application findings (not changed)

- `AdvancedDetector::advanced_color_detection` compares matching pixels to all other matches: quadratic work in the number of matches. A spatial grid/local-neighborhood scan would be a better migration target.
- Basic detection collects every pixel into a vector before Rayon scans it; this allocation is avoidable.
- Screenshot caching clones full images and uses formatted string keys. Capture assumes the first monitor and reconstructs image dimensions, which needs review on scaled displays.
- The bot and webhook manager spawn workers without retaining join handles. Rapid stop/start can let old workers observe a newly true running flag.
- `stop` retains the start timestamp and can count the same session again.
- Input and OCR objects are recreated in clone/start paths; the GUI-side input instance therefore does not reliably describe worker input timing.
- Webhook responses are ignored and the queue is drained before send success. Failed messages can disappear; stopped workers may leave summaries unsent.
- `fish_per_feed == 0` remains unsafe in the legacy app.

The historical `docs/suggestions.md` referred only to this legacy code; its old line references should not be treated as current Tauri documentation.

## Remaining limits

- No live-game test, Retina/multiple-monitor calibration, Windows/Linux build, or installed-app permission test has been performed. Pixel colors/regions and the feeding slot sequence require in-game verification.
- Tesseract and screen capture are synchronous. Stop signals immediately and ordinary waits check every 20 ms, but an in-flight OS/OCR call can delay actual worker exit.
- Lifetime statistics save at session end. Force-quit/crash before finalization may lose the current session; periodic checkpoints and graceful app-close coordination remain future work.
- Runtime UI updates occur between fishing attempts, so long bite/reel phases can leave displayed minutes temporarily stale.
- Defaults target a 3440×1440 layout; choose/calibrate regions before starting.
- Failsafe, screenshots, webhooks, advanced detection and config auto-save are not implemented in Tauri. This pass did not add those features.
- Compatible npm audit fixes reduced findings from 13 to 7 (six moderate, one high at review time). Remaining findings involve the Svelte/Vite dependency chain; npm proposes major upgrades. A full frontend upgrade and Rust dependency security audit remain separate work.

## Validation

- Svelte type check: zero errors/warnings; production frontend build passed.
- Six Rust unit tests passed: validation, OCR parsing, threshold detection, timeout bounds, config defaults, and atomic file replacement.
- Rust Clippy with warnings denied passed.
- Browser preview checks: start, stop, tab switching, save, reset, profile selection. These verify UI behavior, not native automation.

- `npm run tauri build` passed; produced the optimized macOS executable at `src-tauri/target/release/Arcane Fishing Bot` (approximately 3.8 MB). No installer/notarization was configured or verified.
