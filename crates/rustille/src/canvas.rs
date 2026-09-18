//! A bitmap you draw on with Braille dots.
//!
//! [`Canvas`] is sized in **dots**, not characters: a 100×50 canvas is 50
//! characters wide and 13 characters tall (50 = ⌈100/2⌉, 13 = ⌈50/4⌉).
//!
//! Storage is exactly one bit per dot: the canvas keeps one `u8` per character
//! cell, and that byte *is* the Braille dot mask, so rendering is a table-free
//! walk over the buffer.
//!
//! ```
//! use rustille::Canvas;
//!
//! let mut canvas = Canvas::new(8, 8);
//! canvas.line(0, 0, 7, 7);
//! canvas.set(7, 0);
//! assert!(canvas.get(0, 0));
//! println!("{}", canvas.render());
//! ```

use core::fmt;

use crate::braille::{self, CELL_HEIGHT, CELL_WIDTH};
use crate::error::{Error, Result};

/// Largest canvas Rustille will allocate, in character cells.
///
/// One cell is one byte, so this caps a canvas at 256 MiB — roughly a
/// 32,768 × 65,536 dot surface. Anything larger is rejected with
/// [`Error::InvalidDimensions`] rather than aborting on allocation failure.
pub const MAX_CANVAS_CELLS: u64 = 1 << 28;

/// A fixed-size grid of Braille dots.
#[derive(Clone, PartialEq, Eq)]
pub struct Canvas {
    width: u32,
    height: u32,
    cells_width: u32,
    cells_height: u32,
    cells: Vec<u8>,
}

