//! The image → Braille pipeline.
//!
//! ```text
//! decode ─► alpha compositing ─► fit/crop ─► resize ─► luminance
//!        ─► invert ─► dither ─► threshold ─► 2×4 packing ─► Unicode
//! ```
//!
//! Every entry point funnels into the same private `render_surface`, so the
//! CLI, the C ABI and the Python/Node/WASM bindings all execute identical code.

use crate::braille;
use crate::color::{self, RESET};
use crate::dithering::dither_plane;
use crate::error::{Error, Result};
use crate::image::Surface;
use crate::options::{DEFAULT_WIDTH, Fit, MAX_OUTPUT_CELLS, RenderOptions};

/// Renders images to Braille according to a [`RenderOptions`].
///
/// The renderer is a plain value: cloning it is free and it holds no buffers
/// between calls.
///
/// ```
/// use rustille::{RenderOptions, Renderer};
///
/// # fn main() -> Result<(), rustille::Error> {
/// let renderer = Renderer::new(RenderOptions::default().width(8));
/// let rgba = vec![255u8; 16 * 16 * 4];
/// assert!(renderer.render_rgba(16, 16, &rgba)?.starts_with('\u{28FF}'));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Renderer {
    options: RenderOptions,
}

impl Renderer {
    /// Creates a renderer from the given options.
    #[must_use]
    pub const fn new(options: RenderOptions) -> Self {
        Self { options }
    }

    /// The options this renderer was built with.
    #[must_use]
    pub const fn options(&self) -> &RenderOptions {
        &self.options
    }

    /// Replaces the options.
    pub const fn set_options(&mut self, options: RenderOptions) {
        self.options = options;
    }

    /// The output size, in character cells, that a `source_width` ×
    /// `source_height` image would render to.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOptions`] for inconsistent options and
    /// [`Error::InvalidDimensions`] for a zero-sized or absurdly large result.
    pub fn output_size(&self, source_width: u32, source_height: u32) -> Result<(u32, u32)> {
        self.options.validate()?;
        resolve_target(source_width, source_height, &self.options)
    }

    /// Renders a tightly packed RGBA8 buffer.
    ///
    /// `rgba` must hold exactly `width * height * 4` bytes. Transparent pixels
    /// are composited over [`RenderOptions::background`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDimensions`], [`Error::InvalidBufferLength`] or
    /// [`Error::InvalidOptions`].
    pub fn render_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> Result<String> {
        self.options.validate()?;
        let surface = Surface::from_rgba(width, height, rgba, self.options.background.rgb())?;
        self.render_surface(&surface)
    }

    /// Renders a tightly packed RGB8 buffer (`width * height * 3` bytes).
    ///
    /// # Errors
    ///
    /// As [`Renderer::render_rgba`].
    pub fn render_rgb(&self, width: u32, height: u32, rgb: &[u8]) -> Result<String> {
        self.options.validate()?;
        let surface = Surface::from_rgb(width, height, rgb)?;
        self.render_surface(&surface)
    }

    /// Renders a tightly packed 8-bit grayscale buffer (`width * height` bytes).
    ///
    /// # Errors
    ///
    /// As [`Renderer::render_rgba`].
    pub fn render_luma(&self, width: u32, height: u32, luma: &[u8]) -> Result<String> {
        self.options.validate()?;
        let surface = Surface::from_luma(width, height, luma)?;
        self.render_surface(&surface)
    }

    /// Decodes an encoded image from memory and renders it.
    ///
    /// The format is sniffed from the bytes, not from a file name.
    ///
    /// # Errors
    ///
    /// [`Error::Decode`] or [`Error::UnsupportedFormat`] for bad input, plus
    /// the errors of [`Renderer::render_rgba`].
    #[cfg(feature = "_image")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decode")))]
    pub fn render_bytes(&self, bytes: &[u8]) -> Result<String> {
        self.options.validate()?;
        let decoded = crate::image::from_bytes(bytes)?;
        self.render_decoded(decoded)
    }

