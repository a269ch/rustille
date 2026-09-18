//! PyO3 bindings for Rustille.
//!
//! This module is a thin adapter: it parses Python arguments into
//! [`rustille::RenderOptions`] and calls the same core API that the CLI and the
//! other bindings use. No rendering logic lives here.

use pyo3::create_exception;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;

use rustille::{
    Background, Canvas as CoreCanvas, ColorMode, Dither, ErrorKind, Fit, RenderOptions, Renderer,
};

create_exception!(
    _rustille,
    RustilleError,
    pyo3::exceptions::PyException,
    "Base class for every Rustille-specific error."
);
create_exception!(
    _rustille,
    DecodeError,
    RustilleError,
    "Raised when image bytes cannot be decoded."
);
create_exception!(
    _rustille,
    UnsupportedFormatError,
    DecodeError,
    "Raised when the image format is not supported by this build."
);

/// Converts a core error into the closest Python exception.
fn to_py_error(error: rustille::Error) -> PyErr {
    let message = error.to_string();
    match error.kind() {
        ErrorKind::Decode => DecodeError::new_err(message),
        ErrorKind::UnsupportedFormat => UnsupportedFormatError::new_err(message),
        ErrorKind::InvalidDimensions
        | ErrorKind::InvalidBufferLength
        | ErrorKind::InvalidOptions => PyValueError::new_err(message),
        ErrorKind::Io => PyOSError::new_err(message),
        _ => RustilleError::new_err(message),
    }
}

/// Parses a `str` option into an enum, reporting a `ValueError` on failure.
fn parse_option<T: std::str::FromStr<Err = rustille::Error>>(
    value: Option<&str>,
    fallback: T,
) -> PyResult<T> {
    match value {
        None => Ok(fallback),
        Some(text) => text.parse().map_err(to_py_error),
    }
}

/// Every keyword argument the renderers share.
#[allow(clippy::too_many_arguments)]
fn build_options(
    width: Option<u32>,
    height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<RenderOptions> {
    let defaults = RenderOptions::default();
    let mut options = RenderOptions::default()
        .threshold(threshold.unwrap_or(defaults.threshold))
        .invert(invert)
        .dither(parse_option::<Dither>(dither, defaults.dither)?)
        .color(parse_option::<ColorMode>(color, defaults.color)?)
        .background(parse_option::<Background>(background, defaults.background)?)
        .fit(parse_option::<Fit>(fit, defaults.fit)?)
        .cell_aspect_ratio(cell_aspect_ratio.unwrap_or(defaults.cell_aspect_ratio));
    options.width = width;
    options.height = height;
    options.validate().map_err(to_py_error)?;
    Ok(options)
}

/// Renders an image file.
#[pyfunction]
#[pyo3(signature = (
    path,
    *,
    width = None,
    height = None,
    threshold = None,
    invert = false,
    dither = None,
    color = None,
    background = None,
    fit = None,
    cell_aspect_ratio = None,
))]
#[allow(clippy::too_many_arguments)]
fn render_file(
    py: Python<'_>,
    path: std::path::PathBuf,
    width: Option<u32>,
    height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    let options = build_options(
        width,
        height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )?;
    // Rendering is pure CPU work with no Python objects involved, so other
    // threads may run meanwhile.
    py.detach(|| Renderer::new(options).render_file(path))
        .map_err(to_py_error)
}

/// Renders an encoded image held in memory.
#[pyfunction]
#[pyo3(signature = (
    data,
    *,
    width = None,
    height = None,
    threshold = None,
    invert = false,
    dither = None,
    color = None,
    background = None,
    fit = None,
    cell_aspect_ratio = None,
))]
#[allow(clippy::too_many_arguments)]
fn render_bytes(
    py: Python<'_>,
    data: &[u8],
    width: Option<u32>,
    height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    let options = build_options(
        width,
        height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )?;
    py.detach(|| Renderer::new(options).render_bytes(data))
        .map_err(to_py_error)
}

/// Shared body of the raw-pixel entry points.
#[allow(clippy::too_many_arguments)]
fn render_raw(
    py: Python<'_>,
    render: fn(&Renderer, u32, u32, &[u8]) -> rustille::Result<String>,
    data: &[u8],
    width: u32,
    height: u32,
    output_width: Option<u32>,
    output_height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    let options = build_options(
        output_width,
        output_height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )?;
    let renderer = Renderer::new(options);
    py.detach(|| render(&renderer, width, height, data))
        .map_err(to_py_error)
}

/// Renders a tightly packed RGBA8 buffer (`width * height * 4` bytes).
#[pyfunction]
#[pyo3(signature = (
    data,
    *,
    width,
    height,
    output_width = None,
    output_height = None,
    threshold = None,
    invert = false,
    dither = None,
    color = None,
    background = None,
    fit = None,
    cell_aspect_ratio = None,
))]
#[allow(clippy::too_many_arguments)]
fn render_rgba(
    py: Python<'_>,
    data: &[u8],
    width: u32,
    height: u32,
    output_width: Option<u32>,
    output_height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    render_raw(
        py,
        |renderer, w, h, bytes| renderer.render_rgba(w, h, bytes),
        data,
        width,
        height,
        output_width,
        output_height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )
}

/// Renders a tightly packed RGB8 buffer (`width * height * 3` bytes).
#[pyfunction]
#[pyo3(signature = (
    data,
    *,
    width,
    height,
    output_width = None,
    output_height = None,
    threshold = None,
    invert = false,
    dither = None,
    color = None,
    background = None,
    fit = None,
    cell_aspect_ratio = None,
))]
#[allow(clippy::too_many_arguments)]
fn render_rgb(
    py: Python<'_>,
    data: &[u8],
    width: u32,
    height: u32,
    output_width: Option<u32>,
    output_height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    render_raw(
        py,
        |renderer, w, h, bytes| renderer.render_rgb(w, h, bytes),
        data,
        width,
        height,
        output_width,
        output_height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )
}

