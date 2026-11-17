// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporary filesystem view for legacy readers of published FSpec assets.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Materializes a prefix from the pinned FSpec package into process scratch.
/// Every byte enters through `published_files`; the repository is never read.
pub fn directory(prefix: &str) -> PathBuf {
    static ROOTS: OnceLock<Mutex<std::collections::BTreeMap<String, tempfile::TempDir>>> =
        OnceLock::new();
    let roots = ROOTS.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()));
    let mut roots = roots
        .lock()
        .expect("published FSpec view lock is not poisoned");
    if !roots.contains_key(prefix) {
        let directory = tempfile::tempdir().expect("a temporary FSpec view is available");
        let root = directory.path();
        for (path, bytes) in filament_specification::published_files(prefix) {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().expect("published file has a parent"))
                .expect("temporary FSpec directories are writable");
            std::fs::write(target, bytes).expect("temporary FSpec files are writable");
        }
        roots.insert(prefix.to_owned(), directory);
    }
    roots
        .get(prefix)
        .expect("root was inserted")
        .path()
        .to_path_buf()
}
