//! FR-810: create the compile's output file without replacing what is there.

use super::{Result, RunCause};
use std::{
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

/// Distinguishes the temporary files of concurrent writes in one process.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Temporary names tried before the write reports the I/O failure.
const TEMPORARY_ATTEMPTS: usize = 64;

/// Create `path` holding `bytes`, never replacing what is there.
pub(super) fn write_new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    write_new_file_linking(path, bytes, |temporary, target| {
        std::fs::hard_link(temporary, target)
    })
}

/// [`write_new_file`] with the link step injected. The bytes go to a sibling
/// temporary file that `link` attaches to `path` and fails on when `path`
/// exists, so the target is untouched on every failure and never holds a
/// partial file. When `link` fails for another reason, such as a filesystem
/// without hard links, the bytes are written to a target this call creates
/// exclusively, and that target is removed again if the write fails.
fn write_new_file_linking(
    path: &Path,
    bytes: &[u8],
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<()> {
    let io = |error| RunCause::Io {
        path: path.to_path_buf(),
        error,
    };
    let exists = || RunCause::OutputExists {
        path: path.to_path_buf(),
    };
    if path.symlink_metadata().is_ok() {
        return Err(exists());
    }
    let name = path.file_name().ok_or_else(|| {
        io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the output path names no file",
        ))
    })?;
    let (temporary, mut file) = create_temporary(path, name).map_err(io)?;
    // This call created `temporary`, so it alone removes it, on every path.
    let staged = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let linked = staged.and_then(|()| link(&temporary, path));
    let outcome = match linked {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(exists()),
        Err(_) => write_exclusively(path, bytes).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                exists()
            } else {
                io(error)
            }
        }),
    };
    let _ = std::fs::remove_file(&temporary);
    outcome
}

/// Create a sibling temporary file no other call holds.
fn create_temporary(
    path: &Path,
    name: &std::ffi::OsStr,
) -> io::Result<(std::path::PathBuf, std::fs::File)> {
    let mut last = io::Error::from(io::ErrorKind::AlreadyExists);
    for _ in 0..TEMPORARY_ATTEMPTS {
        let mut temporary_name = std::ffi::OsString::from(".");
        temporary_name.push(name);
        temporary_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
        let temporary = path.with_file_name(temporary_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = error,
            Err(error) => return Err(error),
        }
    }
    Err(last)
}

/// Fallback: create `path` itself, fail if it exists, and remove it again when
/// the write fails, since this call created it.
fn write_exclusively(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    if written.is_err() {
        drop(file);
        let _ = std::fs::remove_file(path);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory_names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// FR-810: two writes to one output give one success, one `OutputExists`,
    /// the first bytes, and no temporary file left behind.
    #[test]
    #[ix_trace_rs::trace("TC-2297", "FR-810-AC-2")]
    fn a_second_write_to_the_same_output_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("out.json");
        write_new_file(&target, b"first").unwrap();
        let error = write_new_file(&target, b"second").unwrap_err();
        assert!(matches!(error, RunCause::OutputExists { .. }), "{error}");
        assert_eq!(std::fs::read(&target).unwrap(), b"first");
        assert_eq!(directory_names(directory.path()), ["out.json"]);
    }

    /// FR-810: concurrent writes in one process never share a temporary file,
    /// so exactly one wins and the others refuse.
    #[test]
    #[ix_trace_rs::trace("TC-2297", "FR-810-AC-2")]
    fn concurrent_writes_to_one_output_have_one_winner() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("out.json");
        let results: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| write_new_file(&target, b"bytes")))
                .collect();
            handles
                .into_iter()
                .map(|handle| match handle.join().unwrap() {
                    Ok(()) => true,
                    Err(RunCause::OutputExists { .. }) => false,
                    Err(other) => panic!("{other}"),
                })
                .collect()
        });
        assert_eq!(results.iter().filter(|won| **won).count(), 1);
        assert_eq!(std::fs::read(&target).unwrap(), b"bytes");
        assert_eq!(directory_names(directory.path()), ["out.json"]);
    }

    /// FR-810: a link step that fails for a reason other than the target
    /// existing falls back to an exclusive create of the target.
    #[test]
    #[ix_trace_rs::trace("TC-2297", "FR-810-AC-2")]
    fn a_failed_link_falls_back_to_an_exclusive_create() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("out.json");
        write_new_file_linking(&target, b"bytes", |_, _| {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        })
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"bytes");
        assert_eq!(directory_names(directory.path()), ["out.json"]);
    }

    /// FR-810: the fallback never replaces a target that appeared meanwhile,
    /// and leaves it and the directory otherwise untouched.
    #[test]
    #[ix_trace_rs::trace("TC-2297", "FR-810-AC-2")]
    fn the_fallback_refuses_a_target_that_appeared() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("out.json");
        let error = write_new_file_linking(&target, b"mine", |_, target| {
            std::fs::write(target, b"theirs").unwrap();
            Err(io::Error::from(io::ErrorKind::Unsupported))
        })
        .unwrap_err();
        assert!(matches!(error, RunCause::OutputExists { .. }), "{error}");
        assert_eq!(std::fs::read(&target).unwrap(), b"theirs");
        assert_eq!(directory_names(directory.path()), ["out.json"]);
    }
}
