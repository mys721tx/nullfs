//! Benchmarks of mount option parsing.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use nullfs::{parse_mount_option, parse_options};

fn parse_single(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_mount_option");
    for (label, opt) in [
        ("keyword_first", "auto_unmount"),
        ("keyword_last", "async"),
        ("key_value", "fsname=nullfs"),
        ("custom", "max_read=131072"),
    ] {
        g.bench_function(label, |b| b.iter(|| parse_mount_option(black_box(opt))));
    }
    g.finish();
}

fn parse_many(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_options");
    g.bench_function("empty", |b| {
        b.iter(|| parse_options(black_box(std::iter::empty())))
    });
    g.bench_function("typical", |b| {
        let args = ["ro,nosuid,nodev", "fsname=nullfs", "allow_other"];
        b.iter(|| parse_options(black_box(args).into_iter()))
    });
    g.bench_function("long_list", |b| {
        let arg = ["rw", "noexec", "noatime", "subtype=null", "foo=bar"].repeat(20);
        let arg = arg.join(",");
        b.iter(|| parse_options(std::iter::once(black_box(arg.as_str()))))
    });
    g.finish();
}

criterion_group!(benches, parse_single, parse_many);
criterion_main!(benches);
