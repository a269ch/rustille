//! Node-API bindings for Rustille, generated with napi-rs.
//!
//! Like the other bindings this is a pure adapter: arguments in, core calls
//! out. Everything that decides what a Braille cell looks like lives in the
//! `rustille` crate.

#![deny(clippy::all)]

use napi::bindgen_prelude::{Error, Result, Status, Uint8Array};
use napi_derive::napi;

use rustille::{Canvas as CoreCanvas, ErrorKind, RenderOptions as CoreOptions, Renderer};

/// Options accepted by every render function.
///
/// `width` and `height` are measured in characters, not pixels. Omit both and
/// the width defaults to 80.
#[napi(object)]
#[derive(Default)]
pub struct RenderOptions {
    /// Output width in characters.
    pub width: Option<u32>,
    /// Output height in characters.
    pub height: Option<u32>,
    /// Luminance at or above which a dot is drawn (0-255).
    pub threshold: Option<u8>,
    /// Draw dark pixels instead of bright ones.
    pub invert: Option<bool>,
    /// `"none"` or `"floyd-steinberg"`.
    pub dither: Option<String>,
    /// `"none"`, `"ansi256"` or `"truecolor"`.
    pub color: Option<String>,
    /// `"black"`, `"white"`, `"#rrggbb"` or `"r,g,b"`.
    pub background: Option<String>,
    /// `"contain"`, `"fill"` or `"stretch"`.
    pub fit: Option<String>,
    /// Physical height/width ratio of one terminal character cell.
    pub cell_aspect_ratio: Option<f64>,
}

fn to_napi_error(error: rustille::Error) -> Error {
    let status = match error.kind() {
        ErrorKind::InvalidDimensions
        | ErrorKind::InvalidBufferLength
        | ErrorKind::InvalidOptions => Status::InvalidArg,
        _ => Status::GenericFailure,
    };
    Error::new(status, error.to_string())
}

fn parse<T: std::str::FromStr<Err = rustille::Error>>(
    value: &Option<String>,
    fallback: T,
) -> Result<T> {
    match value {
        None => Ok(fallback),
        Some(text) => text.parse().map_err(to_napi_error),
    }
}

impl RenderOptions {
    fn to_core(&self) -> Result<CoreOptions> {
        let defaults = CoreOptions::default();
        let mut options = CoreOptions::default()
            .threshold(self.threshold.unwrap_or(defaults.threshold))
            .invert(self.invert.unwrap_or(false))
            .dither(parse(&self.dither, defaults.dither)?)
            .color(parse(&self.color, defaults.color)?)
            .background(parse(&self.background, defaults.background)?)
            .fit(parse(&self.fit, defaults.fit)?)
            .cell_aspect_ratio(
                self.cell_aspect_ratio
                    .map_or(defaults.cell_aspect_ratio, |ratio| ratio as f32),
            );
        options.width = self.width;
        options.height = self.height;
        options.validate().map_err(to_napi_error)?;
        Ok(options)
    }
}

fn renderer(options: Option<RenderOptions>) -> Result<Renderer> {
    let options = options.unwrap_or_default().to_core()?;
    Ok(Renderer::new(options))
}

/// The version of the underlying Rustille core.
#[napi]
pub fn version() -> &'static str {
    rustille::VERSION
}

/// Renders an image file (PNG, JPEG, WebP, BMP, GIF or TIFF).
#[napi]
pub fn render_file(path: String, options: Option<RenderOptions>) -> Result<String> {
    renderer(options)?.render_file(path).map_err(to_napi_error)
}

/// Renders an encoded image held in memory. The format is sniffed from the
/// bytes, so a file name is not needed.
#[napi]
pub fn render_bytes(data: Uint8Array, options: Option<RenderOptions>) -> Result<String> {
    renderer(options)?
        .render_bytes(data.as_ref())
        .map_err(to_napi_error)
}

