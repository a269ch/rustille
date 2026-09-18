//! C ABI for Rustille.
//!
//! This crate is a thin, `unsafe`-containing shell around the `rustille`
//! library: it converts pointers to slices, enums to integers and Rust errors
//! to status codes, and contains no rendering logic of its own. The core crate
//! is `#![forbid(unsafe_code)]`; every `unsafe` block in Rustille lives here.
//!
//! # Contract
//!
//! * Every function is safe to call with null pointers: they return
//!   [`RustilleStatus::NullPointer`] instead of dereferencing.
//! * Buffers passed in are only read during the call and are never retained.
//! * Strings returned through an `out` parameter are NUL-terminated UTF-8,
//!   owned by the caller, and must be released with [`rustille_string_free`].
//! * No panic can cross the boundary: every entry point runs inside
//!   [`std::panic::catch_unwind`] and reports [`RustilleStatus::Panic`]. (If the
//!   library is built with `panic = "abort"`, the process aborts instead — that
//!   is the only way a panic ever leaves this crate.)
//! * Functions may be called from any thread. The detailed error message is
//!   stored per thread.

#![deny(missing_docs)]
#![warn(missing_debug_implementations)]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rustille_core::{
    Background, Canvas, ColorMode, Dither, ErrorKind, Fit, RenderOptions, Renderer,
};

/// Result of a Rustille call.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustilleStatus {
    /// The call succeeded.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// A string argument was not valid UTF-8.
    InvalidUtf8 = 2,
    /// An enum field held a value outside its defined range.
    InvalidEnum = 3,
    /// The image data could not be decoded.
    Decode = 4,
    /// The image format is not supported by this build.
    UnsupportedFormat = 5,
    /// A width or height was zero, or the result would be too large.
    InvalidDimensions = 6,
    /// A pixel buffer did not match the declared dimensions.
    InvalidBufferLength = 7,
    /// An option field was out of range.
    InvalidOptions = 8,
    /// Reading or writing failed.
    Io = 9,
    /// A panic was caught at the boundary. This is a bug; please report it.
    Panic = 10,
    /// The requested feature was not compiled into this build.
    Unsupported = 11,
}

/// Dithering algorithm, mirroring `rustille::Dither`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustilleDither {
    /// Plain thresholding.
    None = 0,
    /// Floyd–Steinberg error diffusion.
    FloydSteinberg = 1,
}

/// Colour mode, mirroring `rustille::ColorMode`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustilleColorMode {
    /// No escape sequences.
    None = 0,
    /// 256-colour ANSI foreground.
    Ansi256 = 1,
    /// 24-bit ANSI foreground.
    TrueColor = 2,
}

/// Fit mode, mirroring `rustille::Fit`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustilleFit {
    /// Fit inside the box, preserving the aspect ratio.
    Contain = 0,
    /// Cover the box and crop, preserving the aspect ratio.
    Fill = 1,
    /// Use the box exactly.
    Stretch = 2,
}

/// Rendering options.
///
/// Always initialise with [`rustille_options_init`] so that fields added in a
/// later version keep sensible values.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RustilleOptions {
    /// Output width in characters; `0` means "derive from the height".
    pub width: u32,
    /// Output height in characters; `0` means "derive from the width".
    pub height: u32,
    /// Luminance at or above which a dot is drawn.
    pub threshold: u8,
    /// Draw dark pixels instead of bright ones (`0` or `1`).
    pub invert: u8,
    /// Red channel of the colour composited under transparent pixels.
    pub background_r: u8,
    /// Green channel of the background colour.
    pub background_g: u8,
    /// Blue channel of the background colour.
    pub background_b: u8,
    /// Dithering algorithm.
    pub dither: RustilleDither,
    /// Colour mode.
    pub color: RustilleColorMode,
    /// Fit mode.
    pub fit: RustilleFit,
    /// Physical height/width ratio of a terminal character cell.
    pub cell_aspect_ratio: f32,
}

