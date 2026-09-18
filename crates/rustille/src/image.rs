//! Pixel handling: alpha compositing, resampling, luminance, decoding.
//!
//! The renderer works on an opaque RGB [`Surface`]. Everything that turns bytes
//! into one lives here, including the resampler — Rustille implements its own
//! so that `render_rgba` keeps working (and stays small) when the `image`
//! decoder is compiled out, as it is for the WebAssembly build.

use crate::color;
use crate::error::{Error, Result};

/// Bytes per pixel in an RGBA buffer.
pub(crate) const RGBA_CHANNELS: usize = 4;
/// Bytes per pixel in an RGB buffer.
pub(crate) const RGB_CHANNELS: usize = 3;

/// An opaque, tightly packed 8-bit RGB image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Surface {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Surface {
    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    /// Checks dimensions and returns the pixel count.
    fn checked_pixels(width: u32, height: u32) -> Result<usize> {
        if width == 0 || height == 0 {
            return Err(Error::invalid_dimensions(format!(
                "image dimensions must be non-zero, got {width}x{height}"
            )));
        }
        usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| Error::invalid_dimensions(format!("{width}x{height} pixels is too large")))
    }

    fn expect_len(data: &[u8], pixels: usize, channels: usize) -> Result<()> {
        let expected = pixels
            .checked_mul(channels)
            .ok_or_else(|| Error::invalid_dimensions("pixel buffer size overflows"))?;
        if data.len() != expected {
            return Err(Error::InvalidBufferLength {
                expected,
                actual: data.len(),
            });
        }
        Ok(())
    }

    /// Composites an RGBA buffer over `background`.
    pub(crate) fn from_rgba(
        width: u32,
        height: u32,
        data: &[u8],
        background: [u8; 3],
    ) -> Result<Self> {
        let pixels = Self::checked_pixels(width, height)?;
        Self::expect_len(data, pixels, RGBA_CHANNELS)?;

        let mut out = vec![0u8; pixels * RGB_CHANNELS];
        for (pixel, chunk) in data.chunks_exact(RGBA_CHANNELS).enumerate() {
            let alpha = u32::from(chunk[3]);
            let base = pixel * RGB_CHANNELS;
            match alpha {
                255 => out[base..base + 3].copy_from_slice(&chunk[..3]),
                0 => out[base..base + 3].copy_from_slice(&background),
                _ => {
                    let inverse = 255 - alpha;
                    for channel in 0..RGB_CHANNELS {
                        let source = u32::from(chunk[channel]) * alpha;
                        let under = u32::from(background[channel]) * inverse;
                        // Rounded division by 255.
                        out[base + channel] = ((source + under + 127) / 255) as u8;
                    }
                }
            }
        }
        Ok(Self {
            width,
            height,
            data: out,
        })
    }

    /// Wraps an already-opaque RGB buffer.
    pub(crate) fn from_rgb(width: u32, height: u32, data: &[u8]) -> Result<Self> {
        let pixels = Self::checked_pixels(width, height)?;
        Self::expect_len(data, pixels, RGB_CHANNELS)?;
        Ok(Self {
            width,
            height,
            data: data.to_vec(),
        })
    }

    /// Expands an 8-bit grayscale buffer.
    pub(crate) fn from_luma(width: u32, height: u32, data: &[u8]) -> Result<Self> {
        let pixels = Self::checked_pixels(width, height)?;
        Self::expect_len(data, pixels, 1)?;
        let mut out = vec![0u8; pixels * RGB_CHANNELS];
        for (pixel, value) in data.iter().enumerate() {
            let base = pixel * RGB_CHANNELS;
            out[base] = *value;
            out[base + 1] = *value;
            out[base + 2] = *value;
        }
        Ok(Self {
            width,
            height,
            data: out,
        })
    }

    /// Returns the sub-image at `(x, y)` of size `width`×`height`.
    ///
    /// The rectangle is clamped to the surface.
    pub(crate) fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        let width = width.min(self.width - x).max(1);
        let height = height.min(self.height - y).max(1);
        let mut data = Vec::with_capacity(width as usize * height as usize * RGB_CHANNELS);
        for row in 0..height {
            let start = ((y + row) as usize * self.width as usize + x as usize) * RGB_CHANNELS;
            let end = start + width as usize * RGB_CHANNELS;
            data.extend_from_slice(&self.data[start..end]);
        }
        Self {
            width,
            height,
            data,
        }
    }

    /// Resamples to `width`×`height`.
    ///
    /// Shrinking uses an area (box) filter, which keeps thin features visible
    /// at the tiny sizes Braille art works at; growing uses linear
    /// interpolation. The two passes are separable, so the cost is
    /// `O(src·dst)` per axis rather than per pixel pair.
    pub(crate) fn resize(&self, width: u32, height: u32) -> Result<Self> {
        if width == self.width && height == self.height {
            return Ok(self.clone());
        }
        let pixels = Self::checked_pixels(width, height)?;

        let horizontal = AxisMap::new(self.width, width);
        let vertical = AxisMap::new(self.height, height);

        // Pass 1: horizontal, u8 -> f32.
        let mut intermediate = vec![0f32; width as usize * self.height as usize * RGB_CHANNELS];
        for row in 0..self.height as usize {
            let src_row = row * self.width as usize * RGB_CHANNELS;
            let dst_row = row * width as usize * RGB_CHANNELS;
            for target in 0..width as usize {
                let mut acc = [0f32; RGB_CHANNELS];
                for (source, weight) in horizontal.taps(target) {
                    let base = src_row + source * RGB_CHANNELS;
                    acc[0] += f32::from(self.data[base]) * weight;
                    acc[1] += f32::from(self.data[base + 1]) * weight;
                    acc[2] += f32::from(self.data[base + 2]) * weight;
                }
                let base = dst_row + target * RGB_CHANNELS;
                intermediate[base] = acc[0];
                intermediate[base + 1] = acc[1];
                intermediate[base + 2] = acc[2];
            }
        }

        // Pass 2: vertical, f32 -> u8.
        let mut out = vec![0u8; pixels * RGB_CHANNELS];
        let stride = width as usize * RGB_CHANNELS;
        for target_row in 0..height as usize {
            let dst_row = target_row * stride;
            for column in 0..width as usize {
                let mut acc = [0f32; RGB_CHANNELS];
                for (source, weight) in vertical.taps(target_row) {
                    let base = source * stride + column * RGB_CHANNELS;
                    acc[0] += intermediate[base] * weight;
                    acc[1] += intermediate[base + 1] * weight;
                    acc[2] += intermediate[base + 2] * weight;
                }
                let base = dst_row + column * RGB_CHANNELS;
                for channel in 0..RGB_CHANNELS {
                    out[base + channel] = acc[channel].round().clamp(0.0, 255.0) as u8;
                }
            }
        }

        Ok(Self {
            width,
            height,
            data: out,
        })
    }

    /// Luminance of every pixel, row-major, in `0.0..=255.0`.
    pub(crate) fn luminance_plane(&self) -> Vec<f32> {
        self.data
            .chunks_exact(RGB_CHANNELS)
            .map(|p| color::luminance(p[0], p[1], p[2]))
            .collect()
    }
}