/// Renders a tightly packed RGBA8 buffer (`width * height * 4` bytes).
#[napi]
pub fn render_rgba(
    data: Uint8Array,
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String> {
    renderer(options)?
        .render_rgba(width, height, data.as_ref())
        .map_err(to_napi_error)
}

/// Renders a tightly packed RGB8 buffer (`width * height * 3` bytes).
#[napi]
pub fn render_rgb(
    data: Uint8Array,
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String> {
    renderer(options)?
        .render_rgb(width, height, data.as_ref())
        .map_err(to_napi_error)
}

/// Renders a tightly packed 8-bit grayscale buffer (`width * height` bytes).
#[napi]
pub fn render_luma(
    data: Uint8Array,
    width: u32,
    height: u32,
    options: Option<RenderOptions>,
) -> Result<String> {
    renderer(options)?
        .render_luma(width, height, data.as_ref())
        .map_err(to_napi_error)
}

/// The Braille character for a dot mask (0-255).
#[napi]
pub fn braille_char(mask: u8) -> String {
    rustille::braille::char_for_mask(mask).to_string()
}

/// The dot mask of a Braille character, or `null` if it is not one.
#[napi]
pub fn braille_mask(character: String) -> Option<u8> {
    let mut chars = character.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    rustille::braille::mask_for_char(first)
}

/// A fixed-size grid of Braille dots.
///
/// Sizes are in dots: a 100x50 canvas renders as 50x13 characters.
#[napi]
pub struct Canvas {
    inner: CoreCanvas,
}

#[napi]
impl Canvas {
    /// Creates an empty canvas `width` x `height` dots in size.
    #[napi(constructor)]
    pub fn new(width: u32, height: u32) -> Result<Self> {
        CoreCanvas::try_new(width, height)
            .map(|inner| Self { inner })
            .map_err(to_napi_error)
    }

    /// Canvas width in dots.
    #[napi(getter)]
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Canvas height in dots.
    #[napi(getter)]
    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Rendered width in characters.
    #[napi(getter)]
    pub fn cells_width(&self) -> u32 {
        self.inner.cells_width()
    }

    /// Rendered height in characters.
    #[napi(getter)]
    pub fn cells_height(&self) -> u32 {
        self.inner.cells_height()
    }

    /// Turns the dot at `(x, y)` on. Returns false if it is out of range.
    #[napi]
    pub fn set(&mut self, x: i32, y: i32) -> bool {
        self.inner.set(x, y)
    }

    /// Turns the dot at `(x, y)` off.
    #[napi]
    pub fn unset(&mut self, x: i32, y: i32) -> bool {
        self.inner.unset(x, y)
    }

    /// Flips the dot at `(x, y)`.
    #[napi]
    pub fn toggle(&mut self, x: i32, y: i32) -> bool {
        self.inner.toggle(x, y)
    }

    /// Reads the dot at `(x, y)`. Out-of-range dots read as false.
    #[napi]
    pub fn get(&self, x: i32, y: i32) -> bool {
        self.inner.get(x, y)
    }

    /// Clears every dot.
    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Sets every dot inside the canvas.
    #[napi]
    pub fn fill(&mut self) {
        self.inner.fill();
    }

    /// Number of dots currently set.
    #[napi]
    pub fn count(&self) -> u32 {
        self.inner.count()
    }

    /// Draws a Bresenham line from `(x0, y0)` to `(x1, y1)`.
    #[napi]
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.line(x0, y0, x1, y1);
    }

    /// Draws a rectangle outline through two opposite corners.
    #[napi]
    pub fn rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.rectangle(x0, y0, x1, y1);
    }

    /// Fills a rectangle through two opposite corners.
    #[napi]
    pub fn filled_rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.filled_rectangle(x0, y0, x1, y1);
    }

    /// Draws a circle outline of `radius` dots centred on `(cx, cy)`.
    #[napi]
    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.circle(cx, cy, radius);
    }

    /// Fills a disc of `radius` dots centred on `(cx, cy)`.
    #[napi]
    pub fn filled_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.filled_circle(cx, cy, radius);
    }

    /// Renders the canvas to a newline-separated Braille string.
    #[napi]
    pub fn render(&self) -> String {
        self.inner.render()
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.render()
    }
}
