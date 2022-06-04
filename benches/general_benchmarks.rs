use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("segment read, no data", |b| {
        b.iter(|| tdms::TDMSFile::from_path("./data/standard.tdms", true))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
