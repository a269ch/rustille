//! Benchmarks for the encoder and the drawing canvas.

use criterion::{Criterion, criterion_group, criterion_main};
use rustille::{Canvas, braille};
use std::hint::black_box;

fn braille_packing(c: &mut Criterion) {
    c.bench_function("char_for_mask/256", |b| {
        b.iter(|| {
            let mut sum = 0u32;
            for mask in 0u8..=255 {
                sum += black_box(braille::char_for_mask(black_box(mask))) as u32;
            }
            sum
        });
    });

    c.bench_function("pack/256", |b| {
        b.iter(|| {
            let mut sum = 0u32;
            for mask in 0u8..=255 {
                let dots = braille::unpack(black_box(mask));
                sum += u32::from(braille::pack(black_box(&dots)));
            }
            sum
        });
    });
}

fn canvas_operations(c: &mut Criterion) {
    c.bench_function("canvas/line/1000x1000", |b| {
        let mut canvas = Canvas::new(1000, 1000);
        b.iter(|| {
            canvas.clear();
            canvas.line(0, 0, 999, 999);
            black_box(canvas.count())
        });
    });

    c.bench_function("canvas/render/1920x1080", |b| {
        let mut canvas = Canvas::new(1920, 1080);
        canvas.fill();
        let mut buffer = String::new();
        b.iter(|| {
            buffer.clear();
            canvas.render_into(&mut buffer);
            black_box(buffer.len())
        });
    });
}

criterion_group!(benches, braille_packing, canvas_operations);
criterion_main!(benches);
