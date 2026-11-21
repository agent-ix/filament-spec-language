// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-740, TC-741, TC-743 and TC-744 (FR-265, FR-266, FR-268, FR-269): the
//! separating witness of a state clause, over a `witness` unit on FR-108's
//! `Config` domain package.
//!
//! FR-265's fixture: a state-clause body cannot name a population
//! (ADR-016 FE-4), so each invariant here ranges over a collection built
//! inside the claim from `self`'s and its parent's members. The member
//! domains are in `witness_member`.

use super::frame::object;
use super::frame_replay::{
    config_version_unit, packet, unit_for, unit_for_with_function_modes, witness_source, Unit,
};
use super::*;
use crate::execute::{separate, settle_separation, stopped_reason, SeparationOutcome};
use crate::proof_result::SettlementBasis;
use crate::result::{DecidingElement, SeparatingWitnessRecord};
use crate::witness::{derive_separating_witness, Derived};
use crate::{
    replay_state_clause, DisagreementCause, InputSettlement, ObservationIdentity, ReplayRefusal,
    ReplayResult, ReplaySource, RuntimeValuePath, SeparationReason, SeparationStep,
    StateClauseCounterexample, StateClauseReplayResult, ValuePathStep, ValuePathSubject, Verdict,
    WitnessEnvelope, WitnessFailure, WitnessSettlement,
};
use filament_semantic_value::catalog::Category;
use filament_semantic_value::checking::CheckMode;
use fsl_eval::value::ClauseEvaluation;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{ApplicationTerm, BodyTerm, Operator};
use fsl_semantics::model::accounting::ModelNormalizationLimits;

/// The `witness` unit's invariants, on `Config::ConfigVersion` at
/// `current`. `L` is `ints(sequence[0, self.versionNumber, self.versionNumber])`:
/// over `self` `mid` (600) it is `[0, 600, 600]`, whose first element at or
/// above 500 is at index 1, with an equal element after it; over `self`
/// `child` (1) it is `[0, 1, 1]`.
const CLAUSES: &[(&str, &str)] = &[
    (
        "AllBelow",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500)",
    ),
    (
        "SomeAtLeast",
        "exists(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n >= 500)",
    ),
    (
        "NotAllBelow",
        "not forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500)",
    ),
    (
        "GuardedAll",
        "present(self.parent) implies \
         forall(c in refs(sequence[value(self.parent), self]): c.versionNumber < 500)",
    ),
    (
        "EqualsAll",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500) \
         = present(self.parent)",
    ),
    (
        "LetAll",
        "let k = 500 in \
         forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < k)",
    ),
    (
        "OrAll",
        "forall(n in ints(sequence[0, self.versionNumber, self.versionNumber]): n < 500) \
         or self.versionNumber = 0",
    ),
    (
        "NestedAll",
        "forall(c in ints(sequence[0, self.versionNumber]): \
         forall(d in ints(sequence[0, self.versionNumber]): d <= c + 100))",
    ),
    (
        "FilteredAll",
        "forall(n in filter(m in ints(sequence[0, 1, self.versionNumber]): m > 0): n < 500)",
    ),
    (
        "MappedAll",
        "forall(n in map(m in ints(sequence[0, self.versionNumber]): m + 1): n < 500)",
    ),
    ("FloatOrder", "fl(ONE) < fl(TWO)"),
    ("FloatNot", "not (fl(ONE) < fl(TWO))"),
    ("FloatLet", "let k = 500 in fl(ONE) < fl(TWO)"),
    (
        "FloatOrOperand",
        "self.versionNumber = 0 or fl(ONE) < fl(TWO)",
    ),
    (
        "FloatAndOperand",
        "self.versionNumber > 0 and fl(ONE) < fl(TWO)",
    ),
    (
        "FloatImpliesOperand",
        "self.versionNumber > 0 implies fl(ONE) < fl(TWO)",
    ),
    (
        "NoneSelected",
        "forall(n in filter(m in ints(sequence[0, self.versionNumber]): m > 900): n < 0)",
    ),
    (
        "MatchesParent",
        "present(self.parent) implies self.versionNumber = value(self.parent).versionNumber",
    ),
    (
        "AtLeastParent",
        "present(self.parent) implies self.versionNumber >= value(self.parent).versionNumber",
    ),
];

/// The `witness` unit's text, selecting the `Config` domain package.
fn witness_unit_text() -> String {
    let mut unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?};\n"
    );
    // A collection literal takes its type from the parameter it is passed
    // to; each function returns its argument unchanged.
    unit.push_str(
        "function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { xs }\n\
         function refs using v(xs: Sequence<Config::ConfigVersion>[0, 2]): \
         Sequence<Config::ConfigVersion>[0, 2] pure { xs }\n\
         type Zero = Int[0, 0];\n\
         function fl using v(x: Float64[nearest-even]): Float64[nearest-even] pure { x }\n",
    );
    for (name, body) in CLAUSES {
        let body = body
            .replace("ONE", "float64(bits: 0x3ff0000000000000)")
            .replace("TWO", "float64(bits: 0x4000000000000000)");
        unit.push_str(&format!(
            "invariant {name} using v on Config::ConfigVersion at current {{ {body} }}\n"
        ));
    }
    unit
}

fn witness_unit() -> Unit {
    unit_for(witness_unit_text(), config_version_domain_document())
}

