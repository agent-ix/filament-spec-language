// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-884-AC-3 / FR-865: a protocol refusal's typed subject survives in the
//! rendered outcome text together with the source location.

use std::path::Path;

use filament_exact::Cancel;
use fsl_command::command;
use fsl_inspect::{render_outcome, RenderForm, RenderSources};
use fsl_replay::{spine::CompileRefusal, Operation, RenderLimits};
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

fn request(directory: &Path, unit: &str) -> std::path::PathBuf {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
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
    let path = request(directory.path(), UNIT);
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
            refusal.cause.as_ref(),
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

/// FR-884-AC-3: a decision refusal names its typed subject and locates the
/// exact source span selected from that cause.
#[trace("TC-2508", "FR-884-AC-3", "FR-865-AC-5")]
#[test]
fn a_wrong_observation_refusal_renders_its_decision_and_read_location() {
    let unit = UNIT
        .replace(
            "role R on M::Widget;",
            "role R on M::Widget; role S on M::Widget; \
             channel C from R to S carries Boolean ordering unordered delivery unknown \
             capacity 2 overflow block;",
        )
        .replace(
            "event Tick by R as (tick: Boolean) { true };",
            "send Tx via C as (tx: Boolean) { true }; \
             receive Rx via C of Tx as (rx: Boolean) { true };",
        )
        .replace("input.code > 0", "rx")
        .replace("input.code <= 0", "not rx");
    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path(), &unit);
    let Err(error) = command::compile_outcome(&path) else {
        panic!("the decision reads an unlisted fact of another role");
    };
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler check refusal: {:?}", error.cause);
    };
    let CompileRefusal::Check { refusals, region } = &failure.refusal else {
        panic!("a check refusal: {:?}", failure.refusal);
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause.as_ref(),
            CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::WrongObservation {
                    node, owner, binder, ..
                } if node == "Pick" && owner == "R" && binder == "rx")
        )),
        "{refusals:?}"
    );
    let start = unit.find("when { rx }").expect("the refused read") as u64 + "when { ".len() as u64;
    let end = start + "rx".len() as u64;
    let region = region.as_ref().expect("the refusal has a subject region");
    assert_eq!((region.start(), region.end()), (start, end));
    let rendered = render_refusal(&error);
    assert!(rendered.contains("`Pick` reads `rx`"), "{rendered}");
    assert!(
        rendered.contains(&format!("offered issuer/offered:unit:{start}-{end}")),
        "the rendered locus is the read at {start}..{end}: {rendered}"
    );
}

/// FR-884-AC-3: an undeclared owner refusal names and locates that owner.
#[trace("TC-2508", "FR-884-AC-3", "FR-865-AC-5")]
#[test]
fn an_undeclared_owner_refusal_renders_its_name_and_location() {
    let unit = UNIT
        .replace("by R visible ()", "by Absent visible ()")
        .replace("input.code > 0", "true")
        .replace("input.code <= 0", "false");
    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path(), &unit);
    let Err(error) = command::compile_outcome(&path) else {
        panic!("the choice names no declared role");
    };
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler check refusal: {:?}", error.cause);
    };
    let CompileRefusal::Check { refusals, region } = &failure.refusal else {
        panic!("a check refusal: {:?}", failure.refusal);
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause.as_ref(),
            CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::NodeRole { node, name, .. }
                    if node == "Pick" && name == "Absent")
        )),
        "{refusals:?}"
    );
    let start = unit.find("Absent").expect("the undeclared owner") as u64;
    let end = start + "Absent".len() as u64;
    let region = region.as_ref().expect("the refusal has a subject region");
    assert_eq!((region.start(), region.end()), (start, end));
    let rendered = render_refusal(&error);
    assert!(
        rendered.contains("`Pick` names `Absent`, which is no role of the protocol"),
        "{rendered}"
    );
    assert!(
        rendered.contains(&format!("offered issuer/offered:unit:{start}-{end}")),
        "the rendered locus is the owner at {start}..{end}: {rendered}"
    );
}

