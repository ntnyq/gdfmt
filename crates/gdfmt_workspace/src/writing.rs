use crate::{WorkspaceError, io_error};
use std::{
    fs::{self, Metadata},
    io::{self, Read, Write},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteStage {
    Create,
    Write,
    Permissions,
    Sync,
    Replace,
}

/// Same-directory atomic replacement, guarded against stale bytes, identity and
/// metadata changes. Permissions are preserved; symlinks and Unix hardlinks are refused.
/// The final check is best-effort: portable filesystem APIs cannot lock out a
/// concurrent writer between the last check and rename. This is not a multi-file
/// transaction or a guarantee of directory-entry durability after power loss.
pub fn write_source(path: &Path, expected: &str, output: &str) -> Result<(), WorkspaceError> {
    replace_source(path, expected, output, |_| Ok(()))
}

fn same_metadata(before: &Metadata, after: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.nlink() != after.nlink()
        {
            return false;
        }
    }
    before.is_file()
        && after.is_file()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && before.created().ok() == after.created().ok()
        && before.permissions() == after.permissions()
}

fn replace_source(
    path: &Path,
    expected: &str,
    output: &str,
    mut before: impl FnMut(WriteStage) -> io::Result<()>,
) -> Result<(), WorkspaceError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if !metadata.is_file() {
        return Err(WorkspaceError::Input(format!(
            "{}: expected a regular file for replacement",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(WorkspaceError::Input(format!(
                "{}: refusing to replace a hard-linked source",
                path.display()
            )));
        }
    }
    if metadata.permissions().readonly() {
        return Err(io_error(
            path,
            io::Error::new(io::ErrorKind::PermissionDenied, "source is read-only"),
        ));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    before(WriteStage::Create).map_err(|error| io_error(path, error))?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| io_error(path, error))?;
    before(WriteStage::Write).map_err(|error| io_error(path, error))?;
    temporary
        .write_all(output.as_bytes())
        .map_err(|error| io_error(path, error))?;
    before(WriteStage::Permissions).map_err(|error| io_error(path, error))?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())
        .map_err(|error| io_error(path, error))?;
    before(WriteStage::Sync).map_err(|error| io_error(path, error))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| io_error(path, error))?;
    before(WriteStage::Replace).map_err(|error| io_error(path, error))?;
    let stale = || {
        WorkspaceError::Input(format!(
            "{}: source changed before replacement",
            path.display()
        ))
    };
    let current = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if !same_metadata(&metadata, &current) {
        return Err(stale());
    }
    let source = fs::File::open(path).map_err(|error| io_error(path, error))?;
    if !same_metadata(
        &metadata,
        &source.metadata().map_err(|error| io_error(path, error))?,
    ) {
        return Err(stale());
    }
    let mut bytes = Vec::new();
    source
        .take(expected.len() as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    if bytes != expected.as_bytes()
        || !same_metadata(
            &metadata,
            &fs::symlink_metadata(path).map_err(|error| io_error(path, error))?,
        )
    {
        return Err(stale());
    }
    temporary
        .persist(path)
        .map_err(|error| io_error(path, error.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_io_failures_never_truncate_and_clean_temporary_files() {
        for fail in [
            WriteStage::Create,
            WriteStage::Write,
            WriteStage::Permissions,
            WriteStage::Sync,
            WriteStage::Replace,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("case.gd");
            fs::write(&path, "original").unwrap();
            let permissions = fs::metadata(&path).unwrap().permissions();
            let result = replace_source(&path, "original", "output", |stage| {
                if stage == fail {
                    Err(io::Error::other("injected I/O failure"))
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err(), "{fail:?}");
            assert_eq!(fs::read_to_string(&path).unwrap(), "original");
            assert_eq!(fs::metadata(&path).unwrap().permissions(), permissions);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn external_changes_during_preparation_are_not_overwritten() {
        for mutation in ["edit", "delete", "replace", "readonly"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("case.gd");
            fs::write(&path, "original").unwrap();
            let permissions = fs::metadata(&path).unwrap().permissions();
            let result = replace_source(&path, "original", "output", |stage| {
                if stage == WriteStage::Replace {
                    match mutation {
                        "edit" => fs::write(&path, "external")?,
                        "delete" => fs::remove_file(&path)?,
                        "replace" => {
                            let other = directory.path().join("other");
                            fs::write(&other, "original")?;
                            fs::remove_file(&path)?;
                            fs::rename(other, &path)?;
                        }
                        _ => {
                            let mut readonly = permissions.clone();
                            readonly.set_readonly(true);
                            fs::set_permissions(&path, readonly)?;
                        }
                    }
                }
                Ok(())
            });
            assert!(result.is_err(), "{mutation}");
            if mutation == "delete" {
                assert!(!path.exists());
            } else {
                assert_eq!(
                    fs::read_to_string(&path).unwrap(),
                    if mutation == "edit" {
                        "external"
                    } else {
                        "original"
                    }
                );
                fs::set_permissions(&path, permissions).unwrap();
            }
            assert_eq!(
                fs::read_dir(directory.path()).unwrap().count(),
                usize::from(mutation != "delete")
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn concurrent_symlink_swap_keeps_the_target_and_link_intact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("case.gd");
        let target = directory.path().join("target.gd");
        fs::write(&path, "original").unwrap();
        fs::write(&target, "external").unwrap();
        assert!(
            replace_source(&path, "original", "output", |stage| {
                if stage == WriteStage::Replace {
                    fs::remove_file(&path)?;
                    std::os::unix::fs::symlink(&target, &path)?;
                }
                Ok(())
            })
            .is_err()
        );
        assert!(fs::symlink_metadata(&path).unwrap().is_symlink());
        assert_eq!(fs::read_to_string(target).unwrap(), "external");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn interrupted_preparation_keeps_original_and_later_writes_can_recover() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("case.gd");
        fs::write(&path, "original").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "writing::tests::interruption_worker",
            ])
            .env("GDFMT_WRITE_INTERRUPTION_PATH", &path)
            .output()
            .unwrap()
            .status;
        assert_eq!(status.code(), Some(77));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        // Forced process termination skips destructors. An orphan temp is harmless.
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
        write_source(&path, "original", "recovered").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "recovered");
    }

    #[test]
    #[ignore = "launched by the interruption parent test"]
    fn interruption_worker() {
        let path =
            std::env::var_os("GDFMT_WRITE_INTERRUPTION_PATH").expect("parent provides a path");
        let _ = replace_source(Path::new(&path), "original", "prepared", |stage| {
            if stage == WriteStage::Sync {
                std::process::exit(77);
            }
            Ok(())
        });
        panic!("worker must terminate before replacement");
    }
}