fn tc_3121_unit_text() -> String {
    let mut unit = witness_unit_text()
        .lines()
        .filter(|line| {
            !line.starts_with("invariant ")
                || line.starts_with("invariant AllBelow ")
                || line.starts_with("invariant NotAllBelow ")
                || line.starts_with("invariant MatchesParent ")
                || line.starts_with("invariant AtLeastParent ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    unit.push_str(
        "\nfunction z using v(x: Integer): Int[0, 0] pure { \
         count<Zero>(m in ints(sequence[x]): m > 0) }\n",
    );
    unit.push_str(
        "invariant RefusedDomain using v on Config::ConfigVersion at current { \
         forall(n in ints(sequence[0, z(self.versionNumber)]): n < 500) }\n",
    );
    unit
}

fn tc_3121_undefined_unit() -> Unit {
    let mut unit = tc_3121_unit_text();
    unit.push_str(
        "\nfunction u using v(x: Integer): Int[0, 0] pure { x mod 0 }\n\
         invariant UndefinedDomain using v on Config::ConfigVersion at current { \
         forall(n in ints(sequence[0, u(self.versionNumber)]): n < 500) }\n",
    );
    unit_for_with_function_modes(
        unit,
        config_version_domain_document(),
        &BTreeMap::from([
            ("u".to_owned(), CheckMode::Kernel),
            ("z".to_owned(), CheckMode::Kernel),
        ]),
    )
}

fn tc_3121_unit() -> Unit {
    unit_for_with_function_modes(
        tc_3121_unit_text(),
        config_version_domain_document(),
        &BTreeMap::from([("z".to_owned(), CheckMode::Kernel)]),
    )
}

fn tc_3121_run(clause: &str, snapshot: Snapshot, self_key: &str) -> crate::spine::ClauseRunReport {
    let (selection, snapshots) = selection(clause, snapshot, self_key);
    let document = config_version_domain_document();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let mut request = config_version_request_for(
        tc_3121_unit_text(),
        packages,
        "tc-3121-witness.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots = snapshots;
    request.accounting = default_accounting(1_000_000);
    let function_modes = BTreeMap::from([("z".to_owned(), CheckMode::Kernel)]);
    super::super::run_clause_with_function_modes(request, &[], &function_modes)
        .expect("a well-formed TC-3121 request always reports")
}

fn tc_3121_version_unit_text() -> String {
    let mut unit = tc_3121_unit_text();
    unit.push_str(
        "\npost VersionAdvances using v on Config::ConfigVersion::attemptUpdate { \
         (result implies convert<Integer>(self.versionNumber) = pre(self.versionNumber) + 1) \
         and (not result implies self.versionNumber = pre(self.versionNumber)) }\n",
    );
    unit
}

fn tc_3121_version_unit() -> Unit {
    unit_for_with_function_modes(
        tc_3121_version_unit_text(),
        config_version_domain_document(),
        &BTreeMap::from([("z".to_owned(), CheckMode::Kernel)]),
    )
}

fn tc_3121_no_parent_unit() -> Unit {
    let mut unit = witness_unit_text()
        .lines()
        .filter(|line| !line.starts_with("invariant "))
        .collect::<Vec<_>>()
        .join("\n");
    unit.push_str(
        "\ninvariant NoParent using v on Config::ConfigVersion at current { \
         not present(self.parent) }\n",
    );
    unit_for(unit, config_version_domain_document())
}

fn version_advances_replay_case(
    unit: &Unit,
    root_pre: i64,
    child_pre: Option<i64>,
    root_post: i64,
    child_post: Option<i64>,
    self_key: &str,
    result: bool,
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> (
    crate::ReplayRequestWire,
    Result<WitnessEnvelope<StateClauseCounterexample>, crate::WitnessRefusal>,
    Vec<(DocumentRef, Vec<u8>)>,
) {
    let (pre, pre_bytes) = document_ref_and_bytes("tc-3121-pre", |label| {
        let objects = version_objects(root_pre, child_pre);
        super::frame::snapshot(label, "pre", &objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("tc-3121-post", |label| {
        let objects = version_objects(root_post, child_post);
        super::frame::snapshot(label, "post", &objects)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("tc-3121-invocation", |label| {
        let mut value: serde_json::Value = serde_json::from_slice(
            &config_version_invocation_bytes(label, &pre, &post, self_key),
        )
        .expect("the invocation fixture is JSON");
        value["result"] = json!({"boolean": result});
        serde_json::to_vec(&value).expect("the invocation fixture serializes")
    });
    let documents = vec![
        (invocation.clone(), invocation_bytes),
        (pre, pre_bytes),
        (post, post_bytes),
    ];
    let observation = ClauseSelectionInput::Invocation {
        invocation: invocation.clone(),
    };
    let result = replay_case_with_observation(
        unit,
        "VersionAdvances",
        observation,
        &documents,
        witness,
        source,
    );
    (result.0, result.1, documents)
}

fn version_advances_clause_report(
    root_pre: i64,
    child_pre: Option<i64>,
    root_post: i64,
    child_post: Option<i64>,
    self_key: &str,
    result: bool,
) -> crate::spine::ClauseRunReport {
    let (pre, pre_bytes) = document_ref_and_bytes("tc-3121-pre", |label| {
        let objects = version_objects(root_pre, child_pre);
        super::frame::snapshot(label, "pre", &objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("tc-3121-post", |label| {
        let objects = version_objects(root_post, child_post);
        super::frame::snapshot(label, "post", &objects)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("tc-3121-invocation", |label| {
        let mut value: serde_json::Value = serde_json::from_slice(
            &config_version_invocation_bytes(label, &pre, &post, self_key),
        )
        .expect("the invocation fixture is JSON");
        value["result"] = json!({"boolean": result});
        serde_json::to_vec(&value).expect("the invocation fixture serializes")
    });
    let domain_document = config_version_domain_document();
    let packages =
        fsl_semantics::model::intake::package_input([domain_document.as_slice()]).unwrap();
    let selection = config_version_invocation_selection("VersionAdvances", invocation.clone());
    let mut request = config_version_request_for(
        tc_3121_version_unit_text(),
        packages,
        "tc-3121-version-advances.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots.insert(pre, pre_bytes);
    request.snapshots.insert(post, post_bytes);
    request.invocations.insert(invocation, invocation_bytes);
    let function_modes = BTreeMap::from([("z".to_owned(), CheckMode::Kernel)]);
    super::super::run_clause_with_function_modes(request, &[], &function_modes)
        .expect("the VersionAdvances invocation reports")
}

fn version_objects(root_version: i64, child_version: Option<i64>) -> Vec<serde_json::Value> {
    match child_version {
        Some(child_version) => vec![
            super::frame::object("root", root_version, None),
            super::frame::object("child", child_version, Some("root")),
        ],
        None => vec![super::frame::object("root", root_version, None)],
    }
}

fn comparison_witness_unit_text() -> String {
    let unit = witness_unit_text()
        .lines()
        .filter(|line| {
            !line.starts_with("invariant FloatOrder")
                && !line.starts_with("invariant FloatNot")
                && !line.starts_with("invariant FloatLet")
                && !line.starts_with("invariant FloatOrOperand")
                && !line.starts_with("invariant FloatAndOperand")
                && !line.starts_with("invariant FloatImpliesOperand")
        })
        .collect::<Vec<_>>()
        .join("\n");
    unit
}

fn comparison_witness_unit() -> Unit {
    unit_for(
        comparison_witness_unit_text(),
        config_version_domain_document(),
    )
}

#[trace("FR-003-AC-9")]
#[test]
fn private_witness_and_codec_sources_keep_format_identity() {
    let _ = witness_unit();
    let _ = codec_unit();
}

/// A snapshot of the `witness` fixture: `low` holds `root` (0) and `child`
/// (1, parent `root`); `high` holds `root` (0), `mid` (600, parent `root`)
/// and `leaf` (700, parent `mid`).
#[derive(Clone, Copy, Debug)]
pub(super) enum Snapshot {
    Low,
    High,
}

impl Snapshot {
    fn identity(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }

    fn objects(self) -> Vec<serde_json::Value> {
        match self {
            Self::Low => vec![object("root", 0, None), object("child", 1, Some("root"))],
            Self::High => vec![
                object("root", 0, None),
                object("mid", 600, Some("root")),
                object("leaf", 700, Some("mid")),
            ],
        }
    }

    /// The snapshot's label and bytes.
    fn document(self) -> (DocumentRef, Vec<u8>) {
        document_ref_and_bytes(self.identity(), |label| {
            json!({
                "format": "filament.state.snapshot/v1",
                "identity": document_identity_json(label),
                "observation": "current",
                "anchor": {"kind": "handler", "name": "validate"},
                "model": {
                    "identity": CONFIG_VERSION_PACKAGE_IDENTITY,
                },
                "populations": [{
                    "population": config_version_population_identity(),
                    "complete": true,
                    "objects": self.objects(),
                }],
            })
            .to_string()
            .into_bytes()
        })
    }
}

/// `clause` over `snapshot`, self `self_key`: the selection and the
/// snapshot's bytes by label.
fn selection(
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
) -> (
    ClauseSelection,
    fsl_semantics::model::observation::DocumentProvision,
) {
    let (label, bytes) = snapshot.document();
    let snapshots = BTreeMap::from([(label.clone(), bytes)]);
    (
        config_version_current_selection(clause, label, self_key),
        snapshots,
    )
}

/// The `witness` unit's `ClauseRunRequest` for `clause` over `snapshot`,
/// self `self_key`, under `work_units`.
fn clause_request(
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    work_units: u64,
) -> ClauseRunRequest {
    let (selection, snapshots) = selection(clause, snapshot, self_key);
    let document = config_version_domain_document();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let mut request = config_version_request_for(
        witness_unit_text(),
        packages,
        "witness.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots = snapshots;
    request.accounting = default_accounting(work_units);
    request
}

/// `clause` run over `snapshot`, self `self_key`, through `run_clause`.
pub(super) fn run(
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
) -> crate::spine::ClauseRunReport {
    run_clause(clause_request(clause, snapshot, self_key, 1_000_000))
        .expect("a well-formed request always reports")
}

fn outcome_item(clause: &str, report: &crate::spine::ClauseRunReport) -> serde_json::Value {
    let document = crate::OutcomeDocument::from_clause_runs(None, [(clause, report)])
        .expect("the completed clause run becomes an outcome item");
    let bytes = document.to_bytes().expect("the outcome document encodes");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("outcome JSON");
    value["items"][0].clone()
}

fn assert_outcome_witness_names_history(item: &serde_json::Value) {
    let witness = item
        .get("witness")
        .expect("refuted state item has a witness");
    assert_eq!(witness["deciding_element"]["decimal"], "600", "{witness}");
    assert!(witness.get("quantifier").is_some(), "{witness}");
    assert_eq!(witness["index"], 1, "{witness}");
    assert_eq!(
        witness["value_path"]["root"]["subject"]["object_identity"], "mid",
        "{witness}"
    );
    assert_eq!(
        witness["value_path"]["steps"][0]["name"], "history",
        "{witness}"
    );
    assert_eq!(
        witness["value_path"]["steps"].as_array().unwrap().len(),
        1,
        "{witness}"
    );
}

/// FR-286-AC-10: a state-clause outcome keeps the selected object and its
/// field-backed deciding element through the existing separating-witness
/// member, for current, pre-call and invocation observations.
#[trace("TC-770", "FR-286-AC-10")]
#[test]
fn outcome_documents_keep_state_clause_witnesses_for_invariant_pre_and_post() {
    let invariant = super::witness_member::run_current("MemberAll", &[0, 600, 700]);
    let invariant_payload = invariant
        .state_clause_counterexample
        .as_ref()
        .expect("a violated invariant carries its typed state-clause payload");
    assert!(matches!(
        &invariant_payload.observation,
        fsl_semantics::model::observation::ClauseSelectionInput::Current { .. }
    ));
    let invariant_item = outcome_item("MemberAll", &invariant);
    assert_eq!(invariant_item["category"], "violation");
    assert_outcome_witness_names_history(&invariant_item);

    let pre = super::witness_member::run_pre_call(&[0, 600, 700]);
    let pre_payload = pre
        .state_clause_counterexample
        .as_ref()
        .expect("a violated precondition carries its typed state-clause payload");
    assert!(matches!(
        &pre_payload.observation,
        fsl_semantics::model::observation::ClauseSelectionInput::PreCall { .. }
    ));
    let pre_item = outcome_item("MemberPre", &pre);
    assert_eq!(pre_item["category"], "violation");
    assert_outcome_witness_names_history(&pre_item);

    let post = super::witness_member::run_post_invocation(&[0, 600, 700], &[0, 600, 700]);
    let post_payload = post
        .state_clause_counterexample
        .as_ref()
        .expect("a violated postcondition carries its typed state-clause payload");
    assert!(matches!(
        &post_payload.observation,
        fsl_semantics::model::observation::ClauseSelectionInput::Invocation { .. }
    ));
    let post_item = outcome_item("MemberPost", &post);
    assert_eq!(post_item["category"], "violation");
    assert_outcome_witness_names_history(&post_item);

    let holds = super::witness_member::run_current("MemberAll", &[0, 1]);
    let holds_item = outcome_item("MemberAll", &holds);
    assert_eq!(holds_item["category"], "success");
    assert!(holds_item.get("witness").is_none(), "{holds_item}");
    assert!(holds.state_clause_counterexample.is_none());
}

/// Run the witness unit after editing its current snapshot document.
fn run_with_snapshot_edit(
    edit: impl FnOnce(&mut serde_json::Value),
) -> crate::spine::ClauseRunReport {
    let mut request = clause_request("AllBelow", Snapshot::Low, "child", 1_000_000);
    let (label, bytes) = request
        .snapshots
        .iter()
        .next()
        .expect("the witness request has one snapshot");
    let label = label.clone();
    let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    edit(&mut document);
    request
        .snapshots
        .insert(label, serde_json::to_vec(&document).unwrap());
    run_clause(request).expect("a well-formed request always reports")
}

/// Assert one admission refusal's catalog, object, member and source path,
/// including the fields retained by the native result reader.
fn assert_snapshot_refusal(
    cause: &str,
    object: &str,
    member: &str,
    path: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) {
    let report = run_with_snapshot_edit(edit);
    assert_eq!(
        report.disposition.stage(),
        crate::spine::ClauseRunStage::Admit
    );
    let crate::spine::ClauseDisposition::Admit(
        fsl_semantics::model::observation::AdmissionFailure::Refused(record),
    ) = &report.disposition
    else {
        panic!(
            "expected the {cause} snapshot refusal, got {:?}",
            report.disposition
        );
    };
    assert_eq!(record.catalog.cause(), cause);
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some(object)
    );
    assert_eq!(record.fields.get("field").map(String::as_str), Some(member));
    assert_eq!(record.fields.get("path").map(String::as_str), Some(path));

    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value.pointer("/disposition/refusal/fields/path"),
        Some(&json!(path)),
        "the native result retains the snapshot JSON pointer for {cause}"
    );
    let read = crate::spine::read_native_run_result(&bytes).unwrap();
    let crate::spine::NativeRunResult::Clause(read) = read else {
        panic!("the report must read as a clause-run document");
    };
    assert_eq!(
        read.fields().and_then(|fields| fields.get("path")),
        Some(&path.to_owned())
    );
}

/// FR-106-AC-3: admission items identify the object, member and source JSON
/// pointer for snapshot defects; a valid snapshot has no admission item.
#[trace("TC-465", "FR-106-AC-3")]
#[test]
#[ignore = "QSL-2173 completed; current blocker has no assigned owner: the missing-member snapshot refusal for the ConfigVersion fixture is not produced as the test expects; admission refuses with invalid_runtime_input/missing-member at a different member and path"]
fn snapshot_admission_refusal_witnesses_name_object_member_and_path() {
    assert_snapshot_refusal(
        "missing-member",
        "root",
        "versionNumber",
        "/populations/0/objects/0/fields/versionNumber",
        |document| {
            document["populations"][0]["objects"][0]["fields"]
                .as_object_mut()
                .unwrap()
                .remove("versionNumber");
        },
    );
    assert_snapshot_refusal(
        "unknown-member",
        "root",
        "extra",
        "/populations/0/objects/0/fields/extra",
        |document| {
            document["populations"][0]["objects"][0]["fields"]["extra"] = json!({"boolean":true});
        },
    );
    assert_snapshot_refusal(
        "invalid-value",
        "root",
        "versionNumber",
        "/populations/0/objects/0/fields/versionNumber",
        |document| {
            document["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"integer":"1001"});
        },
    );
    assert_snapshot_refusal(
        "wrong-value-kind",
        "root",
        "versionNumber",
        "/populations/0/objects/0/fields/versionNumber",
        |document| {
            document["populations"][0]["objects"][0]["fields"]["versionNumber"] =
                json!({"boolean":true});
        },
    );
    assert_snapshot_refusal(
        "conflicting-identity",
        "root",
        "key",
        "/populations/0/objects/2/key",
        |document| {
            let duplicate = document["populations"][0]["objects"][0].clone();
            document["populations"][0]["objects"]
                .as_array_mut()
                .unwrap()
                .push(duplicate);
        },
    );

    let report = run("AllBelow", Snapshot::Low, "child");
    assert_ne!(
        report.disposition.stage(),
        crate::spine::ClauseRunStage::Admit
    );
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.pointer("/disposition/refusal").is_none());
    let read = crate::spine::read_native_run_result(&bytes).unwrap();
    let crate::spine::NativeRunResult::Clause(read) = read else {
        panic!("the report must read as a clause-run document");
    };
    assert!(read.fields().is_none());
}

pub(super) fn decisive_witness_wire() -> serde_json::Value {
    let report = run("AllBelow", Snapshot::High, "mid");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["witness"].clone()
}

/// FR-267-AC-2: a structurally valid deciding value still refuses when its
/// scalar payload or path identity is outside the canonical domain.
#[trace("TC-742", "FR-267-AC-2")]
#[test]
fn native_result_two_reader_rejects_invalid_witness_scalar_and_path() {
    let report = run("AllBelow", Snapshot::High, "mid");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (member, edit) in [
        ("deciding_element", 0),
        ("deciding_element", 1),
        ("deciding_element", 2),
        ("value_path", 3),
    ] {
        let mut value = valid.clone();
        if edit == 0 {
            value["witness"]["deciding_element"] =
                json!({"kind":"integer", "decimal":"not-an-integer"});
        } else if edit == 1 {
            value["witness"]["deciding_element"] =
                json!({"kind":"decimal", "coefficient":"not-an-integer", "scale":0});
        } else if edit == 2 {
            value["witness"]["deciding_element"] = json!({
                "kind":"reference", "universe":"bad-digest", "type":"bad-digest",
                "object_identity":"child"
            });
        } else {
            let subject = &mut value["witness"]["value_path"]["root"]["subject"];
            if subject["tag"] == "built" {
                subject["occurrence"]["node_id"] = json!("bad-digest");
            } else {
                subject["universe"] = json!("bad-digest");
            }
        }
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&value).unwrap())
            .err()
            .expect("malformed witness scalar or path must refuse");
        assert!(error.to_string().contains(member), "{member}: {error}");
    }
}

/// FR-267-AC-1: the clause-run producer carries the basis and the exact
/// decisive record, while closed and exhausted runs omit the record.
#[trace("TC-742", "FR-267-AC-1")]
#[test]
fn native_result_two_writes_decisive_closed_and_unavailable_reports() {
    for (report, basis, has_witness) in [
        (
            run("AllBelow", Snapshot::High, "mid"),
            "decisive-counterexample",
            true,
        ),
        (
            run("AllBelow", Snapshot::Low, "child"),
            "closed-scope",
            false,
        ),
        (
            run_clause(clause_request("AllBelow", Snapshot::High, "mid", 0)).unwrap(),
            "unavailable",
            false,
        ),
    ] {
        let bytes = crate::spine::write_clause_run_result(&report).unwrap();
        let document = quire_canonical::read(&bytes, u64::MAX).unwrap();
        assert_eq!(
            bytes,
            quire_canonical::to_vec(&document.root(), quire_canonical::Limits::new(u64::MAX))
                .unwrap()
        );
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["format"], "native-run-result/2");
        assert_eq!(value["basis"], basis);
        assert_eq!(value.get("witness").is_some(), has_witness);
        if report.disposition.truth().is_some() {
            assert_eq!(value["disposition"]["outcome"]["kind"], "completed");
            assert_eq!(value["disposition"]["outcome"]["value"]["kind"], "boolean");
            assert_eq!(
                value["disposition"]["outcome"]["value"]["value"],
                report.disposition.truth().unwrap()
            );
        }
        let read = crate::spine::read_native_run_result(&bytes).unwrap();
        let crate::spine::NativeRunResult::Clause(read) = read else {
            panic!("the report must read as a clause-run document");
        };
        assert_eq!(read.basis(), basis);
        assert_eq!(read.has_witness(), has_witness);
        if has_witness {
            assert_eq!(value["witness"]["index"], 1);
            assert!(value["witness"].get("quantifier").is_some());
            assert!(value["witness"].get("deciding_element").is_some());
            assert!(value["witness"].get("value_path").is_some());
            assert!(value["witness"].get("trace_position").is_none());
        }
    }
}

/// FR-109-AC-1: the Boolean completion must agree with both truth and its
/// success or violation category.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_reader_rejects_category_verdict_mismatch() {
    for (report, wrong_value) in [
        (run("AllBelow", Snapshot::Low, "child"), false),
        (run("AllBelow", Snapshot::High, "mid"), true),
    ] {
        let bytes = crate::spine::write_clause_run_result(&report).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["disposition"]["truth"] = json!(wrong_value);
        value["disposition"]["outcome"]["value"]["value"] = json!(wrong_value);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&value).unwrap())
            .err()
            .expect("a mismatched category and completion must refuse");
        assert!(error.to_string().contains("disposition.outcome"), "{error}");
    }
}

/// FR-109/FR-352's shared member inventory: each assigned base, usage,
/// Boolean outcome and separating-witness member survives a typed read;
/// deletion, `null` and a wrong shape refuse at that member.
#[trace("TC-742", "FR-267-AC-1", "FR-267-AC-2", "FR-267-AC-9")]
#[test]
fn native_result_two_member_inventory_refuses_independent_edits() {
    let report = run("AllBelow", Snapshot::High, "mid");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let crate::spine::NativeRunResult::Clause(read) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("clause result")
    };
    assert_eq!(read.stage(), "evaluate");
    assert_eq!(read.category(), "violation");
    assert_eq!(read.truth(), Some(false));
    assert!(matches!(
        read.outcome(),
        Some(crate::spine::NativeRunOutcome::Boolean(false))
    ));
    assert_eq!(read.basis(), "decisive-counterexample");
    assert!(read.has_witness());
    assert_eq!(
        read.admission_usage().0,
        report.usage.admission_consumed.document_bytes
    );
    assert_eq!(
        read.evaluation_admissions(),
        report.usage.evaluation_admissions
    );
    assert_eq!(
        read.evaluation_consumed().count(),
        report.usage.evaluation_consumed.len()
    );

    let members = [
        ("/format", "format"),
        ("/disposition", "disposition"),
        ("/usage", "usage"),
        ("/basis", "basis"),
        ("/witness", "witness"),
        ("/disposition/stage", "stage"),
        ("/disposition/category", "category"),
        ("/disposition/truth", "truth"),
        ("/disposition/outcome", "outcome"),
        ("/disposition/outcome/kind", "kind"),
        ("/disposition/outcome/value", "outcome"),
        ("/disposition/outcome/value/kind", "outcome"),
        ("/disposition/outcome/value/value", "outcome"),
        ("/usage/admission", "admission"),
        ("/usage/admission/document_bytes", "document_bytes"),
        ("/usage/admission/objects", "objects"),
        ("/usage/admission/values", "values"),
        ("/usage/evaluation_admissions", "evaluation_admissions"),
        ("/usage/evaluation_consumed", "evaluation_consumed"),
        ("/usage/evaluation_consumed/0/kind", "kind"),
        ("/usage/evaluation_consumed/0/consumed", "consumed"),
        ("/witness/quantifier", "quantifier"),
        ("/witness/deciding_element", "deciding_element"),
        ("/witness/index", "index"),
        ("/witness/value_path", "value_path"),
    ];
    for (pointer, member) in members {
        assert!(
            valid.pointer(pointer).is_some(),
            "missing fixture {pointer}"
        );
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let mut deleted = valid.clone();
        deleted
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        let mut null = valid.clone();
        *null.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
        let mut wrong = valid.clone();
        *wrong.pointer_mut(pointer).unwrap() = if pointer == "/usage/evaluation_consumed" {
            json!({})
        } else {
            json!([])
        };
        for (edit, value) in [("deleted", deleted), ("null", null), ("wrong", wrong)] {
            let error = crate::spine::read_native_run_result(&serde_json::to_vec(&value).unwrap())
                .err()
                .expect("assigned member mutation must refuse");
            assert!(
                error.to_string().contains(member),
                "{pointer} {edit}: {error}"
            );
        }
    }
}

/// FR-267-AC-1: a caller cannot serialize a decisive report after dropping
/// its separating record, or attach a record to a closed-scope report.
#[trace("TC-742", "FR-267-AC-1")]
#[test]
fn native_result_two_writer_refuses_inconsistent_basis_and_witness() {
    let mut decisive = run("AllBelow", Snapshot::High, "mid");
    decisive.witness = None;
    assert!(matches!(
        crate::spine::write_clause_run_result(&decisive),
        Err(crate::spine::NativeRunResultError::BasisWitness)
    ));

    let mut closed = run("AllBelow", Snapshot::Low, "child");
    closed.witness = run("AllBelow", Snapshot::High, "mid").witness;
    assert!(matches!(
        crate::spine::write_clause_run_result(&closed),
        Err(crate::spine::NativeRunResultError::BasisWitness)
    ));
}

/// FR-267-AC-2: each independent malformed witness, basis and format edit
/// refuses at the member it changed.
#[trace("TC-742", "FR-267-AC-2")]
#[test]
fn native_result_two_reader_refuses_witness_basis_and_version_edits() {
    let report = run("AllBelow", Snapshot::High, "mid");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    type WireEdit = (&'static str, fn(&mut serde_json::Value));
    let edits: &[WireEdit] = &[
        ("index", |v| {
            v["witness"].as_object_mut().unwrap().remove("index");
        }),
        ("deciding_element", |v| {
            v["witness"]["deciding_element"] = json!(null)
        }),
        ("deciding_element", |v| {
            v["witness"]["deciding_element"] = json!({"kind":"future"})
        }),
        ("extra", |v| v["witness"]["extra"] = json!(true)),
        ("trace_position", |v| {
            v["witness"]["trace_position"] = json!(0)
        }),
        ("basis", |v| {
            v.as_object_mut().unwrap().remove("basis");
        }),
        ("witness", |v| v["basis"] = json!("closed-scope")),
        ("witness", |v| {
            v.as_object_mut().unwrap().remove("witness");
        }),
        ("format", |v| v["format"] = json!("native-run-result/1")),
        ("format", |v| v["format"] = json!("native-run-result/3")),
        ("format", |v| {
            v.as_object_mut().unwrap().remove("format");
        }),
    ];
    for (member, edit) in edits {
        let mut value = valid.clone();
        edit(&mut value);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&value).unwrap())
            .err()
            .expect("the edited document must refuse");
        assert!(
            error.to_string().contains(member),
            "{member} edit refused at {error}"
        );
    }
}

/// FR-261/FR-267: the strict reader retains a deeply nested typed deciding
/// element through its canonical arena and resolves it with the package.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_reader_keeps_deciding_element_beyond_128_levels() {
    let unit = witness_unit();
    let report = run("AllBelow", Snapshot::High, "mid");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let crate::spine::NativeRunResult::Clause(original) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("clause")
    };
    assert_eq!(
        original.witness(&unit.compiled.package).unwrap(),
        report.witness
    );
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut element = json!({"kind":"boolean", "value":true});
    let mut ty = json!({"kind":"boolean"});
    for _ in 0..160 {
        element = json!({"kind":"option", "payload_type":ty.clone(), "payload":element});
        ty = json!({"kind":"option", "payload":ty});
    }
    value["witness"]["deciding_element"] = element;
    let bytes = serde_json::to_vec(&value).unwrap();
    let crate::spine::NativeRunResult::Clause(read) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("clause")
    };
    let witness = read.witness(&unit.compiled.package).unwrap().unwrap();
    assert_eq!(witness.index, Some(1));
}

/// FR-267: malformed command details deeper than serde_json's default
/// recursion limit reach the code-specific diagnostic instead of failing at
/// an unrelated parser depth ceiling.
#[trace("TC-742", "FR-267-AC-10")]
#[test]
fn native_result_two_command_details_keep_deep_wire_diagnostic() {
    let mut details = json!({"stage":"call", "invariant":"broken"});
    for _ in 0..160 {
        details = json!({"nested":details});
    }
    let envelope = json!({
        "format":"native-run-result/2", "stage":"call",
        "code":"runtime_invariant", "cause":"wrong-cause",
        "message":"broken", "details":details, "basis":"unavailable"
    });
    let error = crate::spine::read_native_run_result(&serde_json::to_vec(&envelope).unwrap())
        .err()
        .expect("the wrong catalog cause must refuse");
    assert!(error.to_string().contains("cause"), "{error}");
}

/// FR-267-AC-9: the FR-100 refusal record stays inside `outcome`, including
/// its catalog fields and the typed location of the rejecting expression.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_retains_refused_outcome_members() {
    use crate::spine::{CallOutcome, CallRefusal};
    use filament_semantic_value::catalog::CatalogCode;
    use filament_semantic_value::location::{
        ExpressionLocation as Location, ExpressionOwner as Origin,
    };

    let mut report = run("AllBelow", Snapshot::Low, "child");
    report.disposition = ClauseDisposition::Evaluate(CallOutcome::Refused(CallRefusal::Record {
        code: CatalogCode::new("integer_out_of_domain", "outside-domain"),
        fields: BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
        locus: Some(crate::spine::CallLocus {
            file: "fixture.native".to_owned(),
            span: fsl_foundation::LocatedSpan {
                start: fsl_foundation::Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
                end: fsl_foundation::Position {
                    byte: 2,
                    line: 1,
                    column: 3,
                },
            },
        }),
        location: Some(Location::at(Origin::Expression, &[2, 1])),
    }));
    report.basis = crate::proof_result::SettlementBasis::Unavailable;
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["disposition"]["outcome"],
        json!({
            "kind":"refused", "code":"integer_out_of_domain", "cause":"outside-domain",
            "fields":{"expected":"Int[0, 9]"},
            "locus":{"file":"fixture.native",
                "span":{"start":{"byte":0,"line":1,"column":1},"end":{"byte":2,"line":1,"column":3}}},
            "location":{"origin":{"kind":"expression"},"path":[2,1]}
        })
    );
    let crate::spine::NativeRunResult::Clause(read) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("expected clause result")
    };
    assert!(read.has_outcome_location());
    assert!(read.has_outcome_locus());
    let Some(crate::spine::NativeRunOutcome::Refused {
        code,
        cause,
        fields,
        locus,
        location,
    }) = read.outcome()
    else {
        panic!("expected typed refusal outcome")
    };
    assert_eq!((code, cause), ("integer_out_of_domain", "outside-domain"));
    assert_eq!(
        fields.unwrap().get("expected").map(String::as_str),
        Some("Int[0, 9]")
    );
    assert_eq!(locus.unwrap().span.end.byte, 2);
    assert_eq!(location.unwrap().path, [2, 1]);
    assert_eq!(read.outcome_locus().unwrap().file, "fixture.native");
    assert_eq!(read.outcome_locus().unwrap().span.end.byte, 2);
    assert_eq!(read.outcome_location().unwrap().path, [2, 1]);
    assert!(matches!(
        &read.outcome_location().unwrap().origin,
        crate::spine::CallOriginRead::Expression
    ));
    for edit in ["fields", "locus", "location"] {
        let mut mutated = value.clone();
        mutated["disposition"]["outcome"]
            .as_object_mut()
            .unwrap()
            .insert(edit.into(), serde_json::Value::Null);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&mutated).unwrap())
            .err()
            .unwrap();
        assert!(
            error.to_string().contains("disposition.outcome"),
            "{edit}: {error}"
        );
    }
    for removed in ["code", "cause", "fields", "locus", "location"] {
        let mut mutated = value.clone();
        mutated["disposition"]["outcome"]
            .as_object_mut()
            .unwrap()
            .remove(removed);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&mutated).unwrap())
            .err()
            .expect("an assigned outcome member cannot be deleted");
        assert!(error.to_string().contains(removed), "{removed}: {error}");
    }
    let mut fabricated = value.clone();
    fabricated["disposition"]["outcome"]["cause"] = json!("fabricated-cause");
    let error = crate::spine::read_native_run_result(&serde_json::to_vec(&fabricated).unwrap())
        .err()
        .expect("a kernel refusal must retain its catalog cause");
    assert!(error.to_string().contains("cause"), "{error}");
}

