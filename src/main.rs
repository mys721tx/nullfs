use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, SystemTime};

use clap::{command, Arg};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};
use libc::{ENOENT, EPERM, ERANGE};

const TTL: Duration = Duration::from_secs(1);

const DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: SystemTime::UNIX_EPOCH,
    mtime: SystemTime::UNIX_EPOCH,
    ctime: SystemTime::UNIX_EPOCH,
    crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o777,
    nlink: 2,
    uid: 0,
    gid: 0,
    rdev: 0,
    flags: 0,
    blksize: 0,
};

const NULL_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 0,
    blocks: 1,
    atime: SystemTime::UNIX_EPOCH,
    mtime: SystemTime::UNIX_EPOCH,
    ctime: SystemTime::UNIX_EPOCH,
    crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o666,
    nlink: 1,
    uid: 0,
    gid: 0,
    rdev: 0,
    flags: 0,
    blksize: 0,
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
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
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
        _flags: i32,
        _lock_owner: Option<u64>,
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
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
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
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        if parent == 1 && name == "null" {
            reply.created(&TTL, &NULL_ATTR, 0, 2, flags as u32);
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
        _umask: u32,
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
        _flags: i32,
        _lock_owner: Option<u64>,
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

    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        match ino {
            1 => reply.error(EPERM),
            2 => reply.opened(2, flags as u32),
            _ => reply.error(ENOENT),
        }
    }

    fn releasedir(&mut self, _req: &Request, ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
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

    fn opendir(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        match ino {
            1 => reply.opened(1, flags as u32),
            2 => reply.error(EPERM),
            _ => reply.error(ENOENT),
        }
    }

    fn access(&mut self, _req: &Request, ino: u64, _mask: i32, reply: ReplyEmpty) {
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
    let matches = command!()
        .arg(
            Arg::new("MOUNT")
                .help("path to the mounting point")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("OPTION")
                .help("mount options")
                .short('o')
                .long("option")
                .takes_value(true)
                .number_of_values(1)
                .multiple_occurrences(true),
        )
        .get_matches();

    let path = Path::new(matches.value_of("MOUNT").unwrap());

    let options: Vec<&OsStr> = matches
        .values_of_os("OPTION")
        .unwrap()
        .flat_map(|x| vec![OsStr::new("-o"), x])
        .collect();

    fuser::mount(NullFS, &path, &options).unwrap();
}
