// SPDX-License-Identifier: AGPL-3.0-or-later
//! Determinism of the published FSpec sample across child test processes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Barrier, OnceLock};

use ix_trace_rs::trace;
use serde_json::Value;

const FSPEC_ROOT: &str = "QSL_2095_FSPEC_ROOT";
const CORPUS_ROOT: &str = "QSL_2095_CORPUS_ROOT";
const OUTPUT_ROOT: &str = "QSL_2095_OUTPUT_ROOT";
const CONCURRENT_CHILD: &str = "QSL_2095_CONCURRENT_CHILD";

fn fspec_directory() -> tempfile::TempDir {
    xtask::published_fspec::directory("proposals/")
}

fn fsl_profiles() -> BTreeSet<String> {
    let mut profiles = fsl_semantics::value::DefinitionRole::ALL
        .into_iter()
        .map(|role| role.identity().to_owned())
        .collect::<BTreeSet<_>>();
    profiles.extend(
        fsl_semantics::value::ModelRole::ALL
            .into_iter()
            .map(|role| role.identity().to_owned()),
    );
    profiles.extend(
        fsl_semantics::value::TemporalRole::ALL
            .into_iter()
            .map(|role| role.identity().to_owned()),
    );
    profiles.extend(
        fsl_semantics::value::HeaderLayer::ALL
            .into_iter()
            .map(|layer| layer.identity().to_owned()),
    );
    profiles.extend(
        fsl_semantics::check::TemporalProfile::ALL
            .into_iter()
            .map(|profile| profile.identity().to_owned()),
    );
    profiles
}

fn is_fsl_subject(subject: &Value) -> bool {
    matches!(
        (
            subject["consumer_type"].as_str(),
            subject["operation"].as_str()
        ),
        (Some("language"), Some("compile"))
            | (Some("runtime"), Some("run" | "replay"))
            | (Some("tool"), Some("format"))
    )
}

fn default_capability_ids(corpus: &Value) -> BTreeSet<String> {
    let profiles = fsl_profiles();
    corpus["coverage"]["primary"]
        .as_array()
        .expect("published corpus capabilities")
        .iter()
        .filter(|capability| {
            capability["applicability"]["consumer_types"]
                .as_array()
                .is_some_and(|types| {
                    types
                        .iter()
                        .any(|kind| matches!(kind.as_str(), Some("language" | "runtime" | "tool")))
                })
                && capability["applicability"]["semantic_profiles"]
                    .as_array()
                    .is_some_and(|items| {
                        items.iter().any(|profile| {
                            profile
                                .as_str()
                                .is_some_and(|profile| profiles.contains(profile))
                        })
                    })
        })
        .map(|capability| {
            capability["capability_id"]
                .as_str()
                .expect("a capability id")
                .to_owned()
        })
        .collect()
}

