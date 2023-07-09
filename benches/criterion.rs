use axum_demo::feature::hello::hello_service::hello;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn greet_benchmark(c: &mut Criterion) {
    c.bench_function("greet", |b| b.iter(|| hello(black_box("World"))));
}

criterion_group!(benches, greet_benchmark);
criterion_main!(benches);