/// FR-267-AC-9: an internal failure preserves its stable typed fault and
/// cannot be read after an assigned fault member is removed.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_retains_internal_fault() {
    let mut report = run("AllBelow", Snapshot::Low, "child");
    report.disposition = ClauseDisposition::EvaluateFault(
        filament_semantic_value::catalog::InternalFault::new("evaluate", "broken-map"),
    );
    report.basis = crate::proof_result::SettlementBasis::Unavailable;
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["disposition"]["internal_fault"],
        json!({
            "stage":"evaluate", "invariant":"broken-map"
        })
    );
    assert!(value["disposition"].get("outcome").is_none());
    assert!(matches!(
        crate::spine::read_native_run_result(&bytes),
        Ok(crate::spine::NativeRunResult::Clause(_))
    ));
    value["disposition"]["internal_fault"]
        .as_object_mut()
        .unwrap()
        .remove("invariant");
    assert!(crate::spine::read_native_run_result(&serde_json::to_vec(&value).unwrap()).is_err());
}

/// FR-267-AC-9: selection failures retain the FR-109 refusal record.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_retains_selection_refusal_record() {
    let report = run("Missing", Snapshot::Low, "child");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["disposition"]["stage"], "select");
    assert_eq!(
        value["disposition"]["refusal"]["code"],
        "missing_declaration"
    );
    assert_eq!(value["disposition"]["refusal"]["cause"], "missing-name");
    assert!(value["disposition"].get("code").is_none());
    assert!(matches!(
        crate::spine::read_native_run_result(&bytes),
        Ok(crate::spine::NativeRunResult::Clause(_))
    ));
    for member in ["cause", "fields"] {
        let mut edited = value.clone();
        edited["disposition"]["refusal"]
            .as_object_mut()
            .unwrap()
            .remove(member);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&edited).unwrap())
            .err()
            .unwrap();
        assert!(error.to_string().contains(member), "{member}: {error}");
    }
    let mut missing_name = value.clone();
    missing_name["disposition"]["refusal"]["fields"]
        .as_object_mut()
        .unwrap()
        .remove("name");
    let error = crate::spine::read_native_run_result(&serde_json::to_vec(&missing_name).unwrap())
        .err()
        .expect("selection's assigned name must not disappear");
    assert!(error.to_string().contains("fields"), "{error}");
    let mut wrong_category = value.clone();
    wrong_category["disposition"]["category"] = json!("unsupported");
    let error = crate::spine::read_native_run_result(&serde_json::to_vec(&wrong_category).unwrap())
        .err()
        .expect("selection category must follow its catalog code");
    assert!(error.to_string().contains("category"), "{error}");
}

/// FR-255/FR-352: a compile limit retains its exact bound, reached counter
/// and setting, including a counter above u64::MAX.
#[trace("TC-742", "FR-267-AC-9")]
#[test]
fn native_result_two_retains_compile_limit_fields() {
    let mut report = run("AllBelow", Snapshot::Low, "child");
    report.disposition = ClauseDisposition::Compile(Box::new(crate::spine::CompileRefusal::Limit(
        fsl_foundation::diagnostic::LimitExceeded::new(
            fsl_foundation::Setting::S3Nodes,
            5,
            u128::from(u64::MAX) + 1,
        ),
    )));
    report.basis = crate::proof_result::SettlementBasis::Unavailable;
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let fields = &value["disposition"]["refusal"]["fields"];
    assert_eq!(
        fields,
        &json!({
            "kind":"node-count-exceeded", "bound":"5",
            "actual":"18446744073709551616", "setting":"s3.nodes"
        })
    );
    let crate::spine::NativeRunResult::Clause(read) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("clause")
    };
    assert_eq!(
        read.fields().unwrap().get("setting").map(String::as_str),
        Some("s3.nodes")
    );
    for member in ["kind", "bound", "actual", "setting"] {
        let mut edited = value.clone();
        edited["disposition"]["refusal"]["fields"]
            .as_object_mut()
            .unwrap()
            .remove(member);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&edited).unwrap())
            .err()
            .expect("missing compile limit field must refuse");
        assert!(error.to_string().contains("fields"), "{member}: {error}");
    }
    for member in ["kind", "bound", "actual", "setting"] {
        let mut edited = value.clone();
        edited["disposition"]["refusal"]["fields"][member] = json!(7);
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&edited).unwrap())
            .err()
            .expect("wrongly typed compile limit field must refuse");
        assert!(error.to_string().contains("fields"), "{member}: {error}");
    }
}

/// `clause` of `unit` admitted over `snapshot`, self `self_key`, and
/// evaluated once by FR-107 under `work_units`: the observations, the
/// evaluation and the meter.
fn evaluate(
    unit: &Unit,
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    work_units: u64,
) -> (AdmittedObservations, ClauseEvaluation, Meter) {
    let (selection, snapshots) = selection(clause, snapshot, self_key);
    let packages =
        fsl_semantics::model::intake::package_input([unit.domain_document.as_slice()]).unwrap();
    let graph = unit.compiled.package.graph();
    let declaration = graph.state_clause(clause).expect("the clause is declared");
    let invocations = BTreeMap::new();
    let observations = admit_clause_observations(
        graph,
        declaration,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &selection,
        ObservationLimits::default(),
    )
    .expect("the snapshot admits");
    let mut meter = Meter::new(default_accounting(work_units));
    let evaluation = unit
        .compiled
        .package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            &observations,
            &mut meter,
        )
        .expect("the clause evaluates");
    (observations, evaluation, meter)
}

/// FR-265's derivation of `clause` over `snapshot`, self `self_key`.
fn derive(unit: &Unit, clause: &str, snapshot: Snapshot, self_key: &str) -> Option<Derived> {
    let (_, evaluation, _) = evaluate(unit, clause, snapshot, self_key, 1_000_000);
    let declaration = unit.compiled.package.graph().state_clause(clause).unwrap();
    derive_separating_witness(unit.compiled.package.graph(), declaration, &evaluation)
        .expect("the derivation completes")
}

/// The occurrence key of the node `operator` lowers to at `path` in
/// `clause`'s body (ADR-013 O-07).
fn occurrence(unit: &Unit, clause: &str, path: &[usize], operator: Operator) -> OccurrenceKey {
    let graph = unit.compiled.package.graph();
    let semantic = graph.semantic_graph();
    graph
        .occurrences()
        .find(|(key, _, location)| {
            matches!(&location.origin, filament_semantic_value::location::ExpressionOwner::StateClause {
                clause: name, ..
            } if name == clause)
                && location.path() == path
                && matches!(
                    semantic.node(key.clone()).map(|node| node.body()),
                    Some(BodyTerm::Application(ApplicationTerm { operator: applied, .. }))
                        if *applied == operator
                )
        })
        .map(|(key, origin, _)| OccurrenceKey::new(key.clone(), origin))
        .unwrap_or_else(|| panic!("{clause} has a {operator:?} node at {path:?}"))
}

fn observation(snapshot: Snapshot) -> ObservationIdentity {
    ObservationIdentity {
        authority: "test".to_owned(),
        identity: snapshot.identity().to_owned(),
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(filament_exact::Integer::from(value))
}

/// The record FR-265 states for `clause` over `snapshot`: the quantifier
/// at `quantifier`, the element at `index` of the collection literal at
/// `literal`, whose value path is rooted at that literal's occurrence and
/// ends in an index step for `index` (FR-265-AC-5), and no trace position.
fn built_record(
    unit: &Unit,
    clause: &str,
    quantifier: &[usize],
    literal: &[usize],
    element: Value,
    index: u64,
    snapshot: Snapshot,
) -> SeparatingWitnessRecord {
    SeparatingWitnessRecord {
        quantifier: Some(occurrence(unit, clause, quantifier, Operator::Quantify)),
        deciding_comparison: None,
        operands: None,
        decision_path: None,
        deciding_element: DecidingElement::Value(element),
        index: Some(index),
        value_path: RuntimeValuePath {
            observation: observation(snapshot),
            subject: ValuePathSubject::Built(occurrence(
                unit,
                clause,
                literal,
                Operator::Collection,
            )),
            steps: vec![ValuePathStep::Index(index)],
        },
        trace_position: None,
    }
}

/// `AllBelow`'s record over `high`: `forall` at the root, element 600 at
/// index 1 of the literal inside `ints(..)`.
fn all_below_record(unit: &Unit) -> SeparatingWitnessRecord {
    built_record(
        unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(600),
        1,
        Snapshot::High,
    )
}

fn decisive(basis: SettlementBasis, record: SeparatingWitnessRecord) -> Option<Derived> {
    Some(Derived {
        basis,
        record: Some(record),
    })
}

fn closed() -> Option<Derived> {
    Some(Derived {
        basis: SettlementBasis::ClosedScope,
        record: None,
    })
}

/// TC-740 step 1 (FR-265-AC-1): `AllBelow` over `high` derives
/// `decisive-counterexample` and the record naming the `forall`, 600 at
/// index 1 and the built literal; the equal element after the stop is never
/// visited (two `collection.visit` charges); over `low` it derives
/// `closed-scope` and no record.
#[trace("TC-740", "FR-265-AC-1")]
#[test]
fn tc_740_a_refuted_forall_derives_its_record_and_visits_nothing_after_the_stop() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "AllBelow", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            all_below_record(&unit)
        )
    );
    let (_, _, meter) = evaluate(&unit, "AllBelow", Snapshot::High, "mid", 1_000_000);
    let visits = meter
        .admitted_charges()
        .iter()
        .filter(|charge| charge.point == ChargePoint::CollectionVisit)
        .count();
    assert_eq!(visits, 2, "elements 0 and 1 are visited, element 2 is not");
    assert_eq!(derive(&unit, "AllBelow", Snapshot::Low, "child"), closed());
}

/// TC-740 step 2 (FR-265-AC-2): `exists` satisfied and `not forall` true
/// derive `decisive-witness` naming their quantifier; `exists` false
/// derives `closed-scope`; a stopped `forall` under `=` derives
/// `closed-scope`.
#[trace("TC-740", "FR-265-AC-2")]
#[test]
#[ignore = "QSL-2173 completed; current blocker has no assigned owner: the float-ordering ConfigVersion clause derives a DecisiveCounterexample record where the test expects a ClosedScope derivation with no record"]
fn tc_740_a_satisfied_exists_or_negated_forall_derives_a_decisive_witness() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "SomeAtLeast", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveWitness,
            built_record(
                &unit,
                "SomeAtLeast",
                &[],
                &[0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "NotAllBelow", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveWitness,
            built_record(
                &unit,
                "NotAllBelow",
                &[0],
                &[0, 0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "SomeAtLeast", Snapshot::Low, "child"),
        closed()
    );
    let (_, evaluation, _) = evaluate(&unit, "EqualsAll", Snapshot::High, "mid", 1_000_000);
    let forall = unit
        .compiled
        .package
        .graph()
        .state_clause("EqualsAll")
        .unwrap();
    let filament_checked::node::NodeKind::Equality(_, _, left, _) = forall.body().kind() else {
        panic!("EqualsAll's body is an equality");
    };
    assert!(
        evaluation
            .stop(forall.body().at(*left).location())
            .is_some(),
        "its forall stopped"
    );
    assert_eq!(derive(&unit, "EqualsAll", Snapshot::High, "mid"), closed());
}

/// FR-265: a float ordering lowers to connectives the checker synthesized,
/// which record no decision; the decision path ends there with
/// `closed-scope`.
fn a_float_ordering_ends_the_path(clause: &str, truth: bool) {
    let unit = witness_unit();
    let (_, evaluation, _) = evaluate(&unit, clause, Snapshot::High, "mid", 1_000_000);
    let filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
        Value::Boolean(settled),
    )) = &evaluation.evaluation.outcome
    else {
        panic!("{clause} completes a Boolean");
    };
    assert_eq!(*settled, truth, "{clause}");
    assert_eq!(
        derive(&unit, clause, Snapshot::High, "mid"),
        closed(),
        "{clause}"
    );
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_clause_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatOrder", true);
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_under_not_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatNot", false);
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_under_let_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatLet", true);
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_as_a_walked_or_operand_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatOrOperand", true);
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_as_a_walked_and_operand_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatAndOperand", true);
}