fn sample_corpus(fspec: &Path, root: &Path) -> Vec<String> {
    let published = fspec.join("proposals/conformance-corpus-v1/fixtures/complete-v1.json");
    let mut corpus: Value = serde_json::from_slice(
        &fs::read(&published).expect("published determinism corpus is readable"),
    )
    .expect("published determinism corpus is JSON");
    let default = default_capability_ids(&corpus);
    let mut selected = BTreeMap::<(String, String), Value>::new();
    for vector in corpus["vectors"].as_array().expect("published vectors") {
        let capability = vector["capability_id"].as_str().expect("capability id");
        let class = vector["class"].as_str().expect("vector class");
        let consumer = vector["subject"]["consumer_type"]
            .as_str()
            .expect("subject consumer type");
        if !default.contains(capability)
            || !is_fsl_subject(&vector["subject"])
            || !vector["applicability"]["consumer_types"]
                .as_array()
                .is_some_and(|types| types.iter().any(|kind| kind.as_str() == Some(consumer)))
        {
            continue;
        }
        let key = (capability.to_owned(), class.to_owned());
        let replace = selected.get(&key).is_none_or(|previous| {
            vector["id"].as_str().unwrap() < previous["id"].as_str().unwrap()
        });
        if replace {
            selected.insert(key, vector.clone());
        }
    }
    let vectors = selected.into_values().collect::<Vec<_>>();
    assert!(!vectors.is_empty(), "the determinism sample is nonempty");
    let ids = vectors
        .iter()
        .map(|vector| vector["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    let original_root = published.parent().expect("fixture directory");
    let mut inputs = BTreeSet::new();
    for vector in &vectors {
        for file in vector["input"]["files"].as_array().expect("input files") {
            inputs.insert(file["path"].as_str().expect("input path").to_owned());
        }
    }
    fs::create_dir_all(root).expect("sample directory is created");
    for input in inputs {
        let source = original_root.join(&input);
        let target = root.join(&input);
        fs::create_dir_all(target.parent().expect("input parent"))
            .expect("sample input parent is created");
        fs::copy(source, target).expect("sample input is copied from published corpus");
    }
    corpus["vectors"] = serde_json::json!(vectors);
    let bytes = serde_json_canonicalizer::to_vec(&corpus).expect("sample corpus is canonical");
    fs::write(root.join("complete-v1.json"), bytes).expect("sample corpus is written");
    ids
}

fn run_sample(fspec: &Path, corpus: &Path) -> xtask::qualification::Execution {
    let arguments = [
        std::ffi::OsString::from("--corpus"),
        corpus.as_os_str().to_owned(),
    ];
    xtask::qualification::execute_with_fspec(fspec, &arguments)
        .expect("the published sample runs through qualification")
}

fn child_barrier() -> &'static Barrier {
    static BARRIER: OnceLock<Barrier> = OnceLock::new();
    BARRIER.get_or_init(|| Barrier::new(2))
}

fn child_run(name: &str) {
    let (Some(fspec), Some(corpus), Some(output)) = (
        std::env::var_os(FSPEC_ROOT),
        std::env::var_os(CORPUS_ROOT),
        std::env::var_os(OUTPUT_ROOT),
    ) else {
        return;
    };
    let output = PathBuf::from(output).join(name);
    fs::create_dir_all(&output).expect("child output directory is created");
    let execution = run_sample(&PathBuf::from(fspec), &PathBuf::from(corpus));
    assert!(
        !execution.outputs.is_empty(),
        "the published sample has passed vector outputs: {}",
        execution.outcome.stdout
    );
    fs::write(output.join("report.json"), execution.outcome.stdout)
        .expect("child report is written");
    for (id, bytes) in execution.outputs {
        fs::write(output.join(format!("{id}.bytes")), bytes)
            .expect("passed vector bytes are written");
    }
}

#[test]
fn child_determinism_sample() {
    if std::env::var_os(CONCURRENT_CHILD).is_some() {
        child_barrier().wait();
    }
    child_run("sample");
}

#[test]
fn child_determinism_companion() {
    if std::env::var_os(CONCURRENT_CHILD).is_some() {
        child_barrier().wait();
        child_run("companion");
    }
}

fn cargo_child(
    manifest: &Path,
    fspec: &Path,
    corpus: &Path,
    output: &Path,
    threads: &str,
    concurrent: bool,
) -> Output {
    let mut command = Command::new("cargo");
    command
        .current_dir(manifest.parent().expect("xtask parent is workspace root"))
        .args([
            "test",
            "--locked",
            "-p",
            "xtask",
            "-j6",
            "--test",
            "qualification_determinism",
        ]);
    if concurrent {
        command.args(["child_determinism", "--", "--test-threads=2"]);
    } else {
        command.args([
            "child_determinism_sample",
            "--",
            "--exact",
            "--test-threads=1",
        ]);
    }
    command
        .env("RUST_TEST_THREADS", threads)
        .env(FSPEC_ROOT, fspec)
        .env(CORPUS_ROOT, corpus)
        .env(OUTPUT_ROOT, output)
        .env_remove(CONCURRENT_CHILD);
    if concurrent {
        command.env(CONCURRENT_CHILD, "1");
    }
    let executable = std::env::current_exe().expect("test executable path is available");
    let target = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("test executable is under a target directory");
    command.env("CARGO_TARGET_DIR", target);
    command.output().expect("child cargo test starts")
}

fn assert_child_succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "child cargo test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn files_with_suffix(root: &Path, suffix: &str) -> Vec<PathBuf> {
    let mut files = fs::read_dir(root)
        .expect("child output directory is readable")
        .map(|entry| entry.expect("child output entry is readable").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
#[trace("TC-1192", "FR-350-AC-6")]
fn report_and_passed_output_bytes_match_across_processes_and_thread_counts() {
    let fspec = fspec_directory();
    let scratch = tempfile::tempdir().expect("determinism scratch directory");
    let corpus = scratch.path().join("corpus");
    let ids = sample_corpus(fspec.path(), &corpus);
    let outputs = (0..4)
        .map(|index| scratch.path().join(format!("child-{index}")))
        .collect::<Vec<_>>();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    let first = cargo_child(manifest, fspec.path(), &corpus, &outputs[0], "1", false);
    assert_child_succeeded(&first);
    let second = cargo_child(manifest, fspec.path(), &corpus, &outputs[1], "1", false);
    assert_child_succeeded(&second);

    for output in &outputs[2..] {
        let child = cargo_child(manifest, fspec.path(), &corpus, output, "2", true);
        assert_child_succeeded(&child);
    }

    let reports = outputs
        .iter()
        .map(|root| fs::read(root.join("sample/report.json")).expect("sample report exists"))
        .collect::<Vec<_>>();
    assert!(reports.iter().all(|report| report == &reports[0]));
    for root in &outputs[2..] {
        let companion = fs::read(root.join("companion/report.json"))
            .expect("concurrent companion report exists");
        assert_eq!(companion, reports[0]);
    }
    let baseline = files_with_suffix(&outputs[0].join("sample"), ".bytes");
    let baseline_names = baseline
        .iter()
        .map(|path| path.file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(baseline_names.len(), ids.len());
    for root in &outputs[1..] {
        for name in &baseline_names {
            assert_eq!(
                fs::read(root.join("sample").join(name)).expect("sample output exists"),
                fs::read(outputs[0].join("sample").join(name)).unwrap(),
                "passed output bytes differ for {}",
                name.to_string_lossy()
            );
        }
    }
    for root in &outputs[2..] {
        for name in &baseline_names {
            assert_eq!(
                fs::read(root.join("companion").join(name)).expect("companion output exists"),
                fs::read(outputs[0].join("sample").join(name)).unwrap(),
                "companion output bytes differ for {}",
                name.to_string_lossy()
            );
        }
    }
}
