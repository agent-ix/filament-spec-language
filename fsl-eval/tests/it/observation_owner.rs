// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-861-AC-4 (TC-2442): `fsl-eval` takes the partial-value, completeness,
//! activation and repair authority rules from `filament-observation` and
//! holds no second implementation of them.

use fsl_eval::completeness::{revise_complete_through, CompleteThroughRefusal};
use fsl_eval::temporal::{CoveredList, ObservedPosition, ProgressCompleteness};
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("a readable source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
#[ignore = "QSL-2145: fsl-eval still carries its own partial, completeness, activation and repair rules"]
fn fsl_eval_calls_the_observation_owner_for_its_four_authority_rule_sets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("the manifest");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(
        dependencies.contains("filament-observation"),
        "fsl-eval names filament-observation in [dependencies]"
    );
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let text: String = files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect();
    let grouped: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.trim_start()
                .starts_with("use filament_observation::authority::{")
        })
        .collect();
    for module in ["partial", "completeness", "activation", "repair"] {
        let named = text.contains(&format!("filament_observation::authority::{module}"))
            || grouped.iter().any(|line| {
                line.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == module)
            });
        assert!(named, "fsl-eval calls the owner's `{module}` module");
    }
}

fn at(identity: &str, stamp: u64) -> ObservedPosition {
    ObservedPosition {
        identity: identity.to_owned(),
        stamp,
        fingerprint: None,
    }
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn the_complete_through_rule_accepts_and_refuses_as_fr_944_states() {
    let held = [at("a", 0), at("b", 50), at("c", 100)];
    let covered: CoveredList = held.iter().cloned().collect();
    let retained = ProgressCompleteness::Complete { through: 100 };
    for through in [100, 140] {
        assert_eq!(
            revise_complete_through(
                retained,
                ProgressCompleteness::Complete { through },
                &covered,
                &held
            ),
            Ok(()),
            "Complete through {through} after Complete through 100"
        );
    }
    assert_eq!(
        revise_complete_through(
            retained,
            ProgressCompleteness::Complete { through: 60 },
            &covered,
            &held
        ),
        Err(CompleteThroughRefusal::Retreat),
        "Complete through 60"
    );
    assert_eq!(
        revise_complete_through(retained, ProgressCompleteness::Incomplete, &covered, &held),
        Err(CompleteThroughRefusal::Retreat),
        "Incomplete"
    );
    let uncovered = at("d", 80);
    assert_eq!(
        revise_complete_through(
            retained,
            ProgressCompleteness::Complete { through: 100 },
            &covered,
            std::slice::from_ref(&uncovered)
        ),
        Err(CompleteThroughRefusal::UncoveredPosition(Box::new(
            uncovered
        ))),
        "an uncovered position stamped 80"
    );
    assert_eq!(
        revise_complete_through(
            ProgressCompleteness::Incomplete,
            ProgressCompleteness::Incomplete,
            &covered,
            &held
        ),
        Ok(()),
        "a retained Incomplete refuses nothing"
    );
}
