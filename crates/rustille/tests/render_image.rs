//! End-to-end rendering: decoding, resizing, colour, dithering, error paths.
//!
//! Decoding is optional, so this file only exists when a decoder is compiled
//! in. `golden.rs` covers the raw-pixel paths in every configuration.

#![cfg(feature = "_image")]

mod common;

use image::ImageFormat;
use rustille::{
    Background, ColorMode, Dither, ErrorKind, Fit, RenderOptions, Renderer, render_bytes,
    render_file, render_rgba,
};

fn renderer(options: RenderOptions) -> Renderer {
    Renderer::new(options)
}

#[test]
fn every_bundled_format_decodes_to_the_same_art() {
    let source = common::checkerboard(64, 64, 8);
    let options = RenderOptions::default().width(16);

    let png = render_bytes(&common::encode(&source, ImageFormat::Png), &options).unwrap();
    assert!(!png.is_empty());

    for format in [
        ImageFormat::Png,
        ImageFormat::Bmp,
        ImageFormat::Gif,
        ImageFormat::Tiff,
        ImageFormat::WebP,
    ] {
        let Ok(bytes) = std::panic::catch_unwind(|| common::encode(&source, format)) else {
            continue;
        };
        let rendered = render_bytes(&bytes, &options).unwrap();
        assert_eq!(rendered, png, "{format:?} decoded differently");
    }

    let jpeg = render_bytes(&common::encode(&source, ImageFormat::Jpeg), &options).unwrap();
    assert_eq!(jpeg.lines().count(), png.lines().count());
}

#[test]
fn rendering_a_file_matches_rendering_its_bytes() {
    let bytes = common::encode(&common::vertical_bars(32, 32, 4), ImageFormat::Png);
    let path = common::write_fixture("bars.png", &bytes);
    let options = RenderOptions::default().width(8);

    assert_eq!(
        render_file(&path, &options).unwrap(),
        render_bytes(&bytes, &options).unwrap()
    );
}

#[test]
fn rendering_a_reader_matches_rendering_its_bytes() {
    let bytes = common::encode(&common::checkerboard(16, 16, 4), ImageFormat::Png);
    let options = RenderOptions::default().width(4);
    let via_reader = renderer(options)
        .render_reader(std::io::Cursor::new(&bytes))
        .unwrap();
    assert_eq!(via_reader, render_bytes(&bytes, &options).unwrap());
}

#[test]
fn raw_pixel_entry_points_agree() {
    let image = common::rgb_bands(24, 24);
    let rgba = image.as_raw();
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let options = RenderOptions::default().size(6, 3).fit(Fit::Stretch);

    let from_rgba = render_rgba(24, 24, rgba, &options).unwrap();
    let from_rgb = rustille::render_rgb(24, 24, &rgb, &options).unwrap();
    assert_eq!(from_rgba, from_rgb);

    let png = common::encode(&image, ImageFormat::Png);
    assert_eq!(render_bytes(&png, &options).unwrap(), from_rgba);
}

#[test]
fn transparent_pixels_take_the_background_colour() {
    let bytes = common::encode(&common::transparent(16, 16), ImageFormat::Png);
    let base = RenderOptions::default().size(2, 1).fit(Fit::Stretch);

    let on_black = render_bytes(&bytes, &base.background(Background::Black)).unwrap();
    let on_white = render_bytes(&bytes, &base.background(Background::White)).unwrap();
    let on_grey = render_bytes(&bytes, &base.background(Background::Rgb(200, 200, 200))).unwrap();

    assert_eq!(on_black, "\u{2800}\u{2800}");
    assert_eq!(on_white, "\u{28FF}\u{28FF}");
    assert_eq!(on_grey, "\u{28FF}\u{28FF}");
}

