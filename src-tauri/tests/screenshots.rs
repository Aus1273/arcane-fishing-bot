use arcane_fishing_bot_app::{
    capture::{crop_frame, map_reference_region, normalize_capture},
    config::{macbook_profile, rod_selection_region},
    detection::{self, EnergyReading},
    diagnostics::{self, Needs},
    ocr,
};
use std::{path::Path, sync::atomic::AtomicBool};

#[test]
#[ignore = "requires original PNGs in ARCANE_SCREENSHOT_DIR and Tesseract"]
fn original_macbook_screenshots() {
    let folder = std::env::var("ARCANE_SCREENSHOT_DIR").expect("set ARCANE_SCREENSHOT_DIR");
    let config = macbook_profile();
    config.validate().unwrap();
    let calibration = config.calibration.as_ref().unwrap();
    let cancel = AtomicBool::new(false);
    for (file, bite, caught, capacity) in [
        ("1..png", false, false, 956),
        ("2..png", true, false, 953),
        ("3.png", false, false, 954),
        ("4..png", false, true, 954),
    ] {
        let image = image::open(Path::new(&folder).join(file))
            .unwrap()
            .to_rgba8();
        let observation = diagnostics::observe(
            &image,
            &config,
            Needs {
                energy: true,
                bite: true,
            },
            &cancel,
        )
        .unwrap();
        assert_eq!(
            (observation.bite, observation.caught),
            (bite, caught),
            "{file}"
        );
        assert_eq!(observation.rod_selected, file != "1..png");
        assert!(!observation.food_selected);
        assert_eq!(
            observation.energy,
            Some(EnergyReading {
                usable: capacity,
                capacity
            })
        );
        for region in [
            config.red_region,
            config.yellow_region,
            config.hunger_region,
            rod_selection_region(calibration, 4),
        ] {
            let (x, y, w, h) = map_reference_region(region, calibration, 1512, 982);
            let backing =
                image::imageops::crop_imm(&image, x as u32 * 2, y as u32 * 2, w * 2, h * 2)
                    .to_image();
            assert_eq!(
                normalize_capture(backing, region, calibration, 1512, 982),
                crop_frame(&image, region),
                "{file}: Retina crop"
            );
        }
        if caught {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = std::fs::read(Path::new(&folder).join(file)).unwrap();
            let preview = diagnostics::inspect_png(&STANDARD.encode(bytes), &config).unwrap();
            assert!(preview.observation.caught);
            assert!(preview.observation.error.is_none());
            assert_eq!(preview.regions.len(), 4);
            for region in preview.regions {
                let png = STANDARD
                    .decode(
                        region
                            .data_url
                            .strip_prefix("data:image/png;base64,")
                            .unwrap(),
                    )
                    .unwrap();
                let decoded = image::load_from_memory(&png).unwrap();
                assert!(decoded.width() <= 800 && decoded.height() <= 500);
            }
        }
        if bite {
            let marker = image::imageops::crop_imm(&image, 1412, 535, 249, 287).to_image();
            for (x, y) in [(20, 20), (2000, 1000)] {
                let mut moved = image::RgbaImage::from_pixel(
                    config.red_region.width,
                    config.red_region.height,
                    image::Rgba([30, 30, 30, 255]),
                );
                image::imageops::replace(&mut moved, &marker, x, y);
                assert!(detection::has_bite(&moved, &config));
            }
        }
        println!(
            "{file}: bite={}, caught={}, Energy={capacity}/{capacity}",
            observation.bite, observation.caught
        );
    }
}

#[test]
fn cancelled_ocr_sends_no_process_work() {
    let image = image::RgbaImage::new(10, 10);
    assert!(ocr::read_energy(&image, &AtomicBool::new(true))
        .unwrap_err()
        .to_string()
        .contains("cancelled"));
}
