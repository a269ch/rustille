//! Renders a procedurally generated image without touching the filesystem.
//!
//! ```sh
//! cargo run --example render_gradient
//! ```

use rustille::{Dither, RenderOptions, Renderer};

fn main() -> Result<(), rustille::Error> {
    let (width, height) = (256u32, 256u32);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            // A radial gradient, fully opaque.
            let dx = f64::from(x) - f64::from(width) / 2.0;
            let dy = f64::from(y) - f64::from(height) / 2.0;
            let distance = (dx * dx + dy * dy).sqrt() / f64::from(width / 2);
            let value = (255.0 * (1.0 - distance).clamp(0.0, 1.0)) as u8;
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }

    for dither in [Dither::None, Dither::FloydSteinberg] {
        let options = RenderOptions::default().width(60).dither(dither);
        println!("--- {dither} ---");
        println!(
            "{}",
            Renderer::new(options).render_rgba(width, height, &rgba)?
        );
    }

    Ok(())
}