impl Default for RustilleOptions {
    fn default() -> Self {
        let defaults = RenderOptions::default();
        Self {
            width: 0,
            height: 0,
            threshold: defaults.threshold,
            invert: 0,
            background_r: 0,
            background_g: 0,
            background_b: 0,
            dither: RustilleDither::None,
            color: RustilleColorMode::None,
            fit: RustilleFit::Contain,
            cell_aspect_ratio: defaults.cell_aspect_ratio,
        }
    }
}

impl From<&RustilleOptions> for RenderOptions {
    fn from(raw: &RustilleOptions) -> Self {
        let mut options = RenderOptions::default()
            .threshold(raw.threshold)
            .invert(raw.invert != 0)
            .background(Background::Rgb(
                raw.background_r,
                raw.background_g,
                raw.background_b,
            ))
            .cell_aspect_ratio(raw.cell_aspect_ratio)
            .dither(match raw.dither {
                RustilleDither::None => Dither::None,
                RustilleDither::FloydSteinberg => Dither::FloydSteinberg,
            })
            .color(match raw.color {
                RustilleColorMode::None => ColorMode::None,
                RustilleColorMode::Ansi256 => ColorMode::Ansi256,
                RustilleColorMode::TrueColor => ColorMode::TrueColor,
            })
            .fit(match raw.fit {
                RustilleFit::Contain => Fit::Contain,
                RustilleFit::Fill => Fit::Fill,
                RustilleFit::Stretch => Fit::Stretch,
            });
        options.width = (raw.width != 0).then_some(raw.width);
        options.height = (raw.height != 0).then_some(raw.height);
        options
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: &str) {
    // Interior NULs cannot survive a C string; replace them rather than lose
    // the whole message.
    let sanitized = message.replace('\0', " ");
    let value = CString::new(sanitized).ok();
    LAST_ERROR.with(|slot| *slot.borrow_mut() = value);
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

fn status_for(error: &rustille_core::Error) -> RustilleStatus {
    match error.kind() {
        ErrorKind::Decode => RustilleStatus::Decode,
        ErrorKind::UnsupportedFormat => RustilleStatus::UnsupportedFormat,
        ErrorKind::InvalidDimensions => RustilleStatus::InvalidDimensions,
        ErrorKind::InvalidBufferLength => RustilleStatus::InvalidBufferLength,
        ErrorKind::InvalidOptions => RustilleStatus::InvalidOptions,
        ErrorKind::Io => RustilleStatus::Io,
        _ => RustilleStatus::Decode,
    }
}

/// Runs `body`, converting a panic into [`RustilleStatus::Panic`].
fn guard<F>(body: F) -> RustilleStatus
where
    F: FnOnce() -> RustilleStatus,
{
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(status) => status,
        Err(_) => {
            set_last_error("rustille panicked; this is a bug");
            RustilleStatus::Panic
        }
    }
}

/// Runs `body` for a function that returns no status.
fn guard_unit<F: FnOnce()>(body: F) {
    let _ = catch_unwind(AssertUnwindSafe(body));
}

/// Moves a rendered string into a caller-owned `char *`.
fn deliver(result: rustille_core::Result<String>, out: *mut *mut c_char) -> RustilleStatus {
    match result {
        Ok(text) => match CString::new(text) {
            Ok(c_string) => {
                // SAFETY: `out` was checked for null by the caller of this
                // helper, and points to a writable `char *`.
                unsafe { *out = c_string.into_raw() };
                clear_last_error();
                RustilleStatus::Ok
            }
            Err(_) => {
                set_last_error("rendered output contained an interior NUL byte");
                RustilleStatus::InvalidUtf8
            }
        },
        Err(error) => {
            set_last_error(&error.to_string());
            status_for(&error)
        }
    }
}

/// The Rustille version, as a static NUL-terminated string (for example
/// `"0.1.0"`).
///
/// The returned pointer is valid for the lifetime of the program and must not
/// be freed.
#[unsafe(no_mangle)]
pub extern "C" fn rustille_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// A short, static description of a status code.
///
/// The returned pointer is valid for the lifetime of the program and must not
/// be freed.
#[unsafe(no_mangle)]
pub extern "C" fn rustille_status_message(status: RustilleStatus) -> *const c_char {
    let text: &'static str = match status {
        RustilleStatus::Ok => "ok\0",
        RustilleStatus::NullPointer => "a required pointer argument was null\0",
        RustilleStatus::InvalidUtf8 => "string argument was not valid UTF-8\0",
        RustilleStatus::InvalidEnum => "enum argument was out of range\0",
        RustilleStatus::Decode => "failed to decode image\0",
        RustilleStatus::UnsupportedFormat => "unsupported image format\0",
        RustilleStatus::InvalidDimensions => "invalid dimensions\0",
        RustilleStatus::InvalidBufferLength => "invalid buffer length\0",
        RustilleStatus::InvalidOptions => "invalid options\0",
        RustilleStatus::Io => "i/o error\0",
        RustilleStatus::Panic => "internal panic\0",
        RustilleStatus::Unsupported => "feature not compiled into this build\0",
    };
    text.as_ptr().cast()
}