impl Canvas {
    /// Creates an empty canvas `width`×`height` **dots** in size.
    ///
    /// # Panics
    ///
    /// Panics if the canvas would need more than `usize::MAX` bytes. Use
    /// [`Canvas::try_new`] to handle that case without unwinding.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        match Self::try_new(width, height) {
            Ok(canvas) => canvas,
            Err(err) => panic!("cannot allocate {width}x{height} canvas: {err}"),
        }
    }

    /// Creates an empty canvas `width`×`height` dots in size.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDimensions`] if the number of character cells
    /// does not fit in a `usize`.
    pub fn try_new(width: u32, height: u32) -> Result<Self> {
        let cells_width = width.div_ceil(CELL_WIDTH);
        let cells_height = height.div_ceil(CELL_HEIGHT);
        let cells = u64::from(cells_width) * u64::from(cells_height);
        if cells > MAX_CANVAS_CELLS {
            return Err(Error::invalid_dimensions(format!(
                "{width}x{height} dots needs {cells} character cells, over the \
                 {MAX_CANVAS_CELLS} cell limit"
            )));
        }
        let len = usize::try_from(cells).map_err(|_| {
            Error::invalid_dimensions(format!("{width}x{height} dots is too large"))
        })?;
        Ok(Self {
            width,
            height,
            cells_width,
            cells_height,
            cells: vec![0; len],
        })
    }

    /// Canvas width in dots.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Canvas height in dots.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Rendered width in character cells.
    #[must_use]
    pub const fn cells_width(&self) -> u32 {
        self.cells_width
    }

    /// Rendered height in character cells (that is, in text rows).
    #[must_use]
    pub const fn cells_height(&self) -> u32 {
        self.cells_height
    }

    /// Whether `(x, y)` lies inside the canvas.
    #[must_use]
    pub const fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Index of the cell owning `(x, y)`, plus that dot's bit.
    ///
    /// Returns [`None`] for out-of-bounds coordinates.
    #[inline]
    fn locate(&self, x: i32, y: i32) -> Option<(usize, u8)> {
        if !self.in_bounds(x, y) {
            return None;
        }
        let (x, y) = (x as u32, y as u32);
        let index =
            (y / CELL_HEIGHT) as usize * self.cells_width as usize + (x / CELL_WIDTH) as usize;
        let bit = braille::dot_bit_or_zero(x % CELL_WIDTH, y % CELL_HEIGHT);
        Some((index, bit))
    }

    /// Turns the dot at `(x, y)` on.
    ///
    /// Out-of-range coordinates are ignored; the return value says whether the
    /// dot was inside the canvas.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32) -> bool {
        match self.locate(x, y) {
            Some((index, bit)) => {
                self.cells[index] |= bit;
                true
            }
            None => false,
        }
    }

    /// Turns the dot at `(x, y)` off. Out-of-range coordinates are ignored.
    #[inline]
    pub fn unset(&mut self, x: i32, y: i32) -> bool {
        match self.locate(x, y) {
            Some((index, bit)) => {
                self.cells[index] &= !bit;
                true
            }
            None => false,
        }
    }

    /// Flips the dot at `(x, y)`. Out-of-range coordinates are ignored.
    #[inline]
    pub fn toggle(&mut self, x: i32, y: i32) -> bool {
        match self.locate(x, y) {
            Some((index, bit)) => {
                self.cells[index] ^= bit;
                true
            }
            None => false,
        }
    }

    /// Sets or clears the dot at `(x, y)` depending on `value`.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, value: bool) -> bool {
        if value {
            self.set(x, y)
        } else {
            self.unset(x, y)
        }
    }

    /// Reads the dot at `(x, y)`. Out-of-range coordinates read as `false`.
    #[inline]
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> bool {
        match self.locate(x, y) {
            Some((index, bit)) => self.cells[index] & bit != 0,
            None => false,
        }
    }

    /// Clears every dot.
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Sets every dot inside the canvas.
    ///
    /// Dots that belong to a partially used edge cell stay off, so the rendered
    /// output never spills past `width`×`height`.
    pub fn fill(&mut self) {
        for cy in 0..self.cells_height {
            let rows = (self.height - cy * CELL_HEIGHT).min(CELL_HEIGHT);
            for cx in 0..self.cells_width {
                let cols = (self.width - cx * CELL_WIDTH).min(CELL_WIDTH);
                let index = cy as usize * self.cells_width as usize + cx as usize;
                self.cells[index] = bounded_mask(cols, rows);
            }
        }
    }

    /// Number of dots currently set.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.cells.iter().map(|c| c.count_ones()).sum()
    }

    /// Whether no dot is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| *c == 0)
    }

    /// The raw Braille dot mask of the character cell at `(cell_x, cell_y)`.
    #[must_use]
    pub fn cell_mask(&self, cell_x: u32, cell_y: u32) -> Option<u8> {
        if cell_x >= self.cells_width || cell_y >= self.cells_height {
            return None;
        }
        let index = cell_y as usize * self.cells_width as usize + cell_x as usize;
        self.cells.get(index).copied()
    }

    /// The whole dot-mask buffer, row-major over character cells.
    #[must_use]
    pub fn masks(&self) -> &[u8] {
        &self.cells
    }

    /// Draws a line from `(x0, y0)` to `(x1, y1)` using Bresenham's algorithm.
    ///
    /// Parts of the line outside the canvas are clipped away.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.plot_line(x0, y0, x1, y1, true);
    }

    /// Like [`Canvas::line`], but clears the dots it walks over.
    pub fn unset_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.plot_line(x0, y0, x1, y1, false);
    }

    fn plot_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, value: bool) {
        // Integer Bresenham, written for the general case: the error term
        // tracks twice the distance to the ideal line so no division is needed.
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let step_x = if x0 < x1 { 1 } else { -1 };
        let step_y = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.put(x, y, value);
            if x == x1 && y == y1 {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                if x == x1 {
                    break;
                }
                error += dy;
                x += step_x;
            }
            if doubled <= dx {
                if y == y1 {
                    break;
                }
                error += dx;
                y += step_y;
            }
        }
    }

    /// Draws the outline of the rectangle whose corners are `(x0, y0)` and
    /// `(x1, y1)` (both inclusive).
    pub fn rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (left, right) = (x0.min(x1), x0.max(x1));
        let (top, bottom) = (y0.min(y1), y0.max(y1));
        self.line(left, top, right, top);
        self.line(left, bottom, right, bottom);
        self.line(left, top, left, bottom);
        self.line(right, top, right, bottom);
    }

    /// Fills the rectangle whose corners are `(x0, y0)` and `(x1, y1)` (both
    /// inclusive).
    pub fn filled_rectangle(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (left, right) = (x0.min(x1), x0.max(x1));
        let (top, bottom) = (y0.min(y1), y0.max(y1));
        // Clamp before iterating so a rectangle far outside the canvas does not
        // cost a billion no-op writes.
        let left = left.max(0);
        let top = top.max(0);
        let right = right.min(self.width.saturating_sub(1) as i32);
        let bottom = bottom.min(self.height.saturating_sub(1) as i32);
        for y in top..=bottom {
            for x in left..=right {
                self.set(x, y);
            }
        }
    }

    /// Draws a circle outline of `radius` dots centred on `(cx, cy)` using the
    /// midpoint algorithm.
    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32) {
        if radius < 0 {
            return;
        }
        let mut x = radius;
        let mut y = 0;
        let mut error = 1 - radius;
        while x >= y {
            for (dx, dy) in [
                (x, y),
                (y, x),
                (-y, x),
                (-x, y),
                (-x, -y),
                (-y, -x),
                (y, -x),
                (x, -y),
            ] {
                self.set(cx + dx, cy + dy);
            }
            y += 1;
            if error < 0 {
                error += 2 * y + 1;
            } else {
                x -= 1;
                error += 2 * (y - x) + 1;
            }
        }
    }

    /// Fills a disc of `radius` dots centred on `(cx, cy)`.
    pub fn filled_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        if radius < 0 {
            return;
        }
        let top = (cy - radius).max(0);
        let bottom = (cy + radius).min(self.height.saturating_sub(1) as i32);
        let radius_squared = i64::from(radius) * i64::from(radius);
        for y in top..=bottom {
            let dy = i64::from(y - cy);
            let span = ((radius_squared - dy * dy).max(0) as f64).sqrt() as i32;
            let left = (cx - span).max(0);
            let right = (cx + span).min(self.width.saturating_sub(1) as i32);
            for x in left..=right {
                self.set(x, y);
            }
        }
    }

    /// Renders the canvas to a newline-separated Braille string.
    ///
    /// Rows are separated by `\n`; there is no trailing newline.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.render_into(&mut out);
        out
    }

    /// Renders the canvas by appending to an existing buffer.
    ///
    /// Reuse the buffer across frames to keep an animation allocation-free.
    pub fn render_into(&self, out: &mut String) {
        if self.cells.is_empty() {
            return;
        }
        // Every Braille character is 3 bytes of UTF-8, plus one newline per row.
        let rows = self.cells_height as usize;
        let columns = self.cells_width as usize;
        out.reserve(rows * (columns * 3 + 1));
        for (row_index, row) in self.cells.chunks_exact(columns).enumerate() {
            if row_index > 0 {
                out.push('\n');
            }
            for mask in row {
                out.push(braille::char_for_mask(*mask));
            }
        }
    }
}

