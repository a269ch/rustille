//! Argument parsing and glue for the `rustille` binary.
//!
//! This module is part of the binary target only: the library never depends on
//! `clap`, on `anyhow`, or on the terminal.

use std::fs::File;
use std::io::{BufWriter, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, ValueEnum};
use rustille::{Background, ColorMode, Dither, Fit, RenderOptions, Renderer};

/// Exit code used for any runtime failure (bad input, I/O, decoding).
pub const EXIT_FAILURE: u8 = 1;

/// Turn pixels into Braille.
#[derive(Debug, Parser)]
#[command(
    name = "rustille",
    version,
    about = "Turn pixels into Braille",
    long_about = "Render an image as Unicode Braille art.\n\n\
                  One Braille character holds a 2x4 grid of dots, so every \
                  character carries eight logical pixels. Sizes are given in \
                  characters, not pixels.",
    after_help = "EXAMPLES:\n  \
                  rustille cat.png\n  \
                  rustille cat.png --width 100 --dither floyd-steinberg\n  \
                  cat cat.png | rustille - --color always\n",
    disable_help_flag = false
)]
pub struct Cli {
    /// Image file to render, or `-` to read from standard input.
    #[arg(value_name = "IMAGE")]
    pub input: PathBuf,

    /// Output width in characters.
    #[arg(short = 'w', long, value_name = "COLUMNS")]
    pub width: Option<u32>,

    /// Output height in characters.
    #[arg(short = 'H', long, value_name = "ROWS")]
    pub height: Option<u32>,

    /// Luminance at or above which a dot is drawn (0-255).
    #[arg(short = 't', long, default_value_t = rustille::DEFAULT_THRESHOLD, value_name = "0-255")]
    pub threshold: u8,

    /// Draw dark pixels instead of bright ones.
    #[arg(short = 'i', long, action = ArgAction::SetTrue)]
    pub invert: bool,

    /// Dithering algorithm.
    #[arg(short = 'd', long, value_enum, default_value_t = DitherArg::None)]
    pub dither: DitherArg,

    /// When to emit ANSI colour.
    #[arg(short = 'c', long, value_enum, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,

    /// How the image is mapped onto the requested size.
    #[arg(long, value_enum, default_value_t = FitArg::Contain)]
    pub fit: FitArg,

    /// Colour composited under transparent pixels: black, white, #rrggbb or r,g,b.
    #[arg(short = 'b', long, default_value = "black", value_name = "COLOR")]
    pub background: String,

    /// Physical height/width ratio of one terminal character cell.
    #[arg(long, default_value_t = rustille::DEFAULT_CELL_ASPECT_RATIO, value_name = "RATIO")]
    pub cell_aspect: f32,

    /// Write to this file instead of standard output.
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

/// `--dither` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DitherArg {
    /// Plain thresholding.
    None,
    /// Floyd–Steinberg error diffusion.
    #[value(name = "floyd-steinberg", alias = "fs")]
    FloydSteinberg,
}

/// `--color` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    /// Colour when writing to a capable terminal and `NO_COLOR` is unset.
    Auto,
    /// Always colour, picking the best depth the environment advertises.
    Always,
    /// Never colour.
    Never,
    /// Force the 256-colour palette.
    Ansi256,
    /// Force 24-bit colour.
    Truecolor,
}

/// `--fit` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FitArg {
    /// Fit inside the box, preserving the aspect ratio.
    Contain,
    /// Cover the box, preserving the aspect ratio, then crop.
    #[value(alias = "cover")]
    Fill,
    /// Use the box exactly, ignoring the aspect ratio.
    Stretch,
}

impl From<DitherArg> for Dither {
    fn from(value: DitherArg) -> Self {
        match value {
            DitherArg::None => Dither::None,
            DitherArg::FloydSteinberg => Dither::FloydSteinberg,
        }
    }
}

impl From<FitArg> for Fit {
    fn from(value: FitArg) -> Self {
        match value {
            FitArg::Contain => Fit::Contain,
            FitArg::Fill => Fit::Fill,
            FitArg::Stretch => Fit::Stretch,
        }
    }
}

/// What the environment says about colour support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorEnvironment {
    /// Whether the destination is an interactive terminal.
    pub is_terminal: bool,
    /// Whether `NO_COLOR` is set (to anything at all).
    pub no_color: bool,
    /// Value of `COLORTERM`, lowercased.
    pub colorterm: Option<String>,
    /// Value of `TERM`, lowercased.
    pub term: Option<String>,
}

impl ColorEnvironment {
    /// Reads the environment, treating `is_terminal` as given.
    #[must_use]
    pub fn detect(is_terminal: bool) -> Self {
        let lower = |key: &str| std::env::var(key).ok().map(|v| v.to_ascii_lowercase());
        Self {
            is_terminal,
            no_color: std::env::var_os("NO_COLOR").is_some(),
            colorterm: lower("COLORTERM"),
            term: lower("TERM"),
        }
    }

