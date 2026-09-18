//! Canvas behaviour through the public API.

use rustille::Canvas;
use rustille::braille::mask_for_char;

#[test]
fn the_documented_example_behaves() {
    let mut canvas = Canvas::new(100, 50);
    assert_eq!((canvas.width(), canvas.height()), (100, 50));
    assert_eq!((canvas.cells_width(), canvas.cells_height()), (50, 13));

    canvas.set(10, 10);
    canvas.set(11, 11);
    canvas.unset(10, 10);

    assert!(!canvas.get(10, 10));
    assert!(canvas.get(11, 11));
    assert_eq!(canvas.count(), 1);

    let rendered = canvas.render();
    assert_eq!(rendered.lines().count(), 13);
    for line in rendered.lines() {
        assert_eq!(line.chars().count(), 50);
        assert!(line.chars().all(|c| mask_for_char(c).is_some()));
    }
}

#[test]
fn dimensions_round_up_and_edges_stay_inside() {
    // 3x3 dots -> 2x1 cells, with 10 of the 16 dot slots unused.
    let mut canvas = Canvas::new(3, 3);
    assert_eq!((canvas.cells_width(), canvas.cells_height()), (2, 1));
    canvas.fill();
    assert_eq!(canvas.count(), 9);
    for y in 0..3 {
        for x in 0..3 {
            assert!(canvas.get(x, y), "{x},{y} should be set");
        }
    }
    assert!(!canvas.get(3, 0));
    assert!(!canvas.get(0, 3));
}

#[test]
fn toggle_is_its_own_inverse() {
    let mut canvas = Canvas::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            canvas.toggle(x, y);
        }
    }
    assert_eq!(canvas.count(), 256);
    for y in 0..16 {
        for x in 0..16 {
            canvas.toggle(x, y);
        }
    }
    assert!(canvas.is_empty());
}

#[test]
fn out_of_range_coordinates_never_wrap() {
    let mut canvas = Canvas::new(8, 8);
    let far = [
        (-1, -1),
        (8, 8),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (-1, 4),
        (4, -1),
    ];
    for (x, y) in far {
        assert!(!canvas.set(x, y), "({x}, {y}) reported as in bounds");
    }
    assert!(
        canvas.is_empty(),
        "an out-of-range write leaked into the canvas"
    );
}

#[test]
fn shapes_stay_within_the_canvas() {
    let mut canvas = Canvas::new(32, 32);
    canvas.line(-100, -100, 200, 200);
    canvas.rectangle(-5, -5, 40, 40);
    canvas.circle(16, 16, 100);
    canvas.filled_circle(-20, -20, 10);
    canvas.filled_rectangle(-50, -50, 50, 50);
    assert_eq!(canvas.count(), 32 * 32);

    let rendered = canvas.render();
    assert_eq!(rendered.lines().count(), 8);
    assert!(rendered.chars().all(|c| c == '\n' || c == '\u{28FF}'));
}

#[test]
fn rendering_is_stable_across_buffers() {
    let mut canvas = Canvas::new(20, 12);
    canvas.line(0, 0, 19, 11);
    canvas.circle(10, 6, 5);

    let once = canvas.render();
    let mut twice = String::new();
    canvas.render_into(&mut twice);
    assert_eq!(once, twice);
    assert_eq!(once, canvas.to_string());
}

#[test]
fn a_zero_sized_canvas_is_valid_and_empty() {
    let canvas = Canvas::new(0, 0);
    assert_eq!(canvas.render(), "");
    assert_eq!(canvas.cell_mask(0, 0), None);
}

#[test]
fn oversized_canvases_fail_cleanly() {
    let error = Canvas::try_new(u32::MAX, u32::MAX).unwrap_err();
    assert_eq!(error.kind(), rustille::ErrorKind::InvalidDimensions);
}