/// Mask covering the first `cols`×`rows` dots of a cell.
const fn bounded_mask(cols: u32, rows: u32) -> u8 {
    let mut mask = 0u8;
    let mut y = 0;
    while y < rows {
        let mut x = 0;
        while x < cols {
            mask |= braille::dot_bit_or_zero(x, y);
            x += 1;
        }
        y += 1;
    }
    mask
}

impl fmt::Display for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        self.render_into(&mut buffer);
        f.write_str(&buffer)
    }
}

impl fmt::Debug for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Canvas")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("cells_width", &self.cells_width)
            .field("cells_height", &self.cells_height)
            .field("dots_set", &self.count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_round_up_to_whole_cells() {
        let canvas = Canvas::new(5, 3);
        assert_eq!((canvas.width(), canvas.height()), (5, 3));
        assert_eq!((canvas.cells_width(), canvas.cells_height()), (3, 1));
        assert_eq!(canvas.masks().len(), 3);
    }

    #[test]
    fn zero_sized_canvas_renders_nothing() {
        for (w, h) in [(0, 0), (0, 10), (10, 0)] {
            let canvas = Canvas::new(w, h);
            assert_eq!(canvas.render(), "");
            assert!(canvas.is_empty());
            assert_eq!(canvas.count(), 0);
        }
    }

    #[test]
    fn set_unset_toggle() {
        let mut canvas = Canvas::new(2, 4);
        assert!(!canvas.get(0, 0));
        assert!(canvas.set(0, 0));
        assert!(canvas.get(0, 0));
        assert_eq!(canvas.render(), "\u{2801}");

        assert!(canvas.toggle(1, 3));
        assert_eq!(canvas.cell_mask(0, 0), Some(0x81));
        assert!(canvas.toggle(1, 3));
        assert_eq!(canvas.cell_mask(0, 0), Some(0x01));

        assert!(canvas.unset(0, 0));
        assert!(!canvas.get(0, 0));
        assert_eq!(canvas.render(), "\u{2800}");
    }

    #[test]
    fn every_dot_maps_to_its_bit() {
        for y in 0..4 {
            for x in 0..2 {
                let mut canvas = Canvas::new(2, 4);
                canvas.set(x as i32, y as i32);
                assert_eq!(
                    canvas.cell_mask(0, 0),
                    braille::dot_bit(x, y),
                    "dot {x},{y}"
                );
            }
        }
    }

    #[test]
    fn out_of_range_access_is_a_no_op() {
        let mut canvas = Canvas::new(4, 4);
        for (x, y) in [(-1, 0), (0, -1), (4, 0), (0, 4), (i32::MIN, i32::MAX)] {
            assert!(!canvas.set(x, y), "set({x},{y}) should be out of bounds");
            assert!(!canvas.unset(x, y));
            assert!(!canvas.toggle(x, y));
            assert!(!canvas.get(x, y));
            assert!(!canvas.in_bounds(x, y));
        }
        assert!(canvas.is_empty());
        assert_eq!(canvas.cell_mask(99, 0), None);
    }

    #[test]
    fn clear_and_fill() {
        let mut canvas = Canvas::new(4, 8);
        canvas.fill();
        assert_eq!(canvas.count(), 32);
        assert_eq!(canvas.render(), "\u{28FF}\u{28FF}\n\u{28FF}\u{28FF}");
        canvas.clear();
        assert!(canvas.is_empty());
    }

    #[test]
    fn fill_respects_partial_edge_cells() {
        // 3x2 dots occupies a 2x1 cell grid; only 6 of the 16 dots are real.
        let mut canvas = Canvas::new(3, 2);
        canvas.fill();
        assert_eq!(canvas.count(), 6);
        assert_eq!(canvas.cell_mask(0, 0), Some(0x01 | 0x02 | 0x08 | 0x10));
        assert_eq!(canvas.cell_mask(1, 0), Some(0x01 | 0x02));
        for y in 0..2 {
            for x in 0..3 {
                assert!(canvas.get(x, y));
            }
        }
    }

    #[test]
    fn horizontal_and_vertical_lines() {
        let mut canvas = Canvas::new(8, 8);
        canvas.line(0, 0, 7, 0);
        assert_eq!(canvas.count(), 8);
        for x in 0..8 {
            assert!(canvas.get(x, 0));
        }
        canvas.clear();
        canvas.line(3, 0, 3, 7);
        assert_eq!(canvas.count(), 8);
        for y in 0..8 {
            assert!(canvas.get(3, y));
        }
    }

    #[test]
    fn diagonal_line_is_symmetric() {
        let mut forward = Canvas::new(16, 16);
        forward.line(0, 0, 15, 15);
        let mut backward = Canvas::new(16, 16);
        backward.line(15, 15, 0, 0);
        assert_eq!(forward, backward);
        assert_eq!(forward.count(), 16);
        for i in 0..16 {
            assert!(forward.get(i, i));
        }
    }

    #[test]
    fn lines_are_clipped_not_wrapped() {
        let mut canvas = Canvas::new(8, 8);
        canvas.line(-50, 4, 50, 4);
        assert_eq!(canvas.count(), 8);
        for x in 0..8 {
            assert!(canvas.get(x, 4));
        }
    }

    #[test]
    fn unset_line_erases() {
        let mut canvas = Canvas::new(8, 8);
        canvas.fill();
        canvas.unset_line(0, 0, 7, 7);
        assert_eq!(canvas.count(), 64 - 8);
        assert!(!canvas.get(3, 3));
    }

    #[test]
    fn rectangle_outline_and_fill() {
        let mut canvas = Canvas::new(8, 8);
        canvas.rectangle(1, 1, 6, 6);
        assert_eq!(canvas.count(), 6 * 4 - 4);
        assert!(canvas.get(1, 1));
        assert!(!canvas.get(3, 3));

        canvas.clear();
        canvas.filled_rectangle(1, 1, 6, 6);
        assert_eq!(canvas.count(), 36);
        assert!(canvas.get(3, 3));
    }

    #[test]
    fn filled_rectangle_clamps_to_canvas() {
        let mut canvas = Canvas::new(4, 4);
        canvas.filled_rectangle(-1000, -1000, 1000, 1000);
        assert_eq!(canvas.count(), 16);
    }

    #[test]
    fn circles_stay_inside_their_radius() {
        let mut canvas = Canvas::new(32, 32);
        canvas.circle(16, 16, 10);
        assert!(!canvas.is_empty());
        for y in 0..32 {
            for x in 0..32 {
                if canvas.get(x, y) {
                    let distance = (((x - 16) * (x - 16) + (y - 16) * (y - 16)) as f64).sqrt();
                    assert!((distance - 10.0).abs() <= 1.5, "dot {x},{y} at {distance}");
                }
            }
        }
        canvas.clear();
        canvas.filled_circle(16, 16, 4);
        assert!(canvas.get(16, 16));
        assert!(!canvas.get(16, 25));
        canvas.clear();
        canvas.circle(0, 0, -1);
        canvas.filled_circle(0, 0, -1);
        assert!(canvas.is_empty());
    }

    #[test]
    fn render_into_appends_and_display_matches() {
        let mut canvas = Canvas::new(2, 8);
        canvas.set(0, 0);
        canvas.set(1, 7);
        let mut buffer = String::from("> ");
        canvas.render_into(&mut buffer);
        assert_eq!(buffer, format!("> {}", canvas.render()));
        assert_eq!(canvas.to_string(), canvas.render());
        assert_eq!(canvas.render(), "\u{2801}\n\u{2880}");
    }

    #[test]
    fn debug_is_informative() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set(0, 0);
        let text = format!("{canvas:?}");
        assert!(text.contains("dots_set: 1"), "{text}");
    }

    #[test]
    fn try_new_rejects_absurd_sizes() {
        let err = Canvas::try_new(u32::MAX, u32::MAX).unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::InvalidDimensions);
        assert!(Canvas::try_new(1, 1).is_ok());
    }

    #[test]
    #[should_panic(expected = "cannot allocate")]
    fn new_panics_on_absurd_sizes() {
        let _ = Canvas::new(u32::MAX, u32::MAX);
    }

    #[test]
    fn extreme_but_valid_canvas_works() {
        // 2 dots wide, a million tall: 1 column x 250_000 rows of characters.
        let mut canvas = Canvas::new(2, 1_000_000);
        assert_eq!(canvas.cells_height(), 250_000);
        canvas.set(1, 999_999);
        assert_eq!(canvas.count(), 1);
        assert_eq!(canvas.cell_mask(0, 249_999), Some(0x80));
    }
}
