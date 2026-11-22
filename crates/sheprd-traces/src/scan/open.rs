//! Open only regular files beneath a session root. Directory handles prevent
//! a checked component from being replaced by a link before the file is opened.
use std::{
    fs::{self, File, Metadata},
    io,
    path::{Component, Path},
};

fn plain(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
    }
    #[cfg(not(windows))]
    {
        !metadata.file_type().is_symlink()
    }
}

pub(super) fn directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && plain(&metadata))
}

pub(super) fn file(root: &Path, path: &Path) -> io::Result<Option<File>> {
    if !super::session_file(root, path) {
        return Ok(None);
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return Ok(None);
    };
    open(root, relative)
}

#[cfg(unix)]
fn open(root: &Path, relative: &Path) -> io::Result<Option<File>> {
    use rustix::{
        fs::{Mode, OFlags, open, openat},
        io::Errno,
    };
    let checked = |result: rustix::io::Result<std::os::fd::OwnedFd>| match result {
        Ok(fd) => Ok(Some(fd)),
        Err(Errno::LOOP | Errno::NOTDIR | Errno::NOENT) => Ok(None),
        Err(error) => Err(io::Error::from(error)),
    };
    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW;
    let Some(mut directory) = checked(open(root, flags | OFlags::DIRECTORY, Mode::empty()))? else {
        return Ok(None);
    };
    let mut parts = relative.components().peekable();
    while let Some(Component::Normal(name)) = parts.next() {
        let leaf = parts.peek().is_none();
        // A raced-in FIFO must not block the watcher before its type is checked.
        let flags = flags
            | if leaf {
                OFlags::NONBLOCK
            } else {
                OFlags::DIRECTORY
            };
        let Some(fd) = checked(openat(&directory, name, flags, Mode::empty()))? else {
            return Ok(None);
        };
        if leaf {
            let file = File::from(fd);
            return Ok(file.metadata()?.is_file().then_some(file));
        }
        directory = fd;
    }
    Ok(None)
}

#[cfg(windows)]
fn open(root: &Path, relative: &Path) -> io::Result<Option<File>> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let open = |path: &Path, leaf| {
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(
                FILE_SHARE_READ | FILE_SHARE_WRITE | if leaf { FILE_SHARE_DELETE } else { 0 },
            )
            .open(path)
    };
    let root_handle = open(root, false)?;
    let metadata = root_handle.metadata()?;
    if !metadata.is_dir() || !plain(&metadata) {
        return Ok(None);
    }
    // Keep each directory open without FILE_SHARE_DELETE until the leaf is
    // bound to a handle, so an ancestor cannot be swapped for a junction.
    let mut held = vec![root_handle];
    let mut path = root.to_owned();
    let mut parts = relative.components().peekable();
    while let Some(Component::Normal(name)) = parts.next() {
        path.push(name);
        let leaf = parts.peek().is_none();
        let handle = open(&path, leaf)?;
        let metadata = handle.metadata()?;
        if !plain(&metadata) {
            return Ok(None);
        }
        if leaf {
            return Ok(metadata.is_file().then_some(handle));
        }
        if !metadata.is_dir() {
            return Ok(None);
        }
        held.push(handle);
    }
    Ok(None)
}

#[cfg(not(any(unix, windows)))]
fn open(_: &Path, _: &Path) -> io::Result<Option<File>> {
    Err(io::ErrorKind::Unsupported.into())
}