/// Renders a tightly packed 8-bit grayscale buffer (`width * height` bytes).
#[pyfunction]
#[pyo3(signature = (
    data,
    *,
    width,
    height,
    output_width = None,
    output_height = None,
    threshold = None,
    invert = false,
    dither = None,
    color = None,
    background = None,
    fit = None,
    cell_aspect_ratio = None,
))]
#[allow(clippy::too_many_arguments)]
fn render_luma(
    py: Python<'_>,
    data: &[u8],
    width: u32,
    height: u32,
    output_width: Option<u32>,
    output_height: Option<u32>,
    threshold: Option<u8>,
    invert: bool,
    dither: Option<&str>,
    color: Option<&str>,
    background: Option<&str>,
    fit: Option<&str>,
    cell_aspect_ratio: Option<f32>,
) -> PyResult<String> {
    render_raw(
        py,
        |renderer, w, h, bytes| renderer.render_luma(w, h, bytes),
        data,
        width,
        height,
        output_width,
        output_height,
        threshold,
        invert,
        dither,
        color,
        background,
        fit,
        cell_aspect_ratio,
    )
}

/// The Braille character for a dot mask (`0..=255`).
#[pyfunction]
fn braille_char(mask: u8) -> String {
    rustille::braille::char_for_mask(mask).to_string()
}

/// The dot mask of a Braille character, or `None` if it is not one.
#[pyfunction]
fn braille_mask(character: &str) -> Option<u8> {
    let mut chars = character.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    rustille::braille::mask_for_char(first)
}

/// A fixed-size grid of Braille dots.
#[pyclass(module = "rustille._rustille")]
struct Canvas {
    inner: CoreCanvas,
}

#[pymethods]
impl Canvas {
    /// Creates a canvas ``width`` x ``height`` **dots** in size.
    #[new]
    fn new(width: u32, height: u32) -> PyResult<Self> {
        CoreCanvas::try_new(width, height)
            .map(|inner| Self { inner })
            .map_err(to_py_error)
    }

    /// Canvas width in dots.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Canvas height in dots.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Rendered width in characters.
    #[getter]
    fn cells_width(&self) -> u32 {
        self.inner.cells_width()
    }

    /// Rendered height in characters.
    #[getter]
    fn cells_height(&self) -> u32 {
        self.inner.cells_height()
    }

    /// Turns the dot at ``(x, y)`` on. Returns ``False`` if it is out of range.
    fn set(&mut self, x: i32, y: i32) -> bool {
        self.inner.set(x, y)
    }

    /// Turns the dot at ``(x, y)`` off.
    fn unset(&mut self, x: i32, y: i32) -> bool {
        self.inner.unset(x, y)
    }

    /// Flips the dot at ``(x, y)``.
    fn toggle(&mut self, x: i32, y: i32) -> bool {
        self.inner.toggle(x, y)
    }

    /// Reads the dot at ``(x, y)``. Out-of-range dots read as ``False``.
    fn get(&self, x: i32, y: i32) -> bool {
        self.inner.get(x, y)
    }

    /// Clears every dot.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Sets every dot inside the canvas.
    fn fill(&mut self) {
        self.inner.fill();
    }

    /// Number of dots currently set.
    fn count(&self) -> u32 {
        self.inner.count()
    }

    /// Draws a Bresenham line from ``(x0, y0)`` to ``(x1, y1)``.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.line(x0, y0, x1, y1);
    }

    /// Draws a rectangle outline through two opposite corners.
    fn rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.rectangle(x0, y0, x1, y1);
    }

    /// Fills a rectangle through two opposite corners.
    fn filled_rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.inner.filled_rectangle(x0, y0, x1, y1);
    }

    /// Draws a circle outline of ``radius`` dots centred on ``(cx, cy)``.
    fn circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.circle(cx, cy, radius);
    }

    /// Fills a disc of ``radius`` dots centred on ``(cx, cy)``.
    fn filled_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.inner.filled_circle(cx, cy, radius);
    }

    /// Renders the canvas to a newline-separated Braille string.
    fn render(&self) -> String {
        self.inner.render()
    }

    fn __str__(&self) -> String {
        self.inner.render()
    }

    fn __repr__(&self) -> String {
        format!(
            "Canvas(width={}, height={}, dots_set={})",
            self.inner.width(),
            self.inner.height(),
            self.inner.count()
        )
    }
}

/// Native extension module. The public API is re-exported by `rustille`.
#[pymodule]
fn _rustille(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", rustille::VERSION)?;

    module.add("RustilleError", module.py().get_type::<RustilleError>())?;
    module.add("DecodeError", module.py().get_type::<DecodeError>())?;
    module.add(
        "UnsupportedFormatError",
        module.py().get_type::<UnsupportedFormatError>(),
    )?;

    module.add_function(wrap_pyfunction!(render_file, module)?)?;
    module.add_function(wrap_pyfunction!(render_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(render_rgba, module)?)?;
    module.add_function(wrap_pyfunction!(render_rgb, module)?)?;
    module.add_function(wrap_pyfunction!(render_luma, module)?)?;
    module.add_function(wrap_pyfunction!(braille_char, module)?)?;
    module.add_function(wrap_pyfunction!(braille_mask, module)?)?;
    module.add_class::<Canvas>()?;
    Ok(())
}
