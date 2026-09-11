# Implementation and review record

Work began September 10 and resumed September 11, 2026 after interrupted tool sessions.

## Production changes

Tauri 2 remains the production interface. The redesigned Svelte interface adds readiness checks, observe-only rehearsal, a decision timeline, capture/analysis/frame-age metrics, optional recordings, native JSON export and offline replay comparison. The calibration editor displays a full screenshot with coloured regions, pointer and keyboard editing, zoom, a pixel magnifier and colour sampling. The existing live wireframe overlay remains mutually exclusive with capture and automation.

The shared `fishing-core` crate contains the controller, detectors, configuration and replay without dependencies on Tauri, capture or native input. The desktop library re-exports those modules directly, preserving Rust import paths and saved configuration compatibility without duplicate wrapper files. Both the desktop app and native prototype consume this engine.

Observe-only never constructs a native input device; rehearsals do not change lifetime statistics. Input remains focus-checked. The emergency stop is Control + Shift + F12, and automation requires successful registration. A second app launch stops the existing session and focuses that instance. Start preflight preserves Stop/shutdown requests, including while OCR readiness is running; lifecycle tests cover that race.

Energy monitoring is optional and defaults off. Feeding requires monitoring and verified capacity increases followed by rod restoration before a feed counts. An optional monitoring failure can continue without feeding, while a capture failure remains blocking. Tesseract discovery now supports Finder launches without a shell PATH and verifies English language data. OCR remains bounded and cancellable.

Recording is opt-in and holds the first 30,000 raw controller ticks, including original observation sequence/timestamps and requested actions. The timeline separately holds 200 significant events. Export uses a native Save dialog and atomic file replacement; import/replay has 32 MiB and 30,000-frame bounds. Truncated or interrupted evidence is marked incomplete. Settings comparisons rerun observations, not original pixels, so image threshold changes still require new inspection.

## Native comparison and performance

The SwiftUI/AppKit prototype builds as a separate local app and runs actual Rust replay scenarios. Native glass is available on macOS 26+, with a material fallback. It is a recorded-data evaluation, not a second production automation application. [Native findings](native-evaluation.md) include exact scope, instructions and measurements.

All four supplied PNGs produced the expected bite/catch/rod-selection signals. The production offline benchmark measured 25 detector evaluations and five OCR reads per PNG. Detector medians were 6.2–8.5 ms; production Tesseract medians were 62.3–63.3 ms, with 20/20 correct Energy readings. [Raw measurements](benchmarks/macbook-offline-2026-09-10.json) include tails and sample counts. These exclude live capture and do not establish in-game performance.

Vision recognised the same 20 readings correctly with a warmed median near 16.9 ms, but the first warmup took 28.34 seconds. It remains experimental. ScreenCaptureKit window capture compiled but was not exercised. A persistent warmed service with bounded startup and stale-result handling is the appropriate next experiment, rather than a new process for each read. No GUI efficiency advantage, battery improvement or live capture speedup has been demonstrated.

## Response to structural feedback

| Concern | Action or explanation |
|---|---|
| Custom compile wrapper | Removed `scripts/compile.js`. `npm run compile` now directly runs the check and Tauri build. Dependency installation stays explicit with `npm ci`. |
| Split TypeScript configuration | Merged into a single `tsconfig.json`. No shared base configuration is needed for this application. |
| Tiny `app.d.ts` | Removed it and declared `svelte` and `vite/client` types in the unified config, preserving the required types. |
| Generic `utils.ts` | Removed unused UI scaffolding. Only the button used the class helper afterward, so it was folded into that component and the general utility file removed. Removed `class-variance-authority` and `clsx`; kept Tailwind class merging for button overrides. |
| Too many UI files | Removed unused card, tab, badge, input, label, select and switch wrappers. Kept components with actual independent behaviour: calibration, session evidence, settings and the read-only overlay. Added repeatable formatting, including formerly compressed Svelte/CSS. |
| `src/bin` contains source | This is Cargo's standard location for the source of additional executables. It does not mean compiled programs belong among source files. The shared engine's CLI intentionally uses this convention. |
| Shipping code versus developer tools | Removed the redundant desktop replay executable; use the shared-core CLI. Moved the offline benchmark into Cargo's `examples/`, so it cannot be bundled as an app executable. Production output is `target/release/bundle/macos/Arcane Fishing Bot.app`; build directories are ignored. |
| Generated files mixed into source | Replaced the custom icon-generation build script with Tauri's standard build call. Existing icon resources are explicit source assets, not generated into the source tree at every build. Prototype build products remain ignored. |
| More than one Rust package | The shared core has two real consumers, the desktop app and native comparison, and independently runnable tests. This boundary isolates native input and lets both interfaces use exactly the same controller. Merging it into the UI would remove a useful test and reuse boundary. |

The broad insults supplied with the feedback are not reproducible defects. They do not establish correctness or failure. The concrete findings above were either fixed or checked against the actual build conventions. Rust's [Cargo layout documentation](https://doc.rust-lang.org/cargo/guide/project-layout.html) explains `src/bin`, `examples`, and test placement.

## Validation boundary

Portable controller/replay/configuration tests, runtime cancellation and no-input tests, frontend geometry tests, formatting/type checks, static linting and local macOS release builds are part of this implementation. CI is configured for portable core checks on Linux and desktop checks/builds on Windows and macOS; configuring that workflow does not prove a Windows run has passed.

The final local check passed 41 Rust tests, four frontend geometry tests, Svelte type checks, formatting checks and Clippy with warnings denied. The separate original-screenshot regression also passed, including Retina crop normalization and moved bite-marker examples. The production macOS bundle contains one executable, with developer tools excluded. The packaged app opened successfully; readiness identified the missing Screen Recording permission and confirmed emergency-shortcut registration. Native overlay Show/Hide correctly locked and restored controls. The synthetic shortcut test did not establish actual global-key delivery, which still needs a physical-key check. GUI file-picker testing was interrupted by computer-use tool failures; earlier browser checks confirmed original screenshot rendering.

Supervised game acceptance remains outstanding: fullscreen/camera variants, cast/reset acceptance, click cadence under load, low Energy and food consumption, disappearing notifications, focus changes and long sessions. Start with feeding disabled and observe-only recording. Replays can validate the captured decisions; only a live game test can establish that the game accepts them. No unattended reliability claim is made.
