//! Dithering of the luminance plane.
//!
//! Dithering happens on the dot grid, *before* dots are packed into Braille
//! cells, so a dithered render uses the full 2×4 resolution of every character.
//!
//! Error-diffusion algorithms are described by an [`ErrorDiffusionKernel`],
//! which makes adding Atkinson, Sierra or Stucki a matter of adding a constant
//! — no renderer or public API change. Ordered (Bayer) dithering will slot in
//! as a second branch of [`dither_plane`] for the same reason.

use crate::options::Dither;

/// One neighbour that receives part of the quantization error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffusionTap {
    /// Horizontal offset from the pixel being quantized.
    pub dx: i32,
    /// Vertical offset from the pixel being quantized (never negative).
    pub dy: i32,
    /// Unnormalised weight; divided by [`ErrorDiffusionKernel::divisor`].
    pub weight: f32,
}

/// An error-diffusion kernel, scanned in plain raster order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErrorDiffusionKernel {
    /// Neighbours the error is spread over.
    pub taps: &'static [DiffusionTap],
    /// Sum of the weights.
    pub divisor: f32,
}

/// The Floyd–Steinberg kernel:
///
/// ```text
///          *   7/16
///  3/16  5/16  1/16
/// ```
pub const FLOYD_STEINBERG: ErrorDiffusionKernel = ErrorDiffusionKernel {
    taps: &[
        DiffusionTap {
            dx: 1,
            dy: 0,
            weight: 7.0,
        },
        DiffusionTap {
            dx: -1,
            dy: 1,
            weight: 3.0,
        },
        DiffusionTap {
            dx: 0,
            dy: 1,
            weight: 5.0,
        },
        DiffusionTap {
            dx: 1,
            dy: 1,
            weight: 1.0,
        },
    ],
    divisor: 16.0,
};

/// Quantizes `plane` in place to `0.0` / `255.0` using `dither`.
///
/// `plane` holds luminance in `0.0..=255.0`, row-major, `width * height`
/// entries. [`Dither::None`] leaves the plane untouched: plain thresholding is
/// done by the packer, which saves a pass over the image.
///
/// The result is bit-for-bit deterministic: the scan order is fixed and the
/// arithmetic is plain `f32`.
pub fn dither_plane(plane: &mut [f32], width: usize, height: usize, threshold: u8, dither: Dither) {
    if width == 0 || height == 0 || plane.len() < width * height {
        return;
    }
    match dither {
        Dither::None => {}
        Dither::FloydSteinberg => diffuse(plane, width, height, threshold, &FLOYD_STEINBERG),
    }
}

/// Runs an error-diffusion kernel over the plane in raster order.
fn diffuse(
    plane: &mut [f32],
    width: usize,
    height: usize,
    threshold: u8,
    kernel: &ErrorDiffusionKernel,
) {
    let cut = f32::from(threshold);
    let scale = 1.0 / kernel.divisor;
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let old = plane[index];
            let new = if old >= cut { 255.0 } else { 0.0 };
            plane[index] = new;
            let error = (old - new) * scale;
            if error == 0.0 {
                continue;
            }
            for tap in kernel.taps {
                let nx = x as i64 + i64::from(tap.dx);
                let ny = y as i64 + i64::from(tap.dy);
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                    continue;
                }
                let neighbour = ny as usize * width + nx as usize;
                plane[neighbour] += error * tap.weight;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(width: usize, height: usize) -> Vec<f32> {
        (0..width * height)
            .map(|i| (i % width) as f32 * 255.0 / (width.max(2) - 1) as f32)
            .collect()
    }

    #[test]
    fn none_is_a_no_op() {
        let mut plane = ramp(8, 4);
        let original = plane.clone();
        dither_plane(&mut plane, 8, 4, 128, Dither::None);
        assert_eq!(plane, original);
    }

    #[test]
    fn floyd_steinberg_quantizes_to_two_levels() {
        let mut plane = ramp(16, 8);
        dither_plane(&mut plane, 16, 8, 128, Dither::FloydSteinberg);
        assert!(plane.iter().all(|v| *v == 0.0 || *v == 255.0));
    }

    #[test]
    fn floyd_steinberg_is_deterministic() {
        let first = {
            let mut plane = ramp(32, 16);
            dither_plane(&mut plane, 32, 16, 128, Dither::FloydSteinberg);
            plane
        };
        let second = {
            let mut plane = ramp(32, 16);
            dither_plane(&mut plane, 32, 16, 128, Dither::FloydSteinberg);
            plane
        };
        assert_eq!(first, second);
    }

    #[test]
    fn floyd_steinberg_preserves_average_brightness() {
        let mut plane = vec![100.0f32; 64 * 64];
        dither_plane(&mut plane, 64, 64, 128, Dither::FloydSteinberg);
        let mean = plane.iter().sum::<f32>() / plane.len() as f32;
        // Error diffusion should land close to the original mean.
        assert!((mean - 100.0).abs() < 8.0, "mean was {mean}");
    }

    #[test]
    fn flat_extremes_are_untouched_by_diffusion() {
        let mut white = vec![255.0f32; 16];
        dither_plane(&mut white, 4, 4, 128, Dither::FloydSteinberg);
        assert!(white.iter().all(|v| *v == 255.0));

        let mut black = vec![0.0f32; 16];
        dither_plane(&mut black, 4, 4, 128, Dither::FloydSteinberg);
        assert!(black.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn degenerate_sizes_are_ignored() {
        let mut plane: Vec<f32> = Vec::new();
        dither_plane(&mut plane, 0, 0, 128, Dither::FloydSteinberg);
        assert!(plane.is_empty());

        let mut short = vec![10.0f32; 3];
        dither_plane(&mut short, 4, 4, 128, Dither::FloydSteinberg);
        assert_eq!(short, vec![10.0; 3]);
    }

    #[test]
    fn kernel_weights_sum_to_divisor() {
        let total: f32 = FLOYD_STEINBERG.taps.iter().map(|t| t.weight).sum();
        assert_eq!(total, FLOYD_STEINBERG.divisor);
        assert!(FLOYD_STEINBERG.taps.iter().all(|t| t.dy >= 0));
    }
}
