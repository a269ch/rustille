//! Knobs for the image renderer.

use core::fmt;
use core::str::FromStr;

use crate::error::{Error, Result};

/// Width, in character cells, used when neither a width nor a height is given.
///
/// The library never inspects the terminal; the CLI overrides this with the
/// real terminal size when stdout is a TTY.
pub const DEFAULT_WIDTH: u32 = 80;

/// Default luminance cut-off: pixels at or above it become dots.
pub const DEFAULT_THRESHOLD: u8 = 128;

/// Default physical height/width ratio of one terminal character cell.
///
/// A cell that is twice as tall as it is wide makes the 2×4 dot grid square,
/// which is what most monospace terminal fonts give you.
pub const DEFAULT_CELL_ASPECT_RATIO: f32 = 2.0;

/// Upper bound on the size of a render, in character cells.
///
/// Rendering is linear in the number of cells, so the only purpose of this cap
/// is to turn a nonsensical request into an [`Error::InvalidDimensions`]
/// instead of an out-of-memory abort.
pub const MAX_OUTPUT_CELLS: u64 = 4_194_304; // 2^22 cells ≈ 33.5M dots

/// Error-diffusion / ordered dithering algorithm.
///
/// Dithering runs on the luminance plane *before* pixels are packed into cells,
/// so it sees the real dot grid rather than the Braille characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Dither {
    /// Plain thresholding, no error diffusion.
    #[default]
    None,
    /// Floyd–Steinberg error diffusion (7/16, 3/16, 5/16, 1/16).
    FloydSteinberg,
}

/// How colour is emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ColorMode {
    /// Plain Unicode, no escape sequences. The default for the library.
    #[default]
    None,
    /// 256-colour ANSI foreground (`ESC [ 38;5;N m`).
    Ansi256,
    /// 24-bit ANSI foreground (`ESC [ 38;2;R;G;B m`).
    TrueColor,
}

/// Colour composited under transparent pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Background {
    /// Opaque black, `#000000`. The default.
    #[default]
    Black,
    /// Opaque white, `#FFFFFF`.
    White,
    /// An arbitrary opaque colour.
    Rgb(u8, u8, u8),
}

/// How the image is mapped onto the requested output box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Fit {
    /// Scale to fit inside the box, preserving the aspect ratio. The output may
    /// be smaller than the box in one dimension; nothing is padded. Default.
    #[default]
    Contain,
    /// Scale to cover the box, preserving the aspect ratio, then centre-crop.
    Fill,
    /// Scale to exactly the box, ignoring the aspect ratio.
    Stretch,
}

impl Dither {
    /// Lowercase, hyphenated name (`"none"`, `"floyd-steinberg"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Dither::None => "none",
            Dither::FloydSteinberg => "floyd-steinberg",
        }
    }

    /// Every algorithm, in declaration order. Handy for CLI help and bindings.
    pub const ALL: &'static [Dither] = &[Dither::None, Dither::FloydSteinberg];
}

impl ColorMode {
    /// Lowercase name (`"none"`, `"ansi256"`, `"truecolor"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ColorMode::None => "none",
            ColorMode::Ansi256 => "ansi256",
            ColorMode::TrueColor => "truecolor",
        }
    }

    /// Whether output will contain ANSI escape sequences.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, ColorMode::None)
    }

    /// Every mode, in declaration order.
    pub const ALL: &'static [ColorMode] =
        &[ColorMode::None, ColorMode::Ansi256, ColorMode::TrueColor];
}

impl Background {
    /// The composited colour as RGB bytes.
    #[must_use]
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Background::Black => [0, 0, 0],
            Background::White => [255, 255, 255],
            Background::Rgb(r, g, b) => [r, g, b],
        }
    }
}

impl Fit {
    /// Lowercase name (`"contain"`, `"fill"`, `"stretch"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Fit::Contain => "contain",
            Fit::Fill => "fill",
            Fit::Stretch => "stretch",
        }
    }

    /// Every mode, in declaration order.
    pub const ALL: &'static [Fit] = &[Fit::Contain, Fit::Fill, Fit::Stretch];
}

macro_rules! impl_display_and_from_str {
    ($ty:ty, $label:literal, [$($text:literal => $value:expr),+ $(,)?]) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $ty {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self> {
                let normalized = s.trim().to_ascii_lowercase().replace('_', "-");
                match normalized.as_str() {
                    $($text => Ok($value),)+
                    other => Err(Error::invalid_options(format!(
                        concat!("unknown ", $label, " {:?}; expected one of: "),
                        other
                    ) + &[$($text),+].join(", "))),
                }
            }
        }
    };
}

