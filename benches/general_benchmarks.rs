use criterion::{criterion_group, criterion_main, Criterion};
use std::path::Path;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("segment read, no data", |b| {
        b.iter(|| tdms::TDMSFile::from_path(Path::new("./data/standard.tdms")))
    });

    c.bench_function("segment read, data", |b| {
        b.iter(|| tdms::TDMSFile::from_path(Path::new("./data/standard.tdms")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
