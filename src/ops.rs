//! Pure, side-effect-free implementations of every filesystem operation.
//!
//! The [`fuser::Filesystem`] impl in the crate root only forwards to these
//! functions and turns their results into replies. Keeping the decisions here
//! lets them be unit tested and benchmarked without a FUSE mount.

use std::ffi::OsStr;

use fuser::{Errno, FileAttr, FileHandle, FileType, FopenFlags, INodeNo};

use crate::{DIR_ATTR, NULL_ATTR, NULL_INO, NULL_NAME, ROOT_INO};

/// Flags for every open handle of `/null`.
///
/// Direct I/O bypasses the kernel page cache: writes are handed to nullfs
/// straight from the caller's buffer in requests of up to the negotiated
/// maximum size, instead of being copied into the cache and sent in smaller
/// pieces. Nothing is ever cached for `/null`, so no caching is lost.
///
/// No-flush tells the kernel (Linux 5.16+) not to send a `flush` request on
/// every `close()`; nullfs never has anything to flush. Older kernels ignore
/// the flag and still send `flush`, which [`file_op`] answers with success.
pub const NULL_OPEN_FLAGS: FopenFlags =
    FopenFlags::FOPEN_DIRECT_IO.union(FopenFlags::FOPEN_NOFLUSH);

/// Result of a successful `lookup`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// The name exists with these attributes.
    Found(&'static FileAttr),
    /// The name does not exist. Replied as a negative entry so the kernel
    /// caches the miss instead of asking again on every access.
    Absent,
}

/// A directory entry as handed to `ReplyDirectory::add`:
/// `(inode, offset of the next entry, kind, name)`.
pub type DirEntry = (INodeNo, u64, FileType, &'static str);

const ENTRIES: [(INodeNo, FileType, &str); 3] = [
    (ROOT_INO, FileType::Directory, "."),
    (ROOT_INO, FileType::Directory, ".."),
    (NULL_INO, FileType::RegularFile, NULL_NAME),
];

fn is_null_entry(parent: INodeNo, name: &OsStr) -> bool {
    parent == ROOT_INO && name == NULL_NAME
}

/// `lookup`: only `/null` exists. Other names in the root are cacheable
/// misses; lookups under anything but the root are errors.
pub fn lookup(parent: INodeNo, name: &OsStr) -> Result<Entry, Errno> {
    if parent != ROOT_INO {
        Err(Errno::ENOENT)
    } else if name == NULL_NAME {
        Ok(Entry::Found(&NULL_ATTR))
    } else {
        Ok(Entry::Absent)
    }
}

/// `getattr` and `setattr`: attributes are fixed and never change.
pub fn attr(ino: INodeNo) -> Result<&'static FileAttr, Errno> {
    match ino {
        ROOT_INO => Ok(&DIR_ATTR),
        NULL_INO => Ok(&NULL_ATTR),
        _ => Err(Errno::ENOENT),
    }
}

/// `read`: the null file is always empty.
pub fn read(ino: INodeNo) -> Result<&'static [u8], Errno> {
    if ino == NULL_INO {
        Ok(b"")
    } else {
        Err(Errno::ENOENT)
    }
}

/// `readdir`: the entries of the root directory, starting after `offset`.
pub fn readdir(ino: INodeNo, offset: u64) -> Result<impl Iterator<Item = DirEntry>, Errno> {
    if ino != ROOT_INO {
        return Err(Errno::ENOENT);
    }

    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    Ok(ENTRIES
        .into_iter()
        .enumerate()
        .skip(skip)
        // i + 1 is the offset of the next entry
        .map(|(i, (ino, kind, name))| (ino, (i + 1) as u64, kind, name)))
}

/// `write`: accept and discard everything written to the null file.
pub fn write(ino: INodeNo, data: &[u8]) -> Result<u32, Errno> {
    if ino == NULL_INO {
        Ok(data.len() as u32)
    } else {
        Err(Errno::ENOENT)
    }
}

/// `create` and `mknod`: only `/null` may be "created", and it already exists.
pub fn create(parent: INodeNo, name: &OsStr) -> Result<&'static FileAttr, Errno> {
    if is_null_entry(parent, name) {
        Ok(&NULL_ATTR)
    } else {
        Err(Errno::EPERM)
    }
}

/// `flush`, `release` and `fsync`: file-only operations.
pub fn file_op(ino: INodeNo) -> Result<(), Errno> {
    match ino {
        ROOT_INO => Err(Errno::EPERM),
        NULL_INO => Ok(()),
        _ => Err(Errno::ENOENT),
    }
}

