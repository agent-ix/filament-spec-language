// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-866-AC-3 (TC-2457): a unit whose `model` line selects a package no
//! input supplies refuses `missing_import`, and the message names the
//! identity and both inputs that supply it: the `--model` option of
//! `fsl check` and a compile request's `models` member. FR-866-AC-1, AC-2
//! and AC-4 exercise `fsl_command::command::check_outcome`, which the code
//! ticket adds with its tests.

use crate::command_harness::compile_file;
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::json;

/// FR-866-AC-3: a compile request with `models` `[]` over FR-108's unit.
#[test]
#[trace("TC-2457", "FR-866-AC-3")]
#[ignore = "QSL-1972: the missing_import message naming --model and the models member is not implemented yet"]
fn a_missing_model_names_the_identity_and_the_inputs_that_supply_it() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "spine-unit.native", "authority": "agent-ix",
            "identity": "test:fr-866", "document": "SpineUnit", "formal_revision": 1}},
    }});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = compile_file(&path);
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let failure = String::from_utf8_lossy(&output.stderr);
    assert!(failure.contains("missing_import"), "{failure}");
    assert!(failure.contains(spine::PACKAGE_IDENTITY), "{failure}");
    assert!(failure.contains("--model"), "{failure}");
    assert!(failure.contains("`models`"), "{failure}");
}