/// Precomputed resampling taps for one axis.
struct AxisMap {
    starts: Vec<u32>,
    offsets: Vec<u32>,
    counts: Vec<u32>,
    weights: Vec<f32>,
}

impl AxisMap {
    fn new(source: u32, target: u32) -> Self {
        let mut starts = Vec::with_capacity(target as usize);
        let mut offsets = Vec::with_capacity(target as usize);
        let mut counts = Vec::with_capacity(target as usize);
        let mut weights = Vec::new();
        let scale = f64::from(source) / f64::from(target);

        for index in 0..target {
            let offset = weights.len();
            offsets.push(offset as u32);
            if target >= source {
                // Linear interpolation between the two nearest source samples.
                let center = (f64::from(index) + 0.5) * scale - 0.5;
                let left = center.floor();
                let fraction = (center - left) as f32;
                let first = left.max(0.0) as u32;
                let first = first.min(source - 1);
                let second = ((left as i64) + 1).clamp(0, i64::from(source) - 1) as u32;
                starts.push(first);
                if second == first {
                    counts.push(1);
                    weights.push(1.0);
                } else if second == first + 1 {
                    counts.push(2);
                    weights.push(1.0 - fraction);
                    weights.push(fraction);
                } else {
                    // `first` was clamped away from `second`; fall back to one tap.
                    counts.push(1);
                    weights.push(1.0);
                }
            } else {
                // Area average over the source interval this target covers.
                let begin = f64::from(index) * scale;
                let end = begin + scale;
                let first = begin.floor() as u32;
                let last = ((end.ceil() as u32).max(first + 1)).min(source);
                starts.push(first);
                counts.push(last - first);
                let mut total = 0f32;
                for sample in first..last {
                    let overlap = end.min(f64::from(sample) + 1.0) - begin.max(f64::from(sample));
                    let weight = overlap.max(0.0) as f32;
                    weights.push(weight);
                    total += weight;
                }
                if total > 0.0 {
                    for weight in &mut weights[offset..] {
                        *weight /= total;
                    }
                }
            }
        }

        Self {
            starts,
            offsets,
            counts,
            weights,
        }
    }

    /// Source index / weight pairs contributing to `target`.
    #[inline]
    fn taps(&self, target: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
        let start = self.starts[target] as usize;
        let count = self.counts[target] as usize;
        let offset = self.offsets[target] as usize;
        (0..count).map(move |i| (start + i, self.weights[offset + i]))
    }
}

#[cfg(feature = "_image")]
mod decode {
    use super::*;
    use std::io::{BufRead, Seek};
    use std::path::Path;

    /// A decoded image: width, height and tightly packed RGBA bytes.
    pub(crate) struct Decoded {
        pub width: u32,
        pub height: u32,
        pub rgba: Vec<u8>,
    }

    fn finish(image: image::DynamicImage) -> Decoded {
        let rgba = image.to_rgba8();
        Decoded {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        }
    }

