// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-513-AC-15 (TC-1308 row 25): `quire matrix` backs every retained
//! criterion that once rode the compiled-protocol readers with a tagged test
//! in a checked-package, evaluation or replay crate.
//!
//! The test runs the installed `quire matrix` over this repository and reads
//! the status and binders the CLI computes for each criterion. It never
//! computes or edits a matrix itself.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use ix_trace_rs::trace;

/// The criteria FR-513-AC-15 retains.
const RETAINED: [&str; 9] = [
    "FR-050-AC-2",
    "FR-050-AC-3",
    "FR-054-AC-1",
    "FR-054-AC-2",
    "FR-054-AC-3",
    "FR-054-AC-4",
    "FR-049-AC-2",
    "FR-049-AC-5",
    "FR-513-AC-17",
];

/// The binders `quire matrix` computes for each criterion of the repository.
fn matrix(root: &Path) -> BTreeMap<String, (String, String)> {
    let output = Command::new("quire")
        .args(["matrix", "--format", "tsv", "--scope"])
        .arg(root)
        .output()
        .unwrap_or_else(|error| {
            panic!("FR-513-AC-15 needs the installed `quire` CLI on PATH (0.38 or later): {error}")
        });
    assert!(
        output.status.success(),
        "quire matrix failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("tsv output is text");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("tsv header").split('\t').collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("the matrix tsv has a `{name}` column"))
    };
    let (criterion, status, binders) = (column("criterion"), column("status"), column("binders"));
    lines
        .filter_map(|line| {
            let columns: Vec<&str> = line.split('\t').collect();
            Some((
                (*columns.get(criterion)?).to_owned(),
                (
                    (*columns.get(status)?).to_owned(),
                    (*columns.get(binders)?).to_owned(),
                ),
            ))
        })
        .collect()
}

#[trace("TC-1308", "FR-513-AC-15")]
#[test]
fn every_retained_criterion_has_a_tagged_checked_package_successor_test() {
    let matrix = matrix(Path::new(env!("CARGO_MANIFEST_DIR")));
    for criterion in RETAINED {
        let (status, binders) = matrix
            .get(criterion)
            .unwrap_or_else(|| panic!("{criterion} is in the matrix"));
        assert_eq!(status, "tagged", "{criterion}");
        assert!(
            binders.split(',').any(|binder| {
                ["fsl-package/", "fsl-eval/", "fsl-replay/"]
                    .iter()
                    .any(|root| binder.starts_with(root))
            }),
            "{criterion} is tagged only outside the spine crates: {binders}"
        );
    }
}
