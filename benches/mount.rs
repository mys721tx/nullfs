//! End-to-end benchmarks through a real FUSE mount (kernel round-trips).
//!
//! Needs `/dev/fuse` and permission to mount (Linux). Where mounting fails
//! (e.g. macOS) the benchmarks are skipped with a message instead of failing.

use std::fs::{self, OpenOptions};
use std::hint::black_box;
use std::io::{Read, Write};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fuser::Config;
use nullfs::NullFS;

fn mounted(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create mount point");
    let session = match fuser::spawn_mount(NullFS, dir.path(), &Config::default()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping mount benchmarks: cannot mount nullfs: {e}");
            return;
        }
    };
    let root = dir.path();
    let null = root.join("null");

    let mut g = c.benchmark_group("mount/metadata");
    g.bench_function("stat_null", |b| b.iter(|| fs::metadata(black_box(&null))));
    g.bench_function("stat_missing", |b| {
        let missing = root.join("missing");
        b.iter(|| fs::metadata(black_box(&missing)))
    });
    g.bench_function("readdir_root", |b| {
        b.iter(|| fs::read_dir(root).unwrap().count())
    });
    g.bench_function("open_close", |b| {
        b.iter(|| OpenOptions::new().write(true).open(&null).unwrap())
    });
    g.finish();

    // Each file handle lives in its own block so it is closed before unmounting.
    // As non-root, fuser unmounts lazily (`fusermount -u -z`): the FUSE
    // connection stays alive while any file is still open, so joining the
    // session with a handle open would block forever.
    {
        let mut g = c.benchmark_group("mount/write");
        let mut f = OpenOptions::new().write(true).open(&null).unwrap();
        for size in [4usize << 10, 64 << 10, 128 << 10, 1 << 20] {
            let buf = vec![0u8; size];
            g.throughput(Throughput::Bytes(size as u64));
            g.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
                b.iter(|| f.write_all(black_box(buf)).unwrap())
            });
        }
        g.finish();
    }

    {
        let mut g = c.benchmark_group("mount/read");
        let mut f = OpenOptions::new().read(true).open(&null).unwrap();
        let mut buf = vec![0u8; 128 << 10];
        g.bench_function("read_eof", |b| {
            b.iter(|| f.read(black_box(&mut buf)).unwrap())
        });
        g.finish();
    }

    if let Err(e) = session.umount_and_join() {
        eprintln!("failed to unmount nullfs: {e}");
    }
}

criterion_group!(benches, mounted);
criterion_main!(benches);