    /// Decodes an image from memory, sniffing the format from its magic bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Decoded> {
        let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(Error::Io)?;
        Ok(finish(reader.decode()?))
    }

    /// Decodes an image from a file.
    pub(crate) fn from_path(path: &Path) -> Result<Decoded> {
        let reader = image::ImageReader::open(path)
            .map_err(Error::Io)?
            .with_guessed_format()
            .map_err(Error::Io)?;
        Ok(finish(reader.decode()?))
    }

    /// Decodes an image from a seekable, buffered reader.
    pub(crate) fn from_reader<R: BufRead + Seek>(reader: R) -> Result<Decoded> {
        let reader = image::ImageReader::new(reader)
            .with_guessed_format()
            .map_err(Error::Io)?;
        Ok(finish(reader.decode()?))
    }
}

#[cfg(feature = "_image")]
pub(crate) use decode::{Decoded, from_bytes, from_path, from_reader};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_compositing_over_black_and_white() {
        let data = [255, 0, 0, 255, 255, 0, 0, 0, 0, 255, 0, 128];
        let over_black = Surface::from_rgba(3, 1, &data, [0, 0, 0]).unwrap();
        assert_eq!(&over_black.data()[0..3], &[255, 0, 0]);
        assert_eq!(&over_black.data()[3..6], &[0, 0, 0]);
        assert_eq!(over_black.data()[7], 128);

        let over_white = Surface::from_rgba(3, 1, &data, [255, 255, 255]).unwrap();
        assert_eq!(&over_white.data()[3..6], &[255, 255, 255]);
        assert_eq!(over_white.data()[6], 127);
    }

    #[test]
    fn buffer_length_is_checked() {
        let err = Surface::from_rgba(2, 2, &[0; 8], [0, 0, 0]).unwrap_err();
        match err {
            Error::InvalidBufferLength { expected, actual } => {
                assert_eq!((expected, actual), (16, 8));
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(Surface::from_rgb(2, 2, &[0; 12]).is_ok());
        assert!(Surface::from_rgb(2, 2, &[0; 11]).is_err());
        assert!(Surface::from_luma(2, 2, &[0; 4]).is_ok());
    }

    #[test]
    fn zero_dimensions_are_rejected() {
        for (w, h) in [(0, 1), (1, 0), (0, 0)] {
            let err = Surface::from_rgba(w, h, &[], [0, 0, 0]).unwrap_err();
            assert_eq!(err.kind(), crate::ErrorKind::InvalidDimensions);
        }
    }

    #[test]
    fn luma_expands_to_grey() {
        let surface = Surface::from_luma(2, 1, &[10, 200]).unwrap();
        assert_eq!(surface.data(), &[10, 10, 10, 200, 200, 200]);
        let plane = surface.luminance_plane();
        assert!((plane[0] - 10.0).abs() < 0.01);
        assert!((plane[1] - 200.0).abs() < 0.01);
    }

    #[test]
    fn identity_resize_is_exact() {
        let surface = Surface::from_luma(4, 4, &[7; 16]).unwrap();
        let resized = surface.resize(4, 4).unwrap();
        assert_eq!(surface, resized);
    }

    #[test]
    fn downscale_averages() {
        // Two black pixels and two white ones collapse to mid grey.
        let surface = Surface::from_luma(2, 2, &[0, 255, 255, 0]).unwrap();
        let resized = surface.resize(1, 1).unwrap();
        assert_eq!(resized.width(), 1);
        assert_eq!(resized.data()[0], 128);
    }

    #[test]
    fn upscale_interpolates_without_ringing() {
        let surface = Surface::from_luma(2, 1, &[0, 255]).unwrap();
        let resized = surface.resize(8, 1).unwrap();
        let values: Vec<u8> = resized.data().iter().step_by(3).copied().collect();
        assert_eq!(values.len(), 8);
        assert_eq!(values[0], 0);
        assert_eq!(values[7], 255);
        for pair in values.windows(2) {
            assert!(pair[1] >= pair[0], "not monotonic: {values:?}");
        }
    }

    #[test]
    fn resize_preserves_flat_colour() {
        let surface = Surface::from_rgb(5, 3, &[42; 45]).unwrap();
        for (w, h) in [(1, 1), (2, 7), (9, 2), (13, 11)] {
            let resized = surface.resize(w, h).unwrap();
            assert_eq!(resized.width(), w);
            assert_eq!(resized.height(), h);
            assert!(
                resized.data().iter().all(|v| *v == 42),
                "flat colour drifted at {w}x{h}"
            );
        }
    }

    #[test]
    fn crop_clamps_to_the_surface() {
        let surface = Surface::from_luma(4, 4, &(0..16).collect::<Vec<u8>>()).unwrap();
        let cropped = surface.crop(1, 1, 2, 2);
        assert_eq!(cropped.width(), 2);
        assert_eq!(cropped.height(), 2);
        assert_eq!(cropped.data()[0], 5);
        let clamped = surface.crop(3, 3, 10, 10);
        assert_eq!((clamped.width(), clamped.height()), (1, 1));
    }
}
