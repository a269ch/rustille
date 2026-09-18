//! Turn pixels into Braille.
//!
//! Rustille renders images as Unicode Braille characters and gives you a small
//! drawing canvas built on the same encoder. One Braille character carries a
//! 2×4 grid of dots, so each character is eight logical pixels:
//!
//! ```text
//!         x=0    x=1
//! y=0     dot1   dot4      0x01  0x08
//! y=1     dot2   dot5      0x02  0x10
//! y=2     dot3   dot6      0x04  0x20
//! y=3     dot7   dot8      0x40  0x80
//!
//! character = U+2800 + mask
//! ```
//!
//! # Rendering an image
//!
//! [`Renderer::render_rgba`] is the entry point every binding uses, and it
//! never touches the filesystem:
//!
//! ```
//! use rustille::{Dither, RenderOptions, Renderer};
//!
//! # fn main() -> Result<(), rustille::Error> {
//! let options = RenderOptions::default()
//!     .width(80)
//!     .threshold(128)
//!     .dither(Dither::FloydSteinberg);
//!
//! let rgba = vec![255u8; 256 * 256 * 4];
//! let output = Renderer::new(options).render_rgba(256, 256, &rgba)?;
//! println!("{output}");
//! # Ok(())
//! # }
//! ```
//!
//! With the default `decode` feature you can hand Rustille an encoded image
//! instead, and it will work the format out from the bytes:
//!
//! ```no_run
//! # #[cfg(feature = "decode")]
//! # fn main() -> Result<(), rustille::Error> {
//! use rustille::{RenderOptions, Renderer};
//!
//! let renderer = Renderer::new(RenderOptions::default().width(80));
//! println!("{}", renderer.render_file("cat.png")?);
//! println!("{}", renderer.render_bytes(&std::fs::read("cat.png")?)?);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "decode"))]
//! # fn main() {}
//! ```
//!
//! # Drawing
//!
//! ```
//! use rustille::Canvas;
//!
//! let mut canvas = Canvas::new(40, 20);
//! canvas.rectangle(0, 0, 39, 19);
//! canvas.line(0, 0, 39, 19);
//! println!("{}", canvas.render());
//! ```
//!
//! # Sizes are in character cells
//!
//! [`RenderOptions::width`] and [`RenderOptions::height`] count *characters*,
//! not pixels: a width of 80 produces 80 columns, i.e. 160 dots. Terminal cells
//! are taller than they are wide, so [`RenderOptions::cell_aspect_ratio`]
//! (default `2.0`) keeps the result from looking stretched. The library never
//! queries the terminal; only the CLI does that.
//!
//! # Feature flags
//!
//! - **`decode`** *(default)* — image decoding via the [`image`] crate: PNG,
//!   JPEG, WebP, BMP, GIF and TIFF. Turn it off (`default-features = false`) to
//!   get a dependency-light build with just the raw-pixel and canvas APIs.
//! - **`decode-png`**, **`decode-jpeg`**, **`decode-webp`**, **`decode-bmp`**,
//!   **`decode-gif`**, **`decode-tiff`** — single formats, for smaller builds.
//! - **`cli`** — dependencies of the `rustille` binary. Library users never
//!   need it.
//!
//! [`image`]: https://crates.io/crates/image

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod braille;
pub mod canvas;
pub mod color;
pub mod dithering;
pub mod error;
pub mod options;
pub mod render;

mod image;

pub use crate::canvas::{Canvas, MAX_CANVAS_CELLS};
pub use crate::error::{Error, ErrorKind, Result};
pub use crate::options::{
    Background, ColorMode, DEFAULT_CELL_ASPECT_RATIO, DEFAULT_THRESHOLD, DEFAULT_WIDTH, Dither,
    Fit, MAX_OUTPUT_CELLS, RenderOptions,
};
pub use crate::render::{Renderer, render_luma, render_rgb, render_rgba};

#[cfg(feature = "_image")]
pub use crate::render::{render_bytes, render_file};

/// The version of this crate, as declared in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_ish() {
        assert_eq!(VERSION.split('.').count(), 3, "{VERSION}");
        assert!(
            VERSION
                .split('.')
                .all(|p| p.chars().any(|c| c.is_ascii_digit()))
        );
    }

    #[test]
    fn the_readme_canvas_example_runs() {
        let mut canvas = Canvas::new(100, 50);
        canvas.set(10, 10);
        canvas.set(11, 11);
        canvas.unset(10, 10);
        assert!(!canvas.get(10, 10));
        assert!(canvas.get(11, 11));
        assert_eq!(canvas.render().lines().count(), 13);
    }
}