/// `releasedir` and `fsyncdir`: directory-only operations.
pub fn dir_op(ino: INodeNo) -> Result<(), Errno> {
    match ino {
        ROOT_INO => Ok(()),
        NULL_INO => Err(Errno::EPERM),
        _ => Err(Errno::ENOENT),
    }
}

/// `open`: only the null file can be opened as a file.
pub fn open(ino: INodeNo) -> Result<(FileHandle, FopenFlags), Errno> {
    file_op(ino).map(|()| (FileHandle(NULL_INO.0), NULL_OPEN_FLAGS))
}

/// Flags for every open handle of the root directory.
///
/// Cache-dir lets the kernel keep the entries `readdir` returned and answer
/// later listings itself; keep-cache stops each new `opendir` from throwing
/// that cache away. The root's contents never change, so it never goes stale.
pub const DIR_OPEN_FLAGS: FopenFlags =
    FopenFlags::FOPEN_CACHE_DIR.union(FopenFlags::FOPEN_KEEP_CACHE);

/// `opendir`: only the root can be opened as a directory.
pub fn opendir(ino: INodeNo) -> Result<(FileHandle, FopenFlags), Errno> {
    dir_op(ino).map(|()| (FileHandle(ROOT_INO.0), DIR_OPEN_FLAGS))
}

/// `access`: everything that exists is accessible.
pub fn access(ino: INodeNo) -> Result<(), Errno> {
    attr(ino).map(|_| ())
}

