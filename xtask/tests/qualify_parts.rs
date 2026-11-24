// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1396: the `qualify` gate's verdict covers the corpus, the lifecycle
//! matrix and the parity of the linked backends (FR-567).

use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use ix_trace_rs::trace;
use serde_json::Value;
use xtask::qualify::{self, QualifyInputs};

fn fspec_dir() -> TempDir {
    xtask::published_fspec::directory("proposals/")
}

fn inputs() -> (TempDir, QualifyInputs) {
    let fspec = fspec_dir();
    let fixtures = fspec
        .path()
        .join("proposals/conformance-corpus-v1/fixtures");
    let inputs = QualifyInputs {
        fspec: fspec.path().to_path_buf(),
        operands: vec!["--corpus".into(), fixtures.into_os_string()],
        #[cfg(feature = "test-support")]
        matrix: None,
    };
    (fspec, inputs)
}

fn report(outcome: &xtask::Outcome) -> Value {
    serde_json::from_str(&outcome.stdout).expect("the report is JSON")
}

fn part<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["parts"]
        .as_array()
        .and_then(|parts| parts.iter().find(|part| part["part"] == name))
        .unwrap_or_else(|| panic!("the report names the part {name}: {report}"))
}

fn digits_end(value: &str, from: usize) -> usize {
    value[from..]
        .find(|character: char| !character.is_ascii_digit())
        .map_or(value.len(), |offset| from + offset)
}

fn explicit_criteria(value: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = value[cursor..].find("FR-") {
        let start = cursor + offset;
        let family_start = start + 3;
        let family_end = digits_end(value, family_start);
        if family_end + 4 <= value.len() && &value[family_end..family_end + 4] == "-AC-" {
            let criterion_start = family_end + 4;
            let criterion_end = digits_end(value, criterion_start);
            if criterion_end > criterion_start {
                found.push((
                    format!(
                        "FR-{}-AC-{}",
                        &value[family_start..family_end],
                        &value[criterion_start..criterion_end]
                    ),
                    criterion_end,
                ));
                cursor = criterion_end;
                continue;
            }
        }
        cursor = family_end.max(start + 3);
    }
    found
}

fn criteria_for_location(prefix: &str) -> Vec<String> {
    let segment = prefix
        .rfind(')')
        .map_or(prefix, |close| &prefix[close + 1..]);
    let explicit = explicit_criteria(segment);
    let family = explicit
        .last()
        .map(|(criterion, _)| criterion.split('-').nth(1).expect("family").to_owned())
        .or_else(|| {
            explicit_criteria(prefix)
                .last()
                .map(|(criterion, _)| criterion.split('-').nth(1).expect("family").to_owned())
        });
    let mut criteria = explicit
        .iter()
        .map(|(criterion, _)| criterion.clone())
        .collect::<Vec<_>>();
    if let Some(family) = family {
        let mut cursor = 0;
        while let Some(offset) = segment[cursor..].find("AC-") {
            let start = cursor + offset;
            if start >= 3 && &segment[start - 3..start] == "-AC" {
                cursor = start + 3;
                continue;
            }
            let number_start = start + 3;
            let number_end = digits_end(segment, number_start);
            if number_end > number_start {
                let criterion = format!("FR-{family}-AC-{}", &segment[number_start..number_end]);
                if !criteria.contains(&criterion) {
                    criteria.push(criterion);
                }
                cursor = number_end;
            } else {
                cursor = number_start;
            }
        }
    }
    criteria
}

fn trace_at(source: &str, line_number: usize) -> Option<String> {
    let lines: Vec<_> = source.lines().collect();
    let mut at = line_number
        .checked_sub(1)?
        .min(lines.len().saturating_sub(1));
    loop {
        if lines[at].contains("#[trace(") {
            let mut attribute = lines[at].to_owned();
            while !attribute.contains(")]") && at + 1 < lines.len() {
                at += 1;
                attribute.push_str(lines[at]);
            }
            return Some(attribute);
        }
        if at == 0 {
            return None;
        }
        at -= 1;
    }
}

