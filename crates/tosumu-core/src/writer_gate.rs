use std::path::{Path, PathBuf};
#[cfg(any(unix, windows))]
use std::sync::Arc;

use crate::error::Result;
#[cfg(any(unix, windows))]
use crate::error::TosumuError;

#[cfg(any(unix, windows))]
const ACQUIRE_OPERATION: &str = "acquiring database writer gate";

pub(crate) fn writer_lock_path(database_path: &Path) -> PathBuf {
    let mut lock_path = database_path.as_os_str().to_os_string();
    lock_path.push(".writer.lock");
    PathBuf::from(lock_path)
}

#[cfg(any(unix, windows))]
#[derive(Clone, Debug)]
pub(crate) struct WriterGuard {
    // All retained guards share one OS handle. The lock is therefore released
    // only after the final owner (including an offline maintenance owner) is
    // dropped.
    _file: Arc<std::fs::File>,
    lock_path: PathBuf,
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
            Ok(()) => Ok(Self {
                _file: Arc::new(file),
                lock_path,
            }),
            Err(TryLockError::WouldBlock) => Err(TosumuError::FileBusy {
                path: lock_path,
                operation: ACQUIRE_OPERATION,
            }),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub(crate) fn authorizes(&self, database_path: &Path) -> bool {
        self.lock_path == writer_lock_path(database_path)
    }
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Debug)]
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

    pub(crate) fn authorizes(&self, _database_path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::WriterGuard;
    #[cfg(any(unix, windows))]
    use crate::error::TosumuError;

    #[cfg(any(unix, windows))]
    #[test]
    fn retained_clone_keeps_writer_admission_until_last_owner_drops() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("retained.tsm");

        let first = WriterGuard::acquire(&database).unwrap();
        let retained = first.clone();
        drop(first);

        let error = WriterGuard::acquire(&database).unwrap_err();
        assert!(matches!(error, TosumuError::FileBusy { .. }));

        drop(retained);
        WriterGuard::acquire(&database).unwrap();
    }
}
