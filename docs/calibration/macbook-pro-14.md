# MacBook Pro 14-inch calibration

Select the MacBook Pro 14 - 3024x1964 profile in Settings, inspect a PNG, then save. The bundled profile is `configs/macbook-pro-14-3024x1964.json`; existing settings are preserved until a profile is selected and saved.

Original PNGs are **3024 x 1964**, including the top black strip. Coordinates refer to screenshot pixels, not macOS logical points or resized chat images.

| Region | X | Y | Width | Height |
| --- | ---: | ---: | ---: | ---: |
| Broad bite search | 0 | 70 | 2630 | 1490 |
| Fully visible Fish Caught heading | 2730 | 1598 | 190 | 28 |
| Energy numbers | 1988 | 1707 | 101 | 22 |
| Rod slot 4 selection border | 1258 | 1834 | 117 | 11 |

Capture uses the display containing desktop origin. Simulated Retina mapping reproduces the original regions exactly. Other layouts and live OS capture still need validation. Aspect-ratio mismatch is rejected.

## Detectors

- Bite: red `[233,96,88]`, average RGB tolerance 36, upright connected component of at least 150 pixels with white flanks. The marker can move within the broad region. Bottom notoriety stars and the right player list are excluded. This is a shape/color heuristic, not exact symbol recognition.
- Catch: yellow `[255,255,84]`, tolerance 20, at least 250 pixels in the heading region. Confirmation requires two distinct frames at least 100 ms apart. A lingering notification must disappear before another catch can register.
- Energy: threshold and enlarge the numeric crop, then run Tesseract. Preserve usable Energy and current capacity. Feeding uses an explicit absolute capacity threshold; their ratio is not a measure of food reserve. Invalid readings fail closed.
- Rod: slot 4; verify its selection border. Startup/recovery resets an already equipped rod by toggling it off, verifying, then selecting and verifying again.
- Food: slot 5, automatic feeding disabled by default. Configure a capacity threshold from your character before enabling it. A feed needs verified selection, two fresh readings of increased capacity, and restored rod selection. Low Energy and actual eating remain unverified in game.

## Provisional timings

Startup 5000 ms, scan delay 50 ms, reel clicks 70 ms apart, reel timeout 25000 ms, and two recovery attempts. Bite timeout retains the existing lure-derived formula. Still images cannot establish correct game timings or input acceptance. Capture/processing add latency; no live throughput claim is made.

## Original screenshot results

| File | Bite | Catch | Energy | Rod selected |
| --- | --- | --- | --- | --- |
| `1..png` | No | No | 956 / 956 | No |
| `2..png` | Yes | No | 953 / 953 | Yes |
| `3.png` | No | No | 954 / 954 | Yes |
| `4..png` | No | Yes | 954 / 954 | Yes |

Screenshot 3 contains part of a sliding catch notification; the heading crop excludes it in that frame. Tests also relocate the bite marker, check Retina mapping and exercise the PNG inspector. All four pass through production detection and OCR. They cannot establish reliability across other scenes or animation sequences.

```sh
ARCANE_SCREENSHOT_DIR='/path/to/original/screenshots' cargo test --release \
  --test screenshots -- --ignored --nocapture
```

Originals stay outside the repository. No live game input was performed during this verification.
