// SPDX-License-Identifier: AGPL-3.0-or-later
//! Access to published FSpec files for integration tests.

/// Exact bytes of a file published by the pinned FSpec package.
pub(crate) fn file(path: &str) -> &'static [u8] {
    filament_specification::published_file(path)
        .unwrap_or_else(|| panic!("FSpec does not publish {path}"))
}

/// UTF-8 text of a file published by the pinned FSpec package.
pub(crate) fn text(path: &str) -> &'static str {
    std::str::from_utf8(file(path)).unwrap_or_else(|error| panic!("{path} is not UTF-8: {error}"))
}

/// JSON value from a file published by the pinned FSpec package.
pub(crate) fn json(path: &str) -> serde_json::Value {
    serde_json::from_slice(file(path)).unwrap_or_else(|error| panic!("{path} is not JSON: {error}"))
}
