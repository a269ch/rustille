//! Unicode Braille cell encoding.
//!
//! A Braille character from the `Braille Patterns` block (`U+2800..=U+28FF`)
//! encodes a 2×4 grid of dots. Each dot maps to one bit of an 8-bit mask, and
//! the character is simply `U+2800 + mask`:
//!
//! ```text
//!         x=0    x=1
//! y=0     dot1   dot4      0x01  0x08
//! y=1     dot2   dot5      0x02  0x10
//! y=2     dot3   dot6      0x04  0x20
//! y=3     dot7   dot8      0x40  0x80
//! ```
//!
//! The dot numbering is the historical 6-dot Braille order extended with dots
//! 7 and 8 for the fourth row, which is why the bit layout is not a plain
//! row-major scan.
//!
//! ```
//! use rustille::braille;
//!
//! assert_eq!(braille::char_for_mask(0x00), '\u{2800}');
//! assert_eq!(braille::char_for_mask(0xFF), '\u{28FF}');
//! assert_eq!(braille::dot_bit(1, 3), Some(0x80));
//! ```

/// First code point of the Unicode `Braille Patterns` block.
pub const BRAILLE_BASE: u32 = 0x2800;

/// Number of dot columns in one Braille cell.
pub const CELL_WIDTH: u32 = 2;

/// Number of dot rows in one Braille cell.
pub const CELL_HEIGHT: u32 = 4;

/// Number of dots in one Braille cell.
pub const DOTS_PER_CELL: u32 = CELL_WIDTH * CELL_HEIGHT;

/// The empty Braille pattern, `U+2800`.
///
/// Note that this is *not* a space: it occupies a full cell in most terminals,
/// which is what keeps rendered output rectangular.
pub const BLANK: char = '\u{2800}';

/// Bit values indexed as `DOT_BITS[y][x]`.
const DOT_BITS: [[u8; CELL_WIDTH as usize]; CELL_HEIGHT as usize] = [
    [0x01, 0x08], // dot1, dot4
    [0x02, 0x10], // dot2, dot5
    [0x04, 0x20], // dot3, dot6
    [0x40, 0x80], // dot7, dot8
];

/// Bit value of every dot, indexed by the standard dot number minus one
/// (`DOT_NUMBER_BITS[0]` is dot 1).
pub const DOT_NUMBER_BITS: [u8; DOTS_PER_CELL as usize] =
    [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];

/// Returns the bit for the dot at `(x, y)` inside a cell, or [`None`] when the
/// coordinates fall outside the 2×4 grid.
///
/// ```
/// # use rustille::braille::dot_bit;
/// assert_eq!(dot_bit(0, 0), Some(0x01));
/// assert_eq!(dot_bit(2, 0), None);
/// ```
#[inline]
#[must_use]
pub const fn dot_bit(x: u32, y: u32) -> Option<u8> {
    if x < CELL_WIDTH && y < CELL_HEIGHT {
        Some(DOT_BITS[y as usize][x as usize])
    } else {
        None
    }
}

/// Returns the bit for the dot at `(x, y)`, or `0` when out of range.
///
/// Useful in hot loops where the caller has already bounds-checked.
#[inline]
#[must_use]
pub const fn dot_bit_or_zero(x: u32, y: u32) -> u8 {
    match dot_bit(x, y) {
        Some(bit) => bit,
        None => 0,
    }
}

/// Returns the bit for a dot given by its standard Braille dot number (`1..=8`),
/// or [`None`] for any other number.
#[inline]
#[must_use]
pub const fn dot_number_bit(dot: u32) -> Option<u8> {
    match dot.checked_sub(1) {
        Some(index) if index < DOTS_PER_CELL => Some(DOT_NUMBER_BITS[index as usize]),
        _ => None,
    }
}

/// Maps a dot mask to its Braille character.
///
/// Every one of the 256 masks maps to a distinct character in
/// `U+2800..=U+28FF`, so this never fails.
#[inline]
#[must_use]
pub const fn char_for_mask(mask: u8) -> char {
    match char::from_u32(BRAILLE_BASE + mask as u32) {
        Some(c) => c,
        None => BLANK,
    }
}

/// Returns the dot mask of a Braille character, or [`None`] if the character is
/// outside `U+2800..=U+28FF`.
///
/// ```
/// # use rustille::braille::mask_for_char;
/// assert_eq!(mask_for_char('\u{2801}'), Some(0x01));
/// assert_eq!(mask_for_char('a'), None);
/// ```
#[inline]
#[must_use]
pub const fn mask_for_char(c: char) -> Option<u8> {
    match (c as u32).checked_sub(BRAILLE_BASE) {
        Some(mask) if mask <= 0xFF => Some(mask as u8),
        _ => None,
    }
}

