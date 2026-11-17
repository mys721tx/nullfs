//! A null filesystem: a root directory holding a single file, `null`, that
//! discards everything written to it and always reads back empty.

use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

use fuser::{
    AccessFlags, BsdFileFlags, FileAttr, FileHandle, FileType, Filesystem, Generation, INodeNo,
    InitFlags, KernelConfig, LockOwner, OpenFlags, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request, TimeOrNow,
    WriteFlags,
};

pub mod ops;
mod options;

pub use options::{parse_mount_option, parse_options};

/// Inode of the mount root.
pub const ROOT_INO: INodeNo = INodeNo::ROOT;
/// Inode of `/null`.
pub const NULL_INO: INodeNo = INodeNo(2);
/// Name of the only file in the root directory.
pub const NULL_NAME: &str = "null";

const GENERATION: Generation = Generation(0);

const TTL: Duration = Duration::from_secs(1);

/// Attributes of the root directory.
pub const DIR_ATTR: FileAttr = FileAttr {
    ino: ROOT_INO,
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

/// Attributes of `/null`.
pub const NULL_ATTR: FileAttr = FileAttr {
    ino: NULL_INO,
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

/// A negative entry: inode 0 tells the kernel the name does not exist, and
/// the TTL lets it cache that answer. Only the inode is read.
const NEGATIVE_ATTR: FileAttr = FileAttr {
    ino: INodeNo(0),
    ..NULL_ATTR
};

/// The FUSE filesystem. All decisions live in [`ops`]; this impl only replies.
pub struct NullFS;

impl Filesystem for NullFS {
    fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> std::io::Result<()> {
        // `/null` is opened with direct I/O, which normally makes shared
        // `mmap` fail. Ask the kernel (Linux 6.6+) to allow it anyway; older
        // kernels lack the capability and keep the stricter behaviour.
        let _ = config.add_capabilities(InitFlags::FUSE_DIRECT_IO_ALLOW_MMAP);
        Ok(())
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        match ops::lookup(parent, name) {
            Ok(ops::Entry::Found(attr)) => reply.entry(&TTL, attr, GENERATION),
            Ok(ops::Entry::Absent) => reply.entry(&TTL, &NEGATIVE_ATTR, GENERATION),
            Err(e) => reply.error(e),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match ops::attr(ino) {
            Ok(attr) => reply.attr(&TTL, attr),
            Err(e) => reply.error(e),
        }
    }

    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        match ops::attr(ino) {
            Ok(attr) => reply.attr(&TTL, attr),
            Err(e) => reply.error(e),
        }
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _offset: u64,
        _size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        match ops::read(ino) {
            Ok(data) => reply.data(data),
            Err(e) => reply.error(e),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        match ops::readdir(ino, offset) {
            Ok(entries) => {
                for (ino, next, kind, name) in entries {
                    if reply.add(ino, next, kind, name) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(e) => reply.error(e),
        }
    }

    fn write(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        match ops::write(ino, data) {
            Ok(n) => reply.written(n),
            Err(e) => reply.error(e),
        }
    }

    fn create(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        match ops::create(parent, name) {
            Ok(attr) => reply.created(
                &TTL,
                attr,
                GENERATION,
                FileHandle(NULL_INO.0),
                ops::NULL_OPEN_FLAGS,
            ),
            Err(e) => reply.error(e),
        }
    }

    fn mknod(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        match ops::create(parent, name) {
            Ok(attr) => reply.entry(&TTL, attr, GENERATION),
            Err(e) => reply.error(e),
        }
    }

    fn flush(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _lock_owner: LockOwner,
        reply: ReplyEmpty,
    ) {
        reply_empty(reply, ops::file_op(ino));
    }

    fn release(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply_empty(reply, ops::file_op(ino));
    }

    fn fsync(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply_empty(reply, ops::file_op(ino));
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match ops::open(ino) {
            Ok((fh, flags)) => reply.opened(fh, flags),
            Err(e) => reply.error(e),
        }
    }

    fn releasedir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        reply: ReplyEmpty,
    ) {
        reply_empty(reply, ops::dir_op(ino));
    }

    fn fsyncdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply_empty(reply, ops::dir_op(ino));
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match ops::opendir(ino) {
            Ok((fh, flags)) => reply.opened(fh, flags),
            Err(e) => reply.error(e),
        }
    }

    fn access(&self, _req: &Request, ino: INodeNo, _mask: AccessFlags, reply: ReplyEmpty) {
        reply_empty(reply, ops::access(ino));
    }

    fn getxattr(&self, _req: &Request, ino: INodeNo, _name: &OsStr, size: u32, reply: ReplyXattr) {
        match ops::getxattr(ino, size) {
            Ok(n) => reply.size(n),
            Err(e) => reply.error(e),
        }
    }
}

fn reply_empty(reply: ReplyEmpty, result: Result<(), fuser::Errno>) {
    match result {
        Ok(()) => reply.ok(),
        Err(e) => reply.error(e),
    }
}
