use std::path::{Path, PathBuf};

use crate::error::{Result, TosumuError};

const ACQUIRE_OPERATION: &str = "acquiring database writer gate";

pub(crate) fn writer_lock_path(database_path: &Path) -> PathBuf {
    let mut lock_path = database_path.as_os_str().to_os_string();
    lock_path.push(".writer.lock");
    PathBuf::from(lock_path)
}

#[cfg(any(unix, windows))]
pub(crate) struct WriterGuard {
    _file: std::fs::File,
}

#[cfg(any(unix, windows))]
impl WriterGuard {
    pub(crate) fn acquire(database_path: &Path) -> Result<Self> {
        use fs4::{FileExt, TryLockError};

        let lock_path = writer_lock_path(database_path);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;

        match FileExt::try_lock(&file) {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(TosumuError::FileBusy {
                path: lock_path,
                operation: ACQUIRE_OPERATION,
            }),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) struct WriterGuard;

#[cfg(not(any(unix, windows)))]
impl WriterGuard {
    pub(crate) fn acquire(_database_path: &Path) -> Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "writable Tosumu file storage is unsupported on this target",
        )
        .into())
    }
}
