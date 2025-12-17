use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

// Fallback transparent icon to satisfy Windows resource generation without
// requiring a checked-in binary asset.
const FALLBACK_ICON: &[u8] = &[
    0, 0, 1, 0, 3, 0, 16, 16, 0, 0, 0, 0, 32, 0, 75, 0, 0, 0, 54, 0, 0, 0, 24, 24, 0, 0, 0, 0, 32,
    0, 81, 0, 0, 0, 129, 0, 0, 0, 32, 32, 0, 0, 0, 0, 32, 0, 103, 0, 0, 0, 210, 0, 0, 0, 137, 80,
    78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0, 0, 0, 16, 8, 6, 0, 0, 0,
    31, 243, 255, 97, 0, 0, 0, 18, 73, 68, 65, 84, 120, 156, 99, 96, 24, 5, 163, 96, 20, 140, 2, 8,
    0, 0, 4, 16, 0, 1, 85, 55, 90, 208, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130, 137, 80, 78,
    71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 24, 0, 0, 0, 24, 8, 6, 0, 0, 0, 224,
    119, 61, 248, 0, 0, 0, 24, 73, 68, 65, 84, 120, 156, 237, 193, 1, 1, 0, 0, 0, 128, 144, 254,
    175, 238, 8, 10, 0, 128, 170, 1, 9, 24, 0, 1, 213, 14, 105, 114, 0, 0, 0, 0, 73, 69, 78, 68,
    174, 66, 96, 130, 137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 32, 0,
    0, 0, 32, 8, 6, 0, 0, 0, 115, 122, 122, 244, 0, 0, 0, 46, 73, 68, 65, 84, 120, 156, 237, 206,
    49, 1, 0, 0, 8, 195, 176, 129, 127, 207, 67, 6, 79, 106, 160, 153, 182, 249, 108, 95, 239, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 14, 64, 180, 3, 61, 200, 192, 8, 180, 0, 0, 0, 0,
    73, 69, 78, 68, 174, 66, 96, 130,
];

fn main() {
    ensure_fallback_icon();
    tauri_build::build();
}

fn ensure_fallback_icon() {
    let Ok(out_dir) = env::var("OUT_DIR") else {
        eprintln!("warning: failed to read OUT_DIR for fallback icon");
        return;
    };

    let icon_dir = PathBuf::from(out_dir).join("icons");
    if let Err(error) = fs::create_dir_all(&icon_dir) {
        eprintln!("warning: failed to create icon dir {icon_dir:?}: {error}");
        return;
    }

    let icon_path = icon_dir.join("icon.ico");
    if icon_path.exists() {
        return;
    }

    if let Err(error) = File::create(&icon_path).and_then(|mut file| file.write_all(FALLBACK_ICON))
    {
        eprintln!("warning: failed to write fallback icon to {icon_path:?}: {error}");
    }
}