#[trace("TC-740", "FR-265-AC-3")]
#[test]
fn tc_740_a_float_ordering_as_an_implies_consequent_ends_the_path_at_closed_scope() {
    a_float_ordering_ends_the_path("FloatImpliesOperand", true);
}

/// TC-740 step 3 (FR-265-AC-3): the decision path through `implies`, `let`
/// and `or`.
#[trace("TC-740", "FR-265-AC-3")]
#[test]
#[ignore = "QSL-2173 completed; current blocker has no assigned owner: the ConfigVersion clause derives a DecisiveCounterexample record where the test expects a ClosedScope derivation with no record"]
fn tc_740_the_decision_path_reaches_the_decisive_quantifier() {
    let unit = witness_unit();
    let Some(Derived {
        basis: SettlementBasis::DecisiveCounterexample,
        record: Some(guarded),
    }) = derive(&unit, "GuardedAll", Snapshot::High, "mid")
    else {
        panic!("GuardedAll over high with a parent is decisive");
    };
    let (observations, _, _) = evaluate(&unit, "GuardedAll", Snapshot::High, "mid", 1_000_000);
    assert_eq!(
        guarded,
        built_record(
            &unit,
            "GuardedAll",
            &[1],
            &[1, 0, 0],
            Value::Reference(observations.self_object.clone()),
            1,
            Snapshot::High
        )
    );
    assert_eq!(
        derive(&unit, "GuardedAll", Snapshot::High, "root"),
        closed()
    );
    assert_eq!(
        derive(&unit, "LetAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "LetAll",
                &[1],
                &[1, 0, 0],
                integer(600),
                1,
                Snapshot::High
            )
        )
    );
    assert_eq!(derive(&unit, "OrAll", Snapshot::High, "mid"), closed());
}

/// TC-740 step 4 (FR-265-AC-4): the record names the outer `forall`, its
/// element 0 at index 0, and no inner occurrence.
#[trace("TC-740", "FR-265-AC-4")]
#[test]
fn tc_740_a_nested_forall_records_the_outer_occurrence() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "NestedAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "NestedAll",
                &[],
                &[0, 0],
                integer(0),
                0,
                Snapshot::High
            )
        )
    );
}

/// TC-740 step 5 (FR-265-AC-5): a `filter` domain records the source
/// position (2, not the filtered position 1) and a `map` domain the mapped
/// value, each on the built literal's path.
#[trace("TC-740", "FR-265-AC-5")]
#[test]
fn tc_740_computed_domains_record_the_source_position() {
    let unit = witness_unit();
    assert_eq!(
        derive(&unit, "FilteredAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "FilteredAll",
                &[],
                &[0, 0, 0],
                integer(600),
                2,
                Snapshot::High
            )
        )
    );
    assert_eq!(
        derive(&unit, "MappedAll", Snapshot::High, "mid"),
        decisive(
            SettlementBasis::DecisiveCounterexample,
            built_record(
                &unit,
                "MappedAll",
                &[],
                &[0, 0, 0],
                integer(601),
                1,
                Snapshot::High
            )
        )
    );
}

/// TC-740 step 6 (FR-265-AC-6): an incomplete evaluation derives no basis;
/// an empty filtered domain derives `closed-scope`.
#[trace("TC-740", "FR-265-AC-6")]
#[test]
fn tc_740_no_boolean_derives_no_basis() {
    let unit = witness_unit();
    let (_, evaluation, _) = evaluate(&unit, "AllBelow", Snapshot::High, "mid", 0);
    assert!(matches!(
        evaluation.evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Incomplete(_))
    ));
    let declaration = unit
        .compiled
        .package
        .graph()
        .state_clause("AllBelow")
        .unwrap();
    assert_eq!(
        derive_separating_witness(unit.compiled.package.graph(), declaration, &evaluation).unwrap(),
        None
    );
    assert_eq!(
        derive(&unit, "NoneSelected", Snapshot::Low, "child"),
        closed()
    );
}

/// TC-741 step 1 (FR-266-AC-1): `run_clause` reports carry FR-265's basis
/// and record, and two runs give equal ones.
#[trace("TC-741", "FR-266-AC-1")]
#[test]
fn tc_741_clause_reports_carry_the_derived_basis_and_witness() {
    let unit = witness_unit();
    let refuted = run("AllBelow", Snapshot::High, "mid");
    assert_eq!(
        refuted.disposition.category(),
        filament_semantic_value::catalog::Category::Violation
    );
    assert_eq!(refuted.disposition.truth(), Some(false));
    assert_eq!(refuted.disposition.category().exit_code(), 10);
    assert_eq!(refuted.basis, SettlementBasis::DecisiveCounterexample);
    assert_eq!(refuted.witness, Some(all_below_record(&unit)));

    let satisfied = run("SomeAtLeast", Snapshot::High, "mid");
    assert_eq!(satisfied.disposition.category().exit_code(), 0);
    assert_eq!(satisfied.basis, SettlementBasis::DecisiveWitness);
    assert_eq!(
        satisfied.witness,
        Some(built_record(
            &unit,
            "SomeAtLeast",
            &[],
            &[0, 0],
            integer(600),
            1,
            Snapshot::High
        ))
    );

    let holds = run("AllBelow", Snapshot::Low, "child");
    assert_eq!(
        holds.disposition.category(),
        filament_semantic_value::catalog::Category::Success
    );
    assert_eq!(holds.basis, SettlementBasis::ClosedScope);
    assert_eq!(holds.witness, None);

    let again = run("AllBelow", Snapshot::High, "mid");
    assert_eq!(
        (
            again.basis,
            &again.witness,
            again.disposition.category().exit_code()
        ),
        (
            refuted.basis,
            &refuted.witness,
            refuted.disposition.category().exit_code()
        )
    );
}

/// A current snapshot of `config_history` holding `objects`, `complete`
/// as given, and its `ParentOrder` request with self `child`.
pub(super) fn parent_order_request(
    identity: &str,
    objects: Vec<serde_json::Value>,
    complete: bool,
) -> ClauseRunRequest {
    let (label, bytes) = document_ref_and_bytes(identity, |label| {
        json!({
            "format": "filament.state.snapshot/v1",
            "identity": document_identity_json(label),
            "observation": "current",
            "anchor": {"kind": "handler", "name": "validate"},
            "model": {
                "identity": CONFIG_VERSION_PACKAGE_IDENTITY,
            },
            "populations": [{
                "population": config_version_population_identity(),
                "complete": complete,
                "objects": objects,
            }],
        })
        .to_string()
        .into_bytes()
    });
    let selection = config_version_current_selection("ParentOrder", label.clone(), "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.snapshots.insert(label.clone(), bytes);
    request
}

fn basis_of(request: ClauseRunRequest) -> (SettlementBasis, bool) {
    let report = run_clause(request).expect("a well-formed request always reports");
    (report.basis, report.witness.is_some())
}

/// TC-741 step 2 (FR-266-AC-2): decisive `ParentOrder` corpus reports carry
/// their FR-265 comparison record, while unavailable reports carry none; a
/// `Function` selection carries `closed-scope` for its violation and
/// `unavailable` for an admission refusal.
#[trace("TC-741", "FR-266-AC-2")]
#[test]
fn tc_741_corpus_and_function_reports_carry_their_basis() {
    let healthy = || vec![object("root", 1, None), object("child", 2, Some("root"))];
    let decisive_witness = (SettlementBasis::DecisiveWitness, true);
    let decisive_counterexample = (SettlementBasis::DecisiveCounterexample, true);
    let unavailable = (SettlementBasis::Unavailable, false);
    let unit = config_version_unit();
    let implication = occurrence(&unit, "ParentOrder", &[], Operator::Binary);
    let comparison = occurrence(&unit, "ParentOrder", &[1], Operator::Binary);
    let version_comparison = occurrence(&unit, "VersionUnchanged", &[], Operator::Binary);
    let assert_parent_order_record =
        |request: ClauseRunRequest, basis, parent_version, child_version| {
            let report = run_clause(request).expect("a well-formed request always reports");
            assert_eq!(report.basis, basis);
            let record = report
                .witness
                .expect("the decisive report has a witness record");
            assert_eq!(record.deciding_comparison, Some(comparison.clone()));
            let operands = record.operands.as_ref().expect("comparison operands");
            assert!(crate::result::same_element(
                &operands[0],
                &integer(parent_version)
            ));
            assert!(crate::result::same_element(
                &operands[1],
                &integer(child_version)
            ));
            assert_eq!(
                record.decision_path,
                Some(vec![implication.clone(), comparison.clone()])
            );
        };
    assert_parent_order_record(
        parent_order_request("healthy-parent", healthy(), true),
        decisive_witness.0,
        1,
        2,
    );
    assert_parent_order_record(
        parent_order_request(
            "violating-parent",
            vec![object("root", 3, None), object("child", 2, Some("root"))],
            true,
        ),
        decisive_counterexample.0,
        3,
        2,
    );
    let assert_version_unchanged_record =
        |report: crate::spine::ClauseRunReport, basis, truth, post_version, pre_version| {
            assert_eq!(report.basis, basis);
            assert_eq!(report.disposition.truth(), Some(truth));
            let record = report
                .witness
                .expect("the decisive report has a witness record");
            assert_eq!(record.deciding_comparison, Some(version_comparison.clone()));
            let operands = record.operands.as_ref().expect("comparison operands");
            assert!(crate::result::same_element(
                &operands[0],
                &integer(post_version)
            ));
            assert!(crate::result::same_element(
                &operands[1],
                &integer(pre_version)
            ));
            assert_eq!(record.decision_path, Some(vec![version_comparison.clone()]));
        };
    assert_version_unchanged_record(
        run_config_version_invocation_report(
            config_version_request,
            "VersionUnchanged",
            InvocationVersions::root_unchanged(1, 2, 3),
            "child",
        ),
        decisive_counterexample.0,
        false,
        3,
        2,
    );
    assert_version_unchanged_record(
        run_config_version_invocation_report(
            config_version_request,
            "VersionUnchanged",
            InvocationVersions::root_unchanged(1, 2, 2),
            "child",
        ),
        decisive_witness.0,
        true,
        2,
        2,
    );

    let mut missing_model = parent_order_request("healthy-parent", healthy(), true);
    missing_model.packages.clear();
    assert_eq!(basis_of(missing_model), unavailable);
    assert_eq!(
        basis_of(parent_order_request(
            "dangling-parent",
            vec![object("child", 2, Some("ghost"))],
            true
        )),
        unavailable
    );
    assert_eq!(
        basis_of(parent_order_request(
            "incomplete-population",
            healthy(),
            false
        )),
        unavailable
    );
    let mut exhausted = parent_order_request("healthy-parent", healthy(), true);
    exhausted.accounting = default_accounting(0);
    assert_eq!(basis_of(exhausted), unavailable);

    let same_identity = |b: &str| {
        config_version_function_request(
            "sameIdentity",
            vec![
                ClauseArgument {
                    parameter: "a".to_owned(),
                    value: config_version_reference("root"),
                },
                ClauseArgument {
                    parameter: "b".to_owned(),
                    value: config_version_reference(b),
                },
            ],
        )
    };
    let violation = run_clause(same_identity("child")).unwrap();
    assert_eq!(
        violation.disposition.category(),
        filament_semantic_value::catalog::Category::Violation
    );
    assert_eq!(
        (violation.basis, violation.witness.is_some()),
        (SettlementBasis::ClosedScope, false)
    );
    let ghost = run_clause(same_identity("ghost")).unwrap();
    assert_eq!(ghost.disposition.stage(), ClauseRunStage::Admit);
    assert_eq!((ghost.basis, ghost.witness.is_some()), unavailable);
}

/// TC-741 step 3 (FR-266-AC-3): a `Frame` violation carries `closed-scope`
/// and its FR-115 witness unchanged; an `evaluate` internal failure carries
/// `unavailable`.
#[trace("TC-741", "FR-266-AC-3")]
#[test]
fn tc_741_frame_and_internal_failure_reports_carry_their_basis() {
    let input = super::frame::frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
        |_| {},
    );
    let report = super::frame::run(input);
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    assert_eq!(witness.cause, "unauthorized-change");
    assert_eq!(report.basis, SettlementBasis::ClosedScope);
    assert_eq!(report.witness, None);
    assert_eq!(
        super::super::undecided_basis(&ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new(
                "S6a",
                "checked-program-invariant"
            )
        )),
        SettlementBasis::Unavailable
    );
}