impl_display_and_from_str!(Dither, "dither mode", [
    "none" => Dither::None,
    "floyd-steinberg" => Dither::FloydSteinberg,
    "fs" => Dither::FloydSteinberg,
]);

impl_display_and_from_str!(ColorMode, "color mode", [
    "none" => ColorMode::None,
    "off" => ColorMode::None,
    "ansi256" => ColorMode::Ansi256,
    "256" => ColorMode::Ansi256,
    "truecolor" => ColorMode::TrueColor,
    "24bit" => ColorMode::TrueColor,
]);

impl_display_and_from_str!(Fit, "fit mode", [
    "contain" => Fit::Contain,
    "fill" => Fit::Fill,
    "cover" => Fit::Fill,
    "stretch" => Fit::Stretch,
]);

impl fmt::Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Background::Black => f.write_str("black"),
            Background::White => f.write_str("white"),
            Background::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

impl FromStr for Background {
    type Err = Error;

    /// Accepts `black`, `white`, `#rgb`, `#rrggbb` (with or without the `#`)
    /// and `r,g,b`.
    fn from_str(s: &str) -> Result<Self> {
        let raw = s.trim();
        match raw.to_ascii_lowercase().as_str() {
            "black" => return Ok(Background::Black),
            "white" => return Ok(Background::White),
            _ => {}
        }

        if let Some((r, g, b)) = parse_comma_rgb(raw) {
            return Ok(Background::Rgb(r, g, b));
        }

        let hex = raw.strip_prefix('#').unwrap_or(raw);
        // `#abc` is shorthand for `#aabbcc`.
        let expanded = match hex.len() {
            3 if hex.is_ascii() => Some(hex.chars().flat_map(|c| [c, c]).collect::<String>()),
            6 if hex.is_ascii() => Some(hex.to_owned()),
            _ => None,
        };
        if let Some(hex) = expanded {
            let parsed = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            );
            if let (Ok(r), Ok(g), Ok(b)) = parsed {
                return Ok(Background::Rgb(r, g, b));
            }
        }

        Err(Error::invalid_options(format!(
            "unknown background {raw:?}; expected black, white, #rrggbb or r,g,b"
        )))
    }
}

fn parse_comma_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let mut parts = raw.split(',');
    let r = parts.next()?.trim().parse().ok()?;
    let g = parts.next()?.trim().parse().ok()?;
    let b = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((r, g, b))
}

/// Everything that controls how an image becomes Braille.
///
/// `width` and `height` are measured in **character cells**, not pixels: one
/// cell is 2×4 dots. Fields are public, and every field also has a chainable
/// setter so options read well inline:
///
/// ```
/// use rustille::{Dither, Fit, RenderOptions};
///
/// let options = RenderOptions::default()
///     .width(80)
///     .threshold(140)
///     .dither(Dither::FloydSteinberg)
///     .fit(Fit::Contain);
///
/// assert_eq!(options.width, Some(80));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RenderOptions {
    /// Output width in character cells. Derived from `height` when `None`.
    pub width: Option<u32>,
    /// Output height in character cells. Derived from `width` when `None`.
    pub height: Option<u32>,
    /// Luminance at or above which a dot is set (after any inversion).
    pub threshold: u8,
    /// Invert the luminance plane, so dark pixels become dots.
    pub invert: bool,
    /// Dithering algorithm.
    pub dither: Dither,
    /// ANSI colour output.
    pub color: ColorMode,
    /// Colour composited under transparent pixels.
    pub background: Background,
    /// How the image is mapped onto the output box.
    pub fit: Fit,
    /// Physical height/width ratio of a terminal character cell.
    pub cell_aspect_ratio: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            threshold: DEFAULT_THRESHOLD,
            invert: false,
            dither: Dither::None,
            color: ColorMode::None,
            background: Background::Black,
            fit: Fit::Contain,
            cell_aspect_ratio: DEFAULT_CELL_ASPECT_RATIO,
        }
    }
}

impl RenderOptions {
    /// Options with every field at its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the output width in character cells.
    #[must_use]
    pub const fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the output height in character cells.
    #[must_use]
    pub const fn height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    /// Sets both dimensions at once.
    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Sets the luminance threshold.
    #[must_use]
    pub const fn threshold(mut self, threshold: u8) -> Self {
        self.threshold = threshold;
        self
    }

    /// Inverts the luminance plane.
    #[must_use]
    pub const fn invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    /// Selects the dithering algorithm.
    #[must_use]
    pub const fn dither(mut self, dither: Dither) -> Self {
        self.dither = dither;
        self
    }

    /// Selects the colour mode.
    #[must_use]
    pub const fn color(mut self, color: ColorMode) -> Self {
        self.color = color;
        self
    }

