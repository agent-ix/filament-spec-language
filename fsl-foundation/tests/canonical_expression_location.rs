// SPDX-License-Identifier: AGPL-3.0-or-later

//! FR-273-AC-51 compile-resolution assertion for the shared evaluation interface.
use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
use ix_trace_rs::trace;
use std::process::Command;

#[test]
#[trace("TC-748", "FR-273-AC-51")]
fn evaluation_location_interface_resolves_from_semantic_value_owner() {
    fn resolved_types(_: Option<ExpressionLocation>, _: Option<ExpressionOwner>) {}

    resolved_types(None, None);

    let dependency_dir = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable directory")
        .to_owned();
    let semantic_value_rlib = std::fs::read_dir(&dependency_dir)
        .expect("dependency directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("libfilament_semantic_value-") && name.ends_with(".rlib")
                })
        })
        .expect("compiled filament-semantic-value rlib");
    let temp_dir = std::env::temp_dir().join(format!("fsl-fr273-ac51-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).expect("temporary compile-fail directory");

    for old_type in ["Location", "Origin"] {
        let source = temp_dir.join(format!("missing_{old_type}.rs"));
        std::fs::write(
            &source,
            format!("use filament_semantic_value::location::{old_type};\npub fn unresolved(_: {old_type}) {{}}\n"),
        )
        .expect("compile-fail source");
        let output = Command::new("rustc")
            .arg("--edition=2021")
            .arg("--crate-type=lib")
            .arg("--crate-name")
            .arg(format!("missing_{old_type}"))
            .arg(&source)
            .arg("--extern")
            .arg(format!(
                "filament_semantic_value={}",
                semantic_value_rlib.display()
            ))
            .arg("-L")
            .arg(format!("dependency={}", dependency_dir.display()))
            .arg("--out-dir")
            .arg(&temp_dir)
            .output()
            .expect("rustc compile-fail invocation");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "old type path {old_type} resolved"
        );
        assert!(
            stderr.contains("E0432") && stderr.contains(old_type),
            "expected unresolved import of {old_type}, got: {stderr}",
        );
    }

    std::fs::remove_dir_all(temp_dir).expect("remove temporary compile-fail directory");
}