#[test]
fn threshold_selects_which_tones_become_dots() {
    let image = common::horizontal_ramp(64, 8);
    let rgba = image.as_raw();
    let base = RenderOptions::default().size(32, 2).fit(Fit::Stretch);

    let dark = render_rgba(64, 8, rgba, &base.threshold(32)).unwrap();
    let bright = render_rgba(64, 8, rgba, &base.threshold(224)).unwrap();

    let lit = |text: &str| {
        text.chars()
            .filter_map(rustille::braille::mask_for_char)
            .map(|m| m.count_ones())
            .sum::<u32>()
    };
    assert!(
        lit(&dark) > lit(&bright),
        "a low threshold must light more dots"
    );
    assert_eq!(
        render_rgba(64, 8, rgba, &base.threshold(0))
            .unwrap()
            .chars()
            .filter(|c| *c == '\u{28FF}')
            .count(),
        64,
    );
}

#[test]
fn inversion_is_the_exact_complement_for_a_bilevel_image() {
    let image = common::checkerboard(32, 32, 1);
    let rgba = image.as_raw();
    let base = RenderOptions::default().size(16, 8).fit(Fit::Stretch);

    let plain = render_rgba(32, 32, rgba, &base).unwrap();
    let inverted = render_rgba(32, 32, rgba, &base.invert(true)).unwrap();

    for (a, b) in plain.chars().zip(inverted.chars()) {
        match (
            rustille::braille::mask_for_char(a),
            rustille::braille::mask_for_char(b),
        ) {
            (Some(a), Some(b)) => assert_eq!(a, !b, "masks are not complementary"),
            _ => assert_eq!(a, b, "line structure changed"),
        }
    }
}

#[test]
fn dithering_is_deterministic_and_adds_detail() {
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba([110, 110, 110, 255]));
    let rgba = image.as_raw();
    let base = RenderOptions::default().size(16, 8).fit(Fit::Stretch);

    let plain = render_rgba(64, 64, rgba, &base).unwrap();
    let dithered = render_rgba(64, 64, rgba, &base.dither(Dither::FloydSteinberg)).unwrap();
    let again = render_rgba(64, 64, rgba, &base.dither(Dither::FloydSteinberg)).unwrap();

    assert!(plain.chars().all(|c| c == '\u{2800}' || c == '\n'));
    assert_ne!(plain, dithered);
    assert_eq!(dithered, again, "dithering must be deterministic");
}

#[test]
fn aspect_ratio_is_preserved_by_default() {
    let image = common::checkerboard(200, 100, 10);
    let rgba = image.as_raw();

    let rendered = render_rgba(200, 100, rgba, &RenderOptions::default().width(40)).unwrap();
    assert_eq!(rendered.lines().count(), 10);
    assert!(rendered.lines().all(|l| l.chars().count() == 40));

    let square = render_rgba(
        200,
        100,
        rgba,
        &RenderOptions::default().width(40).cell_aspect_ratio(1.0),
    )
    .unwrap();
    assert_eq!(square.lines().count(), 20);
}

#[test]
fn fit_modes_produce_the_documented_shapes() {
    let image = common::checkerboard(200, 100, 10);
    let rgba = image.as_raw();
    let box_options = RenderOptions::default().size(40, 40);

    let contain = render_rgba(200, 100, rgba, &box_options.fit(Fit::Contain)).unwrap();
    assert_eq!(contain.lines().count(), 10);

    let fill = render_rgba(200, 100, rgba, &box_options.fit(Fit::Fill)).unwrap();
    assert_eq!(fill.lines().count(), 40);
    assert!(fill.lines().all(|l| l.chars().count() == 40));

    let stretch = render_rgba(200, 100, rgba, &box_options.fit(Fit::Stretch)).unwrap();
    assert_eq!(stretch.lines().count(), 40);
    assert_ne!(fill, stretch, "fill must crop, not squash");
}

#[test]
fn plain_output_is_free_of_escape_sequences() {
    let bytes = common::encode(&common::rgb_bands(24, 24), ImageFormat::Png);
    let rendered = render_bytes(&bytes, &RenderOptions::default().width(6)).unwrap();
    assert!(!rendered.contains('\u{1b}'));
    assert!(
        rendered
            .chars()
            .all(|c| c == '\n' || rustille::braille::mask_for_char(c).is_some())
    );
}