    /// Selects the background composited under transparent pixels.
    #[must_use]
    pub const fn background(mut self, background: Background) -> Self {
        self.background = background;
        self
    }

    /// Selects how the image is mapped onto the output box.
    #[must_use]
    pub const fn fit(mut self, fit: Fit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the physical height/width ratio of a terminal character cell.
    #[must_use]
    pub const fn cell_aspect_ratio(mut self, ratio: f32) -> Self {
        self.cell_aspect_ratio = ratio;
        self
    }

    /// Checks the options for self-consistency.
    ///
    /// Called automatically by every renderer entry point.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOptions`] when a dimension is zero or the cell
    /// aspect ratio is not a positive, finite number.
    pub fn validate(&self) -> Result<()> {
        if self.width == Some(0) {
            return Err(Error::invalid_options("width must be greater than zero"));
        }
        if self.height == Some(0) {
            return Err(Error::invalid_options("height must be greater than zero"));
        }
        if !self.cell_aspect_ratio.is_finite() || self.cell_aspect_ratio <= 0.0 {
            return Err(Error::invalid_options(format!(
                "cell_aspect_ratio must be a positive finite number, got {}",
                self.cell_aspect_ratio
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_plain_text() {
        let options = RenderOptions::default();
        assert_eq!(options.color, ColorMode::None);
        assert!(!options.color.is_enabled());
        assert_eq!(options.dither, Dither::None);
        assert_eq!(options.fit, Fit::Contain);
        assert_eq!(options.threshold, DEFAULT_THRESHOLD);
        assert_eq!(options.cell_aspect_ratio, DEFAULT_CELL_ASPECT_RATIO);
        assert!(options.validate().is_ok());
    }

    #[test]
    fn builder_is_chainable() {
        let options = RenderOptions::new()
            .size(40, 20)
            .threshold(200)
            .invert(true)
            .dither(Dither::FloydSteinberg)
            .color(ColorMode::TrueColor)
            .background(Background::White)
            .fit(Fit::Stretch)
            .cell_aspect_ratio(1.75);
        assert_eq!(options.width, Some(40));
        assert_eq!(options.height, Some(20));
        assert_eq!(options.threshold, 200);
        assert!(options.invert);
        assert_eq!(options.dither, Dither::FloydSteinberg);
        assert_eq!(options.color, ColorMode::TrueColor);
        assert_eq!(options.background, Background::White);
        assert_eq!(options.fit, Fit::Stretch);
        assert_eq!(options.cell_aspect_ratio, 1.75);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(RenderOptions::default().width(0).validate().is_err());
        assert!(RenderOptions::default().height(0).validate().is_err());
        assert!(
            RenderOptions::default()
                .cell_aspect_ratio(0.0)
                .validate()
                .is_err()
        );
        assert!(
            RenderOptions::default()
                .cell_aspect_ratio(f32::NAN)
                .validate()
                .is_err()
        );
        assert!(
            RenderOptions::default()
                .cell_aspect_ratio(-1.0)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn enum_strings_round_trip() {
        for dither in Dither::ALL {
            assert_eq!(dither.as_str().parse::<Dither>().unwrap(), *dither);
        }
        for color in ColorMode::ALL {
            assert_eq!(color.as_str().parse::<ColorMode>().unwrap(), *color);
        }
        for fit in Fit::ALL {
            assert_eq!(fit.as_str().parse::<Fit>().unwrap(), *fit);
        }
        assert_eq!(
            "FLOYD_STEINBERG".parse::<Dither>().unwrap(),
            Dither::FloydSteinberg
        );
        assert_eq!(" Cover ".parse::<Fit>().unwrap(), Fit::Fill);
        assert!("nope".parse::<Fit>().is_err());
    }

    #[test]
    fn background_parsing() {
        assert_eq!("black".parse::<Background>().unwrap(), Background::Black);
        assert_eq!("WHITE".parse::<Background>().unwrap(), Background::White);
        assert_eq!(
            "#ff8000".parse::<Background>().unwrap(),
            Background::Rgb(255, 128, 0)
        );
        assert_eq!(
            "f80".parse::<Background>().unwrap(),
            Background::Rgb(255, 136, 0)
        );
        assert_eq!(
            "1,2,3".parse::<Background>().unwrap(),
            Background::Rgb(1, 2, 3)
        );
        assert!("#gg0000".parse::<Background>().is_err());
        assert!("1,2".parse::<Background>().is_err());
        assert!("1,2,3,4".parse::<Background>().is_err());
        assert_eq!(Background::Rgb(255, 128, 0).rgb(), [255, 128, 0]);
        assert_eq!(Background::Rgb(1, 2, 3).to_string(), "#010203");
    }
}
