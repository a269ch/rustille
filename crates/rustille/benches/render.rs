//! Benchmarks for the image pipeline at 1920x1080.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use rustille::{ColorMode, Dither, RenderOptions, Renderer};
use std::hint::black_box;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn gradient_luma() -> Vec<u8> {
    (0..WIDTH * HEIGHT)
        .map(|i| ((i % WIDTH) * 255 / WIDTH) as u8)
        .collect()
}

fn gradient_rgba() -> Vec<u8> {
    let mut data = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            data.extend_from_slice(&[
                (x * 255 / WIDTH) as u8,
                (y * 255 / HEIGHT) as u8,
                ((x + y) % 256) as u8,
                255,
            ]);
        }
    }
    data
}

fn render(c: &mut Criterion) {
    let luma = gradient_luma();
    let rgba = gradient_rgba();
    let base = RenderOptions::default().width(200);

    let mut group = c.benchmark_group("render/1920x1080");
    group.throughput(Throughput::Elements(u64::from(WIDTH) * u64::from(HEIGHT)));

    group.bench_function("grayscale", |b| {
        let renderer = Renderer::new(base);
        b.iter(|| {
            black_box(
                renderer
                    .render_luma(WIDTH, HEIGHT, black_box(&luma))
                    .map(|s| s.len()),
            )
        });
    });

    group.bench_function("rgba", |b| {
        let renderer = Renderer::new(base);
        b.iter(|| {
            black_box(
                renderer
                    .render_rgba(WIDTH, HEIGHT, black_box(&rgba))
                    .map(|s| s.len()),
            )
        });
    });

    group.bench_function("rgba+truecolor", |b| {
        let renderer = Renderer::new(base.color(ColorMode::TrueColor));
        b.iter(|| {
            black_box(
                renderer
                    .render_rgba(WIDTH, HEIGHT, black_box(&rgba))
                    .map(|s| s.len()),
            )
        });
    });

    group.bench_function("floyd-steinberg", |b| {
        let renderer = Renderer::new(base.dither(Dither::FloydSteinberg));
        b.iter(|| {
            black_box(
                renderer
                    .render_luma(WIDTH, HEIGHT, black_box(&luma))
                    .map(|s| s.len()),
            )
        });
    });

    group.finish();
}

criterion_group!(benches, render);
criterion_main!(benches);
