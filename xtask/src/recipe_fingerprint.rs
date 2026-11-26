// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fingerprint the xtask recipe sources and workspace inputs that produce the
//! merge-gate recipe executable.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FIXED_INPUTS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "xtask/Cargo.toml",
    "xtask/build.rs",
    "rust-toolchain.toml",
    ".cargo/config.toml",
    ".cargo/config",
];

pub fn fingerprint(workspace: &Path) -> io::Result<String> {
    let mut inputs = FIXED_INPUTS
        .iter()
        .map(|path| PathBuf::from(path))
        .collect::<Vec<_>>();
    collect_rust_sources(&workspace.join("xtask/src"), workspace, &mut inputs)?;
    inputs.sort();
    let mut hash = 0xcbf29ce484222325_u64;
    for relative in inputs {
        let path = workspace.join(&relative);
        if !path.is_file() {
            continue;
        }
        for byte in relative.to_string_lossy().bytes().chain([0]) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        for byte in fs::read(path)? {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        hash = (hash ^ 0xff).wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

fn collect_rust_sources(
    directory: &Path,
    workspace: &Path,
    output: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, workspace, output)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path
                .strip_prefix(workspace)
                .expect("source is under workspace");
            output.push(relative.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fingerprint;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[ix_trace_rs::trace("NFR-142-AC-2")]
    fn fingerprint_changes_when_xtask_recipe_source_changes() {
        let workspace = tempdir().expect("workspace");
        fs::create_dir_all(workspace.path().join("xtask/src")).expect("src");
        fs::write(workspace.path().join("xtask/src/main.rs"), "fn main() {}\n").expect("source");
        let original = fingerprint(workspace.path()).expect("fingerprint");
        fs::write(
            workspace.path().join("xtask/src/main.rs"),
            "fn main() { }\n",
        )
        .expect("changed source");
        assert_ne!(
            original,
            fingerprint(workspace.path()).expect("new fingerprint")
        );
    }

    #[test]
    #[ix_trace_rs::trace("NFR-142-AC-2")]
    fn fingerprint_changes_when_cargo_lock_changes() {
        let workspace = tempdir().expect("workspace");
        fs::create_dir_all(workspace.path().join("xtask/src")).expect("src");
        fs::write(workspace.path().join("Cargo.lock"), "version = 3\n").expect("lock");
        let original = fingerprint(workspace.path()).expect("fingerprint");
        fs::write(workspace.path().join("Cargo.lock"), "version = 4\n").expect("new lock");
        assert_ne!(
            original,
            fingerprint(workspace.path()).expect("new fingerprint")
        );
    }
}
