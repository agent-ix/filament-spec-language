// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embed a fingerprint of the sources and build inputs used by the recipe CLI.

#[path = "src/recipe_fingerprint.rs"]
mod recipe_fingerprint;

use std::path::Path;

fn main() {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets manifest dir");
    let workspace = Path::new(&manifest)
        .parent()
        .expect("xtask is one level under workspace root");
    let fingerprint = recipe_fingerprint::fingerprint(workspace)
        .expect("read xtask recipe source and build inputs");
    println!("cargo:rustc-env=XTASK_RECIPE_INPUT_FINGERPRINT={fingerprint}");
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "xtask/Cargo.toml",
        "xtask/build.rs",
        "rust-toolchain.toml",
        ".cargo/config.toml",
        ".cargo/config",
        "xtask/src",
    ] {
        println!("cargo:rerun-if-changed={}", workspace.join(path).display());
    }
}
