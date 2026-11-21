// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1330 (FR-530-AC-1 to AC-8) and FSpec TC-983, TC-985 over real native
//! packages: `Producer` and `Consumer` are compiled as checked packages and
//! imported by the composing unit of FSpec TC-985 case 1. Each component
//! guarantee slot holds the FR-331 request item `fsl-route` writes for it and
//! a result entry for it; the supplied guarantees are the pool.

use filament_semantic_value::checking::CheckMode;
use std::sync::Arc;

use filament_semantic_value::checking::CheckingLimits;
use fsl_package::{
    emit_checked, read_import_view, AdmittedPackages, CheckedPackage, EmitLimits, Import,
    V2ReadLimits,
};
use fsl_replay::planner::{
    interface_mismatch, DeclarationRefusal, MismatchReason, PlannerCause, PlannerDeclaration,
    PlannerResult, PlannerValue, SlotRole,
};
use fsl_replay::{Certification, InconclusiveCause, PremiseOutcome};
use fsl_route::request::RequestWriter;
use fsl_route::{CandidateOutcome, Registry};
use fsl_semantics::check::{
    AdmittedImport, Capability, CheckRefusal, PackageDeclarations, SlotRole as SemRole,
};
use fsl_semantics::library::LibraryName;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::planner_chain::{Chain, Failure, PACKAGE};

#[path = "../../fsl-semantics/tests/support/composition.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     profile t = \"filament.temporal.infinite-trace/v1\";\n\
     model M = \"test/orders\";\n";

const PRODUCER: &str = "temporal Sends using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always eventually holds(p.sent) }\n\
     temporal Counted using t over (p: M::Producer) clock \"model-steps\" \
     on origin { always holds(p.sent_count >= 0) }\n";

const CONSUMER: &str = "temporal Done using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always (holds(c.received) implies eventually holds(c.done)) }\n\
     temporal Bounded using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always holds(c.received_count >= 0) }\n";

/// `PRODUCER` with a safety clause conditional on fairness, and a liveness
/// clause, both for interface-mismatch ordering.
const PRODUCER_WITH_FENCE: &str =
    "temporal Sends using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always eventually holds(p.sent) }\n\
     temporal Fenced using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always holds(p.sent_count >= 0) }\n\
     temporal Pings using t over (p: M::Producer) clock \"model-steps\" \
     on origin { always eventually holds(p.sent) }\n";

const COMPONENTS: &str = "component Producer from P using M; component Consumer from Q using M; ";

/// FSpec TC-985 case 1's members.
const CASE_ONE_MEMBERS: &str = "guarantee Producer::Sends; guarantee Consumer::Done; \
     guarantee Producer::Counted; rely Consumer::Done on Producer::Counted; \
     depends Consumer::Done on Producer::Sends; supplies fair weak each M::Producer::send; ";

/// The slots of FSpec TC-985 case 1 and their positions.
const SENDS: usize = 0;
const DONE: usize = 1;
const COUNTED: usize = 2;
const RELY: usize = 3;
const DEPENDS: usize = 4;
const FAIRNESS: usize = 5;

struct Dependency {
    package: Arc<CheckedPackage>,
    import: AdmittedImport,
    identity: LibraryName,
}

fn dependency(identity: &str, clauses: &str) -> Dependency {
    let unit = format!("{MODEL_HEADER}{clauses}");
    let bytes = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let models =
        fsl_semantics::model::intake::package_input([bytes.as_slice()]).expect("the model package");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", identity),
        "dependency.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the dependency");
    assert!(parsed.is_admissible(), "{unit}: {:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the dependency");
    let selected = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &models,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    let declarations =
        PackageDeclarations::assemble(raw, identity.to_owned(), built, selected, Vec::new())
            .expect("the dependency assembles");
    let package = Arc::new(CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("the dependency checks"),
    ));
    let emission = emit_checked(&package, EmitLimits::fixture()).expect("the dependency emits");
    let identity = LibraryName::new(identity).expect("a library identity");
    let view = read_import_view(
        &package,
        &emission,
        identity.clone(),
        &models,
        &mut AdmittedPackages::default(),
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the dependency import view is admitted");
    Dependency {
        import: AdmittedImport {
            identity: identity.clone(),
            view,
            graph: package.shared_graph(),
        },
        package,
        identity,
    }
}