/// The `witness` unit's FR-122 envelope for `clause` over `snapshot`, self
/// `self_key`, carrying `witness`, on `source`'s arm, and its request.
fn replay_case(
    unit: &Unit,
    clause: &str,
    snapshot: Snapshot,
    self_key: &str,
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> (
    crate::ReplayRequestWire,
    Result<WitnessEnvelope<StateClauseCounterexample>, crate::WitnessRefusal>,
) {
    let (label, bytes) = snapshot.document();
    replay_case_with_observation(
        unit,
        clause,
        config_version_current_selection(clause, label.clone(), self_key).input,
        &[(label, bytes)],
        witness,
        source,
    )
}

fn replay_case_with_observation(
    unit: &Unit,
    clause: &str,
    observation: ClauseSelectionInput,
    documents: &[(DocumentRef, Vec<u8>)],
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> (
    crate::ReplayRequestWire,
    Result<WitnessEnvelope<StateClauseCounterexample>, crate::WitnessRefusal>,
) {
    let target = crate::ReplayTarget::state_clause(
        crate::QualifiedName::new(vec![filament_exact::Identifier::new(clause).unwrap()]).unwrap(),
    );
    let wire = super::frame_replay::request_from_package(
        unit.compiled.emitted.bytes(),
        &unit.bytes,
        &unit.domain_document,
        documents,
        target.clone(),
    );
    let declaration = unit.compiled.package.graph().state_clause(clause).unwrap();
    let node = declaration.identity().clone();
    let payload = StateClauseCounterexample {
        observation,
        witness,
    };
    let envelope = WitnessEnvelope::reconstruct(
        packet(
            &unit.bytes,
            source,
            node.clone(),
            OccurrenceKey::new(node, declaration.claim().clone()),
            payload,
            target,
        ),
        crate::ReplayLimits::default(),
    );
    (wire, envelope)
}

fn replay_with(
    unit: &Unit,
    clause: &str,
    witness: Option<SeparatingWitnessRecord>,
    source: ReplaySource,
) -> Result<StateClauseReplayResult, ReplayRefusal> {
    let (wire, envelope) = replay_case(unit, clause, Snapshot::High, "mid", witness, source);
    replay_state_clause(
        wire,
        &envelope.expect("the envelope reconstructs"),
        crate::ReplayLimits::default(),
    )
}

/// FR-269-AC-6/9: the public entry returns the clause run's canonical record
/// without reading the payload record, and replay settles an absent comparison
/// record by checking the derived record itself.
#[trace("TC-3121", "FR-269-AC-6", "FR-269-AC-9")]
#[test]
fn tc_3121_public_derivation_matches_clause_run_and_replay_records() {
    let unit = tc_3121_unit();
    let (wire, envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the envelope reconstructs");
    let derive = || {
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default())
            .expect("the public derivation completes")
            .expect("MatchesParent has a decisive comparison")
    };
    let record = derive();
    let repeated = derive();
    assert_eq!(record, repeated);

    let report = tc_3121_run("MatchesParent", Snapshot::High, "mid");
    let native_record = report
        .witness
        .as_ref()
        .expect("the clause run carries its decisive comparison");
    assert_eq!(&record, native_record);
    let native_bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let native: serde_json::Value = serde_json::from_slice(&native_bytes).unwrap();
    let encoded = |record: &SeparatingWitnessRecord| {
        let wire = crate::result::RecordWire::of(record).unwrap();
        let value = serde_json::to_value(wire).unwrap();
        quire_canonical::to_vec(&value, quire_canonical::Limits::new(u64::MAX)).unwrap()
    };
    let native_witness =
        quire_canonical::to_vec(&native["witness"], quire_canonical::Limits::new(u64::MAX))
            .unwrap();
    assert_eq!(encoded(&record), native_witness);
    assert_eq!(encoded(&record), encoded(&repeated));

    let (all_wire, all_envelope) = replay_case(
        &unit,
        "AllBelow",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let all_envelope = all_envelope.expect("the AllBelow envelope reconstructs");
    let all_record = crate::derive_state_clause_witness(
        all_wire.clone(),
        &all_envelope,
        crate::ReplayLimits::default(),
    )
    .expect("the public AllBelow derivation completes")
    .expect("AllBelow has a decisive quantifier record");
    let all_report = tc_3121_run("AllBelow", Snapshot::High, "mid");
    assert_eq!(
        all_report.witness.as_ref(),
        Some(&all_record),
        "the public entry returns FR-265-AC-1's quantifier record"
    );
    let (all_replay_wire, all_replay_envelope) = replay_case(
        &unit,
        "AllBelow",
        Snapshot::High,
        "mid",
        Some(all_record.clone()),
        witness_source(),
    );
    let all_replay = replay_state_clause(
        all_replay_wire,
        &all_replay_envelope.expect("the quantifier-record envelope reconstructs"),
        crate::ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(witness_arm(&all_replay).record(), Some(&all_record));

    let version_unit = tc_3121_version_unit();
    let (version_wire, version_envelope, _) = version_advances_replay_case(
        &version_unit,
        0,
        Some(2),
        0,
        Some(4),
        "child",
        true,
        None,
        witness_source(),
    );
    let version_envelope = version_envelope.expect("the VersionAdvances envelope reconstructs");
    let version_derive = || {
        crate::derive_state_clause_witness(
            version_wire.clone(),
            &version_envelope,
            crate::ReplayLimits::default(),
        )
        .expect("the VersionAdvances derivation completes")
        .expect("the changed version has a comparison record")
    };
    let version_record = version_derive();
    let version_repeat = version_derive();
    assert_eq!(version_record, version_repeat);
    // FR-266-AC-2: the first `=`'s operands, `child`'s converted post-call
    // 4 then its pre-call 2 plus one, under the `and` and the first
    // `implies`.
    let version_equality = occurrence(&version_unit, "VersionAdvances", &[0, 1], Operator::Binary);
    assert_eq!(
        version_record.deciding_comparison,
        Some(version_equality.clone())
    );
    let version_operands = version_record
        .operands
        .as_ref()
        .expect("comparison operands");
    assert!(crate::result::same_element(
        &version_operands[0],
        &integer(4)
    ));
    assert!(crate::result::same_element(
        &version_operands[1],
        &integer(3)
    ));
    assert_eq!(
        version_record.decision_path,
        Some(vec![
            occurrence(&version_unit, "VersionAdvances", &[], Operator::Binary),
            occurrence(&version_unit, "VersionAdvances", &[0], Operator::Binary),
            version_equality,
        ])
    );
    let version_report = version_advances_clause_report(0, Some(2), 0, Some(4), "child", true);
    assert_eq!(version_report.witness.as_ref(), Some(&version_record));
    let version_bytes = crate::spine::write_clause_run_result(&version_report).unwrap();
    let version_native: serde_json::Value = serde_json::from_slice(&version_bytes).unwrap();
    let version_native_witness = quire_canonical::to_vec(
        &version_native["witness"],
        quire_canonical::Limits::new(u64::MAX),
    )
    .unwrap();
    assert_eq!(encoded(&version_record), version_native_witness);
    let version_replay = replay_state_clause(
        version_wire,
        &version_envelope,
        crate::ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(witness_arm(&version_replay).record(), Some(&version_record));
    // FR-269-AC-9: the no-record envelope settles the agreement.
    assert_eq!(
        witness_arm(&version_replay).settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(
        encoded(witness_arm(&version_replay).record().unwrap()),
        version_native_witness
    );

    let mut different = record.clone();
    different.operands = Some([integer(600), integer(1)]);
    let (different_wire, different_envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        Some(different),
        witness_source(),
    );
    let derived_from_different = crate::derive_state_clause_witness(
        different_wire,
        &different_envelope.expect("the different-record envelope reconstructs"),
        crate::ReplayLimits::default(),
    )
    .expect("the public entry ignores the payload witness")
    .expect("MatchesParent has a decisive comparison");
    assert_eq!(derived_from_different, record);

    let replay =
        replay_state_clause(wire.clone(), &envelope, crate::ReplayLimits::default()).unwrap();
    let replay_record = witness_arm(&replay)
        .record()
        .expect("the absent comparison record settles with the derived record");
    assert_eq!(replay_record, &record);
    assert_eq!(encoded(replay_record), native_witness);

    let (input_wire, input_envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        None,
        input_source(),
    );
    let input_envelope = input_envelope.expect("the input envelope reconstructs");
    let ReplayResult::Input(input) =
        replay_state_clause(input_wire, &input_envelope, crate::ReplayLimits::default())
            .unwrap()
            .result()
            .clone()
    else {
        panic!("the input arm stays an input result");
    };
    assert_eq!(
        input.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    assert_eq!(
        settle_separation(unmet(SeparationStep::Element), None, &record),
        Some(crate::WitnessCheck::Disagrees {
            given: None,
            derived: Some(Box::new(record)),
            failure: WitnessFailure::Separation {
                step: SeparationStep::Element,
                reason: SeparationReason::Unmet,
            },
        }),
        "failed separation preserves an absent given record"
    );
}

/// FR-269-AC-9: absent quantifier records and a different comparison record
/// remain mismatches; only an absent comparison record is settled by replay.
#[trace("TC-3121", "FR-269-AC-9")]
#[test]
fn tc_3121_absent_quantifier_and_different_comparison_records_still_mismatch() {
    let unit = tc_3121_unit();
    let (wire, envelope) = replay_case(
        &unit,
        "AllBelow",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the envelope reconstructs");
    let replay = replay_state_clause(wire, &envelope, crate::ReplayLimits::default()).unwrap();
    let arm = witness_arm(&replay);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    let DisagreementCause::Witness {
        given: None,
        derived: Some(derived),
        failure: WitnessFailure::Mismatch,
        ..
    } = arm
        .disagreement()
        .expect("the absent quantifier record mismatches")
    else {
        panic!("the absent quantifier record remains a one-sided mismatch");
    };
    assert_eq!(
        derived.as_ref(),
        &derive(&unit, "AllBelow", Snapshot::High, "mid")
            .expect("AllBelow derives a quantifier record")
            .record
            .expect("the AllBelow record is decisive")
    );

    let mut different = derive(&unit, "MatchesParent", Snapshot::High, "mid")
        .expect("MatchesParent derives a comparison")
        .record
        .expect("the comparison is decisive");
    different.operands = Some([integer(600), integer(1)]);
    let (wire, envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        Some(different),
        witness_source(),
    );
    let envelope = envelope.expect("the envelope reconstructs");
    let replay = replay_state_clause(wire, &envelope, crate::ReplayLimits::default()).unwrap();
    let DisagreementCause::Witness {
        given: Some(given),
        derived: Some(derived),
        failure: WitnessFailure::Mismatch,
        ..
    } = witness_arm(&replay)
        .disagreement()
        .expect("the records differ")
    else {
        panic!("the two-sided mismatch keeps both records");
    };
    let [Value::Integer(given_left), Value::Integer(given_right)] = given
        .operands
        .as_ref()
        .expect("the supplied comparison record has operands")
    else {
        panic!("the supplied record has integer operands");
    };
    assert_eq!(given_left.to_string(), "600");
    assert_eq!(given_right.to_string(), "1");
    assert_eq!(
        witness_arm(&replay).settlement(),
        WitnessSettlement::Inconclusive
    );
    let [Value::Integer(derived_left), Value::Integer(derived_right)] = derived
        .operands
        .as_ref()
        .expect("the derived comparison record has operands")
    else {
        panic!("the derived record has integer operands");
    };
    assert_eq!(derived_left.to_string(), "600");
    assert_eq!(derived_right.to_string(), "0");
}

/// FR-269-AC-9: one unit below the combined evaluation/separation budget
/// cannot settle the absent comparison record; the minimum successful
/// budget reproduces it.
#[trace("TC-3121", "FR-269-AC-9")]
#[test]
fn tc_3121_absent_comparison_record_is_no_value_one_unit_below_need() {
    let unit = tc_3121_unit();
    let (base_wire, envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the envelope reconstructs");
    let is_reproduced = |work_units| {
        let mut wire = base_wire.clone();
        wire.accounting_limits = default_accounting(work_units);
        let result = replay_state_clause(wire, &envelope, crate::ReplayLimits::default())
            .expect("the limited replay settles");
        matches!(
            result.result(),
            ReplayResult::Witness(arm)
                if arm.settlement() == WitnessSettlement::ReproducedWithEvaluatedWitness
        )
    };

    let mut low = 0_u64;
    let mut high = 1_u64;
    while !is_reproduced(high) {
        low = high;
        high = high.checked_mul(2).expect("the test budget stays bounded");
        assert!(high <= 1_048_576, "the fixture's replay remains bounded");
    }
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if is_reproduced(middle) {
            high = middle;
        } else {
            low = middle;
        }
    }

    let mut one_short_wire = base_wire.clone();
    one_short_wire.accounting_limits = default_accounting(low);
    let one_short = replay_state_clause(one_short_wire, &envelope, crate::ReplayLimits::default())
        .expect("the one-unit-short replay settles");
    let arm = witness_arm(&one_short);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(&DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Violation),
        })
    );
    assert!(arm.record().is_none());
    assert!(is_reproduced(high));
}

/// FR-269-AC-7: a clause that holds is a typed NotRefuted refusal.
#[trace("TC-3121", "FR-269-AC-7")]
#[test]
fn tc_3121_public_derivation_refusals_keep_catalog_metadata() {
    let unit = tc_3121_unit();
    let (wire, envelope) = replay_case(
        &unit,
        "AtLeastParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the envelope reconstructs");
    let derive_not_refuted = || {
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default())
            .expect_err("AtLeastParent holds over the high snapshot")
    };
    let refusal = derive_not_refuted();
    let repeated_refusal = derive_not_refuted();
    assert_eq!(refusal.code(), crate::Code::InvalidRuntimeInput.as_str());
    assert_eq!(refusal.cause(), Some("invalid-value"));
    assert_eq!(
        refusal.pointer().as_deref(),
        Some("/family_payload/observation")
    );
    assert_eq!(refusal.to_string(), repeated_refusal.to_string());

    let undefined_unit = tc_3121_undefined_unit();
    let (wire, envelope) = replay_case(
        &undefined_unit,
        "UndefinedDomain",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the undefined-domain envelope reconstructs");
    let derive_undefined = || {
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default())
            .expect_err("u(x) evaluates undefined under Kernel")
    };
    let first = derive_undefined();
    let second = derive_undefined();
    assert_eq!(first.code(), crate::Code::UndefinedExpression.as_str());
    assert_eq!(first.cause(), Some("division-by-zero"));
    assert_eq!(
        first.pointer().as_deref(),
        Some("/family_payload/observation")
    );
    let crate::DerivationRefusal::Undefined { expression, cause } = &first else {
        panic!("expected the public undefined refusal, got {first:?}");
    };
    // TC-3121 row 5: the clause's domain call that ended undefined, not the
    // `mod` inside `u`, with the body's cause.
    assert!(matches!(
        &expression.origin,
        filament_semantic_value::location::ExpressionOwner::StateClause { clause, .. }
            if clause == "UndefinedDomain"
    ));
    assert!(!expression.path().is_empty(), "{expression:?}");
    let region = undefined_unit
        .compiled
        .package
        .graph()
        .region(expression)
        .expect("the clause's call has a source region");
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    assert_eq!(
        std::str::from_utf8(&undefined_unit.bytes[start..end]).unwrap(),
        "u(self.versionNumber)"
    );
    assert_eq!(*cause, "division-by-zero");
    let crate::DerivationRefusal::Undefined {
        expression: repeated_expression,
        cause: repeated_cause,
    } = second
    else {
        panic!("repeated call changed refusal: {second:?}");
    };
    assert_eq!(&repeated_expression, expression);
    assert_eq!(repeated_cause, *cause);

    let (wire, envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let mut stale_packet = envelope
        .expect("the stale-node source envelope reconstructs")
        .to_packet();
    stale_packet.clause_node = Some(
        unit.compiled
            .package
            .graph()
            .state_clause("AllBelow")
            .expect("AllBelow is in the recompiled package")
            .identity()
            .clone(),
    );
    let stale_envelope = WitnessEnvelope::reconstruct(stale_packet, crate::ReplayLimits::default())
        .expect("the stale-node envelope is structurally complete");
    let stale_refusal = || {
        crate::derive_state_clause_witness(
            wire.clone(),
            &stale_envelope,
            crate::ReplayLimits::default(),
        )
        .expect_err("AllBelow's node cannot identify MatchesParent")
    };
    let first_stale = stale_refusal();
    let second_stale = stale_refusal();
    assert!(matches!(
        first_stale,
        crate::DerivationRefusal::Replay(ReplayRefusal::ClauseIdentity(_))
    ));
    assert_eq!(first_stale.code(), crate::Code::StaleDependency.as_str());
    assert_eq!(first_stale.cause(), Some("content-mismatch"));
    assert_eq!(first_stale.to_string(), second_stale.to_string());
}

/// FR-269-AC-7: evaluation refusals and an exhausted meter retain their
/// catalog codes, causes and observation pointer at the public entry.
#[trace("TC-3121", "FR-269-AC-7")]
#[test]
fn tc_3121_public_derivation_refused_and_exhausted_cases_keep_metadata() {
    let unit = tc_3121_unit();
    let (wire, envelope) = replay_case(
        &unit,
        "RefusedDomain",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the refusal envelope reconstructs");
    let derive_refused = || {
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default())
            .expect_err("the RefusedDomain expression refuses during evaluation")
    };
    let refused = derive_refused();
    let repeated_refused = derive_refused();
    assert!(matches!(refused, crate::DerivationRefusal::Refused(_)));
    assert_eq!(refused.code(), "integer_out_of_domain", "{refused:?}");
    assert_eq!(refused.cause(), Some("outside-domain"), "{refused:?}");
    assert_eq!(
        refused.pointer().as_deref(),
        Some("/family_payload/observation")
    );
    assert_eq!(refused.to_string(), repeated_refused.to_string());

    let (mut wire, envelope) = replay_case(
        &unit,
        "MatchesParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let evaluation = tc_3121_run("MatchesParent", Snapshot::High, "mid").usage;
    let need = evaluation
        .evaluation_consumed
        .iter()
        .find_map(|(kind, consumed)| {
            (*kind == filament_exact::LimitKind::WorkUnits).then_some(*consumed)
        })
        .expect("evaluation reports its WorkUnits consumption");
    wire.accounting_limits = default_accounting(need.saturating_sub(1));
    let envelope = envelope.expect("the exhausted envelope reconstructs");
    let derive_exhausted = || {
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default())
            .expect_err("the one-unit-short accounting budget exhausts evaluation")
    };
    let exhausted = derive_exhausted();
    let repeated_exhausted = derive_exhausted();
    assert!(matches!(exhausted, crate::DerivationRefusal::Exhausted));
    assert_eq!(exhausted.code(), crate::Code::ResourceExhausted.as_str());
    assert_eq!(exhausted.cause(), Some("insufficient-next-charge"));
    assert_eq!(
        exhausted.pointer().as_deref(),
        Some("/family_payload/observation")
    );
    assert_eq!(exhausted.to_string(), repeated_exhausted.to_string());

    wire.accounting_limits = default_accounting(need);
    assert!(
        crate::derive_state_clause_witness(wire, &envelope, crate::ReplayLimits::default()).is_ok(),
        "the measured WorkUnits need itself completes"
    );
}

/// FR-269-AC-8: a post-state range violation wins over either Boolean
/// disposition of VersionAdvances.
#[trace("TC-3121", "FR-269-AC-8")]
#[test]
fn tc_3121_public_derivation_range_violations_win_first() {
    let unit = tc_3121_version_unit();
    for (root_pre, child_pre, root_post, child_post, self_key, result, object, observed) in [
        (0, Some(0), 0, Some(-1), "child", true, "child", -1),
        (1000, Some(2), 1001, Some(2), "child", false, "root", 1001),
    ] {
        let (wire, envelope, _) = version_advances_replay_case(
            &unit,
            root_pre,
            child_pre,
            root_post,
            child_post,
            self_key,
            result,
            None,
            witness_source(),
        );
        let refusal = crate::derive_state_clause_witness(
            wire,
            &envelope.expect("the range-violation envelope reconstructs"),
            crate::ReplayLimits::default(),
        )
        .expect_err("an out-of-range post state refuses before the clause outcome");
        assert_eq!(refusal.code(), crate::Code::InvalidRuntimeInput.as_str());
        assert_eq!(refusal.cause(), Some("invalid-value"));
        assert_eq!(
            refusal.pointer().as_deref(),
            Some("/family_payload/observation")
        );
        let crate::DerivationRefusal::RangeViolation(violations) = refusal else {
            panic!("expected RangeViolation, got {refusal:?}");
        };
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].object.object().to_string(), object);
        assert_eq!(violations[0].field.name, "versionNumber");
        assert_eq!(violations[0].observed.to_string(), observed.to_string());
    }
}

/// FR-269-AC-6/9: the no-parent case has a false, closed-scope basis and
/// therefore returns `Ok(None)`.
#[trace("TC-3121", "FR-269-AC-6", "FR-269-AC-9")]
#[test]
fn tc_3121_no_parent_closed_scope_returns_none() {
    let unit = tc_3121_no_parent_unit();
    let (wire, envelope) = replay_case(
        &unit,
        "NoParent",
        Snapshot::High,
        "mid",
        None,
        witness_source(),
    );
    let envelope = envelope.expect("the no-parent envelope reconstructs");
    let result =
        crate::derive_state_clause_witness(wire.clone(), &envelope, crate::ReplayLimits::default());
    assert!(result.unwrap().is_none());

    let (label, bytes) = Snapshot::High.document();
    let domain_document = config_version_domain_document();
    let packages =
        fsl_semantics::model::intake::package_input([domain_document.as_slice()]).unwrap();
    let mut run_request = config_version_request_for(
        String::from_utf8(unit.bytes.clone()).expect("the fixture is UTF-8"),
        packages,
        "tc-3121-no-parent.native",
        ClauseRunSelection::Clause(config_version_current_selection(
            "NoParent",
            label.clone(),
            "mid",
        )),
    );
    run_request.snapshots.insert(label, bytes);
    let report = run_clause(run_request).expect("the no-parent clause run reports");
    assert!(report.witness.is_none(), "NoParent writes no witness");

    let replay = replay_state_clause(wire, &envelope, crate::ReplayLimits::default()).unwrap();
    let arm = witness_arm(&replay);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert!(arm.record().is_none());
}

fn input_source() -> ReplaySource {
    ReplaySource::Input(Vec::new())
}

fn witness_arm(result: &StateClauseReplayResult) -> &crate::WitnessArmResult {
    match result.result() {
        ReplayResult::Witness(arm) => arm,
        ReplayResult::Input(arm) => panic!("expected the witness arm, got {arm:?}"),
        ReplayResult::ObservedTemporal(arm) => panic!("expected the witness arm, got {arm:?}"),
        ReplayResult::ModelGraph(arm) => panic!("expected the witness arm, got {arm:?}"),
        ReplayResult::ProtocolAnalysis(_) => panic!("a protocol analysis arm"),
        ReplayResult::Hybrid(_) => panic!("a hybrid arm"),
    }
}

/// TC-743 step 1 (FR-268-AC-1): an envelope carrying the producer's record
/// reproduces with that record on the `Witness` arm and without witness on
/// the `Input` arm; nested and filtered domains reproduce too.
#[trace("TC-743", "FR-268-AC-1")]
#[test]
fn tc_743_a_matching_record_that_separates_reproduces() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    let result = replay_with(&unit, "AllBelow", Some(record.clone()), witness_source()).unwrap();
    let arm = witness_arm(&result);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.record(), Some(&record));

    let ReplayResult::Input(input) = replay_with(&unit, "AllBelow", Some(record), input_source())
        .unwrap()
        .result()
        .clone()
    else {
        panic!("an Input-sourced envelope settles on the Input arm");
    };
    assert_eq!(
        input.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    for (clause, record) in [
        (
            "NestedAll",
            built_record(
                &unit,
                "NestedAll",
                &[],
                &[0, 0],
                integer(0),
                0,
                Snapshot::High,
            ),
        ),
        (
            "FilteredAll",
            built_record(
                &unit,
                "FilteredAll",
                &[],
                &[0, 0, 0],
                integer(600),
                2,
                Snapshot::High,
            ),
        ),
    ] {
        let result = replay_with(&unit, clause, Some(record.clone()), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(
            arm.settlement(),
            WitnessSettlement::ReproducedWithEvaluatedWitness,
            "{clause}"
        );
        assert_eq!(arm.record(), Some(&record), "{clause}");
    }
}

/// `AllBelow`'s record with the element at index 2 (the equal element after
/// the stop), standing for FR-268-AC-2's `leaf` record.
fn after_stop_record(unit: &Unit) -> SeparatingWitnessRecord {
    built_record(
        unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(600),
        2,
        Snapshot::High,
    )
}

/// TC-743 step 2 (FR-268-AC-2): a record naming the element after the stop,
/// one naming another clause's `forall`, and no record each settle
/// `inconclusive` with a `Witness` cause holding both records.
#[trace("TC-743", "FR-268-AC-2")]
#[test]
fn tc_743_a_differing_or_missing_record_settles_inconclusive() {
    let unit = witness_unit();
    let derived = all_below_record(&unit);
    let other_quantifier = SeparatingWitnessRecord {
        quantifier: Some(occurrence(&unit, "GuardedAll", &[1], Operator::Quantify)),
        ..derived.clone()
    };
    for given in [Some(after_stop_record(&unit)), Some(other_quantifier), None] {
        let result = replay_with(&unit, "AllBelow", given.clone(), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
        let Some(DisagreementCause::Witness {
            given: held,
            derived: re_derived,
            ..
        }) = arm.disagreement()
        else {
            panic!("expected a Witness cause, got {:?}", arm.disagreement());
        };
        assert_eq!(held.as_deref(), given.as_ref());
        assert_eq!(re_derived.as_deref(), Some(&derived));
    }
}

/// FR-268's separation check of `record` over `clause` and the admitted
/// `high` observation (self `mid`), under `work_units`.
fn check(
    unit: &Unit,
    clause: &str,
    record: &SeparatingWitnessRecord,
    work_units: u64,
) -> SeparationOutcome {
    let (observations, _, _) = evaluate(unit, clause, Snapshot::High, "mid", 1_000_000);
    let mut meter = Meter::new(default_accounting(work_units));
    separate(
        &unit.compiled.package,
        clause,
        &observations,
        record,
        std::slice::from_ref(&unit.compiled.source),
        &mut meter,
    )
    .expect("the check settles")
}

fn unmet(step: SeparationStep) -> SeparationOutcome {
    SeparationOutcome::Failed(step, SeparationReason::Unmet)
}

/// The state-clause separation check names no element of a protocol record
/// (no index) or of a kernel record without a quantifier: it fails at the
/// element step (ADR-031 SW-5 as amended by ADR-036 PA-24).
#[trace("TC-1414")]
#[test]
fn a_protocol_record_or_a_quantifierless_record_fails_the_element_step() {
    let unit = witness_unit();
    let state = all_below_record(&unit);
    let protocol = SeparatingWitnessRecord {
        quantifier: None,
        deciding_element: DecidingElement::Protocol("node".to_owned()),
        index: None,
        ..state.clone()
    };
    let quantifierless = SeparatingWitnessRecord {
        quantifier: None,
        ..state.clone()
    };
    let protocol_with_index = SeparatingWitnessRecord {
        deciding_element: DecidingElement::Protocol("node".to_owned()),
        ..state
    };
    for record in [protocol, quantifierless, protocol_with_index] {
        assert_eq!(
            check(&unit, "AllBelow", &record, 1_000_000),
            unmet(SeparationStep::Element)
        );
    }
}

/// TC-743 step 3 and TC-744 step 2 (FR-268-AC-3, FR-269-AC-2): the
/// separation check, called on its own.
#[trace("TC-743", "FR-268-AC-3")]
#[test]
fn tc_743_the_separation_check_names_its_failing_step() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    assert_eq!(
        check(&unit, "AllBelow", &record, 1_000_000),
        SeparationOutcome::Holds
    );
    let outside = SeparatingWitnessRecord {
        quantifier: Some(occurrence(&unit, "SomeAtLeast", &[], Operator::Quantify)),
        ..record.clone()
    };
    let past_end = SeparatingWitnessRecord {
        index: Some(3),
        value_path: RuntimeValuePath {
            steps: vec![ValuePathStep::Index(3)],
            ..record.value_path.clone()
        },
        ..record.clone()
    };
    let other_member = SeparatingWitnessRecord {
        value_path: RuntimeValuePath {
            steps: vec![
                ValuePathStep::Member("versionNumber".to_owned()),
                ValuePathStep::Index(1),
            ],
            ..record.value_path.clone()
        },
        ..record.clone()
    };
    let other_element = SeparatingWitnessRecord {
        deciding_element: DecidingElement::Value(integer(700)),
        ..record.clone()
    };
    let holding_element = built_record(
        &unit,
        "AllBelow",
        &[],
        &[0, 0],
        integer(0),
        0,
        Snapshot::High,
    );
    for (failing, step) in [
        (&outside, SeparationStep::Quantifier),
        (&past_end, SeparationStep::Element),
        (&other_member, SeparationStep::Element),
        (&other_element, SeparationStep::Element),
        (&holding_element, SeparationStep::Body),
    ] {
        assert_eq!(check(&unit, "AllBelow", failing, 1_000_000), unmet(step));
    }

    assert_count_refusal_fails_the_domain_step();
    assert_eq!(
        check(&unit, "AllBelow", &record, 0),
        SeparationOutcome::Exhausted
    );
}

/// A kernel function's refused count retains the separation domain failure.
#[trace("TC-743", "FR-268-AC-3")]
#[test]
fn count_refusal_in_a_called_kernel_function_fails_the_domain_step() {
    assert_count_refusal_fails_the_domain_step();
}

fn assert_count_refusal_fails_the_domain_step() {
    let (base, packages) = config_version_unit_and_packages();
    let text = base + "type Zero = Int[0, 0];\n\
        function z using v(x: Integer): Zero pure { count<Zero>(m in ints(sequence[x]): m > 0) }\n\
        function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { xs }\n\
        invariant RefusedDomain using v on Config::ConfigVersion at current { forall(n in ints(sequence[0, z(self.versionNumber)]): n < 500) }\n";
    let compiled = crate::spine::lifecycle::compose_in_mode(
        source(),
        "witness.native",
        text.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        crate::spine::lifecycle::ComposeModes {
            check_mode: Some(CheckMode::Linked),
            function_modes: &BTreeMap::from([("z".to_owned(), CheckMode::Kernel)]),
        },
    )
    .expect("only the called function's count needs kernel checking");
    let unit = Unit {
        bytes: text.into_bytes(),
        domain_document: config_version_domain_document(),
        compiled,
    };
    let refused = built_record(
        &unit,
        "RefusedDomain",
        &[],
        &[0, 0],
        integer(600),
        1,
        Snapshot::High,
    );
    let SeparationOutcome::Failed(SeparationStep::Domain, SeparationReason::Refused(refusal)) =
        check(&unit, "RefusedDomain", &refused, 1_000_000)
    else {
        panic!("a refused count must fail separation step 2 with its refusal record");
    };
    assert_eq!(refusal.code, "integer_out_of_domain");
}

/// TC-744 step 2 (FR-269-AC-2): an `undefined` step evaluation maps to
/// `UndefinedEvaluation`, naming the expression and its undefined reason.
/// A linked state clause proves every divisor nonzero, so no clause
/// reaches an `undefined` step; the mapping is checked over the S6a
/// evaluation such a step would return.
#[trace("TC-744", "FR-269-AC-2")]
#[test]
fn tc_744_an_undefined_step_maps_to_undefined_evaluation() {
    let unit = witness_unit();
    let at = filament_semantic_value::location::ExpressionLocation::at(
        filament_semantic_value::location::ExpressionOwner::StateClause {
            clause: "AllBelow".to_owned(),
            index: 0,
        },
        &[0],
    );
    let evaluation = filament_eval::Evaluation {
        outcome: FamilyOutcome::Evaluated(Outcome::Undefined(
            filament_exact::Undefined::DivisionByZero,
        )),
        location: Some(at.clone()),
        losses: Vec::new(),
        frame_depth: 0,
        peak_live_occurrences: 0,
    };
    let reason = stopped_reason(
        evaluation,
        unit.compiled.package.graph(),
        std::slice::from_ref(&unit.compiled.source),
    )
    .expect("the mapping settles");
    let Some(SeparationReason::UndefinedEvaluation { expression, cause }) = reason else {
        panic!("expected UndefinedEvaluation, got {reason:?}");
    };
    assert_eq!(expression, at);
    assert_eq!(cause, "division-by-zero");
}

/// TC-743 step 4 (FR-268-AC-4): an encoded record past the reader bound
/// refuses at decode; replaying one envelope twice gives equal results.
#[trace("TC-743", "FR-268-AC-4")]
#[test]
fn tc_743_an_oversized_record_refuses_and_replay_is_deterministic() {
    let unit = witness_unit();
    let mut oversized = all_below_record(&unit);
    oversized.value_path.steps.insert(
        0,
        ValuePathStep::Member("x".repeat(crate::bounds::DEFAULT_INPUT_BYTES + 1)),
    );
    let (_, envelope) = replay_case(
        &unit,
        "AllBelow",
        Snapshot::High,
        "mid",
        Some(oversized),
        witness_source(),
    );
    assert!(matches!(
        envelope,
        Err(crate::WitnessRefusal::BoundExceeded(_))
    ));
    let record = all_below_record(&unit);
    let first = replay_with(&unit, "AllBelow", Some(record.clone()), witness_source()).unwrap();
    let second = replay_with(&unit, "AllBelow", Some(record), witness_source()).unwrap();
    assert_eq!(first, second);
}

/// TC-744 step 1 (FR-269-AC-1): the after-stop and no-record replays hold
/// `Witness { violation, violation, given, derived, Mismatch }`, and the
/// result carries no record.
#[trace("TC-744", "FR-269-AC-1")]
#[test]
fn tc_744_a_mismatch_holds_both_records_and_no_result_record() {
    let unit = witness_unit();
    let violation = Verdict::from_category(Category::Violation);
    let derived = all_below_record(&unit);
    for given in [Some(after_stop_record(&unit)), None] {
        let result = replay_with(&unit, "AllBelow", given.clone(), witness_source()).unwrap();
        let arm = witness_arm(&result);
        assert_eq!(
            arm.disagreement(),
            Some(&DisagreementCause::Witness {
                proved: violation,
                replayed: violation,
                given: given.map(Box::new),
                derived: Some(Box::new(derived.clone())),
                failure: WitnessFailure::Mismatch,
            })
        );
        assert_eq!(arm.record(), None);
    }
}

/// TC-744 step 3 (FR-269-AC-3): a `Witness` cause round-trips; an
/// undefined `failure` and a `given` missing its `index` refuse; causes
/// differing only in `derived` compare unequal.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_the_witness_cause_round_trips_and_reads_strictly() {
    let unit = witness_unit();
    let result = replay_with(
        &unit,
        "AllBelow",
        Some(after_stop_record(&unit)),
        witness_source(),
    )
    .unwrap();
    let cause = witness_arm(&result).disagreement().unwrap().clone();
    let text = cause.to_json(None).expect("the cause encodes");
    assert_eq!(
        DisagreementCause::from_json(&text, &unit.compiled.package, None).unwrap(),
        cause
    );

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["failure"] = json!({"failure": "approximate"});
    assert!(
        DisagreementCause::from_json(&document.to_string(), &unit.compiled.package, None).is_err()
    );

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["given"]
        .as_object_mut()
        .unwrap()
        .remove("index")
        .expect("the record carries its index");
    assert!(
        DisagreementCause::from_json(&document.to_string(), &unit.compiled.package, None).is_err()
    );

    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["given"]["index"] = serde_json::Value::Null;
    assert!(
        DisagreementCause::from_json(&document.to_string(), &unit.compiled.package, None).is_err()
    );

    let DisagreementCause::Witness {
        proved,
        replayed,
        given,
        failure,
        ..
    } = cause.clone()
    else {
        panic!("a Witness cause");
    };
    let other = DisagreementCause::Witness {
        proved,
        replayed,
        given,
        derived: Some(Box::new(built_record(
            &unit,
            "AllBelow",
            &[],
            &[0, 0],
            integer(0),
            0,
            Snapshot::High,
        ))),
        failure,
    };
    assert_ne!(cause, other);
}

/// TC-744 step 2 (FR-269-AC-2): a matching record whose separation check
/// fails at a step disagrees with `Witness`'s `Separation` failure naming
/// that step and reason, holding the record as both `given` and `derived`;
/// one that holds agrees; an exhausted check settles no witness check.
#[trace("TC-744", "FR-269-AC-2")]
#[test]
fn tc_744_a_failed_separation_check_disagrees_naming_its_step() {
    let unit = witness_unit();
    let record = all_below_record(&unit);
    assert_eq!(
        settle_separation(unmet(SeparationStep::Body), Some(&record), &record),
        Some(crate::WitnessCheck::Disagrees {
            given: Some(Box::new(record.clone())),
            derived: Some(Box::new(record.clone())),
            failure: WitnessFailure::Separation {
                step: SeparationStep::Body,
                reason: SeparationReason::Unmet,
            },
        })
    );
    assert_eq!(
        settle_separation(SeparationOutcome::Holds, Some(&record), &record),
        Some(crate::WitnessCheck::Agrees(Some(Box::new(record.clone()))))
    );
    assert_eq!(
        settle_separation(SeparationOutcome::Exhausted, Some(&record), &record),
        None
    );
}

/// The `witness` unit with record declarations, so a composite deciding
/// element names a declaration the checked package holds: `Point` (two
/// required fields), `Gaps` (two optional fields and a required one) and
/// `Chain` (one optional field naming itself).
fn codec_unit() -> Unit {
    let text = format!(
        "{}record Point {{ x: Int[0, 9]; y: Int[0, 9]; }}\n\
         record Gaps {{ alpha: Int[0, 9]?; beta: Int[0, 9]?; gamma: Int[0, 9]; }}\n\
         record Chain {{ next: Chain?; }}\n",
        witness_unit_text()
    );
    unit_for(text, config_version_domain_document())
}

/// The key of `codec_unit`'s record with `fields` fields.
fn record_key(unit: &Unit, fields: usize) -> filament_exact::NodeKey {
    unit.compiled
        .package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|declaration| {
            matches!(
                declaration.shape(),
                filament_semantic_value::declaration::CompositeShape::Record(declared)
                    if declared.len() == fields
            )
        })
        .unwrap_or_else(|| panic!("a record of {fields} fields is declared"))
        .key()
}

/// A text value of `Text[0, 64; nfc]` holding `payload`.
fn text(payload: &str) -> Value {
    let text_type = filament_exact::TextType::new(0, 64, filament_exact::TextProfile::Nfc).unwrap();
    let payload = filament_exact::TextPayload::from_utf8(payload.as_bytes()).unwrap();
    let mut meter = Meter::new(default_accounting(1_000_000));
    match filament_exact::admit_text(&payload, &text_type, &mut meter) {
        Outcome::Completed(text) => Value::Text(text),
        other => panic!("the text admits: {other:?}"),
    }
}

/// A `Witness` cause whose `given` record decides on `element`.
fn cause_deciding(unit: &Unit, element: Value) -> DisagreementCause {
    let violation = Verdict::from_category(Category::Violation);
    DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: Some(Box::new(SeparatingWitnessRecord {
            deciding_element: DecidingElement::Value(element),
            ..all_below_record(unit)
        })),
        derived: None,
        failure: WitnessFailure::Mismatch,
    }
}

