//! The Unicode Braille contract, exercised through the public API.

use rustille::braille::{
    BLANK, BRAILLE_BASE, CELL_HEIGHT, CELL_WIDTH, DOTS_PER_CELL, char_for_mask, dot_bit,
    dot_number_bit, mask_for_char, pack, unpack,
};

#[test]
fn all_256_masks_map_to_the_braille_block() {
    let mut seen = std::collections::BTreeSet::new();
    for mask in 0u8..=255 {
        let character = char_for_mask(mask);
        let code_point = character as u32;

        assert_eq!(code_point, BRAILLE_BASE + u32::from(mask));
        assert!(
            (0x2800..=0x28FF).contains(&code_point),
            "mask {mask:#04x} escaped the Braille block"
        );
        assert_eq!(mask_for_char(character), Some(mask));
        assert_eq!(character.len_utf8(), 3);
        assert!(seen.insert(character), "duplicate for mask {mask:#04x}");
    }
    assert_eq!(seen.len(), 256);
}

#[test]
fn the_empty_cell_is_u2800_and_not_a_space() {
    assert_eq!(char_for_mask(0), BLANK);
    assert_eq!(BLANK as u32, 0x2800);
    assert_ne!(BLANK, ' ');
}

#[test]
fn the_full_cell_is_u28ff() {
    assert_eq!(char_for_mask(0xFF), '\u{28FF}');
    assert_eq!(char_for_mask(0xFF), '⣿');
}

#[test]
fn dots_one_through_eight_have_the_documented_bits() {
    let expected: [(u32, u8, char); DOTS_PER_CELL as usize] = [
        (1, 0x01, '⠁'),
        (2, 0x02, '⠂'),
        (3, 0x04, '⠄'),
        (4, 0x08, '⠈'),
        (5, 0x10, '⠐'),
        (6, 0x20, '⠠'),
        (7, 0x40, '⡀'),
        (8, 0x80, '⢀'),
    ];
    for (dot, bit, character) in expected {
        assert_eq!(dot_number_bit(dot), Some(bit), "dot {dot}");
        assert_eq!(char_for_mask(bit), character, "dot {dot}");
    }
}

#[test]
fn coordinates_map_to_the_documented_bits() {
    let expected = [
        ((0, 0), 0x01),
        ((0, 1), 0x02),
        ((0, 2), 0x04),
        ((1, 0), 0x08),
        ((1, 1), 0x10),
        ((1, 2), 0x20),
        ((0, 3), 0x40),
        ((1, 3), 0x80),
    ];
    for ((x, y), bit) in expected {
        assert_eq!(dot_bit(x, y), Some(bit), "dot at {x},{y}");
    }
    assert_eq!(dot_bit(CELL_WIDTH, 0), None);
    assert_eq!(dot_bit(0, CELL_HEIGHT), None);
}

#[test]
fn masks_are_the_union_of_their_dots() {
    for mask in 0u8..=255 {
        let dots = unpack(mask);
        let rebuilt: u8 = (0..CELL_HEIGHT)
            .flat_map(|y| (0..CELL_WIDTH).map(move |x| (x, y)))
            .filter(|(x, y)| dots[*y as usize][*x as usize])
            .filter_map(|(x, y)| dot_bit(x, y))
            .fold(0, |acc, bit| acc | bit);
        assert_eq!(rebuilt, mask);
        assert_eq!(pack(&dots), mask);
    }
}

#[test]
fn non_braille_characters_have_no_mask() {
    for character in ['a', ' ', '\u{27FF}', '\u{2900}', '😀'] {
        assert_eq!(mask_for_char(character), None, "{character:?}");
    }
}
