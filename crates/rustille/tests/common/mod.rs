//! Helpers shared by the integration tests.
//!
//! Fixtures are generated in-process rather than checked in as binaries, so the
//! tests stay readable and the repository stays small.

#![allow(dead_code)]

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgba, RgbaImage};

/// A `width`×`height` image split into `cells` columns of alternating black and
/// white, fully opaque.
pub fn vertical_bars(width: u32, height: u32, bars: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, _| {
        let bar = x * bars / width;
        if bar % 2 == 0 {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    })
}

/// A checkerboard with `size`-pixel squares.
pub fn checkerboard(width: u32, height: u32, size: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        if (x / size + y / size) % 2 == 0 {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    })
}

/// A horizontal black-to-white ramp.
pub fn horizontal_ramp(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, _| {
        let value = (x * 255 / width.max(1)) as u8;
        Rgba([value, value, value, 255])
    })
}

/// A red/green/blue banded image, used to exercise colour output.
pub fn rgb_bands(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, _| match x * 3 / width {
        0 => Rgba([255, 0, 0, 255]),
        1 => Rgba([0, 255, 0, 255]),
        _ => Rgba([0, 0, 255, 255]),
    })
}

/// A fully transparent white image.
pub fn transparent(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 0]))
}

/// Encodes an image into memory.
pub fn encode(image: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    let dynamic = image::DynamicImage::ImageRgba8(image.clone());
    // JPEG, BMP, GIF and TIFF cannot all store an alpha channel.
    let dynamic = match format {
        ImageFormat::Jpeg | ImageFormat::Bmp | ImageFormat::Tiff => {
            image::DynamicImage::ImageRgb8(dynamic.to_rgb8())
        }
        _ => dynamic,
    };
    dynamic
        .write_to(&mut Cursor::new(&mut bytes), format)
        .expect("fixtures encode successfully");
    bytes
}

/// Writes an encoded fixture into a scratch directory next to the test binary.
pub fn write_fixture(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = scratch_dir();
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("fixture is writable");
    path
}

/// A directory the tests may write to. It lives under `target/`, so a build
/// never leaves stray files in the working tree.
pub fn scratch_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixtures");
    std::fs::create_dir_all(&dir).expect("scratch directory is creatable");
    dir
}

/// Compares against a checked-in golden file.
///
/// Run the suite with `RUSTILLE_BLESS=1` to rewrite the goldens after an
/// intentional change.
pub fn assert_golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.txt"));

    if std::env::var_os("RUSTILLE_BLESS").is_some() {
        std::fs::write(&path, format!("{actual}\n")).expect("golden file is writable");
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "missing golden file {}: {error}\nre-run with RUSTILLE_BLESS=1 to create it",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected.trim_end_matches('\n'),
        "golden mismatch for {name}; re-run with RUSTILLE_BLESS=1 if this is intentional"
    );
}