fn compile(declaration: &str, dependencies: &[&Dependency]) -> Result<CheckedPackage, Failure> {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         import \"test/producer\" as P;\n\
         import \"test/consumer\" as Q;\n{declaration}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:composition"),
        "program.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(Failure::Syntax(parsed.diagnostics().to_vec()));
    }
    let raw = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let imports: Vec<AdmittedImport> = dependencies
        .iter()
        .map(|dependency| dependency.import.clone())
        .collect();
    let no_models = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("an empty model package input");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &no_models,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("an empty model selection is admitted");
    let assembled = PackageDeclarations::assemble(raw, PACKAGE.to_owned(), built, models, imports)
        .map_err(Failure::Assembly)?;
    let graph = assembled
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map_err(|refusals: Vec<CheckRefusal>| Failure::Check(refusals))?;
    Ok(CheckedPackage::link_with(
        graph,
        dependencies
            .iter()
            .map(|dependency| Import {
                identity: dependency.identity.clone(),
                package: Arc::clone(&dependency.package),
            })
            .collect(),
        EmitLimits::fixture(),
    )
    .expect("the dependency closure links"))
}

struct World {
    producer: Dependency,
    consumer: Dependency,
}

impl World {
    fn new() -> Self {
        Self {
            producer: dependency("test/producer", PRODUCER),
            consumer: dependency("test/consumer", CONSUMER),
        }
    }

    fn declare(&self, rule: &str, members: &str) -> String {
        format!("compose Sys rule {rule} {{ {COMPONENTS}{members}}}")
    }

    fn package(&self, rule: &str, members: &str) -> CheckedPackage {
        let declaration = self.declare(rule, members);
        compile(&declaration, &[&self.producer, &self.consumer])
            .unwrap_or_else(|failure| panic!("{declaration}: the unit compiles: {failure:?}"))
    }

    fn chain(&self, rule: &str, members: &str) -> Chain {
        let package = self.package(rule, members);
        let node = package.graph().compositions()[0].node().clone();
        Chain::of(package, &node).expect("the composition derives")
    }
}

fn invalid(pointer: &str) -> (&'static str, &'static str, String) {
    ("invalid_runtime_input", "invalid-value", pointer.to_owned())
}

fn premise(result: &PlannerResult) -> Option<(u64, PremiseOutcome)> {
    match &result.cause {
        Some(PlannerCause::Inconclusive(InconclusiveCause::PremiseUnproved {
            premise,
            result,
        })) => Some((*premise, *result)),
        _ => None,
    }
}

/// A premise result for the item of component-guarantee slot `slot`, proved.
fn pool_entry(chain: &Chain, slot: usize) -> Value {
    json!({"request": chain.item_json(slot), "result": chain.proved(slot)})
}

/// The positive request: every component guarantee proved, the `Counted` and
/// `Sends` guarantees supplied in the pool.
fn positive(chain: &Chain) -> Value {
    let mut request = chain.request(&|_| Value::Null);
    request["guarantees"] = json!([pool_entry(chain, COUNTED), pool_entry(chain, SENDS)]);
    request
}

// ---- FR-530-AC-3: the positive Producer/Consumer fixture -----------------------

/// FSpec TC-983 case 1, TC-985 case 1: relies, dependencies and fairness are
/// each discharged; the item settles `proved`, `decisive-witness`, with the
/// discharge record.
#[trace("TC-1330", "FR-530-AC-3")]
#[trace("FSpec-TC-983", "FSpec-FR-489-AC-1")]
#[trace("FSpec-TC-985", "FSpec-FR-491-AC-1")]
#[test]
fn a_discharged_producer_consumer_composition_settles_proved_with_its_discharge_record() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);
    assert_eq!(chain.declaration.slots().len(), 6);
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.value.basis().as_str(), "decisive-witness");
    let discharge = result.discharge.as_ref().expect("a discharge record");
    let slots: Vec<usize> = discharge.entries.iter().map(|entry| entry.slot).collect();
    for slot in [RELY, DEPENDS, FAIRNESS] {
        assert!(slots.contains(&slot), "slot {slot} is discharged");
    }
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "proved");
    assert!(wire["discharge"].is_object());
}

// ---- FR-530-AC-2: a mismatched guarantee discharges nothing --------------------