/// Packs a 2×4 block of booleans into a dot mask.
///
/// `dots` is indexed as `dots[y][x]`.
#[inline]
#[must_use]
pub const fn pack(dots: &[[bool; CELL_WIDTH as usize]; CELL_HEIGHT as usize]) -> u8 {
    let mut mask = 0u8;
    let mut y = 0usize;
    while y < CELL_HEIGHT as usize {
        let mut x = 0usize;
        while x < CELL_WIDTH as usize {
            if dots[y][x] {
                mask |= DOT_BITS[y][x];
            }
            x += 1;
        }
        y += 1;
    }
    mask
}

/// Unpacks a dot mask into a 2×4 block of booleans indexed as `dots[y][x]`.
#[inline]
#[must_use]
pub const fn unpack(mask: u8) -> [[bool; CELL_WIDTH as usize]; CELL_HEIGHT as usize] {
    let mut dots = [[false; CELL_WIDTH as usize]; CELL_HEIGHT as usize];
    let mut y = 0usize;
    while y < CELL_HEIGHT as usize {
        let mut x = 0usize;
        while x < CELL_WIDTH as usize {
            dots[y][x] = mask & DOT_BITS[y][x] != 0;
            x += 1;
        }
        y += 1;
    }
    dots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_and_blank_agree() {
        assert_eq!(BLANK as u32, BRAILLE_BASE);
        assert_eq!(char_for_mask(0), BLANK);
    }

    #[test]
    fn every_mask_maps_to_its_code_point() {
        for mask in 0u8..=255 {
            let c = char_for_mask(mask);
            assert_eq!(
                c as u32,
                BRAILLE_BASE + u32::from(mask),
                "mask {mask:#04x} produced U+{:04X}",
                c as u32
            );
            assert_eq!(mask_for_char(c), Some(mask));
            assert_eq!(c.len_utf8(), 3);
        }
    }

    #[test]
    fn all_masks_are_distinct() {
        let mut seen = [false; 256];
        for mask in 0u8..=255 {
            let idx = (char_for_mask(mask) as u32 - BRAILLE_BASE) as usize;
            assert!(!seen[idx], "duplicate character for mask {mask:#04x}");
            seen[idx] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn dot_numbers_match_coordinates() {
        let expected = [
            (1u32, 0u32, 0u32),
            (2, 0, 1),
            (3, 0, 2),
            (4, 1, 0),
            (5, 1, 1),
            (6, 1, 2),
            (7, 0, 3),
            (8, 1, 3),
        ];
        for (dot, x, y) in expected {
            assert_eq!(dot_number_bit(dot), dot_bit(x, y), "dot {dot}");
        }
        assert_eq!(dot_number_bit(0), None);
        assert_eq!(dot_number_bit(9), None);
    }

    #[test]
    fn single_dots_render_expected_characters() {
        assert_eq!(char_for_mask(0x01), '⠁');
        assert_eq!(char_for_mask(0x02), '⠂');
        assert_eq!(char_for_mask(0x04), '⠄');
        assert_eq!(char_for_mask(0x08), '⠈');
        assert_eq!(char_for_mask(0x10), '⠐');
        assert_eq!(char_for_mask(0x20), '⠠');
        assert_eq!(char_for_mask(0x40), '⡀');
        assert_eq!(char_for_mask(0x80), '⢀');
        assert_eq!(char_for_mask(0xFF), '⣿');
    }

    #[test]
    fn out_of_range_dots_have_no_bit() {
        assert_eq!(dot_bit(CELL_WIDTH, 0), None);
        assert_eq!(dot_bit(0, CELL_HEIGHT), None);
        assert_eq!(dot_bit_or_zero(9, 9), 0);
    }

    #[test]
    fn pack_unpack_round_trip() {
        for mask in 0u8..=255 {
            assert_eq!(pack(&unpack(mask)), mask);
        }
    }

    #[test]
    fn pack_matches_bit_table() {
        for y in 0..CELL_HEIGHT {
            for x in 0..CELL_WIDTH {
                let mut dots = [[false; 2]; 4];
                dots[y as usize][x as usize] = true;
                assert_eq!(Some(pack(&dots)), dot_bit(x, y));
            }
        }
    }
}