/// `getxattr`: there are no extended attributes. A size probe (`size == 0`)
/// reports an empty value; any real read gets `ERANGE`.
pub fn getxattr(ino: INodeNo, size: u32) -> Result<u32, Errno> {
    if size != 0 {
        return Err(Errno::ERANGE);
    }
    attr(ino).map(|_| 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: INodeNo = INodeNo(3);

    fn name(s: &str) -> &OsStr {
        OsStr::new(s)
    }

    #[test]
    fn lookup_finds_only_null_in_root() {
        assert_eq!(lookup(ROOT_INO, name("null")), Ok(Entry::Found(&NULL_ATTR)));
        assert_eq!(lookup(ROOT_INO, name("other")), Ok(Entry::Absent));
        assert_eq!(lookup(ROOT_INO, name("")), Ok(Entry::Absent));
        assert_eq!(lookup(ROOT_INO, name("NULL")), Ok(Entry::Absent));
        assert_eq!(lookup(NULL_INO, name("null")), Err(Errno::ENOENT));
        assert_eq!(lookup(MISSING, name("null")), Err(Errno::ENOENT));
    }

    #[test]
    fn attr_matches_inode() {
        let root = attr(ROOT_INO).unwrap();
        assert_eq!(root.ino, ROOT_INO);
        assert_eq!(root.kind, FileType::Directory);
        assert_eq!(root.perm, 0o777);
        assert_eq!(root.nlink, 2);

        let null = attr(NULL_INO).unwrap();
        assert_eq!(null.ino, NULL_INO);
        assert_eq!(null.kind, FileType::RegularFile);
        assert_eq!(null.perm, 0o666);
        assert_eq!(null.size, 0);

        assert_eq!(attr(MISSING), Err(Errno::ENOENT));
        assert_eq!(attr(INodeNo(u64::MAX)), Err(Errno::ENOENT));
    }

    #[test]
    fn read_is_always_empty() {
        assert_eq!(read(NULL_INO), Ok(&b""[..]));
        assert_eq!(read(ROOT_INO), Err(Errno::ENOENT));
        assert_eq!(read(MISSING), Err(Errno::ENOENT));
    }

    #[test]
    fn readdir_lists_all_entries_from_zero() {
        let entries: Vec<_> = readdir(ROOT_INO, 0).unwrap().collect();
        assert_eq!(
            entries,
            [
                (ROOT_INO, 1, FileType::Directory, "."),
                (ROOT_INO, 2, FileType::Directory, ".."),
                (NULL_INO, 3, FileType::RegularFile, "null"),
            ]
        );
    }

    #[test]
    fn readdir_resumes_from_offset() {
        // Each entry's offset must resume right after that entry.
        let all: Vec<_> = readdir(ROOT_INO, 0).unwrap().collect();
        for (i, &(_, next, _, _)) in all.iter().enumerate() {
            let rest: Vec<_> = readdir(ROOT_INO, next).unwrap().collect();
            assert_eq!(rest, all[i + 1..]);
        }
    }

    #[test]
    fn readdir_past_end_is_empty() {
        assert_eq!(readdir(ROOT_INO, 3).unwrap().count(), 0);
        assert_eq!(readdir(ROOT_INO, 100).unwrap().count(), 0);
        assert_eq!(readdir(ROOT_INO, u64::MAX).unwrap().count(), 0);
    }

    #[test]
    fn readdir_rejects_non_directories() {
        assert!(matches!(readdir(NULL_INO, 0), Err(Errno::ENOENT)));
        assert!(matches!(readdir(MISSING, 0), Err(Errno::ENOENT)));
    }

    #[test]
    fn write_reports_full_length() {
        assert_eq!(write(NULL_INO, b""), Ok(0));
        assert_eq!(write(NULL_INO, b"hello"), Ok(5));
        assert_eq!(write(NULL_INO, &[0; 128 * 1024]), Ok(128 * 1024));
        assert_eq!(write(ROOT_INO, b"x"), Err(Errno::ENOENT));
        assert_eq!(write(MISSING, b"x"), Err(Errno::ENOENT));
    }

    #[test]
    fn create_only_allows_null() {
        assert_eq!(create(ROOT_INO, name("null")), Ok(&NULL_ATTR));
        assert_eq!(create(ROOT_INO, name("new")), Err(Errno::EPERM));
        assert_eq!(create(NULL_INO, name("null")), Err(Errno::EPERM));
        assert_eq!(create(MISSING, name("x")), Err(Errno::EPERM));
    }

    #[test]
    fn file_and_dir_ops_are_mirrored() {
        assert_eq!(file_op(NULL_INO), Ok(()));
        assert_eq!(file_op(ROOT_INO), Err(Errno::EPERM));
        assert_eq!(file_op(MISSING), Err(Errno::ENOENT));

        assert_eq!(dir_op(ROOT_INO), Ok(()));
        assert_eq!(dir_op(NULL_INO), Err(Errno::EPERM));
        assert_eq!(dir_op(MISSING), Err(Errno::ENOENT));
    }

    #[test]
    fn open_and_opendir_return_inode_handles() {
        assert_eq!(open(NULL_INO), Ok((FileHandle(2), NULL_OPEN_FLAGS)));
        assert_eq!(open(ROOT_INO), Err(Errno::EPERM));
        assert_eq!(open(MISSING), Err(Errno::ENOENT));

        assert_eq!(opendir(ROOT_INO), Ok((FileHandle(1), DIR_OPEN_FLAGS)));
        assert_eq!(opendir(NULL_INO), Err(Errno::EPERM));
        assert_eq!(opendir(MISSING), Err(Errno::ENOENT));
    }

    #[test]
    fn root_dir_is_cached() {
        assert!(DIR_OPEN_FLAGS.contains(FopenFlags::FOPEN_CACHE_DIR));
        assert!(DIR_OPEN_FLAGS.contains(FopenFlags::FOPEN_KEEP_CACHE));
    }

    #[test]
    fn null_open_flags() {
        assert!(NULL_OPEN_FLAGS.contains(FopenFlags::FOPEN_DIRECT_IO));
        assert!(NULL_OPEN_FLAGS.contains(FopenFlags::FOPEN_NOFLUSH));
        // Keeping the cache would contradict bypassing it.
        assert!(!NULL_OPEN_FLAGS.contains(FopenFlags::FOPEN_KEEP_CACHE));
    }

    #[test]
    fn access_allows_existing_inodes() {
        assert_eq!(access(ROOT_INO), Ok(()));
        assert_eq!(access(NULL_INO), Ok(()));
        assert_eq!(access(MISSING), Err(Errno::ENOENT));
    }

    #[test]
    fn getxattr_has_no_attributes() {
        assert_eq!(getxattr(ROOT_INO, 0), Ok(0));
        assert_eq!(getxattr(NULL_INO, 0), Ok(0));
        assert_eq!(getxattr(MISSING, 0), Err(Errno::ENOENT));
        assert_eq!(getxattr(ROOT_INO, 64), Err(Errno::ERANGE));
        assert_eq!(getxattr(NULL_INO, 1), Err(Errno::ERANGE));
        assert_eq!(getxattr(MISSING, 1), Err(Errno::ERANGE));
    }
}
