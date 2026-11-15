// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-139 (TC-2024): the crates that evaluate, analyse and replay have no
//! path to an external side effect. They observe or recommend only (FSpec
//! FR-182), so their shipped source spawns no process and opens no
//! connection.

use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

/// The crates NFR-139 covers.
const CRATES: [&str; 6] = [
    "fsl-semantics",
    "fsl-package",
    "fsl-eval",
    "fsl-analyze",
    "fsl-route",
    "fsl-replay",
];

/// The source fragments that reach a process, the network or an adapter.
const FORBIDDEN: [&str; 4] = ["std::process", "std::net", "Command::new", "TcpStream"];

/// Every shipped `.rs` file under `dir`: test files (`tests.rs`, anything
/// under a `tests` directory) are outside the rule.
fn shipped_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if name != "tests" {
                shipped_sources(&path, found);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" {
            found.push(path);
        }
    }
}

/// NFR-139-AC-1: no shipped source file of the covered crates names a
/// process, network or connection API.
#[trace("TC-2024", "NFR-139-AC-1", "FSpec-TC-1523", "FSpec-FR-182-AC-6")]
#[ignore = "QSL-2211: the effect-boundary guard is not adopted into the gate yet"]
#[test]
fn evaluation_and_analysis_crates_name_no_process_or_network_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offending = Vec::new();
    for krate in CRATES {
        let mut sources = Vec::new();
        shipped_sources(&root.join(krate).join("src"), &mut sources);
        assert!(!sources.is_empty(), "{krate} has shipped source");
        for path in sources {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            for (index, line) in text.lines().enumerate() {
                if FORBIDDEN.iter().any(|fragment| line.contains(fragment)) {
                    offending.push(format!("{}:{}: {line}", path.display(), index + 1));
                }
            }
        }
    }
    assert!(offending.is_empty(), "{offending:#?}");
}
