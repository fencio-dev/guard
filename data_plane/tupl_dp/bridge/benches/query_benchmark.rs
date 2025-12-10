use criterion::{criterion_group, criterion_main, Criterion};

pub fn noop_bench(c: &mut Criterion) {
    c.bench_function("noop", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, noop_bench);
criterion_main!(benches);
