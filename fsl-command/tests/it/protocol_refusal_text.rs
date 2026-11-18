// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-884-AC-3 / FR-865: a protocol refusal's typed subject survives in the
//! rendered outcome text together with the source location.

use std::path::Path;

use filament_exact::Cancel;
use fsl_command::command;
use fsl_inspect::{RenderForm, RenderSources, render_outcome};
use fsl_replay::{Operation, RenderLimits, spine::CompileRefusal};
use fsl_semantics::check::{CheckCause, ProtocolAnchorCause};
use ix_trace_rs::trace;
use serde_json::json;

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    profile pp = \"filament.protocol.finite-global/v1\";\n\
    model M = \"acme/orders\";\n\
    protocol Flow using pp over (input: M::Widget) on origin {\n\
      role R on M::Widget;\n\
      run sequence Main { event Tick by R as (tick: Boolean) { true };\n\
        choice Pick by R visible () {\n\
          case yes when { input.code > 0 } sequence Yes { }\n\
          case no when { input.code <= 0 } sequence No { }\n\
        }\n\
      }\n\
      finish End as (outcome: Boolean) { true };\n\
    }\n";

fn request(directory: &Path) -> std::path::PathBuf {
    std::fs::write(directory.join("unit.native"), UNIT).unwrap();
    std::fs::write(directory.join("model.json"), MODEL).unwrap();
    let path = directory.join("compile.json");
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {
            "file": "model.json", "authority": "model issuer", "identity": "model:source",
            "document": "DomainModel", "formal_revision": 1}}],
        "program": {"source": {"file": "unit.native", "authority": "offered issuer",
            "identity": "offered:unit", "document": "ProtocolUnit", "formal_revision": 1}}}});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// FR-884-AC-3: the refusal names the typed offending node and the text
/// renderer locates that subject using its source region.
#[trace("TC-2508", "FR-884-AC-3", "FR-865-AC-5")]
#[test]
fn a_protocol_slot_refusal_renders_its_declared_subject_and_location() {
    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path());
    let Err(error) = command::compile_outcome(&path) else {
        panic!("the guard reads model state");
    };
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler check refusal: {:?}", error.cause);
    };
    let CompileRefusal::Check { refusals, region } = &failure.refusal else {
        panic!("one check refusal: {:?}", failure.refusal);
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::SlotModelState { node, .. } if node == "Pick")
        )),
        "{refusals:?}"
    );
    let start = UNIT.find("input.code").expect("the refused read") as u64;
    let end = start + "input.code".len() as u64;
    let region = region.as_ref().expect("the refusal has a subject region");
    assert_eq!((region.start(), region.end()), (start, end));

    let document = error.outcome_document(Operation::Package);
    let rendered = render_outcome(
        &document,
        RenderForm::Text,
        &RenderSources::from_sources(error.sources().as_slice().to_vec()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the refusal renders")
    .value()
    .as_str()
    .to_owned();

    assert!(rendered.contains("Guard of `Pick`"), "{rendered}");
    assert!(
        rendered.contains(&format!("offered issuer/offered:unit:{start}-{end}")),
        "the rendered locus is the read at {start}..{end}: {rendered}"
    );
}