    /// Decodes an image file and renders it.
    ///
    /// ```no_run
    /// use rustille::{RenderOptions, Renderer};
    ///
    /// # fn main() -> Result<(), rustille::Error> {
    /// let renderer = Renderer::new(RenderOptions::default().width(80));
    /// println!("{}", renderer.render_file("cat.png")?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the file cannot be read, plus the errors of
    /// [`Renderer::render_bytes`].
    #[cfg(feature = "_image")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decode")))]
    pub fn render_file(&self, path: impl AsRef<std::path::Path>) -> Result<String> {
        self.options.validate()?;
        let decoded = crate::image::from_path(path.as_ref())?;
        self.render_decoded(decoded)
    }

    /// Decodes an image from a seekable reader and renders it.
    ///
    /// # Errors
    ///
    /// As [`Renderer::render_bytes`].
    #[cfg(feature = "_image")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decode")))]
    pub fn render_reader<R: std::io::BufRead + std::io::Seek>(&self, reader: R) -> Result<String> {
        self.options.validate()?;
        let decoded = crate::image::from_reader(reader)?;
        self.render_decoded(decoded)
    }

    #[cfg(feature = "_image")]
    fn render_decoded(&self, decoded: crate::image::Decoded) -> Result<String> {
        let surface = Surface::from_rgba(
            decoded.width,
            decoded.height,
            &decoded.rgba,
            self.options.background.rgb(),
        )?;
        self.render_surface(&surface)
    }

    fn render_surface(&self, surface: &Surface) -> Result<String> {
        let options = &self.options;
        let (cells_width, cells_height) =
            resolve_target(surface.width(), surface.height(), options)?;

        let source = match options.fit {
            Fit::Fill => cover_crop(
                surface,
                cells_width,
                cells_height,
                options.cell_aspect_ratio,
            ),
            _ => surface.clone(),
        };

        let dots_width = cells_width * braille::CELL_WIDTH;
        let dots_height = cells_height * braille::CELL_HEIGHT;
        let scaled = source.resize(dots_width, dots_height)?;

        let mut plane = scaled.luminance_plane();
        if options.invert {
            for value in &mut plane {
                *value = 255.0 - *value;
            }
        }
        dither_plane(
            &mut plane,
            dots_width as usize,
            dots_height as usize,
            options.threshold,
            options.dither,
        );

        Ok(pack_cells(
            &plane,
            scaled.data(),
            cells_width,
            cells_height,
            options,
        ))
    }
}

/// Turns a thresholded luminance plane into the final string.
fn pack_cells(
    plane: &[f32],
    rgb: &[u8],
    cells_width: u32,
    cells_height: u32,
    options: &RenderOptions,
) -> String {
    let color_mode = options.color;
    let cut = f32::from(options.threshold);
    let dots_width = cells_width as usize * braille::CELL_WIDTH as usize;

    let per_cell = 3 + color::escape_budget(color_mode);
    let per_row = cells_width as usize * per_cell + 1 + RESET.len();
    let mut out = String::with_capacity(cells_height as usize * per_row);

    for cell_y in 0..cells_height as usize {
        if cell_y > 0 {
            out.push('\n');
        }
        let mut active: Option<[u8; 3]> = None;
        for cell_x in 0..cells_width as usize {
            let mut mask = 0u8;
            let mut sum = [0u32; 3];
            let mut lit = 0u32;

            for dot_y in 0..braille::CELL_HEIGHT as usize {
                let row = (cell_y * braille::CELL_HEIGHT as usize + dot_y) * dots_width;
                for dot_x in 0..braille::CELL_WIDTH as usize {
                    let index = row + cell_x * braille::CELL_WIDTH as usize + dot_x;
                    if plane[index] < cut {
                        continue;
                    }
                    mask |= braille::dot_bit_or_zero(dot_x as u32, dot_y as u32);
                    if color_mode.is_enabled() {
                        let base = index * 3;
                        sum[0] += u32::from(rgb[base]);
                        sum[1] += u32::from(rgb[base + 1]);
                        sum[2] += u32::from(rgb[base + 2]);
                        lit += 1;
                    }
                }
            }

            if color_mode.is_enabled() && lit > 0 {
                let average = [
                    (sum[0] / lit) as u8,
                    (sum[1] / lit) as u8,
                    (sum[2] / lit) as u8,
                ];
                if active != Some(average) {
                    color::push_foreground(&mut out, color_mode, average);
                    active = Some(average);
                }
            }

            out.push(braille::char_for_mask(mask));
        }

        if active.is_some() {
            out.push_str(RESET);
        }
    }

    out
}

