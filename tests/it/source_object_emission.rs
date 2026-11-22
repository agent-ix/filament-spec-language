// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569 "Emission": a unit that declares a source object type emits and
//! writes its handoff directory; a unit that declares none emits as before.

use fsl_command::checked_handoff::{write, Corpus};
use fsl_command::emit_checked_v2::{self, Request};
use ix_trace_rs::trace;

use super::checked_handoff::request;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"filament.model.complete/v1\";\n\
    object Order { total: Int[0, 9]; population orders; operation size(): Integer; }\n\
    object Sub extends Order { }\n";

fn source_request(directory: &std::path::Path) -> Request {
    let unit = directory.join("source.native");
    std::fs::write(&unit, UNIT).unwrap();
    Request {
        source: unit,
        package_name: "r".into(),
        models: Vec::new(),
        libraries: Vec::new(),
        execution_points: None,
        temporal_selections: Vec::new(),
        out: None,
    }
}

/// FR-569-AC-11: a unit that declares a source object type is not refused for
/// declaring it; it emits.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn a_unit_with_a_source_object_emits() {
    let holder = tempfile::tempdir().unwrap();
    let source = source_request(holder.path());
    let bytes = emit_checked_v2::emit(&source).unwrap_or_else(|error| panic!("{error}"));
    assert!(!bytes.is_empty());
}

/// FR-569-AC-11: the handoff writer also creates the directory. The reader
/// the writer runs over the package does not yet admit source
/// `operation_bodies` rows.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn a_unit_with_a_source_object_writes_the_handoff() {
    let holder = tempfile::tempdir().unwrap();
    let source = source_request(holder.path());
    let out = holder.path().join("handoff");
    let mut handoff = request("event_position", Corpus::None);
    handoff.emit = source;
    write(&handoff, &out).unwrap_or_else(|error| panic!("{error}"));
    assert!(out.join("identities.json").is_file());
}

/// A unit with no `object` declaration emits as before.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn a_unit_without_a_source_object_still_emits() {
    let holder = tempfile::tempdir().unwrap();
    let mut source = source_request(holder.path());
    std::fs::write(
        &source.source,
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         function f using P (x: Boolean): Boolean pure { x }\n",
    )
    .unwrap();
    source.temporal_selections.clear();
    let bytes = emit_checked_v2::emit(&source).unwrap_or_else(|error| panic!("{error}"));
    assert!(!bytes.is_empty());
}
