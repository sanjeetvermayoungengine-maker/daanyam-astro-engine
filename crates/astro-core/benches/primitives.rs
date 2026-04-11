use astro_core::math::{degrees_to_radians, normalize_degrees, radians_to_degrees};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn primitive_smoke(c: &mut Criterion) {
    c.bench_function("angle_round_trip", |b| {
        b.iter(|| {
            let angle = black_box(725.123_456_f64);
            let normalized = normalize_degrees(angle);
            radians_to_degrees(degrees_to_radians(normalized))
        });
    });
}

criterion_group!(benches, primitive_smoke);
criterion_main!(benches);