/// Crops `surface` to the aspect ratio of the output box, centred.
fn cover_crop(surface: &Surface, cells_width: u32, cells_height: u32, cell_aspect: f32) -> Surface {
    let box_aspect = f64::from(cells_width) / (f64::from(cells_height) * f64::from(cell_aspect));
    let source_aspect = f64::from(surface.width()) / f64::from(surface.height());
    if (source_aspect - box_aspect).abs() < 1e-9 {
        return surface.clone();
    }
    if source_aspect > box_aspect {
        let width =
            ((f64::from(surface.height()) * box_aspect).round() as u32).clamp(1, surface.width());
        let x = (surface.width() - width) / 2;
        surface.crop(x, 0, width, surface.height())
    } else {
        let height =
            ((f64::from(surface.width()) / box_aspect).round() as u32).clamp(1, surface.height());
        let y = (surface.height() - height) / 2;
        surface.crop(0, y, surface.width(), height)
    }
}

/// Works out the output size in character cells.
///
/// One cell is 2 dots wide and 4 dots tall but is physically
/// `cell_aspect_ratio` times taller than it is wide, so a cell covers a square
/// area exactly when that ratio is 2.
fn resolve_target(
    source_width: u32,
    source_height: u32,
    options: &RenderOptions,
) -> Result<(u32, u32)> {
    if source_width == 0 || source_height == 0 {
        return Err(Error::invalid_dimensions(format!(
            "source image must be non-empty, got {source_width}x{source_height}"
        )));
    }

    let aspect = f64::from(source_width) / f64::from(source_height);
    let cell_aspect = f64::from(options.cell_aspect_ratio);
    let height_for = |width: u32| -> u32 {
        ((f64::from(width) / (aspect * cell_aspect)).round() as i64).clamp(1, i64::from(u32::MAX))
            as u32
    };
    let width_for = |height: u32| -> u32 {
        ((f64::from(height) * aspect * cell_aspect).round() as i64).clamp(1, i64::from(u32::MAX))
            as u32
    };

    let (width, height) = match (options.width, options.height) {
        (Some(width), Some(height)) => match options.fit {
            Fit::Stretch | Fit::Fill => (width, height),
            Fit::Contain => {
                let fitted_height = height_for(width);
                if fitted_height <= height {
                    (width, fitted_height)
                } else {
                    (width_for(height).min(width), height)
                }
            }
        },
        (Some(width), None) => (width, height_for(width)),
        (None, Some(height)) => (width_for(height), height),
        (None, None) => (DEFAULT_WIDTH, height_for(DEFAULT_WIDTH)),
    };

    let cells = u64::from(width) * u64::from(height);
    if cells > MAX_OUTPUT_CELLS {
        return Err(Error::invalid_dimensions(format!(
            "output would be {width}x{height} = {cells} character cells, over the \
             {MAX_OUTPUT_CELLS} cell limit"
        )));
    }

    Ok((width, height))
}

/// Renders a tightly packed RGBA8 buffer. See [`Renderer::render_rgba`].
///
/// # Errors
///
/// As [`Renderer::render_rgba`].
pub fn render_rgba(
    width: u32,
    height: u32,
    rgba: &[u8],
    options: &RenderOptions,
) -> Result<String> {
    Renderer::new(*options).render_rgba(width, height, rgba)
}