/// The detailed message for the most recent failure on the calling thread.
///
/// Returns null when the last call succeeded. The returned string is owned by
/// the caller and must be released with [`rustille_string_free`].
#[unsafe(no_mangle)]
pub extern "C" fn rustille_last_error_message() -> *mut c_char {
    let mut result = std::ptr::null_mut();
    guard_unit(|| {
        LAST_ERROR.with(|slot| {
            if let Some(message) = slot.borrow().as_ref() {
                result = message.clone().into_raw();
            }
        });
    });
    result
}

/// Fills `options` with Rustille's defaults.
///
/// # Safety
///
/// `options` must point to a writable [`RustilleOptions`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_options_init(options: *mut RustilleOptions) -> RustilleStatus {
    if options.is_null() {
        return RustilleStatus::NullPointer;
    }
    guard(|| {
        // SAFETY: checked non-null above; the caller guarantees it is writable
        // and correctly aligned.
        unsafe { options.write(RustilleOptions::default()) };
        RustilleStatus::Ok
    })
}

/// Releases a string produced by this library.
///
/// Passing null is a no-op. Passing any other pointer that Rustille did not
/// return is undefined behaviour.
///
/// # Safety
///
/// `text` must be null or a pointer previously returned by a Rustille function
/// and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_string_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    guard_unit(|| {
        // SAFETY: the caller guarantees this pointer came from
        // `CString::into_raw` in this library and has not been freed.
        drop(unsafe { CString::from_raw(text) });
    });
}

/// Shared body for the raw-pixel entry points.
///
/// # Safety
///
/// `data` must be valid for `len` bytes and `out` must be writable.
unsafe fn render_raw(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
    render: fn(&Renderer, u32, u32, &[u8]) -> rustille_core::Result<String>,
) -> RustilleStatus {
    if data.is_null() || options.is_null() || out.is_null() {
        return RustilleStatus::NullPointer;
    }
    guard(|| {
        // SAFETY: non-null checked above; the caller guarantees `data` covers
        // `len` bytes and `options` points to an initialised struct.
        let (bytes, raw_options) = unsafe {
            (
                std::slice::from_raw_parts(data, len),
                &*options.cast::<RustilleOptions>(),
            )
        };
        let renderer = Renderer::new(RenderOptions::from(raw_options));
        deliver(render(&renderer, width, height, bytes), out)
    })
}

/// Renders a tightly packed RGBA8 buffer (`width * height * 4` bytes).
///
/// On success `*out` receives a caller-owned UTF-8 string; release it with
/// [`rustille_string_free`].
///
/// # Safety
///
/// `data` must be valid for `len` bytes, `options` must point to an initialised
/// [`RustilleOptions`], and `out` must point to a writable `char *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_render_rgba(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
) -> RustilleStatus {
    // SAFETY: forwarded unchanged; the caller upholds this function's contract.
    unsafe {
        render_raw(
            data,
            len,
            width,
            height,
            options,
            out,
            |renderer, w, h, b| renderer.render_rgba(w, h, b),
        )
    }
}

