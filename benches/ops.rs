//! Micro-benchmarks of the per-request filesystem logic (no FUSE involved).

use std::ffi::OsStr;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fuser::INodeNo;
use nullfs::{NULL_INO, ROOT_INO, ops};

const MISSING: INodeNo = INodeNo(42);

fn lookup(c: &mut Criterion) {
    let mut g = c.benchmark_group("lookup");
    for (label, parent, name) in [
        ("hit", ROOT_INO, "null"),
        ("miss_name", ROOT_INO, "missing"),
        ("miss_parent", NULL_INO, "null"),
    ] {
        g.bench_function(label, |b| {
            b.iter(|| ops::lookup(black_box(parent), black_box(OsStr::new(name))))
        });
    }
    g.finish();
}

fn attr(c: &mut Criterion) {
    let mut g = c.benchmark_group("attr");
    for (label, ino) in [("root", ROOT_INO), ("null", NULL_INO), ("missing", MISSING)] {
        g.bench_function(label, |b| b.iter(|| ops::attr(black_box(ino))));
    }
    g.finish();
}

fn readdir(c: &mut Criterion) {
    let mut g = c.benchmark_group("readdir");
    for offset in [0u64, 2, 3] {
        g.bench_with_input(BenchmarkId::from_parameter(offset), &offset, |b, &off| {
            b.iter(|| {
                ops::readdir(black_box(ROOT_INO), black_box(off))
                    .unwrap()
                    .for_each(|e| {
                        black_box(e);
                    })
            })
        });
    }
    g.finish();
}

fn write(c: &mut Criterion) {
    // Only the length is used, so cost is constant; no byte throughput here.
    let mut g = c.benchmark_group("write");
    for size in [0usize, 4 << 10, 128 << 10, 1 << 20] {
        let buf = vec![0u8; size];
        g.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| ops::write(black_box(NULL_INO), black_box(buf)))
        });
    }
    g.finish();
}

fn misc(c: &mut Criterion) {
    let mut g = c.benchmark_group("misc");
    g.bench_function("read", |b| b.iter(|| ops::read(black_box(NULL_INO))));
    g.bench_function("open", |b| b.iter(|| ops::open(black_box(NULL_INO))));
    g.bench_function("opendir", |b| b.iter(|| ops::opendir(black_box(ROOT_INO))));
    g.bench_function("create", |b| {
        b.iter(|| ops::create(black_box(ROOT_INO), black_box(OsStr::new("null"))))
    });
    g.bench_function("file_op", |b| b.iter(|| ops::file_op(black_box(NULL_INO))));
    g.bench_function("getxattr", |b| {
        b.iter(|| ops::getxattr(black_box(NULL_INO), black_box(0)))
    });
    g.finish();
}

criterion_group!(benches, lookup, attr, readdir, write, misc);
criterion_main!(benches);
