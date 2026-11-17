use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use clap::{Arg, ArgAction, command, value_parser};

use fuser::{
    AccessFlags, BsdFileFlags, Config, Errno, FileAttr, FileHandle, FileType, Filesystem,
    FopenFlags, Generation, INodeNo, LockOwner, MountOption, OpenFlags, ReplyAttr, ReplyCreate,
    ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, ReplyXattr, Request,
    SessionACL, TimeOrNow, WriteFlags,
};

const ROOT_INO: INodeNo = INodeNo::ROOT;
const NULL_INO: INodeNo = INodeNo(2);

const GENERATION: Generation = Generation(0);

const TTL: Duration = Duration::from_secs(1);

const DIR_ATTR: FileAttr = FileAttr {
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

const NULL_ATTR: FileAttr = FileAttr {
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

struct NullFS;

impl Filesystem for NullFS {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        if parent == ROOT_INO && name == "null" {
            reply.entry(&TTL, &NULL_ATTR, GENERATION);
        } else {
            reply.error(Errno::ENOENT);
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match ino.0 {
            1 => reply.attr(&TTL, &DIR_ATTR),
            2 => reply.attr(&TTL, &NULL_ATTR),
            _ => reply.error(Errno::ENOENT),
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
        match ino.0 {
            1 => reply.attr(&TTL, &DIR_ATTR),
            2 => reply.attr(&TTL, &NULL_ATTR),
            _ => reply.error(Errno::ENOENT),
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
        if ino == NULL_INO {
            reply.data(b"");
        } else {
            reply.error(Errno::ENOENT);
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
        if ino != ROOT_INO {
            reply.error(Errno::ENOENT);
            return;
        }

        let entries = [
            (ROOT_INO, FileType::Directory, "."),
            (ROOT_INO, FileType::Directory, ".."),
            (NULL_INO, FileType::RegularFile, "null"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as u64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
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
        if ino != NULL_INO {
            reply.error(Errno::ENOENT);
            return;
        }

        reply.written(data.len() as u32)
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
        if parent == ROOT_INO && name == "null" {
            reply.created(
                &TTL,
                &NULL_ATTR,
                GENERATION,
                FileHandle(NULL_INO.0),
                FopenFlags::empty(),
            );
        } else {
            reply.error(Errno::EPERM);
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
        if parent == ROOT_INO && name == "null" {
            reply.entry(&TTL, &NULL_ATTR, GENERATION);
        } else {
            reply.error(Errno::EPERM);
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
        match ino.0 {
            1 => reply.error(Errno::EPERM),
            2 => reply.ok(),
            _ => reply.error(Errno::ENOENT),
        }
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
        match ino.0 {
            1 => reply.error(Errno::EPERM),
            2 => reply.ok(),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn fsync(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        match ino.0 {
            1 => reply.error(Errno::EPERM),
            2 => reply.ok(),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match ino.0 {
            1 => reply.error(Errno::EPERM),
            2 => reply.opened(FileHandle(NULL_INO.0), FopenFlags::empty()),
            _ => reply.error(Errno::ENOENT),
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
        match ino.0 {
            1 => reply.ok(),
            2 => reply.error(Errno::EPERM),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn fsyncdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        match ino.0 {
            1 => reply.ok(),
            2 => reply.error(Errno::EPERM),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match ino.0 {
            1 => reply.opened(FileHandle(ROOT_INO.0), FopenFlags::empty()),
            2 => reply.error(Errno::EPERM),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn access(&self, _req: &Request, ino: INodeNo, _mask: AccessFlags, reply: ReplyEmpty) {
        match ino.0 {
            1 => reply.ok(),
            2 => reply.ok(),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn getxattr(&self, _req: &Request, ino: INodeNo, _name: &OsStr, size: u32, reply: ReplyXattr) {
        if size == 0 {
            match ino.0 {
                1 => reply.size(0),
                2 => reply.size(0),
                _ => reply.error(Errno::ENOENT),
            }
            return;
        }
        reply.error(Errno::ERANGE);
    }
}

/// Builds a session configuration from comma-separated `-o` mount options.
fn parse_options<'a>(options: impl Iterator<Item = &'a str>) -> Config {
    let mut config = Config::default();

    for option in options.flat_map(|x| x.split(',')).filter(|x| !x.is_empty()) {
        match option {
            "allow_other" => config.acl = SessionACL::All,
            "allow_root" => config.acl = SessionACL::RootAndOwner,
            _ => config.mount_options.push(parse_mount_option(option)),
        }
    }

    config
}

fn parse_mount_option(option: &str) -> MountOption {
    match option {
        "auto_unmount" => MountOption::AutoUnmount,
        "default_permissions" => MountOption::DefaultPermissions,
        "dev" => MountOption::Dev,
        "nodev" => MountOption::NoDev,
        "suid" => MountOption::Suid,
        "nosuid" => MountOption::NoSuid,
        "ro" => MountOption::RO,
        "rw" => MountOption::RW,
        "exec" => MountOption::Exec,
        "noexec" => MountOption::NoExec,
        "atime" => MountOption::Atime,
        "noatime" => MountOption::NoAtime,
        "dirsync" => MountOption::DirSync,
        "sync" => MountOption::Sync,
        "async" => MountOption::Async,
        _ => match option.split_once('=') {
            Some(("fsname", name)) => MountOption::FSName(name.into()),
            Some(("subtype", name)) => MountOption::Subtype(name.into()),
            _ => MountOption::CUSTOM(option.into()),
        },
    }
}

fn main() -> std::io::Result<()> {
    let matches = command!()
        .arg(
            Arg::new("MOUNT")
                .help("path to the mounting point")
                .index(1)
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("OPTION")
                .help("mount options")
                .short('o')
                .long("option")
                .action(ArgAction::Append),
        )
        .get_matches();

    let path = matches
        .get_one::<PathBuf>("MOUNT")
        .expect("MOUNT is a required argument");

    let config = parse_options(
        matches
            .get_many::<String>("OPTION")
            .unwrap_or_default()
            .map(String::as_str),
    );

    fuser::mount(NullFS, path, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_separated_options() {
        let config =
            parse_options(["ro,nosuid", "fsname=null", "allow_other", "foo=bar"].into_iter());

        assert_eq!(
            config.mount_options,
            [
                MountOption::RO,
                MountOption::NoSuid,
                MountOption::FSName("null".into()),
                MountOption::CUSTOM("foo=bar".into()),
            ]
        );
        assert_eq!(config.acl, SessionACL::All);
    }

    #[test]
    fn no_options_yields_default_config() {
        assert_eq!(parse_options(std::iter::empty()), Config::default());
    }
}