/// FR-500-AC-2 (TC-1294): every tabled successor source location resolves to
/// a test whose nearest `#[trace]` attribute carries the stated criterion.
#[trace("TC-1294", "FR-500-AC-2")]
#[test]
fn every_staged_successor_location_names_its_traced_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let spec = root.join(
        "spec/functional/FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md",
    );
    let text = std::fs::read_to_string(&spec).expect("FR-500 staging table");
    let source_root = root.canonicalize().expect("workspace root");
    let mut locations = 0usize;

    for row in text
        .lines()
        .filter(|line| line.starts_with("| AC-") && line.contains(".rs:"))
    {
        let successor_cell = row.split('|').nth(2).expect("successor cell");
        let mut cursor = 0;
        while let Some(open_offset) = successor_cell[cursor..].find('(') {
            let open = cursor + open_offset;
            let Some(close_offset) = successor_cell[open + 1..].find(')') else {
                break;
            };
            let close = open + 1 + close_offset;
            let group = &successor_cell[open + 1..close];
            if group.contains(".rs:") {
                let criteria = criteria_for_location(&successor_cell[..open]);
                assert!(!criteria.is_empty(), "criteria before {group:?}");
                let mut inherited_directory = Path::new("").to_path_buf();
                for word in group.split_whitespace() {
                    let token = word.trim_matches(['`', '(', ')', ',', ';', ':']);
                    let Some((path, line_numbers)) = token.split_once(':') else {
                        continue;
                    };
                    if !path.ends_with(".rs") {
                        continue;
                    }
                    let relative = if let Some(path) = path.strip_prefix("QP/") {
                        inherited_directory = Path::new("fsl-package/src/emit/tests").to_path_buf();
                        inherited_directory.join(path)
                    } else if let Some(path) = path.strip_prefix("EV/") {
                        inherited_directory = if path == "protocol_slots.rs" {
                            Path::new("fsl-eval/tests/it").to_path_buf()
                        } else {
                            Path::new("fsl-eval/tests/it/protocol_slots").to_path_buf()
                        };
                        inherited_directory.join(path)
                    } else {
                        let path = Path::new(path);
                        if path.components().count() > 1 {
                            inherited_directory = path
                                .parent()
                                .expect("a referenced source path has a parent")
                                .to_path_buf();
                            path.to_path_buf()
                        } else {
                            inherited_directory.join(path)
                        }
                    };
                    let source_path = source_root.join(relative);
                    let source = std::fs::read_to_string(&source_path)
                        .unwrap_or_else(|error| panic!("{}: {error}", source_path.display()));
                    for line in line_numbers.split(',') {
                        let line = line.parse::<usize>().unwrap_or_else(|error| {
                            panic!("invalid source location {token:?}: {error}")
                        });
                        let attribute = trace_at(&source, line).unwrap_or_else(|| {
                            panic!(
                                "{}:{line} has a preceding trace attribute",
                                source_path.display()
                            )
                        });
                        assert!(
                            criteria
                                .iter()
                                .any(|criterion| attribute.contains(criterion)),
                            "{}:{line}: expected one of {criteria:?} in {attribute}",
                            source_path.display()
                        );
                        locations += 1;
                    }
                }
            }
            cursor = close + 1;
        }
    }
    assert!(
        locations > 0,
        "the table contains checked successor locations"
    );
}

/// The no-backend half of FR-567-AC-4 and the passing run of AC-1 without a
/// linked backend: not tagged, because AC-1 needs a linked delegating backend
/// and AC-4 a reference fault, and the FR-563 runner they need is not in this
/// repository yet. Over the passing fixture corpus
/// with every matrix case passing, the report names the three parts, the
/// `parity` part `no-backend`, `complete-v1: qualified`, exit 0.
#[trace("TC-1396")]
#[test]
fn a_passing_run_names_the_three_parts_and_qualifies() {
    let (_fspec, inputs) = inputs();
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["qualification"], "complete-v1");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(part(&report, "lifecycle-matrix")["result"], "passed");
    assert_eq!(part(&report, "parity")["result"], "no-backend");
}

/// FR-567-AC-5's source-tree half (so this test carries no AC tag): the
/// report's FSL version is the root package's `CARGO_PKG_VERSION`, not xtask's,
/// and it is the version the root manifest declares. The criterion's
/// scratch-copy half is asserted by the filtered subprocess test below.
#[trace("TC-1396")]
#[test]
fn the_report_names_the_root_package_version() {
    let (_fspec, inputs) = inputs();
    let outcome = qualify::run(&inputs).expect("the gate runs");
    let version = report(&outcome)["fsl_version"].clone();
    assert_eq!(version, filament_spec_language::VERSION);
    assert_ne!(
        version,
        env!("CARGO_PKG_VERSION"),
        "xtask's own version is not the FSL version"
    );
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
            .expect("the root manifest");
    assert!(
        manifest.contains(&format!(
            "version = \"{}\"",
            filament_spec_language::VERSION
        )),
        "the root manifest declares the reported version"
    );
}

