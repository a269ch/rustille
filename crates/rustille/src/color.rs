//! Luminance and ANSI colour helpers.
//!
//! Rustille never emits an escape sequence unless [`ColorMode`] asks for one,
//! and the library default is [`ColorMode::None`].

use crate::options::ColorMode;

/// ANSI sequence that resets all attributes.
pub const RESET: &str = "\x1b[0m";

/// Rec. 709 luminance of an sRGB triple, in `0.0..=255.0`.
///
/// The weights are applied to the gamma-encoded values directly. That is not
/// photometrically correct, but it is what every terminal-art tool does and it
/// keeps mid-tones where people expect them.
#[inline]
#[must_use]
pub fn luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)
}

/// The six levels of the xterm 6×6×6 colour cube.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Index of the nearest colour-cube level for one channel.
#[inline]
fn cube_index(value: u8) -> usize {
    let mut best = 0usize;
    let mut best_distance = u16::MAX;
    for (index, level) in CUBE_LEVELS.iter().enumerate() {
        let distance = u16::from(value.abs_diff(*level));
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }
    best
}

#[inline]
fn squared_distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    let mut total = 0u32;
    for channel in 0..3 {
        let delta = u32::from(a[channel].abs_diff(b[channel]));
        total += delta * delta;
    }
    total
}

/// Maps an RGB triple onto the closest xterm-256 palette entry.
///
/// Only the 6×6×6 colour cube (16–231) and the 24-step grey ramp (232–255) are
/// considered; the 16 system colours are skipped because their actual RGB
/// values are theme-dependent.
#[must_use]
pub fn ansi256_index(r: u8, g: u8, b: u8) -> u8 {
    let (ri, gi, bi) = (cube_index(r), cube_index(g), cube_index(b));
    let cube_rgb = [CUBE_LEVELS[ri], CUBE_LEVELS[gi], CUBE_LEVELS[bi]];
    let cube_error = squared_distance([r, g, b], cube_rgb);
    let cube_code = 16 + 36 * ri + 6 * gi + bi;

    let grey = luminance(r, g, b).round().clamp(0.0, 255.0) as i32;
    let step = ((grey - 8) as f32 / 10.0).round().clamp(0.0, 23.0) as u8;
    let grey_value = 8 + 10 * step;
    let grey_error = squared_distance([r, g, b], [grey_value, grey_value, grey_value]);

    if grey_error < cube_error {
        232 + step
    } else {
        cube_code as u8
    }
}

/// Appends the decimal form of `value` without going through `format!`.
#[inline]
pub(crate) fn push_u8_decimal(out: &mut String, value: u8) {
    if value >= 100 {
        out.push((b'0' + value / 100) as char);
    }
    if value >= 10 {
        out.push((b'0' + (value / 10) % 10) as char);
    }
    out.push((b'0' + value % 10) as char);
}

/// Appends the foreground escape sequence for `rgb` in the given mode.
///
/// Writes nothing for [`ColorMode::None`].
pub(crate) fn push_foreground(out: &mut String, mode: ColorMode, rgb: [u8; 3]) {
    match mode {
        ColorMode::None => {}
        ColorMode::Ansi256 => {
            out.push_str("\x1b[38;5;");
            push_u8_decimal(out, ansi256_index(rgb[0], rgb[1], rgb[2]));
            out.push('m');
        }
        ColorMode::TrueColor => {
            out.push_str("\x1b[38;2;");
            push_u8_decimal(out, rgb[0]);
            out.push(';');
            push_u8_decimal(out, rgb[1]);
            out.push(';');
            push_u8_decimal(out, rgb[2]);
            out.push('m');
        }
    }
}

/// Upper bound on the bytes one foreground escape can take, used for sizing the
/// output buffer up front.
pub(crate) const fn escape_budget(mode: ColorMode) -> usize {
    match mode {
        ColorMode::None => 0,
        ColorMode::Ansi256 => 11,
        ColorMode::TrueColor => 19,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_endpoints() {
        assert_eq!(luminance(0, 0, 0), 0.0);
        assert!((luminance(255, 255, 255) - 255.0).abs() < 0.01);
        assert!(luminance(0, 255, 0) > luminance(255, 0, 0));
        assert!(luminance(255, 0, 0) > luminance(0, 0, 255));
    }

    #[test]
    fn ansi256_known_entries() {
        assert_eq!(ansi256_index(0, 0, 0), 16);
        assert_eq!(ansi256_index(255, 255, 255), 231);
        assert_eq!(ansi256_index(255, 0, 0), 196);
        assert_eq!(ansi256_index(0, 255, 0), 46);
        assert_eq!(ansi256_index(0, 0, 255), 21);
    }

    #[test]
    fn ansi256_prefers_the_grey_ramp_for_greys() {
        let index = ansi256_index(0x77, 0x77, 0x77);
        assert!((232..=255).contains(&index), "got {index}");
    }

    #[test]
    fn ansi256_stays_in_range() {
        for r in (0..=255u8).step_by(17) {
            for g in (0..=255u8).step_by(17) {
                for b in (0..=255u8).step_by(17) {
                    let index = ansi256_index(r, g, b);
                    assert!(index >= 16, "index {index} for {r},{g},{b}");
                }
            }
        }
    }

    #[test]
    fn decimal_formatting_matches_std() {
        for value in 0..=255u8 {
            let mut out = String::new();
            push_u8_decimal(&mut out, value);
            assert_eq!(out, value.to_string());
        }
    }

    #[test]
    fn escapes_are_written_only_when_asked() {
        let mut out = String::new();
        push_foreground(&mut out, ColorMode::None, [1, 2, 3]);
        assert!(out.is_empty());

        push_foreground(&mut out, ColorMode::TrueColor, [1, 2, 3]);
        assert_eq!(out, "\x1b[38;2;1;2;3m");
        assert!(out.len() <= escape_budget(ColorMode::TrueColor));

        out.clear();
        push_foreground(&mut out, ColorMode::Ansi256, [255, 255, 255]);
        assert_eq!(out, "\x1b[38;5;231m");
        assert!(out.len() <= escape_budget(ColorMode::Ansi256));
    }
}