/// Renders a tightly packed RGB8 buffer (`width * height * 3` bytes).
///
/// # Safety
///
/// As [`rustille_render_rgba`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_render_rgb(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
) -> RustilleStatus {
    // SAFETY: forwarded unchanged; the caller upholds this function's contract.
    unsafe {
        render_raw(
            data,
            len,
            width,
            height,
            options,
            out,
            |renderer, w, h, b| renderer.render_rgb(w, h, b),
        )
    }
}

/// Renders a tightly packed 8-bit grayscale buffer (`width * height` bytes).
///
/// # Safety
///
/// As [`rustille_render_rgba`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_render_luma(
    data: *const u8,
    len: usize,
    width: u32,
    height: u32,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
) -> RustilleStatus {
    // SAFETY: forwarded unchanged; the caller upholds this function's contract.
    unsafe {
        render_raw(
            data,
            len,
            width,
            height,
            options,
            out,
            |renderer, w, h, b| renderer.render_luma(w, h, b),
        )
    }
}

/// Decodes an encoded image (PNG, JPEG, WebP, BMP, GIF, TIFF) from memory and
/// renders it.
///
/// Returns [`RustilleStatus::Unsupported`] when the library was built without
/// the `decode` feature.
///
/// # Safety
///
/// `data` must be valid for `len` bytes, `options` must point to an initialised
/// [`RustilleOptions`], and `out` must point to a writable `char *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_render_bytes(
    data: *const u8,
    len: usize,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
) -> RustilleStatus {
    if data.is_null() || options.is_null() || out.is_null() {
        return RustilleStatus::NullPointer;
    }
    #[cfg(not(feature = "decode"))]
    {
        let _ = len;
        set_last_error("this build of rustille has image decoding disabled");
        RustilleStatus::Unsupported
    }
    #[cfg(feature = "decode")]
    guard(|| {
        // SAFETY: non-null checked above; the caller guarantees the ranges.
        let (bytes, raw_options) = unsafe {
            (
                std::slice::from_raw_parts(data, len),
                &*options.cast::<RustilleOptions>(),
            )
        };
        let renderer = Renderer::new(RenderOptions::from(raw_options));
        deliver(renderer.render_bytes(bytes), out)
    })
}

/// Decodes an image file and renders it.
///
/// Returns [`RustilleStatus::Unsupported`] when the library was built without
/// the `decode` feature.
///
/// # Safety
///
/// `path` must be a NUL-terminated string, `options` must point to an
/// initialised [`RustilleOptions`], and `out` must point to a writable
/// `char *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_render_file(
    path: *const c_char,
    options: *const RustilleOptions,
    out: *mut *mut c_char,
) -> RustilleStatus {
    if path.is_null() || options.is_null() || out.is_null() {
        return RustilleStatus::NullPointer;
    }
    #[cfg(not(feature = "decode"))]
    {
        set_last_error("this build of rustille has image decoding disabled");
        RustilleStatus::Unsupported
    }
    #[cfg(feature = "decode")]
    guard(|| {
        // SAFETY: non-null checked above; the caller guarantees NUL termination.
        let (path, raw_options) =
            unsafe { (CStr::from_ptr(path), &*options.cast::<RustilleOptions>()) };
        let path = match path.to_str() {
            Ok(path) => path,
            Err(_) => {
                set_last_error("path was not valid UTF-8");
                return RustilleStatus::InvalidUtf8;
            }
        };
        let renderer = Renderer::new(RenderOptions::from(raw_options));
        deliver(renderer.render_file(path), out)
    })
}

/// An opaque Braille drawing surface.
#[derive(Debug)]
pub struct RustilleCanvas {
    inner: Canvas,
}

/// Creates a canvas `width`×`height` **dots** in size.
///
/// Returns null if the size is rejected. Release it with
/// [`rustille_canvas_free`].
#[unsafe(no_mangle)]
pub extern "C" fn rustille_canvas_new(width: u32, height: u32) -> *mut RustilleCanvas {
    let mut result = std::ptr::null_mut();
    guard_unit(|| match Canvas::try_new(width, height) {
        Ok(inner) => {
            clear_last_error();
            result = Box::into_raw(Box::new(RustilleCanvas { inner }));
        }
        Err(error) => set_last_error(&error.to_string()),
    });
    result
}