fn render_refusal(error: &command::RunError) -> String {
    let document = error.outcome_document(Operation::Package);
    render_outcome(
        &document,
        RenderForm::Text,
        &RenderSources::from_sources(error.sources().as_slice().to_vec()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the refusal renders")
    .value()
    .as_str()
    .to_owned()
}

/// The unit with one `Boolean` channel declaring `delivery`.
fn delivery_unit(delivery: &str) -> String {
    UNIT.replace(
        "role R on M::Widget;",
        &format!(
            "role R on M::Widget; role S on M::Widget; \
             channel C from R to S carries Boolean ordering unordered delivery {delivery} \
             capacity 2 overflow block;"
        ),
    )
    .replace(
        "event Tick by R as (tick: Boolean) { true };",
        "send Tx via C as (tx: Boolean) { true }; \
         receive Rx via C of Tx as (rx: Boolean) { true };",
    )
    .replace("input.code > 0", "true")
    .replace("input.code <= 0", "false")
}

/// FR-509-AC-2: a delivery interval whose lower count is above its upper
/// count settles `ill_typed`/`type-mismatch` at the interval through the
/// compile command, not `unsupported-feature`; `[3, 3]` compiles.
#[trace("TC-1304", "FR-509-AC-2")]
#[test]
fn an_inverted_delivery_interval_refuses_type_mismatch_at_the_interval() {
    let unit = delivery_unit("[4, 3]");
    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path(), &unit);
    let Err(error) = command::compile_outcome(&path) else {
        panic!("an inverted interval does not compile");
    };
    assert_eq!(error.category().exit_code(), 20, "{error:?}");
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler check refusal: {:?}", error.cause);
    };
    let CompileRefusal::Check { refusals, region } = &failure.refusal else {
        panic!("a check refusal: {:?}", failure.refusal);
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause.as_ref(),
            CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::ChannelDeliveryRange {
                    lower: 4, upper: 3, ..
                })
        )),
        "{refusals:?}"
    );
    assert!(
        !refusals.iter().any(|refusal| matches!(
            refusal.cause.as_ref(),
            CheckCause::UnsupportedFeature { .. }
        )),
        "a refused protocol is not also unsupported: {refusals:?}"
    );
    let headline = refusals.first().expect("one refusal at least");
    assert_eq!(
        (headline.cause.code().as_str(), headline.cause.cause()),
        ("ill_typed", Some("type-mismatch")),
        "{refusals:?}"
    );
    assert_eq!(failure.refusal.code().as_str(), "ill_typed");
    let start = unit.find("[4, 3]").expect("the interval") as u64;
    let region = region.as_ref().expect("the refusal has a subject region");
    assert_eq!(
        (region.start(), region.end()),
        (start, start + "[4, 3]".len() as u64)
    );

    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path(), &delivery_unit("[3, 3]"));
    if let Err(error) = command::compile_outcome(&path) {
        panic!("a degenerate interval compiles: {:?}", error.cause);
    }
}

/// The filed example's shape (filament-examples
/// `examples/lock/rules/protocols/handover-delivery-negative-bound.fsl`) with
/// the interval `[4, 3]`: profile `v`, a record payload, a role pair on one
/// model object, and `send`/`receive` over the channel. Only the model
/// binding is swapped for the fixture model this suite compiles against.
const FILED_HANDOVER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile pp = \"filament.protocol.finite-global/v1\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    model L = \"acme/orders\";\n\
    \n\
    record Request { granted: Boolean; }\n\
    \n\
    protocol Handover using pp over (lock: L::Widget) on origin {\n\
      role Asker on L::Widget;\n\
      role Holder on L::Widget;\n\
      channel Ask from Asker to Holder carries Request\n\
        ordering unordered delivery [4, 3] capacity 4 overflow reject;\n\
      run sequence Main {\n\
        send Asked via Ask as (asked: Request) { true };\n\
        receive Heard via Ask of Asked as (heard: Request) { true };\n\
      }\n\
      finish Done as (done: Boolean) { true };\n\
    }\n";

/// FR-509-AC-2: the filed handover unit with `delivery [4, 3]` settles
/// `ill_typed`/`type-mismatch` at the interval and emits no
/// `unsupported-feature`.
#[trace("TC-1304", "FR-509-AC-2")]
#[test]
fn the_filed_handover_example_with_an_inverted_interval_refuses_type_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    let path = request(directory.path(), FILED_HANDOVER);
    let Err(error) = command::compile_outcome(&path) else {
        panic!("an inverted interval does not compile");
    };
    assert_eq!(error.category().exit_code(), 20, "{error:?}");
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler check refusal: {:?}", error.cause);
    };
    let CompileRefusal::Check { refusals, region } = &failure.refusal else {
        panic!("a check refusal: {:?}", failure.refusal);
    };
    assert!(
        !refusals.iter().any(|refusal| matches!(
            refusal.cause.as_ref(),
            CheckCause::UnsupportedFeature { .. }
        )),
        "a refused protocol is not also unsupported: {refusals:?}"
    );
    let headline = refusals.first().expect("one refusal at least");
    assert_eq!(
        (headline.cause.code().as_str(), headline.cause.cause()),
        ("ill_typed", Some("type-mismatch")),
        "{refusals:?}"
    );
    assert_eq!(failure.refusal.code().as_str(), "ill_typed");
    let start = FILED_HANDOVER.find("[4, 3]").expect("the interval") as u64;
    let region = region.as_ref().expect("the refusal has a subject region");
    assert_eq!(
        (region.start(), region.end()),
        (start, start + "[4, 3]".len() as u64)
    );
}