#[trace("TC-1330", "FR-530-AC-2")]
#[trace("FSpec-TC-983", "FSpec-FR-489-AC-2")]
#[test]
fn a_guarantee_of_another_subject_clause_kind_or_package_leaves_the_slot_undischarged() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);
    let sends = pool_entry(&chain, SENDS);
    let counted = pool_entry(&chain, COUNTED);
    let done = pool_entry(&chain, DONE);
    let mut third = sends.clone();
    third["request"]["claim"]["node"]["package"] = json!("third@1");
    // The dependency's guarantee over `Consumer`, a weaker `Producer`
    // clause, a safety guarantee, and a same-named `Sends` of a third
    // package.
    let mut weaker = sends.clone();
    weaker["request"]["claim"] = counted["request"]["claim"].clone();
    for supplied in [done, weaker, counted.clone(), third] {
        let mut request = chain.request(&|_| Value::Null);
        request["guarantees"] = json!([counted, supplied]);
        let result = chain.settled(&request);
        assert_eq!(result.value, PlannerValue::Inconclusive);
        assert_eq!(
            premise(&result).map(|(slot, _)| slot),
            Some(DEPENDS as u64),
            "{supplied}"
        );
    }
}

// ---- FR-530-AC-4: cycles ---------------------------------------------------------

const MUTUAL: &str = "depends Producer::Sends on Consumer::Done; ";

/// FSpec FR-489 "Cycles": a well-formed declaration whose dependency graph
/// holds a cycle has no planner declaration and writes no request item, under
/// every rule; no `fixed-point` slot is derived.
#[trace("TC-1330", "FR-530-AC-4")]
#[trace("FSpec-TC-983", "FSpec-FR-489-AC-4", "FSpec-TC-985")]
#[test]
fn a_mutual_dependency_derives_no_planner_declaration_and_no_request_item() {
    let world = World::new();
    let members = format!("{CASE_ONE_MEMBERS}{MUTUAL}");
    for rule in [
        "assume-guarantee",
        "interface",
        "protocol-refinement",
        "refinement-transfer",
    ] {
        let package = world.package(rule, &members);
        let composition = &package.graph().compositions()[0];
        let [cycle] = composition.cycles() else {
            panic!("{rule}: one cycle");
        };
        let named: Vec<String> = cycle.guarantees.iter().map(ToString::to_string).collect();
        assert_eq!(named, ["Consumer::Done", "Producer::Sends"], "{rule}");
        if rule == "assume-guarantee" {
            assert_eq!(
                PlannerDeclaration::from_package(&package, composition.node()).err(),
                Some(DeclarationRefusal::Cyclic)
            );
        }
        // route writes no item for the cyclic composition.
        assert!(RequestWriter::new()
            .declaration_items(package.graph())
            .is_empty());
    }
}

// ---- FR-530-AC-9: declared interface mismatch ------------------------------------

/// A declared interface absent from the composition refutes it: the peer
/// guarantee `Counted` is no `guarantee` member.
const ABSENT_MEMBERS: &str = "guarantee Producer::Sends; guarantee Consumer::Done; \
     rely Consumer::Done on Producer::Counted; supplies fair weak each M::Producer::send; ";

/// FSpec FR-492 "Interface mismatch", TC-1330: the refutation names the peer
/// component, the interface node, the `rely` slot and the reason, keeps the
/// complete slot list and carries no cause, certification or conclusion.
#[trace("TC-1330", "FR-530-AC-9")]
#[trace("FSpec-TC-983", "FSpec-FR-348-AC-2")]
#[test]
fn an_absent_peer_interface_refutes_the_composition_with_its_full_slot_list() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", ABSENT_MEMBERS);
    let result = chain.settled(&chain.request(&|_| Value::Null));
    assert_eq!(result.value, PlannerValue::Refuted);
    assert_eq!(result.value.basis().as_str(), "decisive-counterexample");
    assert!(result.cause.is_none());
    assert!(result.certification.is_none());
    assert!(result.conclusion.is_none());
    assert_eq!(result.slots.len(), chain.declaration.slots().len());
    let mismatch = result.mismatch.as_ref().expect("the mismatch");
    assert_eq!(mismatch.component, "Producer");
    assert_eq!(mismatch.premise, 2);
    assert_eq!(mismatch.reason, MismatchReason::Absent);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "refuted");
    assert_eq!(wire["basis"], "decisive-counterexample");
    assert_eq!(wire["mismatch"]["component"], "Producer");
    assert_eq!(wire["mismatch"]["premise"], 2);
    assert_eq!(wire["mismatch"]["reason"], "absent");
    assert!(wire.get("cause").is_none() && wire.get("certification").is_none());
    assert!(wire.get("conclusion").is_none());
}

