//! Filesystem operations whose guarantees differ from the standard library.

use std::{io, path::Path};

/// Moves a file or directory atomically, refusing to replace any existing
/// destination. Both paths must be on the same filesystem. Unsupported
/// platforms or filesystems fail rather than fall back to an overwriting move.
pub fn rename_noreplace(source: &Path, target: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        renameat_with(CWD, source, CWD, target, RenameFlags::NOREPLACE)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        move_windows(source, target)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (source, target);
        Err(io::ErrorKind::Unsupported.into())
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn move_windows(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let wide = |path: &Path| -> io::Result<Vec<u16>> {
        let mut text: Vec<u16> = path.as_os_str().encode_wide().collect();
        if text.contains(&0) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        text.push(0);
        Ok(text)
    };
    let source = wide(source)?;
    let target = wide(target)?;
    // std::fs::rename can replace directories via FileRenameInfoEx. There
    // is no stable std no-replace move; use the existing Win32 binding with
    // neither REPLACE_EXISTING nor COPY_ALLOWED, preserving atomicity.
    // SAFETY: both buffers are NUL-terminated, contain no interior NUL, and
    // remain alive throughout this synchronous call, which retains neither.
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