/// Destroys a canvas. Passing null is a no-op.
///
/// # Safety
///
/// `canvas` must be null or a pointer from [`rustille_canvas_new`] that has not
/// already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_free(canvas: *mut RustilleCanvas) {
    if canvas.is_null() {
        return;
    }
    guard_unit(|| {
        // SAFETY: the caller guarantees this came from `Box::into_raw` here.
        drop(unsafe { Box::from_raw(canvas) });
    });
}

/// Helper for the `&mut Canvas` accessors.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
unsafe fn with_canvas_mut<R>(
    canvas: *mut RustilleCanvas,
    fallback: R,
    body: impl FnOnce(&mut Canvas) -> R,
) -> R {
    if canvas.is_null() {
        return fallback;
    }
    // SAFETY: non-null checked above; the caller guarantees the pointer is live
    // and not aliased (the C ABI is documented as !Sync for a single canvas).
    let canvas = unsafe { &mut *canvas };
    match catch_unwind(AssertUnwindSafe(|| body(&mut canvas.inner))) {
        Ok(value) => value,
        Err(_) => fallback,
    }
}

/// Helper for the `&Canvas` accessors.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
unsafe fn with_canvas<R>(
    canvas: *const RustilleCanvas,
    fallback: R,
    body: impl FnOnce(&Canvas) -> R,
) -> R {
    if canvas.is_null() {
        return fallback;
    }
    // SAFETY: non-null checked above; the caller guarantees the pointer is live.
    let canvas = unsafe { &*canvas };
    match catch_unwind(AssertUnwindSafe(|| body(&canvas.inner))) {
        Ok(value) => value,
        Err(_) => fallback,
    }
}

macro_rules! canvas_dot_fn {
    ($(#[$meta:meta])* $name:ident, $method:ident) => {
        $(#[$meta])*
        ///
        /// # Safety
        ///
        /// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(canvas: *mut RustilleCanvas, x: i32, y: i32) -> bool {
            // SAFETY: forwarded unchanged; the caller upholds the contract.
            unsafe { with_canvas_mut(canvas, false, |c| c.$method(x, y)) }
        }
    };
}

canvas_dot_fn!(
    /// Turns the dot at `(x, y)` on. Returns `false` if it was out of bounds.
    rustille_canvas_set, set
);
canvas_dot_fn!(
    /// Turns the dot at `(x, y)` off. Returns `false` if it was out of bounds.
    rustille_canvas_unset, unset
);
canvas_dot_fn!(
    /// Flips the dot at `(x, y)`. Returns `false` if it was out of bounds.
    rustille_canvas_toggle, toggle
);

/// Reads the dot at `(x, y)`. Out-of-bounds dots read as `false`.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_get(
    canvas: *const RustilleCanvas,
    x: i32,
    y: i32,
) -> bool {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas(canvas, false, |c| c.get(x, y)) }
}

/// Canvas width in dots, or `0` for a null pointer.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_width(canvas: *const RustilleCanvas) -> u32 {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas(canvas, 0, Canvas::width) }
}

/// Canvas height in dots, or `0` for a null pointer.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_height(canvas: *const RustilleCanvas) -> u32 {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas(canvas, 0, Canvas::height) }
}

/// Number of dots currently set, or `0` for a null pointer.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_count(canvas: *const RustilleCanvas) -> u32 {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas(canvas, 0, Canvas::count) }
}

/// Clears every dot.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_clear(canvas: *mut RustilleCanvas) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), Canvas::clear) }
}

/// Sets every dot inside the canvas.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_fill(canvas: *mut RustilleCanvas) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), Canvas::fill) }
}

/// Draws a Bresenham line from `(x0, y0)` to `(x1, y1)`.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_line(
    canvas: *mut RustilleCanvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), |c| c.line(x0, y0, x1, y1)) }
}