#[test]
fn colour_output_is_well_formed() {
    let bytes = common::encode(&common::rgb_bands(24, 24), ImageFormat::Png);
    let base = RenderOptions::default()
        .size(3, 1)
        .fit(Fit::Stretch)
        .threshold(10);

    let truecolor = render_bytes(&bytes, &base.color(ColorMode::TrueColor)).unwrap();
    assert!(truecolor.contains("\x1b[38;2;"));
    assert!(
        truecolor.ends_with("\x1b[0m"),
        "every coloured row is reset"
    );
    for expected in [
        "\x1b[38;2;255;0;0m",
        "\x1b[38;2;0;255;0m",
        "\x1b[38;2;0;0;255m",
    ] {
        assert!(
            truecolor.contains(expected),
            "{expected:?} missing from {truecolor:?}"
        );
    }

    let ansi = render_bytes(&bytes, &base.color(ColorMode::Ansi256)).unwrap();
    assert!(ansi.contains("\x1b[38;5;"));
    assert!(!ansi.contains("\x1b[38;2;"));

    let plain = render_bytes(&bytes, &base).unwrap();
    assert_eq!(strip_ansi(&truecolor), plain);
    assert_eq!(strip_ansi(&ansi), plain);
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        for escape in chars.by_ref() {
            if escape == 'm' {
                break;
            }
        }
    }
    out
}

#[test]
fn malformed_input_is_reported_not_panicked() {
    let options = RenderOptions::default().width(8);
    let cases: [&[u8]; 4] = [
        b"",
        b"not an image at all",
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\x00garbage",
        &[0xFF; 64],
    ];
    for bytes in cases {
        let error = render_bytes(bytes, &options).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Decode | ErrorKind::UnsupportedFormat | ErrorKind::Io
            ),
            "unexpected error for {bytes:?}: {error}"
        );
    }
}

#[test]
fn a_missing_file_is_an_io_error() {
    let missing = common::scratch_dir().join("definitely-not-here.png");
    let error = render_file(&missing, &RenderOptions::default()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Io);
}

#[test]
fn zero_dimensions_are_rejected() {
    let options = RenderOptions::default();
    assert_eq!(
        render_rgba(0, 4, &[], &options).unwrap_err().kind(),
        ErrorKind::InvalidDimensions
    );
    assert_eq!(
        render_rgba(4, 0, &[], &options).unwrap_err().kind(),
        ErrorKind::InvalidDimensions
    );
    assert_eq!(
        render_rgba(2, 2, &[0; 4], &options).unwrap_err().kind(),
        ErrorKind::InvalidBufferLength
    );
    assert_eq!(
        render_rgba(2, 2, &[0; 16], &options.width(0))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidOptions
    );
}

#[test]
fn extreme_but_valid_sizes_still_render() {
    let wide = vec![255u8; 4096 * 4];
    let rendered = render_rgba(4096, 1, &wide, &RenderOptions::default().width(512)).unwrap();
    assert_eq!(rendered.lines().count(), 1);
    assert_eq!(rendered.chars().count(), 512);

    let tall = vec![255u8; 4096 * 4];
    let rendered = render_rgba(1, 4096, &tall, &RenderOptions::default().height(256)).unwrap();
    assert_eq!(rendered.lines().count(), 256);
}

#[test]
fn absurd_sizes_fail_before_allocating() {
    let error = render_rgba(
        4,
        4,
        &[0; 64],
        &RenderOptions::default()
            .size(60_000, 60_000)
            .fit(Fit::Stretch),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidDimensions);
}

#[test]
fn upscaling_a_tiny_image_works() {
    let pixels = [
        255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    let rendered = render_rgba(2, 2, &pixels, &RenderOptions::default().width(20)).unwrap();
    assert_eq!(rendered.lines().count(), 10);
    assert!(rendered.lines().all(|l| l.chars().count() == 20));
}
