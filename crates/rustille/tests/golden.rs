//! Golden-file tests over tiny, programmatically generated images.
//!
//! The fixtures are small enough to read in a diff, which is the point: if the
//! pipeline changes, the review shows exactly which dots moved. Re-run with
//! `RUSTILLE_BLESS=1` to accept an intentional change.

mod common;

use rustille::{Canvas, ColorMode, Dither, Fit, RenderOptions, render_rgba};

fn rgba_of(image: &image::RgbaImage) -> (u32, u32, Vec<u8>) {
    (image.width(), image.height(), image.as_raw().clone())
}

#[test]
fn golden_checkerboard() {
    let image = common::checkerboard(16, 16, 2);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(
        w,
        h,
        &data,
        &RenderOptions::default().size(8, 4).fit(Fit::Stretch),
    )
    .unwrap();
    common::assert_golden("checkerboard", &out);
}

#[test]
fn golden_vertical_bars() {
    let image = common::vertical_bars(48, 24, 6);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(w, h, &data, &RenderOptions::default().width(12)).unwrap();
    common::assert_golden("vertical_bars", &out);
}

#[test]
fn golden_ramp_plain() {
    let image = common::horizontal_ramp(64, 32);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(w, h, &data, &RenderOptions::default().width(16)).unwrap();
    common::assert_golden("ramp_plain", &out);
}

#[test]
fn golden_ramp_floyd_steinberg() {
    let image = common::horizontal_ramp(64, 32);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(
        w,
        h,
        &data,
        &RenderOptions::default()
            .width(16)
            .dither(Dither::FloydSteinberg),
    )
    .unwrap();
    common::assert_golden("ramp_floyd_steinberg", &out);
}

#[test]
fn golden_ramp_inverted() {
    let image = common::horizontal_ramp(64, 32);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(
        w,
        h,
        &data,
        &RenderOptions::default().width(16).invert(true),
    )
    .unwrap();
    common::assert_golden("ramp_inverted", &out);
}

#[test]
fn golden_truecolor_bands() {
    let image = common::rgb_bands(36, 12);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(
        w,
        h,
        &data,
        &RenderOptions::default()
            .size(6, 2)
            .fit(Fit::Stretch)
            .threshold(10)
            .color(ColorMode::TrueColor),
    )
    .unwrap();
    common::assert_golden("truecolor_bands", &escape_ansi(&out));
}

#[test]
fn golden_ansi256_bands() {
    let image = common::rgb_bands(36, 12);
    let (w, h, data) = rgba_of(&image);
    let out = render_rgba(
        w,
        h,
        &data,
        &RenderOptions::default()
            .size(6, 2)
            .fit(Fit::Stretch)
            .threshold(10)
            .color(ColorMode::Ansi256),
    )
    .unwrap();
    common::assert_golden("ansi256_bands", &escape_ansi(&out));
}

#[test]
fn golden_canvas_shapes() {
    let mut canvas = Canvas::new(48, 24);
    canvas.rectangle(0, 0, 47, 23);
    canvas.line(0, 0, 47, 23);
    canvas.line(47, 0, 0, 23);
    canvas.circle(24, 12, 8);
    common::assert_golden("canvas_shapes", &canvas.render());
}

/// Renders ESC as `\e` so the golden file stays readable in a diff.
fn escape_ansi(text: &str) -> String {
    text.replace('\u{1b}', "\\e")
}