/// FR-567-AC-5: a scratch source tree rebuilt with a different root package
/// version reports that build's version, not the version compiled into xtask.
#[trace("TC-1396", "FR-567-AC-5")]
#[test]
fn a_scratch_build_reports_its_changed_root_package_version() {
    const CHILD: &str = "QSL_2095_FR567_VERSION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let (_fspec, inputs) = inputs();
        let outcome = qualify::run(&inputs).expect("the scratch gate runs");
        assert_eq!(outcome.code, 0, "{}", outcome.stdout);
        assert_eq!(report(&outcome)["fsl_version"], "9.9.9");
        return;
    }

    let scratch = TempDir::new().expect("scratch checkout");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member");
    let mut archive = Command::new("git")
        .current_dir(repository)
        .args(["archive", "--format=tar", "HEAD"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("git creates a source archive");
    let unpack = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(scratch.path())
        .stdin(archive.stdout.take().expect("archive stdout"))
        .status()
        .expect("tar unpacks the source archive");
    let archived = archive.wait_with_output().expect("git archive completes");
    assert!(archived.status.success(), "git archive succeeds");
    assert!(unpack.success(), "source archive extraction succeeds");

    let initialized = Command::new("git")
        .current_dir(scratch.path())
        .args(["init", "--quiet"])
        .status()
        .expect("scratch tree initializes for patch application");
    assert!(initialized.success(), "scratch repository initializes");

    // Apply this checkout's uncommitted source edits as well as its committed
    // tree, so the child exercises the exact code under test.
    let changes = Command::new("git")
        .current_dir(repository)
        .args(["diff", "--binary", "HEAD"])
        .output()
        .expect("git reads the working diff");
    assert!(changes.status.success(), "git diff succeeds");
    let applied = Command::new("git")
        .current_dir(scratch.path())
        .arg("apply")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("git apply stdin")
                .write_all(&changes.stdout)?;
            child.wait()
        })
        .expect("working changes apply to the scratch tree");
    assert!(
        applied.success(),
        "the scratch tree includes working changes"
    );

    let manifest_path = scratch.path().join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("root manifest");
    let (before_package, package_and_after) = manifest
        .split_once("[package]")
        .expect("root package section");
    let (package, after_package) = package_and_after
        .split_once("\n[features]")
        .expect("features follow the root package section");
    let changed_package = package.replace("version = \"0.3.0\"", "version = \"9.9.9\"");
    assert_ne!(
        changed_package, package,
        "the root version is replaced once"
    );
    std::fs::write(
        &manifest_path,
        format!("{before_package}[package]{changed_package}\n[features]{after_package}"),
    )
    .expect("scratch manifest is written");

    let lock_path = scratch.path().join("Cargo.lock");
    let lock = std::fs::read_to_string(&lock_path).expect("workspace lockfile");
    let changed_lock = lock.replace(
        "name = \"filament-spec-language\"\nversion = \"0.3.0\"",
        "name = \"filament-spec-language\"\nversion = \"9.9.9\"",
    );
    std::fs::write(&lock_path, changed_lock).expect("scratch lockfile is written");

    let output = Command::new("cargo")
        .current_dir(scratch.path())
        .args([
            "test",
            "--locked",
            "-j",
            "8",
            "-p",
            "xtask",
            "--test",
            "qualify_parts",
            "a_scratch_build_reports_its_changed_root_package_version",
            "--",
            "--exact",
        ])
        .env(CHILD, "1")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_TARGET_DIR", scratch.path().join("target"))
        .output()
        .expect("filtered scratch-version test runs");
    assert!(
        output.status.success(),
        "scratch-version test succeeds:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// FR-567-AC-2: with the matrix replaced by one whose `check`/`refusal` case
/// expects exit 21, the run is `not-qualified`, names
/// `lifecycle-matrix/check/refusal` and exits 1, although the corpus passes.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-2")]
#[test]
fn a_failing_matrix_case_fails_the_gate_by_name() {
    use fsl_conformance::{lifecycle_matrix, MatrixCell, OutcomeKind, Verb};
    let cells: Vec<MatrixCell> = lifecycle_matrix()
        .iter()
        .map(|cell| match cell {
            MatrixCell::Case(case)
                if case.verb == Verb::Check && case.kind == OutcomeKind::Refusal =>
            {
                MatrixCell::Case(fsl_conformance::MatrixCase {
                    expected_exit: 21,
                    ..*case
                })
            }
            other => *other,
        })
        .collect();
    let (_fspec, mut gate) = inputs();
    gate.matrix = Some(cells);
    let outcome = qualify::run(&gate).expect("the gate runs");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(
        part(&report, "lifecycle-matrix")["failures"],
        serde_json::json!(["lifecycle-matrix/check/refusal"])
    );
}
