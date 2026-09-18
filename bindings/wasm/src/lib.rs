//! WebAssembly bindings for Rustille.
//!
//! The browser already knows how to decode images, so the RGBA entry point is
//! the important one: draw an `<img>` (or an `ImageBitmap`) onto a canvas, pull
//! the pixels out with `getImageData`, and hand the buffer straight to
//! [`render_rgba`]. `renderBytes` is also available when the `decode` feature is
//! on, which it is by default.
//!
//! Nothing here touches the filesystem, and no rendering logic is duplicated:
//! this module only converts JavaScript values into
//! [`rustille::RenderOptions`].

#![allow(clippy::needless_pass_by_value)]

use wasm_bindgen::prelude::*;

use rustille::{Canvas as CoreCanvas, RenderOptions as CoreOptions, Renderer};

#[wasm_bindgen(typescript_custom_section)]
const TS_RENDER_OPTIONS: &'static str = r#"
/**
 * Options accepted by every render function.
 *
 * `width` and `height` are measured in characters, not pixels: one character
 * holds a 2x4 grid of dots. Give one and the other is derived from the image's
 * aspect ratio; give neither and the width defaults to 80.
 */
export interface RenderOptions {
  /** Output width in characters. */
  width?: number;
  /** Output height in characters. */
  height?: number;
  /** Luminance at or above which a dot is drawn (0-255). Default 128. */
  threshold?: number;
  /** Draw dark pixels instead of bright ones. */
  invert?: boolean;
  /** Dithering algorithm. */
  dither?: "none" | "floyd-steinberg";
  /** ANSI colour output. */
  color?: "none" | "ansi256" | "truecolor";
  /** Colour composited under transparent pixels. */
  background?: string;
  /** How the image is mapped onto the requested size. */
  fit?: "contain" | "fill" | "stretch";
  /** Physical height/width ratio of one terminal cell. Default 2. */
  cellAspectRatio?: number;
}
"#;

#[wasm_bindgen]
unsafe extern "C" {
    /// The `RenderOptions` object literal, as seen from TypeScript.
    #[wasm_bindgen(typescript_type = "RenderOptions")]
    pub type RenderOptions;
}

