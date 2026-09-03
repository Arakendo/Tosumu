use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicationDurability {
    Confirmed,
}

/// Publication failures are split at the atomic rename boundary. Callers may
/// clean staging artifacts after `PrePublication`, but must never restore an
/// older source after `DurabilityUncertain`.
#[derive(Debug, Error)]
pub(crate) enum PublicationError {
    #[error("atomic VACUUM publication is unsupported on {platform}")]
    UnsupportedPlatform { platform: &'static str },

    #[error("VACUUM publication failed before replacement during {operation}: {source}")]
    PrePublication {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    #[error(
        "VACUUM replaced {destination:?}, but containing-directory durability is uncertain: {source}"
    )]
    DurabilityUncertain {
        destination: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub(crate) fn ensure_supported() -> Result<(), PublicationError> {
    #[cfg(unix)]
    {
        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err(PublicationError::UnsupportedPlatform {
            platform: std::env::consts::OS,
        })
    }
}

pub(crate) fn replace_database(
    staging: &Path,
    destination: &Path,
) -> Result<PublicationDurability, PublicationError> {
    ensure_supported()?;

    #[cfg(unix)]
    {
        replace_database_unix(staging, destination)
    }

    #[cfg(not(unix))]
    {
        let _ = (staging, destination);
        unreachable!("unsupported targets return before publication")
    }
}

#[cfg(unix)]
fn replace_database_unix(
    staging: &Path,
    destination: &Path,
) -> Result<PublicationDurability, PublicationError> {
    let staging_parent = canonical_parent(staging)?;
    let destination_parent = canonical_parent(destination)?;
    if staging_parent != destination_parent {
        return Err(pre_publication(
            "validating same-directory staging",
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "staging and destination are not siblings",
            ),
        ));
    }

    require_regular_file(staging, "validating staging database")?;
    require_regular_file(destination, "validating source database")?;

    let staging_file = std::fs::File::open(staging)
        .map_err(|source| pre_publication("opening staging database", source))?;
    staging_file
        .sync_all()
        .map_err(|source| pre_publication("synchronizing staging database", source))?;

    // Open before rename so an open/permission failure cannot occur after the
    // publication point.
    let parent_directory = std::fs::File::open(&destination_parent)
        .map_err(|source| pre_publication("opening containing directory", source))?;

    finish_publication(
        destination,
        || std::fs::rename(staging, destination),
        || parent_directory.sync_all(),
    )
}

fn finish_publication(
    destination: &Path,
    replace: impl FnOnce() -> io::Result<()>,
    sync_directory: impl FnOnce() -> io::Result<()>,
) -> Result<PublicationDurability, PublicationError> {
    replace().map_err(|source| PublicationError::PrePublication {
        operation: "atomically replacing source database",
        source,
    })?;

    sync_directory().map_err(|source| PublicationError::DurabilityUncertain {
        destination: destination.to_path_buf(),
        source,
    })?;

    Ok(PublicationDurability::Confirmed)
}

#[cfg(unix)]
fn canonical_parent(path: &Path) -> Result<PathBuf, PublicationError> {
    let parent = path.parent().ok_or_else(|| {
        pre_publication(
            "validating publication paths",
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "path has no containing directory",
            ),
        )
    })?;
    parent
        .canonicalize()
        .map_err(|source| pre_publication("resolving containing directory", source))
}

#[cfg(unix)]
fn require_regular_file(path: &Path, operation: &'static str) -> Result<(), PublicationError> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|source| pre_publication(operation, source))?;
    if metadata.file_type().is_file() {
        Ok(())
    } else {
        Err(pre_publication(
            operation,
            io::Error::new(io::ErrorKind::InvalidInput, "path is not a regular file"),
        ))
    }
}

#[cfg(unix)]
fn pre_publication(operation: &'static str, source: io::Error) -> PublicationError {
    PublicationError::PrePublication { operation, source }
}

#[cfg(test)]
mod tests {
    use super::{ensure_supported, finish_publication, replace_database, PublicationError};
    use std::cell::Cell;
    use std::io;
    use std::path::Path;

    #[test]
    fn replacement_failure_is_pre_publication_and_skips_directory_sync() {
        let sync_called = Cell::new(false);

        let error = finish_publication(
            Path::new("database.tsm"),
            || Err(io::Error::new(io::ErrorKind::PermissionDenied, "blocked")),
            || {
                sync_called.set(true);
                Ok(())
            },
        )
        .unwrap_err();

        assert!(matches!(error, PublicationError::PrePublication { .. }));
        assert!(!sync_called.get());
    }

    #[test]
    fn directory_sync_failure_is_post_publication_and_durability_uncertain() {
        let replaced = Cell::new(false);

        let error = finish_publication(
            Path::new("database.tsm"),
            || {
                replaced.set(true);
                Ok(())
            },
            || Err(io::Error::new(io::ErrorKind::Other, "sync failed")),
        )
        .unwrap_err();

        assert!(replaced.get());
        assert!(matches!(
            error,
            PublicationError::DurabilityUncertain { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unix_replacement_publishes_new_bytes_and_removes_staging_name() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("database.tsm");
        let staging = directory.path().join("database.vacuum.tmp");
        std::fs::write(&destination, b"old").unwrap();
        std::fs::write(&staging, b"new").unwrap();

        let durability = replace_database(&staging, &destination).unwrap();

        assert_eq!(durability, super::PublicationDurability::Confirmed);
        assert_eq!(std::fs::read(destination).unwrap(), b"new");
        assert!(!staging.exists());
    }

    #[cfg(unix)]
    #[test]
    fn unix_replacement_rejects_a_staging_file_in_another_directory() {
        let destination_directory = tempfile::tempdir().unwrap();
        let staging_directory = tempfile::tempdir().unwrap();
        let destination = destination_directory.path().join("database.tsm");
        let staging = staging_directory.path().join("database.vacuum.tmp");
        std::fs::write(&destination, b"old").unwrap();
        std::fs::write(&staging, b"new").unwrap();

        let error = replace_database(&staging, &destination).unwrap_err();

        assert!(matches!(error, PublicationError::PrePublication { .. }));
        assert_eq!(std::fs::read(destination).unwrap(), b"old");
        assert_eq!(std::fs::read(staging).unwrap(), b"new");
    }

    #[cfg(not(unix))]
    #[test]
    fn unsupported_platform_refuses_before_touching_either_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("database.tsm");
        let staging = directory.path().join("database.vacuum.tmp");
        std::fs::write(&destination, b"old").unwrap();
        std::fs::write(&staging, b"new").unwrap();

        let error = replace_database(&staging, &destination).unwrap_err();

        assert!(matches!(
            error,
            PublicationError::UnsupportedPlatform { .. }
        ));
        assert_eq!(std::fs::read(destination).unwrap(), b"old");
        assert_eq!(std::fs::read(staging).unwrap(), b"new");
        assert!(ensure_supported().is_err());
    }
}