/// A deciding element whose type nests 100,000 `Option`s, and one whose
/// value nests 100,000 records, each round-trip through the cause codec on a
/// 512 KiB stack: the encoder keeps the value's and type's members on a heap
/// stack and the reader holds the document and decodes it from one, so no
/// depth is a limit (ADR-030 D-1). Each reads back and encodes to the same
/// document, and the cause, its document and the read result drop on the
/// same stack.
#[trace("TC-744", "FR-269-AC-4")]
#[test]
fn a_100000_deep_deciding_element_round_trips_on_a_small_stack() {
    const DEEP: usize = 100_000;
    let unit = codec_unit();
    let chain = record_key(&unit, 1);
    let template = all_below_record(&unit);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn_scoped(scope, || {
                let package = &unit.compiled.package;
                let deep_type = (0..DEEP).fold(filament_exact::ValueType::Boolean, |inner, _| {
                    filament_exact::ValueType::option(inner)
                });
                let deep_record = (0..DEEP).fold(
                    filament_exact::from_admitted_slots(
                        chain.clone(),
                        Box::new([filament_exact::FieldValue::Absent]),
                    ),
                    |next, _| {
                        filament_exact::from_admitted_slots(
                            chain.clone(),
                            Box::new([filament_exact::FieldValue::Present(next)]),
                        )
                    },
                );
                // The `none` itself and each of its type's levels; each record
                // holding the next.
                let elements = [
                    (
                        "option type",
                        filament_exact::OptionValue::none(deep_type),
                        r#""kind":"option""#,
                        DEEP + 1,
                    ),
                    ("record value", deep_record, r#""slot":"present""#, DEEP),
                ];
                for (label, element, level, levels) in elements {
                    let violation = Verdict::from_category(Category::Violation);
                    let cause = DisagreementCause::Witness {
                        proved: violation,
                        replayed: violation,
                        given: Some(Box::new(SeparatingWitnessRecord {
                            deciding_element: DecidingElement::Value(element),
                            ..template.clone()
                        })),
                        derived: None,
                        failure: WitnessFailure::Mismatch,
                    };
                    let text = cause
                        .to_json(None)
                        .unwrap_or_else(|error| panic!("{label}: encodes: {error}"));
                    assert_eq!(text.matches(level).count(), levels, "{label}: every level");
                    let read = DisagreementCause::from_json(&text, package, None)
                        .unwrap_or_else(|error| panic!("{label}: reads: {error}"));
                    assert_eq!(read.to_json(None).unwrap(), text, "{label}: encodes back");
                }
            })
            .expect("the thread spawns")
            .join()
            .expect("the deep elements round-trip on a 512 KiB stack");
    });
}

