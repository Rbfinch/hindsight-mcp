use criterion::{criterion_group, criterion_main, Criterion};

fn copilot_benchmark(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // TODO: Add benchmarks for Copilot log parsing
            std::hint::black_box(1 + 1)
        })
    });
}

criterion_group!(benches, copilot_benchmark);
criterion_main!(benches);
