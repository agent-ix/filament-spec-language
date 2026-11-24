// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315's downstream replay facade and opaque hyper-product identity.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn dependency(directory: &Path, prefix: &str) -> PathBuf {
    fs::read_dir(directory)
        .expect("the integration test dependency directory exists")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".rlib"))
        })
        .max_by_key(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .unwrap_or_else(|| panic!("the test build contains {prefix} rlib"))
}

fn compile_consumer(source: &str, with_serde: bool) -> Output {
    let deps = std::env::current_exe()
        .expect("the integration test executable has a path")
        .parent()
        .expect("the test executable has a dependency directory")
        .to_path_buf();
    let replay = dependency(&deps, "libfsl_replay-");
    let temp = tempfile::tempdir().expect("a private compile fixture directory");
    let source_path = temp.path().join("consumer.rs");
    fs::write(&source_path, source).expect("the consumer source writes");
    let mut command = Command::new("rustc");
    command
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--error-format=short")
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("fsl_replay={}", replay.display()));
    if with_serde {
        let serde = dependency(&deps, "libserde-");
        command
            .arg("--extern")
            .arg(format!("serde={}", serde.display()));
    }
    command
        .arg("--out-dir")
        .arg(temp.path())
        .arg(source_path)
        .output()
        .expect("the configured Rust compiler runs")
}

#[trace("TC-1190", "FR-315-AC-14")]
#[test]
fn replay_facade_compiles_and_keeps_product_identity_opaque() {
    let positive = compile_consumer(
        r#"
use fsl_replay::replay::{
    encode_hyper_query, HyperProductIdentity, HyperQuery, HyperQueryRequest, SubjectIdentity,
};
use fsl_replay::SmtQueryKind;

fn facade(request: HyperQueryRequest<'_>, subject: &SubjectIdentity) {
    let _: Option<&SubjectIdentity> = Some(subject);
    let _: Result<HyperProductIdentity, _> = HyperProductIdentity::of(&request);
    let _: Result<HyperQuery, _> = encode_hyper_query(
        request,
        SmtQueryKind::InductionBase { depth: 1 },
    );
}
"#,
        false,
    );
    assert!(
        positive.status.success(),
        "consumer using only fsl-replay compiles:\n{}",
        String::from_utf8_lossy(&positive.stderr)
    );

    let literal = compile_consumer(
        r#"
use fsl_replay::replay::HyperProductIdentity;

fn fabricate() -> HyperProductIdentity {
    HyperProductIdentity {
        clause: todo!(),
        subjects: Vec::new(),
        instance: Vec::new(),
    }
}
"#,
        false,
    );
    let literal_errors = String::from_utf8_lossy(&literal.stderr);
    assert!(
        !literal.status.success(),
        "the identity cannot be fabricated"
    );
    assert!(literal_errors.contains("E0451"), "{literal_errors}");

    let serde = compile_consumer(
        r#"
use fsl_replay::replay::HyperProductIdentity;

fn serializes<T: serde::Serialize>() {}
fn deserializes<T: serde::de::DeserializeOwned>() {}
fn wire_forms() {
    serializes::<HyperProductIdentity>();
    deserializes::<HyperProductIdentity>();
}
"#,
        true,
    );
    let serde_errors = String::from_utf8_lossy(&serde.stderr);
    assert!(!serde.status.success(), "the identity has no wire form");
    assert!(serde_errors.contains("E0277"), "{serde_errors}");
}