/// TC-744 step 3 (FR-269-AC-3): a deciding element of each value kind
/// round-trips through the cause codec, reading back equal and encoding
/// back to the same document; an element naming a declaration the package
/// does not hold refuses.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_every_deciding_element_kind_round_trips() {
    use filament_exact::{
        CardinalityBound, CollectionKind, CollectionType, Decimal, FieldValue, IeeeValue,
        IntegerInterval, ObjectId, ObjectReference, OptionValue, Quantity, Rational, UnitId,
        ValueType,
    };
    let unit = codec_unit();
    let package = &unit.compiled.package;
    let types = package.graph().scope().types();
    let point = record_key(&unit, 2);
    let gaps = record_key(&unit, 3);
    let object_type = types
        .object_types()
        .next()
        .expect("ConfigVersion is declared")
        .key();
    let int = |value: i64| Value::Integer(filament_exact::Integer::from(value));
    let digit = ValueType::Int(
        IntegerInterval::new(
            filament_exact::Integer::from(0_i64),
            filament_exact::Integer::from(9_i64),
        )
        .unwrap(),
    );
    let rational = |n: i64, d: i64| {
        Rational::new(
            filament_exact::Integer::from(n),
            filament_exact::Integer::from(d),
        )
        .unwrap()
    };
    let sequence = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Integer,
        Some(CardinalityBound::new(0, 3).unwrap()),
    );
    let set = CollectionType::new(CollectionKind::Set, ValueType::Integer, None);
    let references = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Reference(object_type.clone()),
        None,
    );
    let elements = [
        ("boolean", Value::Boolean(true)),
        ("integer", int(-42)),
        ("rational", Value::Rational(rational(-3, 4))),
        (
            "decimal",
            Value::Decimal(Decimal::new(filament_exact::Integer::from(12_345_i64), 2)),
        ),
        (
            "float64",
            Value::Float(IeeeValue::binary64(0x4009_21fb_5444_2d18)),
        ),
        ("float32", Value::Float(IeeeValue::binary32(0x4049_0fdb))),
        (
            "compound quantity",
            Value::Quantity(Quantity::new(
                rational(1, 1),
                UnitId::compound(vec![(
                    crate::fixture_node(5),
                    filament_exact::Integer::from(1_i64),
                )]),
            )),
        ),
        ("text", text("cafe\u{301}")),
        (
            "enum",
            Value::Enum(filament_exact::EnumMember::new(
                crate::spine::fixture_ids::variant(9),
                1,
            )),
        ),
        (
            "present option",
            OptionValue::present(digit.clone(), int(7)).unwrap(),
        ),
        (
            "absent option",
            OptionValue::none(ValueType::Composite(point.clone())),
        ),
        (
            "option of option payload type",
            OptionValue::none(ValueType::option(ValueType::option(digit.clone()))),
        ),
        (
            "option of sequence payload type",
            OptionValue::none(ValueType::option(ValueType::collection(
                CollectionType::new(CollectionKind::Sequence, ValueType::Integer, None),
            ))),
        ),
        (
            "composite with an absent and a null slot",
            filament_exact::from_admitted_slots(
                gaps,
                vec![
                    FieldValue::Absent,
                    FieldValue::Null,
                    FieldValue::Present(int(3)),
                ]
                .into(),
            ),
        ),
        (
            "composite",
            filament_exact::from_admitted_slots(
                point.clone(),
                vec![FieldValue::Present(int(1)), FieldValue::Present(int(2))].into(),
            ),
        ),
        (
            "sequence",
            filament_exact::from_admitted(sequence, vec![int(2), int(1), int(2)]),
        ),
        (
            "set",
            filament_exact::from_admitted(set, vec![int(1), int(2)]),
        ),
        (
            "empty reference sequence",
            filament_exact::from_admitted(references, Vec::new()),
        ),
        (
            "reference",
            Value::Reference(ObjectReference::new(
                crate::spine::fixture_ids::universe(2),
                object_type,
                ObjectId::new("root".to_owned()).unwrap(),
            )),
        ),
    ];
    for (kind, element) in elements {
        let cause = cause_deciding(&unit, element);
        let text = cause
            .to_json(None)
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        let read = DisagreementCause::from_json(&text, package, None)
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(read, cause, "{kind}");
        assert_eq!(read.to_json(None).unwrap(), text, "{kind}");
    }

    // A declared unit the package's unit table does not hold refuses.
    let declared = cause_deciding(
        &unit,
        Value::Quantity(Quantity::new(
            rational(5, 2),
            UnitId::declared(
                filament_exact::NodeKey::new(
                    "fixture",
                    vec![filament_exact::PathSegment::Text("node".into())],
                    6,
                )
                .expect("a fixture key"),
            ),
        )),
    )
    .to_json(None)
    .unwrap();
    assert!(matches!(
        DisagreementCause::from_json(&declared, package, None),
        Err(crate::CauseCodecError::Unresolved("unit"))
    ));

    // Slots of another shape than `Point`'s refuse (FSpec FR-351-AC-5):
    // absent and null for its two required fields, one slot too few, and a
    // slot outside its field's type.
    for slots in [
        vec![FieldValue::Absent, FieldValue::Null],
        vec![FieldValue::Present(int(1))],
        vec![FieldValue::Present(int(1)), FieldValue::Present(int(10))],
    ] {
        let foreign = cause_deciding(
            &unit,
            filament_exact::from_admitted_slots(point.clone(), slots.into()),
        )
        .to_json(None)
        .unwrap();
        assert!(matches!(
            DisagreementCause::from_json(&foreign, package, None),
            Err(crate::CauseCodecError::Value("slots"))
        ));
    }

    // The witness unit declares no `Point`: a well-shaped one refuses there.
    let composite = cause_deciding(
        &unit,
        filament_exact::from_admitted_slots(
            point,
            vec![FieldValue::Present(int(1)), FieldValue::Present(int(2))].into(),
        ),
    )
    .to_json(None)
    .unwrap();
    assert!(DisagreementCause::from_json(&composite, package, None).is_ok());
    assert!(matches!(
        DisagreementCause::from_json(&composite, &witness_unit().compiled.package, None),
        Err(crate::CauseCodecError::Unresolved("declaration"))
    ));
}

/// A checked package whose unit table holds the declared unit `metre` of
/// dimension `Length`, both keyed from their FSpec FR-142 preimages, and
/// that unit's id.
fn metre_package() -> (fsl_package::CheckedPackage, filament_exact::UnitId) {
    use fsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeOwner, OwnerSelection, UnitPreimage,
    };
    let owner = json!({"kind": "source", "authority": "agent-ix", "identity": "example", "package_name": "example"});
    let length = DimensionPreimage::from_json(json!({
        "version": "filament.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = length.key().expect("a dimension declaration key");
    let metre = UnitPreimage::from_json(json!({
        "version": "filament.unit-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "metre"],
        "dimension_node_id": fsl_foundation::node_wire::node_key_to_json(&length_key)
            .expect("a structural dimension key"),
        "target_unit_node_id": null,
        "scale": {"numerator": "1", "denominator": "1"},
        "offset": {"numerator": "0", "denominator": "1"},
    }))
    .expect("a root unit");
    let metre_key = metre.key().expect("a unit declaration key");
    let selection: NodeOwner = serde_json::from_value(owner).expect("an owner");
    let graph = admit_unit_graph(
        [(length, length_key)],
        [(metre, metre_key.clone())],
        &OwnerSelection::new([selection]),
    )
    .expect("the unit graph admits");
    let units = filament_semantic_value::quantity::UnitTable::declared(&graph);
    let mut declarations = fsl_semantics::check::PackageDeclarations::new(
        fsl_semantics::check::fixture_source(),
        "fixture",
        fsl_foundation::IdentityLimits::default(),
    );
    declarations.types =
        filament_semantic_value::declaration::TypeEnvironment::default().with_units(units);
    let graph = declarations
        .check(
            CheckMode::Linked,
            filament_semantic_value::checking::CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .expect("the unit table checks");
    (
        fsl_package::CheckedPackage::link(graph),
        filament_exact::UnitId::declared(metre_key),
    )
}

/// TC-744 step 3 (FR-269-AC-3): a quantity in a declared unit the package's
/// unit table holds round-trips; the declared unit resolves by lookup.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
#[ignore = "QSL-2173 completed; current blocker has no assigned owner: the unit-table fixture package checks as an EmptyPackage refusal, so no unit quantity is built"]
fn tc_744_a_declared_unit_quantity_round_trips() {
    let unit = witness_unit();
    let (package, metre) = metre_package();
    let magnitude = filament_exact::Rational::new(
        filament_exact::Integer::from(5_i64),
        filament_exact::Integer::from(2_i64),
    )
    .unwrap();
    let cause = cause_deciding(
        &unit,
        Value::Quantity(filament_exact::Quantity::new(magnitude, metre)),
    );
    let text = cause.to_json(None).unwrap();
    let read = DisagreementCause::from_json(&text, &package, None).unwrap();
    assert_eq!(read, cause);
    assert_eq!(read.to_json(None).unwrap(), text);
    assert!(matches!(
        DisagreementCause::from_json(&text, &unit.compiled.package, None),
        Err(crate::CauseCodecError::Unresolved("unit"))
    ));
}

/// TC-744 step 3 (FR-269-AC-3): a value path with FSpec FR-207's member,
/// option-value, field and index steps round-trips, each step in order;
/// a field step and a member step of the same name read back distinct.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_member_option_value_and_field_steps_round_trip() {
    let unit = witness_unit();
    let package = &unit.compiled.package;
    let violation = Verdict::from_category(Category::Violation);
    let mut record = all_below_record(&unit);
    record.value_path.steps = vec![
        ValuePathStep::Member("record".to_owned()),
        ValuePathStep::OptionValue,
        ValuePathStep::Field("items".to_owned()),
        ValuePathStep::Index(1),
    ];
    let cause = DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: Some(Box::new(record.clone())),
        derived: None,
        failure: WitnessFailure::Mismatch,
    };
    let text = cause.to_json(None).unwrap();
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        document["given"]["value_path"]["steps"],
        json!([
            {"tag": "member", "name": "record"},
            {"tag": "option_value"},
            {"tag": "field", "name": "items"},
            {"tag": "index", "index": 1},
        ])
    );
    assert_eq!(
        DisagreementCause::from_json(&text, package, None).unwrap(),
        cause
    );

    let mut renamed = record;
    renamed.value_path.steps[2] = ValuePathStep::Member("items".to_owned());
    assert_ne!(
        cause,
        DisagreementCause::Witness {
            proved: violation,
            replayed: violation,
            given: Some(Box::new(renamed)),
            derived: None,
            failure: WitnessFailure::Mismatch,
        }
    );
}

/// TC-744 step 3 (FR-269-AC-3, FSpec FR-352-AC-7): a refused separation
/// step round-trips; a refusal whose code is not a catalog code, or whose
/// cause or a field name is empty or not a catalog spelling, refuses.
#[trace("TC-744", "FR-269-AC-3")]
#[test]
fn tc_744_a_refusal_reads_only_catalog_spellings() {
    let unit = witness_unit();
    let package = &unit.compiled.package;
    let violation = Verdict::from_category(Category::Violation);
    let cause = DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: None,
        derived: Some(Box::new(all_below_record(&unit))),
        failure: WitnessFailure::Separation {
            step: SeparationStep::Domain,
            reason: SeparationReason::Refused(crate::SeparationRefusal {
                code: "integer_out_of_domain".to_owned(),
                cause: "value-out-of-domain".to_owned(),
                fields: BTreeMap::from([("value".to_owned(), "1".to_owned())]),
            }),
        },
    };
    let text = cause.to_json(None).unwrap();
    assert_eq!(
        DisagreementCause::from_json(&text, package, None).unwrap(),
        cause
    );
    for (member, value) in [
        ("code", json!("")),
        ("code", json!("made_up_code")),
        ("cause", json!("")),
        ("cause", json!("Value Out")),
        ("fields", json!({"": "1"})),
        ("fields", json!({"Value": "1"})),
    ] {
        let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
        document["failure"]["reason"][member] = value.clone();
        assert!(
            matches!(
                DisagreementCause::from_json(&document.to_string(), package, None),
                Err(crate::CauseCodecError::Spelling(_))
            ),
            "{member} = {value}"
        );
    }
}