/// Draws a rectangle outline through the two given corners (inclusive).
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_rectangle(
    canvas: *mut RustilleCanvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), |c| c.rectangle(x0, y0, x1, y1)) }
}

/// Fills the rectangle through the two given corners (inclusive).
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_filled_rectangle(
    canvas: *mut RustilleCanvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), |c| c.filled_rectangle(x0, y0, x1, y1)) }
}

/// Draws a circle outline of `radius` dots centred on `(cx, cy)`.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_circle(
    canvas: *mut RustilleCanvas,
    cx: i32,
    cy: i32,
    radius: i32,
) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), |c| c.circle(cx, cy, radius)) }
}

/// Fills a disc of `radius` dots centred on `(cx, cy)`.
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_filled_circle(
    canvas: *mut RustilleCanvas,
    cx: i32,
    cy: i32,
    radius: i32,
) {
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe { with_canvas_mut(canvas, (), |c| c.filled_circle(cx, cy, radius)) }
}

/// Renders the canvas into a caller-owned UTF-8 string.
///
/// Release the string with [`rustille_string_free`].
///
/// # Safety
///
/// `canvas` must be null or a live pointer from [`rustille_canvas_new`], and
/// `out` must point to a writable `char *`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustille_canvas_render(
    canvas: *const RustilleCanvas,
    out: *mut *mut c_char,
) -> RustilleStatus {
    if canvas.is_null() || out.is_null() {
        return RustilleStatus::NullPointer;
    }
    // SAFETY: forwarded unchanged; the caller upholds the contract.
    unsafe {
        with_canvas(canvas, RustilleStatus::Panic, |c| {
            deliver(Ok(c.render()), out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> RustilleOptions {
        let mut options = RustilleOptions::default();
        let status = unsafe { rustille_options_init(&raw mut options) };
        assert_eq!(status, RustilleStatus::Ok);
        options
    }

    fn take_string(pointer: *mut c_char) -> String {
        assert!(!pointer.is_null());
        let text = unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .expect("rendered output is UTF-8")
            .to_owned();
        unsafe { rustille_string_free(pointer) };
        text
    }

    #[test]
    fn version_is_exported() {
        let version = unsafe { CStr::from_ptr(rustille_version()) };
        assert_eq!(version.to_str().unwrap(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn status_messages_exist_for_every_code() {
        for status in [
            RustilleStatus::Ok,
            RustilleStatus::NullPointer,
            RustilleStatus::InvalidUtf8,
            RustilleStatus::InvalidEnum,
            RustilleStatus::Decode,
            RustilleStatus::UnsupportedFormat,
            RustilleStatus::InvalidDimensions,
            RustilleStatus::InvalidBufferLength,
            RustilleStatus::InvalidOptions,
            RustilleStatus::Io,
            RustilleStatus::Panic,
            RustilleStatus::Unsupported,
        ] {
            let message = unsafe { CStr::from_ptr(rustille_status_message(status)) };
            assert!(!message.to_bytes().is_empty(), "{status:?}");
        }
    }

    #[test]
    fn options_init_matches_library_defaults() {
        let options = default_options();
        let converted = RenderOptions::from(&options);
        assert_eq!(
            converted,
            RenderOptions::default().background(Background::Rgb(0, 0, 0))
        );
        assert_eq!(converted.width, None);
    }

    #[test]
    fn renders_rgba() {
        let mut options = default_options();
        options.width = 1;
        options.height = 1;
        options.fit = RustilleFit::Stretch;
        let pixels = [255u8; 4];
        let mut out: *mut c_char = std::ptr::null_mut();
        let status = unsafe {
            rustille_render_rgba(
                pixels.as_ptr(),
                pixels.len(),
                1,
                1,
                &raw const options,
                &raw mut out,
            )
        };
        assert_eq!(status, RustilleStatus::Ok);
        assert_eq!(take_string(out), "\u{28FF}");
    }

    #[test]
    fn null_pointers_are_rejected() {
        let options = default_options();
        let mut out: *mut c_char = std::ptr::null_mut();
        let status = unsafe {
            rustille_render_rgba(std::ptr::null(), 0, 1, 1, &raw const options, &raw mut out)
        };
        assert_eq!(status, RustilleStatus::NullPointer);

        let pixels = [0u8; 4];
        let status = unsafe {
            rustille_render_rgba(
                pixels.as_ptr(),
                pixels.len(),
                1,
                1,
                std::ptr::null(),
                &raw mut out,
            )
        };
        assert_eq!(status, RustilleStatus::NullPointer);
        assert_eq!(
            unsafe { rustille_options_init(std::ptr::null_mut()) },
            RustilleStatus::NullPointer
        );
        unsafe { rustille_string_free(std::ptr::null_mut()) };
    }

    #[test]
    fn bad_buffer_length_reports_an_error_message() {
        let options = default_options();
        let pixels = [0u8; 3];
        let mut out: *mut c_char = std::ptr::null_mut();
        let status = unsafe {
            rustille_render_rgba(
                pixels.as_ptr(),
                pixels.len(),
                2,
                2,
                &raw const options,
                &raw mut out,
            )
        };
        assert_eq!(status, RustilleStatus::InvalidBufferLength);
        assert!(out.is_null());
        let message = take_string(rustille_last_error_message());
        assert!(message.contains("invalid buffer length"), "{message}");
    }

    #[test]
    fn canvas_round_trip() {
        let canvas = rustille_canvas_new(4, 4);
        assert!(!canvas.is_null());
        assert_eq!(unsafe { rustille_canvas_width(canvas) }, 4);
        assert_eq!(unsafe { rustille_canvas_height(canvas) }, 4);
        assert!(unsafe { rustille_canvas_set(canvas, 0, 0) });
        assert!(unsafe { rustille_canvas_get(canvas, 0, 0) });
        assert!(!unsafe { rustille_canvas_set(canvas, 99, 99) });
        assert_eq!(unsafe { rustille_canvas_count(canvas) }, 1);
        unsafe { rustille_canvas_line(canvas, 0, 0, 3, 3) };
        unsafe { rustille_canvas_rectangle(canvas, 0, 0, 3, 3) };
        unsafe { rustille_canvas_filled_rectangle(canvas, 1, 1, 2, 2) };
        unsafe { rustille_canvas_circle(canvas, 2, 2, 1) };
        unsafe { rustille_canvas_filled_circle(canvas, 2, 2, 1) };
        unsafe { rustille_canvas_fill(canvas) };
        assert_eq!(unsafe { rustille_canvas_count(canvas) }, 16);

        let mut out: *mut c_char = std::ptr::null_mut();
        let status = unsafe { rustille_canvas_render(canvas, &raw mut out) };
        assert_eq!(status, RustilleStatus::Ok);
        assert_eq!(take_string(out), "\u{28FF}\u{28FF}");

        unsafe { rustille_canvas_clear(canvas) };
        assert_eq!(unsafe { rustille_canvas_count(canvas) }, 0);
        unsafe { rustille_canvas_free(canvas) };
        unsafe { rustille_canvas_free(std::ptr::null_mut()) };
    }

    #[test]
    fn canvas_rejects_absurd_sizes() {
        assert!(rustille_canvas_new(u32::MAX, u32::MAX).is_null());
        let message = take_string(rustille_last_error_message());
        assert!(message.contains("cell limit"), "{message}");
    }

    #[test]
    fn null_canvas_operations_are_inert() {
        assert!(!unsafe { rustille_canvas_set(std::ptr::null_mut(), 0, 0) });
        assert!(!unsafe { rustille_canvas_get(std::ptr::null(), 0, 0) });
        assert_eq!(unsafe { rustille_canvas_width(std::ptr::null()) }, 0);
        unsafe { rustille_canvas_clear(std::ptr::null_mut()) };
        let mut out: *mut c_char = std::ptr::null_mut();
        assert_eq!(
            unsafe { rustille_canvas_render(std::ptr::null(), &raw mut out) },
            RustilleStatus::NullPointer
        );
    }
}