    /// The best colour depth this environment claims to support.
    fn best_depth(&self) -> ColorMode {
        if self.term.as_deref() == Some("dumb") {
            return ColorMode::None;
        }
        match self.colorterm.as_deref() {
            Some("truecolor" | "24bit") => ColorMode::TrueColor,
            _ => ColorMode::Ansi256,
        }
    }
}

/// Resolves `--color` against the environment.
///
/// `auto` honours `NO_COLOR` and only colours a real terminal; `always` ignores
/// both but still picks the depth the terminal advertises.
#[must_use]
pub fn resolve_color(arg: ColorArg, env: &ColorEnvironment) -> ColorMode {
    match arg {
        ColorArg::Never => ColorMode::None,
        ColorArg::Ansi256 => ColorMode::Ansi256,
        ColorArg::Truecolor => ColorMode::TrueColor,
        ColorArg::Always => match env.best_depth() {
            ColorMode::None => ColorMode::Ansi256,
            depth => depth,
        },
        ColorArg::Auto => {
            if env.no_color || !env.is_terminal {
                ColorMode::None
            } else {
                env.best_depth()
            }
        }
    }
}

/// Builds [`RenderOptions`] from parsed arguments.
///
/// `terminal_size` is `Some((columns, rows))` when the output is an interactive
/// terminal; it is only consulted when neither `--width` nor `--height` is
/// given, so the library stays terminal-independent.
pub fn build_options(
    cli: &Cli,
    color: ColorMode,
    terminal_size: Option<(u32, u32)>,
) -> Result<RenderOptions> {
    let background: Background = cli
        .background
        .parse()
        .with_context(|| format!("invalid --background {:?}", cli.background))?;

    let fit = Fit::from(cli.fit);
    let mut options = RenderOptions::default()
        .threshold(cli.threshold)
        .invert(cli.invert)
        .dither(cli.dither.into())
        .color(color)
        .background(background)
        .fit(fit)
        .cell_aspect_ratio(cli.cell_aspect);

    options.width = cli.width;
    options.height = cli.height;

    if options.width.is_none() && options.height.is_none() {
        if let Some((columns, rows)) = terminal_size {
            options.width = Some(columns.max(1));
            if fit == Fit::Contain {
                options.height = Some(rows.saturating_sub(1).max(1));
            }
        }
    }

    options
        .validate()
        .context("invalid rendering options")
        .map(|()| options)
}

/// Reads every byte of an input, where `-` means standard input.
fn read_input(path: &Path) -> Result<Vec<u8>> {
    if path.as_os_str() == "-" {
        let mut buffer = Vec::new();
        std::io::stdin()
            .lock()
            .read_to_end(&mut buffer)
            .context("failed to read image from standard input")?;
        if buffer.is_empty() {
            anyhow::bail!("no image data on standard input");
        }
        Ok(buffer)
    } else {
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))
    }
}