/// A peer guarantee whose fairness set no supplied constraint implies is
/// `weaker`; supplying the constraint removes the mismatch.
#[trace("TC-1330", "FR-530-AC-9")]
#[test]
fn a_peer_guarantee_conditional_on_unsupplied_fairness_is_weaker() {
    let world = World::new();
    let members = "guarantee Producer::Sends; guarantee Consumer::Done; \
         depends Consumer::Done on Producer::Sends; ";
    let chain = world.chain("assume-guarantee", members);
    let result = chain.settled(&chain.request(&|_| Value::Null));
    assert_eq!(result.value, PlannerValue::Refuted);
    let mismatch = result.mismatch.as_ref().expect("the mismatch");
    assert_eq!(
        (mismatch.premise, mismatch.reason),
        (2, MismatchReason::Weaker)
    );
    // The same declaration with the constraint supplied has no mismatch.
    let supplied = format!("{members}supplies fair weak each M::Producer::send; ");
    let chain = world.chain("assume-guarantee", &supplied);
    assert!(chain.declaration.mismatch().is_none());
}

/// FSpec FR-492 "Interface mismatch": the order is reason-major, so an
/// `absent` slot is reported although a lower slot is `weaker`.
#[trace("TC-1330", "FR-530-AC-9")]
#[test]
fn an_absent_mismatch_is_reported_before_a_weaker_one_at_a_lower_slot() {
    let producer = dependency("test/producer", PRODUCER_WITH_FENCE);
    let consumer = dependency("test/consumer", CONSUMER);
    let members = "guarantee Producer::Sends; guarantee Consumer::Done; \
         guarantee Producer::Fenced; rely Consumer::Done on Producer::Fenced; \
         depends Consumer::Done on Producer::Pings; ";
    let declaration = format!("compose Sys rule assume-guarantee {{ {COMPONENTS}{members}}}");
    let package = compile(&declaration, &[&producer, &consumer]).expect("the unit compiles");
    let composition = &package.graph().compositions()[0];
    let found = interface_mismatch(composition).expect("a mismatch");
    // `Fenced` (slot 3) is weaker, `Pings` (slot 4) is absent.
    assert_eq!((found.premise, found.reason), (4, MismatchReason::Absent));
    assert_eq!(found.component, "Producer");
}

/// A wrong or unproved supplied result under a declared, non-weaker
/// interface is never a mismatch (FSpec FR-492 "Interface mismatch").
#[trace("TC-1330", "FR-530-AC-9")]
#[test]
fn a_supplied_result_that_does_not_prove_a_declared_interface_is_not_a_mismatch() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);
    assert!(chain.declaration.mismatch().is_none());
    let mut request = positive(&chain);
    request["guarantees"] = json!([]);
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert!(result.mismatch.is_none());
    assert!(premise(&result).is_some());
}

// ---- FR-530-AC-6: trust, assumption, unsupported, failed, exhausted --------------

#[trace("TC-1330", "FR-530-AC-6")]
#[trace("FSpec-TC-983", "FSpec-FR-489-AC-5")]
#[test]
fn each_outcome_keeps_its_own_trust_basis_and_extent() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);

    // A `trusted` plugin discharger settles `proved`, `trusted`.
    let mut trusted = positive(&chain);
    let plugin = chain.entry(
        SENDS,
        json!({"result": "proved", "backend": "acme", "basis": {"type": "exhaustive"}, "certification": "trusted"}),
    );
    trusted["slots"][SENDS]["result"]["result"] = plugin;
    let read = chain.read(&trusted).expect("admitted");
    let settled = fsl_replay::planner::settle(
        &chain.package,
        &read,
        fsl_replay::planner::Budget::default(),
    )
    .expect("settles");
    assert_eq!(settled.value, PlannerValue::Proved);
    assert_eq!(settled.certification, Some(Certification::Trusted));

    // An assumption-only premise: the only discharger of the dependency is an
    // environment record.
    let members = format!(
        "{CASE_ONE_MEMBERS}environment Net by \"network\" fair weak each M::Producer::send; "
    );
    let assumed = world.chain("assume-guarantee", &members);
    let env = assumed.declaration.slots().len() - 1;
    assert_eq!(assumed.declaration.slots()[env].role, SlotRole::Environment);
    let context = match &assumed.declaration.environment()[0].supplies {
        fsl_replay::planner::EnvSupplies::Fairness(premise) => {
            fsl_foundation::node_wire::node_key_to_json(&premise.operation.context).expect("a node")
        }
        fsl_replay::planner::EnvSupplies::Assumption(_) => panic!("a scheduling member"),
    };
    let record = |_: &str| {
        json!({"name": "Net", "supplier": "network", "supplies": {"fairness": {
            "kind": "weak", "granularity": "each", "context": context, "operation": "send",
        }}})
    };
    let mut request = assumed.request(&record);
    request["guarantees"] = json!([pool_entry(&assumed, COUNTED)]);
    request["discharges"] = json!([{"slot": DEPENDS, "environment": env}]);
    let result = assumed.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((DEPENDS as u64, PremiseOutcome::Assumption))
    );

    // A failed component check leaves its slot undischarged.
    let mut failed = positive(&chain);
    failed["slots"][SENDS]["result"]["result"] = chain.entry(
        SENDS,
        json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
    );
    let result = chain.settled(&failed);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(premise(&result).map(|(slot, _)| slot), Some(SENDS as u64));

    // An exhausted budget.
    let mut exhausted = positive(&chain);
    exhausted["limits"] = json!({"max_work": 2});
    let result = chain.settled(&exhausted);
    assert_eq!(result.value, PlannerValue::Incomplete);
    assert_eq!(
        result.to_wire().expect("wire")["cause"]["limit"],
        "max_work"
    );

    // An unsupported rule: `interface` derives a FR-331 `composition` request
    // item, and with no composition-capable backend registered its candidate
    // set is empty (FSpec FR-348-AC-4).
    let package = world.package("interface", CASE_ONE_MEMBERS);
    let node = package.graph().compositions()[0].node().clone();
    assert_eq!(
        PlannerDeclaration::from_package(&package, &node).err(),
        Some(DeclarationRefusal::NotPlannerLocal("interface"))
    );
    let written = RequestWriter::new().declaration_items(package.graph());
    let own = written
        .iter()
        .find(|declared| declared.slot.is_none())
        .expect("the composition request item");
    assert_eq!(own.item.kind(), Capability::Composition);
    assert!(!own.item.premises().is_empty());
    assert!(own
        .item
        .premises()
        .iter()
        .any(|premise| premise.role() == SemRole::ComponentGuarantee));
    let CandidateOutcome::Candidates(candidates) =
        Registry::new().candidates(own.item.kind(), None)
    else {
        panic!("a computed candidate set");
    };
    assert!(candidates.is_empty(), "unsupported: no composition backend");
}