/// Reads one property, treating `null` and `undefined` as absent.
fn property(options: &JsValue, key: &str) -> Option<JsValue> {
    if options.is_undefined() || options.is_null() {
        return None;
    }
    js_sys::Reflect::get(options, &JsValue::from_str(key))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

fn number(options: &JsValue, key: &str) -> Option<f64> {
    property(options, key).and_then(|value| value.as_f64())
}

fn string(options: &JsValue, key: &str) -> Option<String> {
    property(options, key).and_then(|value| value.as_string())
}

fn boolean(options: &JsValue, key: &str) -> Option<bool> {
    property(options, key).and_then(|value| value.as_bool())
}

fn parse<T: std::str::FromStr<Err = rustille::Error>>(
    value: Option<String>,
    fallback: T,
) -> Result<T, JsError> {
    match value {
        None => Ok(fallback),
        Some(text) => text
            .parse()
            .map_err(|error: rustille::Error| JsError::new(&error.to_string())),
    }
}

/// Converts a plain JavaScript object into core options.
fn to_core_options(options: Option<RenderOptions>) -> Result<CoreOptions, JsError> {
    let defaults = CoreOptions::default();
    let raw: JsValue = match options {
        Some(options) => options.into(),
        None => JsValue::UNDEFINED,
    };

    let clamp_dimension = |value: f64| -> Option<u32> {
        if value.is_finite() && value >= 0.0 {
            Some(value as u32)
        } else {
            None
        }
    };

    let threshold = number(&raw, "threshold")
        .map(|value| value.clamp(0.0, 255.0).round() as u8)
        .unwrap_or(defaults.threshold);

    let mut core = CoreOptions::default()
        .threshold(threshold)
        .invert(boolean(&raw, "invert").unwrap_or(false))
        .dither(parse(string(&raw, "dither"), defaults.dither)?)
        .color(parse(string(&raw, "color"), defaults.color)?)
        .background(parse(string(&raw, "background"), defaults.background)?)
        .fit(parse(string(&raw, "fit"), defaults.fit)?)
        .cell_aspect_ratio(
            number(&raw, "cellAspectRatio").map_or(defaults.cell_aspect_ratio, |v| v as f32),
        );
    core.width = number(&raw, "width").and_then(clamp_dimension);
    core.height = number(&raw, "height").and_then(clamp_dimension);
    core.validate()
        .map_err(|error| JsError::new(&error.to_string()))?;
    Ok(core)
}

fn renderer(options: Option<RenderOptions>) -> Result<Renderer, JsError> {
    Ok(Renderer::new(to_core_options(options)?))
}

fn to_js_error(error: rustille::Error) -> JsError {
    JsError::new(&error.to_string())
}

/// The version of the underlying Rustille core.
#[wasm_bindgen]
pub fn version() -> String {
    rustille::VERSION.to_owned()
}

/// Renders a tightly packed RGBA8 buffer (`width * height * 4` bytes).
///
/// This is the entry point browsers want: `ctx.getImageData(...).data` is
/// exactly this layout.
#[wasm_bindgen(js_name = renderRgba)]
pub fn render_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String, JsError> {
    renderer(options)?
        .render_rgba(width, height, data)
        .map_err(to_js_error)
}

/// Renders a tightly packed RGB8 buffer (`width * height * 3` bytes).
#[wasm_bindgen(js_name = renderRgb)]
pub fn render_rgb(
    data: &[u8],
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String, JsError> {
    renderer(options)?
        .render_rgb(width, height, data)
        .map_err(to_js_error)
}

/// Renders a tightly packed 8-bit grayscale buffer (`width * height` bytes).
#[wasm_bindgen(js_name = renderLuma)]
pub fn render_luma(
    data: &[u8],
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String, JsError> {
    renderer(options)?
        .render_luma(width, height, data)
        .map_err(to_js_error)
}

/// Decodes an encoded image (PNG, JPEG, WebP, BMP, GIF, TIFF) and renders it.
///
/// Only available when the crate is built with the `decode` feature, which is
/// the default. Prefer [`render_rgba`] when the host can decode for you: it
/// keeps the WebAssembly module far smaller.
#[cfg(feature = "decode")]
#[wasm_bindgen(js_name = renderBytes)]
pub fn render_bytes(data: &[u8], options: Option<RenderOptions>) -> Result<String, JsError> {
    renderer(options)?.render_bytes(data).map_err(to_js_error)
}

/// The Braille character for a dot mask (0-255).
#[wasm_bindgen(js_name = brailleChar)]
pub fn braille_char(mask: u8) -> String {
    rustille::braille::char_for_mask(mask).to_string()
}

/// The dot mask of a Braille character, or `undefined` if it is not one.
#[wasm_bindgen(js_name = brailleMask)]
pub fn braille_mask(character: &str) -> Option<u8> {
    let mut chars = character.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    rustille::braille::mask_for_char(first)
}

/// A fixed-size grid of Braille dots.
///
/// Sizes are in dots: a 100x50 canvas renders as 50x13 characters. Call
/// [`Canvas::free`] (generated by wasm-bindgen) when you are done with it.
#[wasm_bindgen]
pub struct Canvas {
    inner: CoreCanvas,
}

#[wasm_bindgen]
impl Canvas {
    /// Creates an empty canvas `width` x `height` dots in size.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Result<Canvas, JsError> {
        CoreCanvas::try_new(width, height)
            .map(|inner| Canvas { inner })
            .map_err(to_js_error)
    }

    /// Canvas width in dots.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Canvas height in dots.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Rendered width in characters.
    #[wasm_bindgen(getter, js_name = cellsWidth)]
    pub fn cells_width(&self) -> u32 {
        self.inner.cells_width()
    }

    /// Rendered height in characters.
    #[wasm_bindgen(getter, js_name = cellsHeight)]
    pub fn cells_height(&self) -> u32 {
        self.inner.cells_height()
    }

    /// Turns the dot at `(x, y)` on. Returns false if it is out of range.
    pub fn set(&mut self, x: i32, y: i32) -> bool {
        self.inner.set(x, y)
    }

    /// Turns the dot at `(x, y)` off.
    pub fn unset(&mut self, x: i32, y: i32) -> bool {
        self.inner.unset(x, y)
    }

    /// Flips the dot at `(x, y)`.
    pub fn toggle(&mut self, x: i32, y: i32) -> bool {
        self.inner.toggle(x, y)
    }

    /// Reads the dot at `(x, y)`. Out-of-range dots read as false.
    pub fn get(&self, x: i32, y: i32) -> bool {
        self.inner.get(x, y)
    }

    /// Clears every dot.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Sets every dot inside the canvas.
    pub fn fill(&mut self) {
        self.inner.fill();
    }

    /// Number of dots currently set.
    pub fn count(&self) -> u32 {
        self.inner.count()
    }

    /// Draws a Bresenham line from `(x0, y0)` to `(x1, y1)`.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.line(x0, y0, x1, y1);
    }

    /// Draws a rectangle outline through two opposite corners.
    pub fn rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.rectangle(x0, y0, x1, y1);
    }

    /// Fills a rectangle through two opposite corners.
    #[wasm_bindgen(js_name = filledRectangle)]
    pub fn filled_rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.filled_rectangle(x0, y0, x1, y1);
    }

    /// Draws a circle outline of `radius` dots centred on `(cx, cy)`.
    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.circle(cx, cy, radius);
    }

    /// Fills a disc of `radius` dots centred on `(cx, cy)`.
    #[wasm_bindgen(js_name = filledCircle)]
    pub fn filled_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.filled_circle(cx, cy, radius);
    }

    /// Renders the canvas to a newline-separated Braille string.
    pub fn render(&self) -> String {
        self.inner.render()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_js_string(&self) -> String {
        self.inner.render()
    }
}
