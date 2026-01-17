use criterion::{Criterion, criterion_group, criterion_main};

fn tests_benchmark(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // TODO: Add benchmarks for test parsing operations
            std::hint::black_box(1 + 1)
        })
    });
}

criterion_group!(benches, tests_benchmark);
criterion_main!(benches);