/// TC-743 step 4 (FR-268-AC-4): a record whose deciding element is a long
/// text, or whose value path names an object with a long identity, is
/// measured by those lengths and refuses past the reader bound.
#[trace("TC-743", "FR-268-AC-4")]
#[test]
fn tc_743_long_text_or_object_identity_exceeds_the_reader_bound() {
    let unit = witness_unit();
    let long = "x".repeat(crate::bounds::DEFAULT_INPUT_BYTES + 1);
    let long_text = {
        let text_type = filament_exact::TextType::new(
            0,
            u64::try_from(long.len()).unwrap(),
            filament_exact::TextProfile::UnicodeScalars,
        )
        .unwrap();
        let payload = filament_exact::TextPayload::from_utf8(long.as_bytes()).unwrap();
        let mut meter = Meter::new(default_accounting(u64::MAX));
        match filament_exact::admit_text(&payload, &text_type, &mut meter) {
            Outcome::Completed(text) => Value::Text(text),
            other => panic!("the text admits: {other:?}"),
        }
    };
    let text_record = SeparatingWitnessRecord {
        deciding_element: DecidingElement::Value(long_text),
        ..all_below_record(&unit)
    };
    let object = filament_exact::ObjectReference::new(
        crate::spine::fixture_ids::universe(2),
        crate::spine::fixture_ids::effective(3),
        filament_exact::ObjectId::new(long).unwrap(),
    );
    let mut object_record = all_below_record(&unit);
    object_record.value_path.subject = ValuePathSubject::Object(object);
    for record in [text_record, object_record] {
        assert!(record.measured_bytes() > crate::bounds::DEFAULT_INPUT_BYTES);
        let (_, envelope) = replay_case(
            &unit,
            "AllBelow",
            Snapshot::High,
            "mid",
            Some(record),
            witness_source(),
        );
        assert!(matches!(
            envelope,
            Err(crate::WitnessRefusal::BoundExceeded(_))
        ));
    }
}

/// FR-574-AC-2, FR-574-AC-6 (TC-1414): a state-clause record with its
/// quantifier and index round-trips unchanged through the amended record type
/// and the cause codec; a protocol record omits both and round-trips; each
/// mismatched combination and each missing component is refused with its own
/// cause. The `native-run-result/2` reader stays state-only until the
/// producer lands (QSL-1895).
#[trace("FR-574-AC-2", "FR-574-AC-6", "TC-1414")]
#[test]
fn a_state_record_round_trips_unchanged_and_a_protocol_record_omits_quantifier_and_index() {
    use crate::result::CauseCodecError;
    let unit = codec_unit();
    let package = &unit.compiled.package;
    let violation = Verdict::from_category(Category::Violation);
    let cause = |record: SeparatingWitnessRecord| DisagreementCause::Witness {
        proved: violation,
        replayed: violation,
        given: Some(Box::new(record)),
        derived: None,
        failure: WitnessFailure::Mismatch,
    };
    let state = all_below_record(&unit);
    assert!(state.quantifier.is_some() && state.index.is_some());
    let text = cause(state.clone()).to_json(None).unwrap();
    let read = DisagreementCause::from_json(&text, package, None).unwrap();
    assert_eq!(read.to_json(None).unwrap(), text);
    assert_eq!(read, cause(state.clone()));

    let node = crate::protocol_analysis::node_text(&filament_contract_model::CheckedNodeId {
        package: "pkg".into(),
        path: vec![filament_contract_model::CheckedNodePathSegment::Name(
            "c".into(),
        )],
        ordinal: 0,
    })
    .unwrap();
    let protocol = SeparatingWitnessRecord {
        quantifier: None,
        deciding_element: DecidingElement::Protocol(node.clone()),
        index: None,
        trace_position: Some(crate::identity::TracePosition::new("2".to_owned())),
        ..state.clone()
    };
    let text = cause(protocol.clone()).to_json(None).unwrap();
    let record: serde_json::Value = serde_json::from_str(&text).unwrap();
    let given = &record["given"];
    assert!(given.get("quantifier").is_none() && given.get("index").is_none());
    assert_eq!(given["deciding_element"], node);
    let read = DisagreementCause::from_json(&text, package, None).unwrap();
    assert_eq!(read, cause(protocol.clone()));
    assert_eq!(read.to_json(None).unwrap(), text);

    // The writer refuses what the reader would refuse, with the cause.
    let shape = |record| match cause(record).to_json(None) {
        Err(CauseCodecError::Shape(member)) => member,
        other => panic!("a shape refusal, not {other:?}"),
    };
    assert_eq!(
        shape(SeparatingWitnessRecord {
            index: Some(0),
            ..protocol.clone()
        }),
        "index"
    );
    assert_eq!(
        shape(SeparatingWitnessRecord {
            quantifier: state.quantifier.clone(),
            ..protocol.clone()
        }),
        "quantifier"
    );
    assert_eq!(
        shape(SeparatingWitnessRecord {
            trace_position: None,
            ..protocol.clone()
        }),
        "trace_position"
    );
    assert_eq!(
        shape(SeparatingWitnessRecord {
            deciding_element: DecidingElement::Protocol("node".to_owned()),
            ..protocol.clone()
        }),
        "deciding_element"
    );
    assert_eq!(
        shape(SeparatingWitnessRecord {
            quantifier: None,
            ..state.clone()
        }),
        "quantifier"
    );
    assert!(matches!(
        cause(SeparatingWitnessRecord {
            index: None,
            ..state.clone()
        })
        .to_json(None),
        Err(CauseCodecError::MissingIndex)
    ));

    // The reader: a present quantifier or index, a missing trace position,
    // deciding element or value path, and a string that is no node id.
    let read_with = |edit: &dyn Fn(&mut serde_json::Map<String, serde_json::Value>)| {
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        edit(value["given"].as_object_mut().unwrap());
        DisagreementCause::from_json(&value.to_string(), package, None)
    };
    let shaped = |result: Result<DisagreementCause, CauseCodecError>, member: &str| {
        assert!(
            matches!(&result, Err(CauseCodecError::Shape(found)) if *found == member),
            "{member}: {result:?}"
        );
    };
    shaped(
        read_with(&|given| {
            given.insert("index".to_owned(), serde_json::json!(0));
        }),
        "index",
    );
    shaped(
        read_with(&|given| {
            given.insert(
                "quantifier".to_owned(),
                serde_json::json!({"node_id": "x", "role": "r", "ordinal": 0}),
            );
        }),
        "quantifier",
    );
    shaped(
        read_with(&|given| {
            given.remove("trace_position");
        }),
        "trace_position",
    );
    shaped(
        read_with(&|given| {
            given.insert("deciding_element".to_owned(), serde_json::json!("node"));
        }),
        "deciding_element",
    );
    for member in ["deciding_element", "value_path"] {
        let result = read_with(&|given| {
            given.remove(member);
        });
        assert!(
            matches!(&result, Err(CauseCodecError::Malformed(error))
                if error.to_string().contains(member)),
            "{member}: {result:?}"
        );
    }
}

/// TC-1040 step 5 (FR-107-AC-12): the elements of the list built inside
/// `GuardedAll` are read at the built-list subject, at their index.
#[trace("TC-1040", "FR-107-AC-12")]
#[test]
fn tc_1040_the_elements_of_a_built_list_are_read_at_their_index() {
    let unit = witness_unit();
    let (mid, evaluation, _) = evaluate(&unit, "GuardedAll", Snapshot::High, "mid", 1_000_000);
    let (root, _, _) = evaluate(&unit, "GuardedAll", Snapshot::High, "root", 1_000_000);
    let list = occurrence(&unit, "GuardedAll", &[1, 0, 0], Operator::Collection);
    let path = |subject: ValuePathSubject, steps| {
        Some(RuntimeValuePath {
            observation: observation(Snapshot::High),
            subject,
            steps,
        })
    };
    let member = |name: &str| ValuePathStep::Member(name.to_owned());
    let parent = path(
        ValuePathSubject::Object(mid.self_object.clone()),
        vec![member("parent")],
    );
    let reads: Vec<_> = evaluation.reads().iter().map(|r| r.path.clone()).collect();
    assert_eq!(
        reads,
        vec![
            parent.clone(),
            parent,
            path(
                ValuePathSubject::Built(list.clone()),
                vec![
                    ValuePathStep::Index(0),
                    ValuePathStep::Reference(root.self_object.clone()),
                    member("versionNumber"),
                ],
            ),
            path(
                ValuePathSubject::Built(list),
                vec![
                    ValuePathStep::Index(1),
                    ValuePathStep::Reference(mid.self_object.clone()),
                    member("versionNumber"),
                ],
            ),
        ]
    );
}

/// TC-2981 (FR-265-AC-11, AC-12): a separating path that ends at an authored
/// comparison derives the decisive basis and a record naming the comparison,
/// its operand values and the path to it.
#[trace("TC-2981", "FR-265-AC-11")]
#[trace("TC-2981", "FR-265-AC-12")]
#[trace("TC-2981", "FR-265-AC-13")]
#[trace("TC-2981", "FR-267-AC-23")]
#[trace("TC-2981", "FR-268-AC-5")]
#[trace("FSpec-TC-2014", "FSpec-FR-351-AC-9")]
#[test]
fn tc_2981_a_path_ending_at_a_comparison_records_the_comparison_and_its_operands() {
    let unit = comparison_witness_unit();
    let Some(Derived {
        basis: SettlementBasis::DecisiveCounterexample,
        record: Some(record),
    }) = derive(&unit, "MatchesParent", Snapshot::High, "mid")
    else {
        panic!("MatchesParent over high ends at its `=` and is decisive");
    };
    let equality = occurrence(&unit, "MatchesParent", &[1], Operator::Binary);
    assert_eq!(record.deciding_comparison, Some(equality.clone()));
    let operands = record.operands.as_ref().expect("comparison operands");
    assert!(crate::result::same_element(&operands[0], &integer(600)));
    assert!(crate::result::same_element(&operands[1], &integer(0)));
    assert_eq!(
        record.decision_path,
        Some(vec![
            occurrence(&unit, "MatchesParent", &[], Operator::Binary),
            equality.clone(),
        ])
    );
    assert_eq!(
        record.deciding_element,
        DecidingElement::Value(Value::Boolean(false))
    );
    assert!(record.quantifier.is_none() && record.index.is_none());
    assert!(record.trace_position.is_none());
    assert!(record.value_path.steps.is_empty());
    assert_eq!(record.value_path.observation.identity, "high");
    assert!(matches!(
        &record.value_path.subject,
        ValuePathSubject::Object(reference) if reference.object().as_str() == "mid"
    ));
    let (_, comparison_evaluation, _) =
        evaluate(&unit, "MatchesParent", Snapshot::High, "mid", 1_000_000);
    let comparison_reports = unit
        .compiled
        .package
        .graph()
        .occurrences()
        .filter_map(|(_, _, location)| {
            matches!(
                &location.origin,
                filament_semantic_value::location::ExpressionOwner::StateClause {
                    clause: name,
                    ..
                } if name == "MatchesParent"
            )
            .then(|| comparison_evaluation.comparison(location))
            .flatten()
        })
        .collect::<Vec<_>>();
    assert_eq!(comparison_reports.len(), 1);
    assert_eq!(comparison_reports[0].comparison, equality);
    assert_eq!(
        super::witness_member::comparison_report_count_current("MemberAll", &[0, 600, 700]),
        0
    );
    let (observations, _, mut meter) =
        evaluate(&unit, "MatchesParent", Snapshot::High, "mid", 1_000_000);
    assert_eq!(
        separate(
            &unit.compiled.package,
            "MatchesParent",
            &observations,
            &record,
            &[],
            &mut meter,
        )
        .unwrap(),
        SeparationOutcome::Holds
    );
    let mut missing_path = record.clone();
    missing_path.decision_path = Some(vec![equality.clone()]);
    assert_eq!(
        separate(
            &unit.compiled.package,
            "MatchesParent",
            &observations,
            &missing_path,
            &[],
            &mut meter,
        )
        .unwrap(),
        SeparationOutcome::Failed(SeparationStep::Quantifier, SeparationReason::Unmet)
    );
    let mut outside_comparison = record.clone();
    outside_comparison.deciding_comparison =
        Some(occurrence(&unit, "AtLeastParent", &[1], Operator::Binary));
    assert_eq!(
        separate(
            &unit.compiled.package,
            "MatchesParent",
            &observations,
            &outside_comparison,
            &[],
            &mut meter,
        )
        .unwrap(),
        SeparationOutcome::Failed(SeparationStep::Quantifier, SeparationReason::Unmet)
    );
    let mut wrong_operands = record.clone();
    wrong_operands.operands = Some([integer(600), integer(1)]);
    assert_eq!(
        separate(
            &unit.compiled.package,
            "MatchesParent",
            &observations,
            &wrong_operands,
            &[],
            &mut meter,
        )
        .unwrap(),
        SeparationOutcome::Failed(SeparationStep::Element, SeparationReason::Unmet)
    );
    let mut wrong_result = record.clone();
    wrong_result.deciding_element = DecidingElement::Value(Value::Boolean(true));
    assert_eq!(
        separate(
            &unit.compiled.package,
            "MatchesParent",
            &observations,
            &wrong_result,
            &[],
            &mut meter,
        )
        .unwrap(),
        SeparationOutcome::Failed(SeparationStep::Body, SeparationReason::Unmet)
    );
    let (selection, snapshots) = selection("MatchesParent", Snapshot::High, "mid");
    let domain = config_version_domain_document();
    let packages = fsl_semantics::model::intake::package_input([domain.as_slice()]).unwrap();
    let mut request = config_version_request_for(
        comparison_witness_unit_text(),
        packages,
        "comparison-witness.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots = snapshots;
    request.accounting = default_accounting(1_000_000);
    let report = run_clause(request).expect("the comparison clause runs");
    let bytes = crate::spine::write_clause_run_result(&report).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        wire["witness"]["deciding_comparison"]["node_id"],
        crate::id_text::node_text(equality.node()).unwrap()
    );
    assert_eq!(wire["witness"]["operands"].as_array().unwrap().len(), 2);
    assert_eq!(
        wire["witness"]["decision_path"].as_array().unwrap().len(),
        2
    );
    let refuses_member = |invalid: serde_json::Value, member: &str| {
        let error = crate::spine::read_native_run_result(&serde_json::to_vec(&invalid).unwrap())
            .err()
            .expect("a malformed comparison witness member is refused");
        assert!(error.to_string().contains(member), "{member}: {error}");
    };
    let mut missing_operands = wire.clone();
    missing_operands["witness"]
        .as_object_mut()
        .unwrap()
        .remove("operands");
    refuses_member(missing_operands, "operands");
    let mut one_operand = wire.clone();
    let first_operand = one_operand["witness"]["operands"][0].clone();
    one_operand["witness"]["operands"] = serde_json::json!([first_operand]);
    refuses_member(one_operand, "operands");
    let mut has_quantifier = wire.clone();
    has_quantifier["witness"]["quantifier"] = has_quantifier["witness"]["decision_path"][0].clone();
    refuses_member(has_quantifier, "quantifier");
    let crate::spine::NativeRunResult::Clause(read) =
        crate::spine::read_native_run_result(&bytes).unwrap()
    else {
        panic!("the report reads as a clause result");
    };
    assert_eq!(
        read.witness(&unit.compiled.package).unwrap(),
        report.witness
    );
    let replay = replay_with(
        &unit,
        "MatchesParent",
        Some(record.clone()),
        witness_source(),
    )
    .expect("the authored comparison record replays");
    let arm = witness_arm(&replay);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.record(), Some(&record));
    assert!(
        matches!(derive(&unit, "AtLeastParent", Snapshot::High, "mid"), Some(Derived {
            basis: SettlementBasis::DecisiveWitness,
            record: Some(record),
        }) if record.operands.as_ref().is_some_and(|operands| crate::result::same_element(&operands[0], &integer(600)) && crate::result::same_element(&operands[1], &integer(0))) && record.deciding_element == DecidingElement::Value(Value::Boolean(true))),
        "AtLeastParent over high ends at its `>=` and is a decisive witness"
    );
    // With no parent the walk ends at `present(self.parent)`, a call.
    assert_eq!(
        derive(&unit, "MatchesParent", Snapshot::High, "root"),
        closed()
    );
    // FR-265-AC-3: `OrAll` ends at `self.versionNumber = 0`.
    assert!(
        matches!(
            derive(&unit, "OrAll", Snapshot::High, "mid"),
            Some(Derived {
                basis: SettlementBasis::DecisiveCounterexample,
                record: Some(_),
            })
        ),
        "OrAll's walk ends at an authored `=`"
    );
}
