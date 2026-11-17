//! End-to-end tests against a real FUSE mount.
//!
//! These need `/dev/fuse` and permission to mount, so they are ignored by
//! default. Run them on Linux with `cargo test -- --include-ignored`.

#![cfg(target_os = "linux")]

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use fuser::{BackgroundSession, Config};
use nullfs::NullFS;
use tempfile::TempDir;

/// A mounted NullFS that unmounts when dropped.
struct Mount {
    session: Option<BackgroundSession>,
    dir: TempDir,
}

impl Mount {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("create mount point");
        let session =
            fuser::spawn_mount(NullFS, dir.path(), &Config::default()).expect("mount nullfs");
        Mount {
            session: Some(session),
            dir,
        }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn null(&self) -> PathBuf {
        self.root().join("null")
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.umount_and_join();
        }
    }
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn root_contains_only_null() {
    let m = Mount::new();
    let names: Vec<_> = fs::read_dir(m.root())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["null"]);
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn root_and_null_attributes() {
    let m = Mount::new();

    let root = fs::metadata(m.root()).unwrap();
    assert!(root.is_dir());
    assert_eq!(root.ino(), 1);
    assert_eq!(root.permissions().mode() & 0o777, 0o777);

    let null = fs::metadata(m.null()).unwrap();
    assert!(null.is_file());
    assert_eq!(null.ino(), 2);
    assert_eq!(null.len(), 0);
    assert_eq!(null.permissions().mode() & 0o777, 0o666);
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn writes_are_discarded() {
    let m = Mount::new();

    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(m.null())
        .unwrap();
    f.write_all(b"hello, world").unwrap();
    f.write_all(&vec![0xAB; 1 << 20]).unwrap();
    f.sync_all().unwrap();

    f.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    assert_eq!(f.read_to_end(&mut buf).unwrap(), 0);
    drop(f);

    assert_eq!(fs::read(m.null()).unwrap(), b"");
    assert_eq!(fs::metadata(m.null()).unwrap().len(), 0);
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn fs_write_and_truncate_succeed() {
    let m = Mount::new();
    fs::write(m.null(), b"data").unwrap();
    File::create(m.null()).unwrap().write_all(b"more").unwrap();
    OpenOptions::new()
        .append(true)
        .open(m.null())
        .unwrap()
        .write_all(b"tail")
        .unwrap();
    assert_eq!(fs::read(m.null()).unwrap(), b"");
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn missing_paths_are_not_found() {
    let m = Mount::new();
    let err = fs::metadata(m.root().join("missing")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    let err = File::open(m.root().join("missing")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn cannot_create_new_files() {
    let m = Mount::new();
    let err = File::create(m.root().join("other")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
    assert!(fs::create_dir(m.root().join("dir")).is_err());
    assert_eq!(
        fs::read_dir(m.root()).unwrap().count(),
        1,
        "nothing new appeared"
    );
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn null_is_not_a_directory() {
    let m = Mount::new();
    assert!(fs::read_dir(m.null()).is_err());
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn root_cannot_be_opened_as_file_for_writing() {
    let m = Mount::new();
    assert!(OpenOptions::new().write(true).open(m.root()).is_err());
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn concurrent_writers() {
    let m = Mount::new();
    let path = m.null();
    std::thread::scope(|s| {
        for t in 0..8u8 {
            let path = &path;
            s.spawn(move || {
                let mut f = OpenOptions::new().write(true).open(path).unwrap();
                for _ in 0..64 {
                    f.write_all(&[t; 4096]).unwrap();
                }
            });
        }
    });
    assert_eq!(fs::read(&path).unwrap(), b"");
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn large_write_completes_in_one_call() {
    // With direct I/O the kernel splits one write() into as many FUSE
    // requests as needed and reports the full length back.
    let m = Mount::new();
    let mut f = OpenOptions::new().write(true).open(m.null()).unwrap();
    let buf = vec![0x5A; 4 << 20];
    assert_eq!(f.write(&buf).unwrap(), buf.len());
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn shared_mmap_still_works() {
    // Direct I/O files reject shared mmap unless the kernel grants
    // FUSE_DIRECT_IO_ALLOW_MMAP (Linux 6.6+), which nullfs requests.
    use std::os::fd::AsRawFd;

    let m = Mount::new();
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(m.null())
        .unwrap();
    let len = 4096;
    // SAFETY: plain mmap/munmap of a valid fd; the mapping is never touched,
    // since accessing past EOF of the empty file would raise SIGBUS.
    unsafe {
        let addr = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            f.as_raw_fd(),
            0,
        );
        assert_ne!(
            addr,
            libc::MAP_FAILED,
            "mmap failed: {}",
            std::io::Error::last_os_error()
        );
        assert_eq!(libc::munmap(addr, len), 0);
    }
}

#[test]
#[ignore = "needs /dev/fuse and permission to mount"]
fn missing_names_stay_missing() {
    // Misses are cached as negative entries; repeated lookups and a create
    // after a cached miss must still behave as before.
    let m = Mount::new();
    let missing = m.root().join("missing");
    for _ in 0..3 {
        assert_eq!(
            fs::metadata(&missing).unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }
    assert_eq!(
        File::create(&missing).unwrap_err().kind(),
        ErrorKind::PermissionDenied
    );
    assert_eq!(
        fs::metadata(&missing).unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert!(fs::metadata(m.null()).unwrap().is_file());
}
