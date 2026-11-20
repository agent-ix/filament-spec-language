// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporary filesystem view for legacy readers of published FSpec assets.

use tempfile::TempDir;

/// Materializes a prefix from the pinned FSpec package into process scratch.
/// Every byte enters through `published_files`; the repository is never read.
pub fn directory(prefix: &str) -> TempDir {
    let directory = tempfile::tempdir().expect("a temporary FSpec view is available");
    for (path, bytes) in filament_specification::published_files(prefix) {
        let target = directory.path().join(path);
        std::fs::create_dir_all(target.parent().expect("published file has a parent"))
            .expect("temporary FSpec directories are writable");
        std::fs::write(target, bytes).expect("temporary FSpec files are writable");
    }
    directory
}