/// Renders a tightly packed RGB8 buffer. See [`Renderer::render_rgb`].
///
/// # Errors
///
/// As [`Renderer::render_rgb`].
pub fn render_rgb(width: u32, height: u32, rgb: &[u8], options: &RenderOptions) -> Result<String> {
    Renderer::new(*options).render_rgb(width, height, rgb)
}

/// Renders a tightly packed grayscale buffer. See [`Renderer::render_luma`].
///
/// # Errors
///
/// As [`Renderer::render_luma`].
pub fn render_luma(
    width: u32,
    height: u32,
    luma: &[u8],
    options: &RenderOptions,
) -> Result<String> {
    Renderer::new(*options).render_luma(width, height, luma)
}

/// Decodes and renders an encoded image. See [`Renderer::render_bytes`].
///
/// # Errors
///
/// As [`Renderer::render_bytes`].
#[cfg(feature = "_image")]
#[cfg_attr(docsrs, doc(cfg(feature = "decode")))]
pub fn render_bytes(bytes: &[u8], options: &RenderOptions) -> Result<String> {
    Renderer::new(*options).render_bytes(bytes)
}

/// Decodes and renders an image file. See [`Renderer::render_file`].
///
/// # Errors
///
/// As [`Renderer::render_file`].
#[cfg(feature = "_image")]
#[cfg_attr(docsrs, doc(cfg(feature = "decode")))]
pub fn render_file(path: impl AsRef<std::path::Path>, options: &RenderOptions) -> Result<String> {
    Renderer::new(*options).render_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Background, ColorMode, Dither};

    fn options() -> RenderOptions {
        RenderOptions::default()
    }

    #[test]
    fn single_white_pixel_fills_one_cell() {
        let renderer = Renderer::new(options().size(1, 1).fit(Fit::Stretch));
        let out = renderer.render_luma(1, 1, &[255]).unwrap();
        assert_eq!(out, "\u{28FF}");
    }

    #[test]
    fn single_black_pixel_is_blank() {
        let renderer = Renderer::new(options().size(1, 1).fit(Fit::Stretch));
        assert_eq!(renderer.render_luma(1, 1, &[0]).unwrap(), "\u{2800}");
    }

    #[test]
    fn inversion_flips_the_output() {
        let plain = Renderer::new(options().size(1, 1).fit(Fit::Stretch))
            .render_luma(1, 1, &[0])
            .unwrap();
        let inverted = Renderer::new(options().size(1, 1).fit(Fit::Stretch).invert(true))
            .render_luma(1, 1, &[0])
            .unwrap();
        assert_eq!(plain, "\u{2800}");
        assert_eq!(inverted, "\u{28FF}");
    }

    #[test]
    fn threshold_decides_which_dots_light_up() {
        let pixels = [100u8];
        let below = Renderer::new(options().size(1, 1).fit(Fit::Stretch).threshold(150))
            .render_luma(1, 1, &pixels)
            .unwrap();
        let above = Renderer::new(options().size(1, 1).fit(Fit::Stretch).threshold(50))
            .render_luma(1, 1, &pixels)
            .unwrap();
        assert_eq!(below, "\u{2800}");
        assert_eq!(above, "\u{28FF}");
    }

    #[test]
    fn each_dot_position_maps_to_the_right_bit() {
        for y in 0..4u32 {
            for x in 0..2u32 {
                let mut pixels = [0u8; 8];
                pixels[(y * 2 + x) as usize] = 255;
                let out = Renderer::new(options().size(1, 1).fit(Fit::Stretch))
                    .render_luma(2, 4, &pixels)
                    .unwrap();
                let mask = braille::mask_for_char(out.chars().next().unwrap()).unwrap();
                assert_eq!(Some(mask), braille::dot_bit(x, y), "pixel {x},{y}");
            }
        }
    }

    #[test]
    fn alpha_is_composited_over_the_background() {
        let rgba = [255, 255, 255, 0];
        let over_black = Renderer::new(options().size(1, 1).fit(Fit::Stretch))
            .render_rgba(1, 1, &rgba)
            .unwrap();
        let over_white = Renderer::new(
            options()
                .size(1, 1)
                .fit(Fit::Stretch)
                .background(Background::White),
        )
        .render_rgba(1, 1, &rgba)
        .unwrap();
        assert_eq!(over_black, "\u{2800}");
        assert_eq!(over_white, "\u{28FF}");
    }

    #[test]
    fn aspect_ratio_drives_the_derived_dimension() {
        let renderer = Renderer::new(options().width(40));
        assert_eq!(renderer.output_size(100, 100).unwrap(), (40, 20));
        assert_eq!(renderer.output_size(200, 100).unwrap(), (40, 10));

        let square_cells = Renderer::new(options().width(40).cell_aspect_ratio(1.0));
        assert_eq!(square_cells.output_size(100, 100).unwrap(), (40, 40));

        let by_height = Renderer::new(options().height(20));
        assert_eq!(by_height.output_size(100, 100).unwrap(), (40, 20));
    }

    #[test]
    fn default_size_is_terminal_independent() {
        let renderer = Renderer::new(options());
        assert_eq!(renderer.output_size(100, 100).unwrap(), (DEFAULT_WIDTH, 40));
    }

    #[test]
    fn fit_modes_differ_on_a_mismatched_box() {
        let contain = Renderer::new(options().size(40, 40).fit(Fit::Contain));
        assert_eq!(contain.output_size(100, 100).unwrap(), (40, 20));

        let stretch = Renderer::new(options().size(40, 40).fit(Fit::Stretch));
        assert_eq!(stretch.output_size(100, 100).unwrap(), (40, 40));

        let fill = Renderer::new(options().size(40, 40).fit(Fit::Fill));
        assert_eq!(fill.output_size(100, 100).unwrap(), (40, 40));
    }

    #[test]
    fn contain_shrinks_the_other_axis_too() {
        let contain = Renderer::new(options().size(80, 10).fit(Fit::Contain));
        let (w, h) = contain.output_size(100, 400).unwrap();
        assert_eq!(h, 10);
        assert!(w < 80, "expected the width to shrink, got {w}");
    }

    #[test]
    fn fill_crops_instead_of_squashing() {
        let mut rgb = vec![0u8; 8 * 4 * 3];
        for y in 0..4 {
            for x in 0..4 {
                let base = (y * 8 + x) * 3;
                rgb[base] = 255;
                rgb[base + 1] = 255;
                rgb[base + 2] = 255;
            }
        }
        let out = Renderer::new(options().size(2, 1).fit(Fit::Fill))
            .render_rgb(8, 4, &rgb)
            .unwrap();
        assert_eq!(out.chars().count(), 2);
    }

    #[test]
    fn output_lines_match_the_requested_height() {
        let pixels = vec![255u8; 64 * 64];
        let out = Renderer::new(options().size(10, 5).fit(Fit::Stretch))
            .render_luma(64, 64, &pixels)
            .unwrap();
        assert_eq!(out.lines().count(), 5);
        for line in out.lines() {
            assert_eq!(line.chars().count(), 10);
        }
    }

    #[test]
    fn plain_output_has_no_escape_sequences() {
        let pixels = vec![255u8; 16 * 16 * 4];
        let out = Renderer::new(options().size(4, 2).fit(Fit::Stretch))
            .render_rgba(16, 16, &pixels)
            .unwrap();
        assert!(!out.contains('\u{1b}'));
    }

    #[test]
    fn truecolor_emits_escapes_and_resets_each_row() {
        let pixels = vec![255u8; 16 * 16 * 4];
        let out = Renderer::new(
            options()
                .size(4, 2)
                .fit(Fit::Stretch)
                .color(ColorMode::TrueColor),
        )
        .render_rgba(16, 16, &pixels)
        .unwrap();
        assert!(out.contains("\x1b[38;2;255;255;255m"));
        assert_eq!(out.matches(RESET).count(), 2);
        assert_eq!(out.matches("\x1b[38;2;").count(), 2);
    }

    #[test]
    fn ansi256_uses_the_palette_form() {
        let pixels = vec![255u8; 8 * 8 * 4];
        let out = Renderer::new(
            options()
                .size(1, 1)
                .fit(Fit::Stretch)
                .color(ColorMode::Ansi256),
        )
        .render_rgba(8, 8, &pixels)
        .unwrap();
        assert!(out.starts_with("\x1b[38;5;231m"), "{out:?}");
        assert!(out.ends_with(RESET));
    }

    #[test]
    fn blank_rows_carry_no_colour() {
        let pixels = vec![0u8; 8 * 8 * 4];
        let out = Renderer::new(
            options()
                .size(1, 1)
                .fit(Fit::Stretch)
                .color(ColorMode::TrueColor),
        )
        .render_rgba(8, 8, &pixels)
        .unwrap();
        assert_eq!(out, "\u{2800}");
    }

    #[test]
    fn dithering_changes_a_flat_midtone() {
        let pixels = vec![120u8; 32 * 32];
        let plain = Renderer::new(options().size(8, 4).fit(Fit::Stretch))
            .render_luma(32, 32, &pixels)
            .unwrap();
        let dithered = Renderer::new(
            options()
                .size(8, 4)
                .fit(Fit::Stretch)
                .dither(Dither::FloydSteinberg),
        )
        .render_luma(32, 32, &pixels)
        .unwrap();
        assert!(plain.chars().all(|c| c == '\u{2800}' || c == '\n'));
        assert_ne!(plain, dithered);

        let again = Renderer::new(
            options()
                .size(8, 4)
                .fit(Fit::Stretch)
                .dither(Dither::FloydSteinberg),
        )
        .render_luma(32, 32, &pixels)
        .unwrap();
        assert_eq!(dithered, again);
    }

    #[test]
    fn rejects_empty_sources_and_bad_buffers() {
        let renderer = Renderer::new(options());
        assert!(renderer.render_luma(0, 4, &[]).is_err());
        assert!(renderer.render_luma(4, 0, &[]).is_err());
        assert!(renderer.render_rgba(2, 2, &[0; 3]).is_err());
        assert!(
            Renderer::new(options().width(0))
                .render_luma(2, 2, &[0; 4])
                .is_err()
        );
    }

    #[test]
    fn rejects_absurd_output_sizes() {
        let renderer = Renderer::new(options().size(100_000, 100_000).fit(Fit::Stretch));
        let err = renderer.render_luma(4, 4, &[0; 16]).unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::InvalidDimensions);
    }

    #[test]
    fn extreme_but_valid_output_size_works() {
        let pixels = vec![255u8; 4];
        let out = Renderer::new(options().size(2000, 2000).fit(Fit::Stretch))
            .render_luma(2, 2, &pixels)
            .unwrap();
        assert_eq!(out.lines().count(), 2000);
    }

    #[test]
    fn free_functions_match_the_renderer() {
        let opts = options().size(2, 2).fit(Fit::Stretch);
        let pixels = vec![200u8; 16];
        assert_eq!(
            render_luma(4, 4, &pixels, &opts).unwrap(),
            Renderer::new(opts).render_luma(4, 4, &pixels).unwrap()
        );
        let rgb = vec![200u8; 48];
        assert_eq!(
            render_rgb(4, 4, &rgb, &opts).unwrap(),
            Renderer::new(opts).render_rgb(4, 4, &rgb).unwrap()
        );
        let rgba = vec![200u8; 64];
        assert_eq!(
            render_rgba(4, 4, &rgba, &opts).unwrap(),
            Renderer::new(opts).render_rgba(4, 4, &rgba).unwrap()
        );
    }
}