/// Runs the CLI end to end.
///
/// # Errors
///
/// Any failure to read, decode, render or write.
pub fn run(cli: &Cli) -> Result<()> {
    let writing_to_stdout = cli.output.is_none();
    let is_terminal = writing_to_stdout && std::io::stdout().is_terminal();
    let env = ColorEnvironment::detect(is_terminal);
    let color = resolve_color(cli.color, &env);

    let terminal_size = if is_terminal {
        terminal_size::terminal_size()
            .map(|(terminal_size::Width(w), terminal_size::Height(h))| (u32::from(w), u32::from(h)))
    } else {
        None
    };

    let options = build_options(cli, color, terminal_size)?;
    let renderer = Renderer::new(options);

    let bytes = read_input(&cli.input)?;
    let rendered = renderer
        .render_bytes(&bytes)
        .with_context(|| format!("failed to render {}", cli.input.display()))?;

    match &cli.output {
        Some(path) => {
            let file = File::create(path)
                .with_context(|| format!("failed to create {}", path.display()))?;
            let mut writer = BufWriter::new(file);
            writeln!(writer, "{rendered}")
                .with_context(|| format!("failed to write {}", path.display()))?;
            writer.flush().context("failed to flush output")?;
        }
        None => {
            let stdout = std::io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            writeln!(writer, "{rendered}").context("failed to write to standard output")?;
            writer.flush().context("failed to flush standard output")?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn cli(args: &[&str]) -> Cli {
        Cli::parse_from(std::iter::once("rustille").chain(args.iter().copied()))
    }

    fn env(is_terminal: bool) -> ColorEnvironment {
        ColorEnvironment {
            is_terminal,
            no_color: false,
            colorterm: None,
            term: None,
        }
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn defaults_match_the_library() {
        let parsed = cli(&["cat.png"]);
        assert_eq!(parsed.threshold, rustille::DEFAULT_THRESHOLD);
        assert_eq!(parsed.dither, DitherArg::None);
        assert_eq!(parsed.fit, FitArg::Contain);
        assert_eq!(parsed.color, ColorArg::Auto);
        assert!(!parsed.invert);
        assert_eq!(parsed.width, None);
    }

    #[test]
    fn flags_are_parsed() {
        let parsed = cli(&[
            "cat.png",
            "--width",
            "100",
            "--height",
            "40",
            "--threshold",
            "150",
            "--invert",
            "--dither",
            "floyd-steinberg",
            "--color",
            "always",
            "--fit",
            "stretch",
            "--background",
            "#102030",
        ]);
        assert_eq!(parsed.width, Some(100));
        assert_eq!(parsed.height, Some(40));
        assert_eq!(parsed.threshold, 150);
        assert!(parsed.invert);
        assert_eq!(parsed.dither, DitherArg::FloydSteinberg);
        assert_eq!(parsed.color, ColorArg::Always);
        assert_eq!(parsed.fit, FitArg::Stretch);

        let options = build_options(&parsed, ColorMode::TrueColor, None).unwrap();
        assert_eq!(options.width, Some(100));
        assert_eq!(options.height, Some(40));
        assert_eq!(options.dither, Dither::FloydSteinberg);
        assert_eq!(options.fit, Fit::Stretch);
        assert_eq!(options.background, Background::Rgb(0x10, 0x20, 0x30));
    }

    #[test]
    fn dither_alias_is_accepted() {
        assert_eq!(
            cli(&["a.png", "-d", "fs"]).dither,
            DitherArg::FloydSteinberg
        );
        assert_eq!(cli(&["a.png", "--fit", "cover"]).fit, FitArg::Fill);
    }

    #[test]
    fn bad_background_is_an_error() {
        let parsed = cli(&["cat.png", "--background", "chartreuse"]);
        assert!(build_options(&parsed, ColorMode::None, None).is_err());
    }

    #[test]
    fn bad_cell_aspect_is_an_error() {
        let parsed = cli(&["cat.png", "--cell-aspect", "0"]);
        assert!(build_options(&parsed, ColorMode::None, None).is_err());
    }

    #[test]
    fn terminal_size_is_used_only_when_no_size_is_given() {
        let auto = build_options(&cli(&["cat.png"]), ColorMode::None, Some((120, 30))).unwrap();
        assert_eq!(auto.width, Some(120));
        assert_eq!(auto.height, Some(29));

        let explicit = build_options(
            &cli(&["cat.png", "-w", "10"]),
            ColorMode::None,
            Some((120, 30)),
        )
        .unwrap();
        assert_eq!(explicit.width, Some(10));
        assert_eq!(explicit.height, None);

        let piped = build_options(&cli(&["cat.png"]), ColorMode::None, None).unwrap();
        assert_eq!(piped.width, None);
        assert_eq!(piped.height, None);
    }

    #[test]
    fn stretch_does_not_borrow_the_terminal_height() {
        let options = build_options(
            &cli(&["cat.png", "--fit", "stretch"]),
            ColorMode::None,
            Some((120, 30)),
        )
        .unwrap();
        assert_eq!(options.width, Some(120));
        assert_eq!(options.height, None);
    }

    #[test]
    fn color_auto_needs_a_terminal() {
        assert_eq!(resolve_color(ColorArg::Auto, &env(false)), ColorMode::None);
        assert_eq!(
            resolve_color(ColorArg::Auto, &env(true)),
            ColorMode::Ansi256
        );
    }

    #[test]
    fn no_color_disables_auto_but_not_always() {
        let mut environment = env(true);
        environment.no_color = true;
        assert_eq!(resolve_color(ColorArg::Auto, &environment), ColorMode::None);
        assert_eq!(
            resolve_color(ColorArg::Always, &environment),
            ColorMode::Ansi256
        );
    }

    #[test]
    fn colorterm_selects_the_depth() {
        let mut environment = env(true);
        environment.colorterm = Some("truecolor".to_owned());
        assert_eq!(
            resolve_color(ColorArg::Auto, &environment),
            ColorMode::TrueColor
        );
        environment.colorterm = Some("24bit".to_owned());
        assert_eq!(
            resolve_color(ColorArg::Always, &environment),
            ColorMode::TrueColor
        );
    }

    #[test]
    fn dumb_terminals_get_no_colour_on_auto() {
        let mut environment = env(true);
        environment.term = Some("dumb".to_owned());
        assert_eq!(resolve_color(ColorArg::Auto, &environment), ColorMode::None);
        assert_eq!(
            resolve_color(ColorArg::Never, &environment),
            ColorMode::None
        );
        assert_eq!(
            resolve_color(ColorArg::Truecolor, &environment),
            ColorMode::TrueColor
        );
    }
}