// ---- FR-530-AC-7: discharge records ---------------------------------------------

#[trace("TC-1330", "FR-530-AC-7")]
#[trace("FSpec-TC-983", "FSpec-FR-489-AC-10")]
#[test]
fn a_discharge_record_is_admitted_against_the_pool_and_discharges_only_a_proved_result() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);
    // Naming no pool entry.
    let mut missing = positive(&chain);
    missing["discharges"] = json!([{"slot": DEPENDS, "guarantee": 9}]);
    assert_eq!(chain.refusal(&missing), invalid("/discharges/0"));
    // A pool entry over another subject.
    let mut other = positive(&chain);
    other["guarantees"] = json!([pool_entry(&chain, COUNTED), pool_entry(&chain, DONE)]);
    other["discharges"] = json!([{"slot": DEPENDS, "guarantee": 1}]);
    assert_eq!(chain.refusal(&other), invalid("/discharges/0"));
    // An `inconclusive` result over the right subject is admitted and
    // discharges nothing.
    let mut weak = chain.request(&|_| Value::Null);
    let mut sends = pool_entry(&chain, SENDS);
    sends["result"] = chain.entry(
        SENDS,
        json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
    );
    weak["guarantees"] = json!([pool_entry(&chain, COUNTED), sends]);
    weak["discharges"] = json!([{"slot": DEPENDS, "guarantee": 1}]);
    let result = chain.settled(&weak);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(premise(&result).map(|(slot, _)| slot), Some(DEPENDS as u64));
}

// ---- FR-530-AC-8: forged slots and foreign identities ---------------------------

#[trace("TC-1330", "FR-530-AC-8")]
#[trace("FSpec-TC-983", "FSpec-FR-492-AC-2", "FSpec-FR-492-AC-5")]
#[test]
fn a_forged_or_reordered_slot_and_a_result_for_another_identity_are_refused() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS);
    let mut swapped = positive(&chain);
    swapped["slots"].as_array_mut().expect("slots").swap(0, 1);
    assert_eq!(chain.refusal(&swapped), invalid("/slots/0"));
    let mut forged = positive(&chain);
    forged["slots"][DONE]["identity"]["claim"]["node"]["package"] = json!("other@2");
    assert_eq!(chain.refusal(&forged), invalid("/slots/1"));
    let mut foreign = positive(&chain);
    foreign["slots"][SENDS]["result"] = foreign["slots"][COUNTED]["result"].clone();
    assert_eq!(chain.refusal(&foreign), invalid("/slots/0/result"));
    // The unrelated route items of the package are unchanged by the
    // declaration: its component-guarantee items are the only ones written.
    let written = RequestWriter::new().declaration_items(chain.package.graph());
    assert_eq!(written.len(), 3);
}
