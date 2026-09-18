//! Draws a few primitives on a Braille canvas.
//!
//! ```sh
//! cargo run --example canvas_shapes
//! ```

use rustille::Canvas;

fn main() {
    let (width, height) = (120, 60);
    let mut canvas = Canvas::new(width, height);

    // Border.
    canvas.rectangle(0, 0, width as i32 - 1, height as i32 - 1);

    // Crossing diagonals.
    canvas.line(0, 0, width as i32 - 1, height as i32 - 1);
    canvas.line(width as i32 - 1, 0, 0, height as i32 - 1);

    // A circle in the middle, with a filled dot at its centre.
    let (cx, cy) = (width as i32 / 2, height as i32 / 2);
    canvas.circle(cx, cy, 24);
    canvas.filled_circle(cx, cy, 4);

    // A sine wave across the canvas.
    for x in 0..width as i32 {
        let phase = f64::from(x) / f64::from(width) * std::f64::consts::TAU * 2.0;
        let y = cy + (phase.sin() * 10.0) as i32;
        canvas.set(x, y);
    }

    println!("{}", canvas.render());
    println!("{} of {} dots set", canvas.count(), width * height);
}
