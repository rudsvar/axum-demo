use axum_demo::service::greet_service::greet;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn greet_benchmark(c: &mut Criterion) {
    c.bench_function("greet", |b| b.iter(|| greet(black_box("World"))));
}

criterion_group!(benches, greet_benchmark);
criterion_main!(benches);
