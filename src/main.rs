use std::ffi::OsStr;
use std::path::Path;

use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg};

use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request,
};
use libc::{ENOENT, EPERM, ERANGE};
use time::Timespec;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

const DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: Timespec { sec: 0, nsec: 0 },
    mtime: Timespec { sec: 0, nsec: 0 },
    ctime: Timespec { sec: 0, nsec: 0 },
    crtime: Timespec { sec: 0, nsec: 0 },
    kind: FileType::Directory,
    perm: 0o777,
    nlink: 2,
    uid: 0,
    gid: 0,
    rdev: 0,
    flags: 0,
};

const NULL_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 0,
    blocks: 1,
    atime: Timespec { sec: 0, nsec: 0 },
    mtime: Timespec { sec: 0, nsec: 0 },
    ctime: Timespec { sec: 0, nsec: 0 },
    crtime: Timespec { sec: 0, nsec: 0 },
    kind: FileType::RegularFile,
    perm: 0o666,
    nlink: 1,
    uid: 0,
    gid: 0,
    rdev: 0,
    flags: 0,
};

struct NullFS;

impl Filesystem for NullFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name == "null" {
            reply.entry(&TTL, &NULL_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &DIR_ATTR),
            2 => reply.attr(&TTL, &NULL_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        match ino {
            1 => reply.attr(&TTL, &DIR_ATTR),
            2 => reply.attr(&TTL, &NULL_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(b"");
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "null"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        if ino != 2 {
            reply.error(ENOENT);
            return;
        }

        reply.written(data.len() as u32)
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        flags: u32,
        reply: ReplyCreate,
    ) {
        if parent == 1 && name == "null" {
            reply.created(&TTL, &NULL_ATTR, 0, 2, flags);
        } else {
            reply.error(EPERM);
        }
    }

    fn mknod(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        if parent == 1 && name == "null" {
            reply.entry(&TTL, &NULL_ATTR, 0);
        } else {
            reply.error(EPERM);
        }
    }

    fn flush(&mut self, _req: &Request, ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        match ino {
            1 => reply.error(EPERM),
            2 => reply.ok(),
            _ => reply.error(ENOENT),
        }
    }

    fn release(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        match ino {
            1 => reply.error(EPERM),
            2 => reply.ok(),
            _ => reply.error(ENOENT),
        }
    }

    fn fsync(&mut self, _req: &Request, ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        match ino {
            1 => reply.error(EPERM),
            2 => reply.ok(),
            _ => reply.error(ENOENT),
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        match ino {
            1 => reply.error(EPERM),
            2 => reply.opened(2, flags),
            _ => reply.error(ENOENT),
        }
    }

    fn releasedir(&mut self, _req: &Request, ino: u64, _fh: u64, _flags: u32, reply: ReplyEmpty) {
        match ino {
            1 => reply.ok(),
            2 => reply.error(EPERM),
            _ => reply.error(ENOENT),
        }
    }

    fn fsyncdir(&mut self, _req: &Request, ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        match ino {
            1 => reply.ok(),
            2 => reply.error(EPERM),
            _ => reply.error(ENOENT),
        }
    }

    fn opendir(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        match ino {
            1 => reply.opened(1, flags),
            2 => reply.error(EPERM),
            _ => reply.error(ENOENT),
        }
    }

    fn access(&mut self, _req: &Request, ino: u64, _mask: u32, reply: ReplyEmpty) {
        match ino {
            1 => reply.ok(),
            2 => reply.ok(),
            _ => reply.error(ENOENT),
        }
    }

    fn getxattr(&mut self, _req: &Request, ino: u64, _name: &OsStr, size: u32, reply: ReplyXattr) {
        if size == 0 {
            match ino {
                1 => reply.size(0),
                2 => reply.size(0),
                _ => reply.error(ENOENT),
            }
            return;
        }
        reply.error(ERANGE);
    }
}

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("MOUNT")
                .help("path to the mounting point")
                .index(1)
                .required(true),
        )
        .get_matches();

    let path = Path::new(matches.value_of("MOUNT").unwrap());

    dbg!(path);

    let fuse_args: Vec<&OsStr> = vec![
        &OsStr::new("-o"),
        &OsStr::new("auto_unmount"),
        &OsStr::new("-o"),
        &OsStr::new("rw"),
        &OsStr::new("-o"),
        &OsStr::new("fsname=nullfs"),
    ];

    fuse::mount(NullFS, &path, &fuse_args).unwrap();
}
