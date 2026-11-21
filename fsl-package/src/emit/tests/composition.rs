// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-530-AC-1 (the compile, read-back and lists-separate part), FSpec
//! TC-985 (FR-491-AC-1 to AC-6): a `compose` declaration over the checked
//! `Producer` and `Consumer` packages of TC-983 is compiled from real source
//! through S1, S2 and S3 against two imported checked packages, its derived
//! item and slots are read, its node is written and read back, and every
//! refusal carries FSpec FR-491's code and cause.

use filament_semantic_value::checking::CheckMode;
use std::collections::BTreeSet;
use std::sync::Arc;

use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_semantics::check::{
    AbstractionRefusal, AdmittedImport, CheckCause, CheckRefusal, CheckedComposition,
    CompositionName, CompositionRefusal, CompositionRule, CompositionSlot, CompositionSlotKey,
    ItemPlacement, MalformedMember, SlotBacking, SlotRole,
};
use fsl_semantics::library::LibraryName;
use ix_trace_rs::trace;

use super::*;

#[path = "../../../../fsl-semantics/tests/support/composition.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     profile t = \"filament.temporal.infinite-trace/v1\";\n\
     model M = \"test/orders\";\n";

/// `Producer`'s package: the liveness guarantee `Sends` under its fairness
/// and the safety guarantee `Counted`.
const PRODUCER: &str = "temporal Sends using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always eventually holds(p.sent) }\n\
     temporal Counted using t over (p: M::Producer) clock \"model-steps\" \
     on origin { always holds(p.sent_count >= 0) }\n";

/// `Consumer`'s package: the liveness guarantee `Done` and the safety
/// guarantee `Bounded`.
const CONSUMER: &str = "temporal Done using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always (holds(c.received) implies eventually holds(c.done)) }\n\
     temporal Bounded using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always holds(c.received_count >= 0) }\n";

const COMPONENTS: &str = "component Producer from P using M; component Consumer from Q using M; ";

/// TC-985 case 1's members.
const CASE_ONE_MEMBERS: &str = "guarantee Producer::Sends; guarantee Consumer::Done; \
     guarantee Producer::Counted; rely Consumer::Done on Producer::Counted; \
     depends Consumer::Done on Producer::Sends; supplies fair weak each M::Producer::send; ";

/// Why a unit does not compile to a checked package.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "the refusal is read through `Debug` in a failure message"
)]
enum Failure {
    Syntax(Vec<fsl_cst::CompleteDiagnostic>),
    Assembly(fsl_semantics::check::AssemblyRefusal),
    Check(Vec<CheckRefusal>),
}

/// A checked dependency package and the import that names it.
struct Dependency {
    package: Arc<CheckedPackage>,
    import: AdmittedImport,
    identity: LibraryName,
}

/// Compiles `clauses` over the fixture model as the library `identity`.
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
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .expect("the dependency checks"),
    ));
    let emission = emit_checked(&package).expect("the dependency emits");
    let identity = LibraryName::new(identity).expect("a library identity");
    let view = crate::checked_v2::read_import_view(
        &package,
        &emission,
        identity.clone(),
        &models,
        &mut crate::checked_v2::AdmittedPackages::default(),
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
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

/// The composing unit: imports `P` and `Q` and holds `declaration`.
fn unit(declaration: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile q = \"filament.value.complete/v1\";\n\
         import \"test/producer\" as P;\n\
         import \"test/consumer\" as Q;\n\
         function marker using q(): Boolean pure {{ true }}\n\
         {declaration}\n"
    )
}

fn compile_with(source: &str, dependencies: &[&Dependency]) -> Result<CheckedPackage, Failure> {
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
    let assembled = PackageDeclarations::assemble(raw, "pkg".to_owned(), built, models, imports)
        .map_err(Failure::Assembly)?;
    let graph = assembled
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map_err(Failure::Check)?;
    Ok(CheckedPackage::link_with(
        graph,
        dependencies
            .iter()
            .map(|dependency| crate::checked::Import {
                identity: dependency.identity.clone(),
                package: Arc::clone(&dependency.package),
            })
            .collect(),
        crate::EmitLimits::fixture(),
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

    fn compile(&self, declaration: &str) -> Result<CheckedPackage, Failure> {
        compile_with(&unit(declaration), &[&self.producer, &self.consumer])
    }

    /// `compose Sys rule rule { COMPONENTS members }`.
    fn declare(&self, rule: &str, members: &str) -> String {
        format!("compose Sys rule {rule} {{ {COMPONENTS}{members}}}")
    }

    fn checked(&self, rule: &str, members: &str) -> CheckedPackage {
        let declaration = self.declare(rule, members);
        self.compile(&declaration)
            .unwrap_or_else(|failure| panic!("{declaration}: the unit compiles: {failure:?}"))
    }

    /// The one composition refusal of `declaration`.
    fn refused(&self, declaration: &str) -> CompositionRefusal {
        let Err(Failure::Check(refusals)) = self.compile(declaration) else {
            panic!("{declaration}: a check refusal");
        };
        let [CheckRefusal {
            cause: CheckCause::Abstraction(cause),
            ..
        }] = &refusals[..]
        else {
            panic!("{declaration}: one composition refusal: {refusals:?}");
        };
        let AbstractionRefusal::Composition(refusal) = &**cause else {
            panic!("{declaration}: a composition refusal: {cause:?}");
        };
        (**refusal).clone()
    }
}

fn sole(package: &CheckedPackage) -> &CheckedComposition {
    let [composition] = package.graph().compositions() else {
        panic!("one composition");
    };
    composition
}

fn roles(slots: &[CompositionSlot]) -> Vec<SlotRole> {
    slots.iter().map(|slot| slot.role).collect()
}

fn text(unit: &str, span: fsl_foundation::Span) -> &str {
    &unit[span.start..span.end]
}

/// FR-491-AC-1 (TC-985 case 1): three guarantees, one `rely`, one `depends`
/// and one `supplies` member derive the planner-local item and, in order,
/// three component-guarantee slots each keyed with its FR-176 assumption set,
/// one rely slot, one liveness-dependency slot and one fairness slot, the
/// four lists kept separate and each slot's span its member.
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-1")]
#[test]
fn a_producer_consumer_declaration_derives_its_slots_in_order() {
    let world = World::new();
    let source = unit(&world.declare("assume-guarantee", CASE_ONE_MEMBERS));
    let package = world
        .compile(&world.declare("assume-guarantee", CASE_ONE_MEMBERS))
        .expect("the unit compiles");
    let composition = sole(&package);
    assert!(composition.cycles().is_empty());
    assert_eq!(composition.rule(), CompositionRule::AssumeGuarantee);
    let item = composition.item();
    assert_eq!(item.placement, ItemPlacement::PlannerLocal);
    assert_eq!(item.node, *composition.node());
    assert_eq!(item.subjects.len(), 2);
    let slots = composition.slots();
    assert_eq!(item.premises, slots);
    assert_eq!(
        roles(&slots),
        [
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::Rely,
            SlotRole::LivenessDependency,
            SlotRole::Fairness,
        ]
    );
    // The four lists stay separate.
    assert_eq!(composition.relies().len(), 1);
    assert_eq!(composition.liveness_dependencies().len(), 1);
    assert!(composition.environment().is_empty());
    assert_eq!(composition.guarantees().len(), 3);
    // Each component-guarantee slot is keyed with its assumption set.
    let assumptions = |slot: &CompositionSlot| match &slot.key {
        Some(CompositionSlotKey::ComponentGuarantee(key)) => key.assumptions.len(),
        other => panic!("a component-guarantee key: {other:?}"),
    };
    assert_eq!(
        [
            assumptions(&slots[0]),
            assumptions(&slots[1]),
            assumptions(&slots[2])
        ],
        [0, 2, 0],
        "Sends and Counted assume nothing, Done assumes both peers"
    );
    for slot in &slots[..3] {
        assert_eq!(slot.backing, SlotBacking::Item);
    }
    assert_eq!(slots[3].backing, SlotBacking::PlannerLocal);
    assert_eq!(slots[5].backing, SlotBacking::PlannerLocal);
    let fairness = slots[5].constraint.as_ref().expect("the fairness premise");
    assert_eq!(
        (
            fairness.kind(),
            fairness.granularity(),
            fairness.operation().name()
        ),
        (FairnessKind::Weak, FairnessGranularity::Each, "send")
    );
    for (index, slot) in slots.iter().enumerate() {
        assert_eq!(slot.index, index);
        for other in &slots[..index] {
            assert_ne!(slot, other, "slot {index} is distinct");
        }
    }
    // Each slot's span is the member that produced it.
    let sources: Vec<&str> = slots
        .iter()
        .map(|slot| text(&source, slot.source))
        .collect();
    assert_eq!(sources[0], "guarantee Producer::Sends;");
    assert_eq!(sources[1], "guarantee Consumer::Done;");
    assert_eq!(sources[2], "guarantee Producer::Counted;");
    assert_eq!(sources[3], "rely Consumer::Done on Producer::Counted;");
    assert_eq!(sources[4], "depends Consumer::Done on Producer::Sends;");
    assert_eq!(sources[5], "guarantee Producer::Sends;");
    assert_eq!(
        composition
            .supplies()
            .iter()
            .map(|s| s.operation.as_str())
            .collect::<Vec<_>>(),
        ["send"]
    );
}

/// FR-491-AC-2 (TC-985 case 2): one `component`, an unknown rule spelling and
/// a member of another form refuse `invalid_syntax`; an unknown import alias,
/// subject or clause refuses `missing_declaration`/`missing-name`; two
/// components of one name refuse `invalid_model_binding`/`conflicting-binding`.
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-2")]
#[test]
fn a_declaration_that_does_not_fit_its_packages_refuses() {
    let world = World::new();
    for (declaration, offending) in [
        (
            "compose Sys rule interface { component Producer from P using M; guarantee Producer::Sends; }",
            "guarantee",
        ),
        (
            "compose Sys rule cooperative { component Producer from P using M; component Consumer from Q using M; }",
            "cooperative",
        ),
    ] {
        let source = unit(declaration);
        let Err(Failure::Syntax(diagnostics)) = world.compile(declaration) else {
            panic!("{declaration}: a syntax refusal");
        };
        assert_eq!(diagnostics[0].code, fsl_cst::CompleteCode::InvalidSyntax);
        let span = diagnostics[0].byte_span().expect("located");
        assert_eq!(text(&source, span), offending, "{declaration}");
    }
    let refuse = |declaration: String| world.refused(&declaration);
    let missing = |refusal: &CompositionRefusal| match refusal {
        CompositionRefusal::MissingName { what, name, .. } => (*what, name.clone()),
        other => panic!("a missing name: {other:?}"),
    };
    let unknown_import = refuse(
        "compose Sys rule interface { component Producer from Z using M; \
         component Consumer from Q using M; }"
            .to_owned(),
    );
    assert_eq!(
        missing(&unknown_import),
        (CompositionName::Import, "Z".to_owned())
    );
    let unknown_subject = refuse(
        "compose Sys rule interface { component Producer from P using X; \
         component Consumer from Q using M; }"
            .to_owned(),
    );
    assert_eq!(
        missing(&unknown_subject),
        (CompositionName::Model, "X".to_owned())
    );
    let unknown_clause = refuse(world.declare("interface", "guarantee Producer::Missing; "));
    assert_eq!(
        missing(&unknown_clause),
        (CompositionName::Clause, "Missing".to_owned())
    );
    let unknown_component = refuse(world.declare("interface", "guarantee Nobody::Sends; "));
    assert_eq!(
        missing(&unknown_component),
        (CompositionName::Component, "Nobody".to_owned())
    );
    // A clause of the other component's package is no clause of this one.
    let other_package = refuse(world.declare("interface", "guarantee Producer::Done; "));
    assert_eq!(
        missing(&other_package),
        (CompositionName::Clause, "Done".to_owned())
    );
    for refusal in [&unknown_import, &unknown_subject, &unknown_clause] {
        assert_eq!(refusal.code(), fsl_foundation::Code::MissingDeclaration);
        assert_eq!(refusal.cause(), "missing-name");
    }
    let same_name = refuse(
        "compose Sys rule interface { component Producer from P using M; \
         component Producer from Q using M; }"
            .to_owned(),
    );
    assert!(matches!(
        &same_name,
        CompositionRefusal::ConflictingComponent { name, .. } if name == "Producer"
    ));
    assert_eq!(same_name.code(), fsl_foundation::Code::InvalidModelBinding);
    assert_eq!(same_name.cause(), "conflicting-binding");
    let same_subject = refuse(
        "compose Sys rule interface { component Producer from P using M; \
         component Other from P using M; }"
            .to_owned(),
    );
    assert!(matches!(
        &same_subject,
        CompositionRefusal::ConflictingSubject { first_component, second_component, .. }
            if first_component == "Producer" && second_component == "Other"
    ));
    assert_eq!(same_subject.cause(), "conflicting-binding");
}

/// FR-491-AC-3 (TC-985 case 3): a `rely` on a liveness guarantee, a `depends`
/// on a safety guarantee, a `rely` whose guarantee is undeclared and a `rely`
/// of a component on itself each refuse `invalid_model_binding`/
/// `malformed-declaration` naming the member.
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-3")]
#[test]
fn a_premise_that_does_not_fit_its_kind_refuses() {
    let world = World::new();
    let malformed = |members: &str| {
        let declaration = world.declare("interface", members);
        let refusal = world.refused(&declaration);
        assert_eq!(refusal.code(), fsl_foundation::Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "malformed-declaration");
        match refusal {
            CompositionRefusal::Malformed {
                reason,
                member,
                span,
            } => {
                let written = text(&unit(&declaration), span).to_owned();
                (reason, member, written)
            }
            other => panic!("a malformed member: {other:?}"),
        }
    };
    let declared = "guarantee Producer::Sends; guarantee Consumer::Done; \
                    guarantee Producer::Counted; ";
    let (reason, member, written) = malformed(&format!(
        "{declared}rely Consumer::Done on Producer::Sends; "
    ));
    assert_eq!(reason, MalformedMember::RelyOnLiveness);
    assert_eq!(member, "rely Consumer::Done on Producer::Sends");
    assert_eq!(written, "rely Consumer::Done on Producer::Sends;");
    let (reason, member, _) = malformed(&format!(
        "{declared}depends Consumer::Done on Producer::Counted; "
    ));
    assert_eq!(reason, MalformedMember::DependsNotLiveness);
    assert_eq!(member, "depends Consumer::Done on Producer::Counted");
    let (reason, member, _) = malformed(
        "guarantee Producer::Sends; guarantee Producer::Counted; \
         rely Consumer::Done on Producer::Counted; ",
    );
    assert_eq!(reason, MalformedMember::UndeclaredGuarantee);
    assert_eq!(member, "rely Consumer::Done on Producer::Counted");
    let (reason, member, _) = malformed(&format!(
        "{declared}rely Producer::Sends on Producer::Counted; "
    ));
    assert_eq!(reason, MalformedMember::SelfComponent);
    assert_eq!(member, "rely Producer::Sends on Producer::Counted");
}

/// FR-491-AC-4 (TC-985 case 4): a mutual `depends` pair is a cycle of the
/// dependency graph: the declaration checks, lists the cycle's guarantees in
/// edge order and derives no `fixed-point` slot (FSpec FR-489 "Cycles"; the
/// composition item settles `unsupported`).
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-4")]
#[test]
fn a_mutual_dependency_is_a_cycle_that_derives_no_slot() {
    let world = World::new();
    let members = format!("{CASE_ONE_MEMBERS}depends Producer::Sends on Consumer::Done; ");
    let cyclic = world.checked("assume-guarantee", &members);
    let composition = sole(&cyclic);
    let [cycle] = composition.cycles() else {
        panic!("one cycle: {:?}", composition.cycles());
    };
    let named: Vec<String> = cycle.guarantees.iter().map(ToString::to_string).collect();
    assert_eq!(named, ["Consumer::Done", "Producer::Sends"]);
    assert_eq!(
        roles(&composition.slots()),
        [
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::Rely,
            SlotRole::LivenessDependency,
            SlotRole::LivenessDependency,
            SlotRole::Fairness,
        ]
    );
    // The declaration is not refused: the package's other facts are those of
    // the same unit without the cycle.
    let acyclic = world.checked("assume-guarantee", CASE_ONE_MEMBERS);
    assert_eq!(
        cyclic.graph().requirements(),
        acyclic.graph().requirements()
    );
}

/// FR-491-AC-5 (TC-985 case 5): under `interface` the declaration derives
/// one FR-331 `composition` request item whose `premises` member holds the
/// slots, and no planner-local item.
#[trace("FR-530-AC-1", "FR-530-AC-10", "FSpec-TC-985", "FSpec-FR-491-AC-5")]
#[test]
fn a_non_assume_guarantee_rule_derives_a_request_item() {
    let world = World::new();
    let local = world.checked("assume-guarantee", CASE_ONE_MEMBERS);
    for rule in ["interface", "protocol-refinement", "refinement-transfer"] {
        let package = world.checked(rule, CASE_ONE_MEMBERS);
        let item = sole(&package).item();
        assert_eq!(item.placement, ItemPlacement::Request, "{rule}");
        assert_eq!(
            roles(&item.premises),
            roles(&sole(&local).slots()),
            "{rule}"
        );
        // FSpec FR-491 "Assumption binding": only `assume-guarantee` checks a
        // guarantee under peers, so every other rule derives empty sets.
        assert_eq!(item.assumptions, vec![Vec::<usize>::new(); 3], "{rule}");
    }
    assert_eq!(sole(&local).item().placement, ItemPlacement::PlannerLocal);
    // `Done` (premise 1) assumes both peer guarantees, by premise index.
    assert_eq!(
        sole(&local).item().assumptions,
        vec![vec![], vec![0, 2], vec![]]
    );
}

/// FSpec FR-491 "Assumption binding": a peer `H` no declared `guarantee`
/// names has no index, so it is no assumption of the guarantee that relies on
/// it: the slot's `assumptions` entry is empty and its key carries no node.
/// (One clause node names one peer: a clause is checked over its component's
/// own alias, and two components over one subject are refused, so no node is
/// shared between components.)
#[trace("FR-530-AC-10", "FSpec-FR-491-AC-1")]
#[test]
fn an_undeclared_peer_is_no_assumption() {
    let world = World::new();
    let package = world.checked(
        "assume-guarantee",
        "guarantee Consumer::Done; rely Consumer::Done on Producer::Counted; ",
    );
    let item = sole(&package).item();
    assert_eq!(item.assumptions, vec![Vec::<usize>::new()]);
    let Some(CompositionSlotKey::ComponentGuarantee(key)) = &item.premises[0].key else {
        panic!("a component-guarantee key");
    };
    assert!(
        key.assumptions.is_empty() && key.peers.is_empty(),
        "{key:?}"
    );
}

/// FR-491-AC-6 (TC-985 case 6): the v2 byte emitter writes a composition
/// over components of two dependencies. A foreign clause is named by its
/// operand's dependency position in the composition's operand row, and IR's
/// reader admits the package against the component packages.
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-6")]
#[test]
fn a_composition_over_dependency_components_is_emitted_and_admitted() {
    let world = World::new();
    let members = format!(
        "{CASE_ONE_MEMBERS}environment Sched by \"scheduler\" fair weak each M::Producer::send; "
    );
    let package = world.checked("assume-guarantee", &members);
    let emission = emit_checked(&package).expect("the composition emits");
    assert!(emission.omitted().is_empty());
    let written = raw_wire(&emission);
    let nodes = written["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes");
    let position = nodes
        .iter()
        .position(|node| node["semantic_form"] == "component_composition")
        .expect("the composition node is written");
    // FR-530-AC-1: the body keeps its lists separate, none folded into
    // another.
    let body = &nodes[position]["body"];
    assert_eq!(body["term"], "component_composition");
    assert_eq!(body["rule"], "assume-guarantee");
    let list = |name: &str| body[name].as_array().unwrap_or_else(|| panic!("{name}"));
    let names: Vec<&str> = list("components")
        .iter()
        .map(|entry| entry["name"].as_str().expect("component name"))
        .collect();
    assert_eq!(names, ["Consumer", "Producer"]);
    assert!(list("components")
        .iter()
        .all(|entry| entry["subject"].is_object()));
    assert_eq!(list("guarantees").len(), 3);
    assert_eq!(list("relies").len(), 1);
    assert_eq!(list("liveness_dependencies").len(), 1);
    assert_eq!(list("supplies").len(), 1);
    let environment: Vec<&Value> = list("environment").iter().map(|e| &e["name"]).collect();
    assert_eq!(environment, ["Sched"]);
    let row = written["semantic_graph"]["operand_rows"][position]
        .as_array()
        .expect("the composition's operand row");
    let dependency_positions: BTreeSet<&str> = row
        .iter()
        .filter_map(|entry| entry["package"].as_str())
        .collect();
    assert_eq!(
        dependency_positions,
        BTreeSet::from(["test/consumer", "test/producer"]),
        "each component clause is named by a position in its own package"
    );
    // The importing graph holds no node of either dependency.
    assert!(nodes.iter().all(|node| {
        let owner = node["node_id"]["package"].as_str();
        owner != Some("test/producer") && owner != Some("test/consumer")
    }));
    let bytes = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let models =
        fsl_semantics::model::intake::package_input([bytes.as_slice()]).expect("the model package");
    crate::checked_v2::read_import_view(
        &package,
        &emission,
        library(),
        &models,
        &mut crate::checked_v2::AdmittedPackages::default(),
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
    )
    .expect("IR admits the composition against its component packages");
}

/// Composition metadata and member coordinates survive strict admission of
/// actual source, including canonical sorting of every member array.
#[trace(
    "FR-530-AC-1",
    "FSpec-FR-322-AC-514",
    "FSpec-FR-491-AC-7",
    "FSpec-FR-491-AC-8"
)]
#[test]
fn composition_declaration_and_canonical_members_keep_their_authored_regions() {
    let world = World::new();
    // FR-491 orders clauses by NodeRef, whose admitted identity is an
    // opaque NodeKey, not the temporal declaration's source name. Obtain
    // each real target from its checked dependency, and independently bind
    // it to the position that dependency's emitted graph actually holds.
    let mut expected_guarantees: Vec<_> = [
        ("Producer", &world.producer, "Sends"),
        ("Producer", &world.producer, "Counted"),
        ("Consumer", &world.consumer, "Done"),
        ("Consumer", &world.consumer, "Bounded"),
    ]
    .into_iter()
    .map(|(component, dependency, name)| {
        let target = dependency
            .package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|clause| clause.clause().name() == name)
            .expect("the named source clause in its own checked dependency")
            .identity();
        let emission = emit_checked(&dependency.package).expect("the genuine dependency emits");
        let graph = WireGraph::new(emission.wire_nodes.iter().map(|node| &node.node_id))
            .expect("the dependency's document graph");
        let position = graph
            .keys()
            .iter()
            .position(|key| *key == target)
            .expect("the emitted graph holds the exact source target");
        (
            component,
            dependency.identity.as_str(),
            name,
            target,
            u64::try_from(position).expect("dependency position"),
        )
    })
    .collect();
    expected_guarantees.sort_by(|left, right| (left.0, &left.3).cmp(&(right.0, &right.3)));
    let canonical_guarantees: Vec<_> = expected_guarantees
        .iter()
        .map(|(component, _, name, _, _)| format!("guarantee {component}::{name};"))
        .collect();
    let authored_guarantees = canonical_guarantees
        .iter()
        .rev()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let declaration = format!(
        "compose Sys rule interface {{ \
        component Producer from P using M; \
        component Consumer from Q using M; \
        {authored_guarantees} \
        rely Producer::Sends on Consumer::Bounded; \
        rely Consumer::Done on Producer::Counted; \
        depends Producer::Sends on Consumer::Done; \
        depends Consumer::Done on Producer::Sends; \
        supplies fair weak each M::Producer::send; \
        supplies fair strong each M::Producer::send; \
        environment Zulu by \"scheduler\" fair weak each M::Producer::send; \
        environment Alpha by \"scheduler\" fair weak each M::Producer::send; }}"
    );
    let source = unit(&declaration);
    let package = world
        .compile(&declaration, None)
        .expect("the source checks");
    let emission = emit_checked(&package).expect("the composition emits");
    assert!(emission.omitted().is_empty());
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the actual model package");
    let read = crate::checked_v2::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &models,
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::checked_v2::SourceWitnessSupplier::new(&emission),
    )
    .expect("the pinned strict reader admits the source emission and its dependencies");
    let admitted = read.admitted();
    let named = admitted
        .claim_name_for_path(&["Sys"])
        .expect("Sys is named");
    assert_eq!(named.name.as_ref(), "Sys");
    assert_eq!(named.qualified_name, [Box::<str>::from("Sys")]);
    assert_eq!(named.occurrence.role, CheckedOccurrenceRole::Declaration);
    assert_eq!(named.occurrence.ordinal, 0);
    assert_eq!(
        admitted
            .claim_names()
            .expect("claim names")
            .iter()
            .filter(|row| row.declaration == named.declaration)
            .count(),
        1,
        "the declaration has exactly one naming row"
    );
    let key = admitted
        .structural_key(&named.declaration)
        .expect("declared key");
    let key_json = serde_json::to_value(key).expect("structural identity");
    assert_eq!(key_json["path"], serde_json::json!(["Sys"]));
    let written = raw_wire(&emission);
    let nodes = written["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes");
    let position = nodes
        .iter()
        .position(|node| node["semantic_form"] == "component_composition")
        .expect("the composition is emitted");
    let node = &nodes[position];
    assert_eq!(
        node["declaration"]["qualified_name"],
        serde_json::json!(["Sys"])
    );
    assert_eq!(
        node["owner"],
        serde_json::json!({
            "kind": "source", "authority": "agent-ix", "identity": "test:composition"
        })
    );
    let entries: Vec<_> = admitted
        .source_map()
        .iter()
        .filter(|entry| &entry.node_id == key)
        .collect();
    let declaration_entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.role == CheckedOccurrenceRole::Declaration)
        .collect();
    let [declaration_entry] = declaration_entries.as_slice() else {
        panic!("exactly one declaration source entry: {declaration_entries:?}");
    };
    assert_eq!(declaration_entry.ordinal, 0);
    assert_eq!(
        declaration_entry.regions,
        [named.declaration_region.clone()]
    );
    let whole = &named.declaration_region;
    assert_eq!(whole.source.authority.as_ref(), "agent-ix");
    assert_eq!(whole.source.identity.as_ref(), "test:composition");
    let start = source
        .find(declaration.as_str())
        .expect("whole declaration in source");
    assert_eq!(whole.start, u64::try_from(start).expect("source offset"));
    assert_eq!(
        whole.end,
        u64::try_from(start + declaration.len()).expect("source end")
    );

    // This independent table fixes both canonical group offsets and the
    // authored text each generated coordinate must retain after sorting.
    let guarantee_members: Vec<_> = canonical_guarantees.iter().map(String::as_str).collect();
    let groups: [(&str, &[&str]); 6] = [
        (
            "components",
            &[
                "component Consumer from Q using M;",
                "component Producer from P using M;",
            ],
        ),
        ("guarantees", &guarantee_members),
        (
            "relies",
            &[
                "rely Consumer::Done on Producer::Counted;",
                "rely Producer::Sends on Consumer::Bounded;",
            ],
        ),
        (
            "liveness_dependencies",
            &[
                "depends Consumer::Done on Producer::Sends;",
                "depends Producer::Sends on Consumer::Done;",
            ],
        ),
        (
            "supplies",
            &[
                "supplies fair strong each M::Producer::send;",
                "supplies fair weak each M::Producer::send;",
            ],
        ),
        (
            "environment",
            &[
                "environment Alpha by \"scheduler\" fair weak each M::Producer::send;",
                "environment Zulu by \"scheduler\" fair weak each M::Producer::send;",
            ],
        ),
    ];
    let mut ordinal = 0_u64;
    for (group, members) in groups {
        assert_eq!(
            node["body"][group].as_array().expect("body array").len(),
            members.len(),
            "{group}"
        );
        let starts: Vec<_> = members
            .iter()
            .map(|member| source.find(member).expect("authored member"))
            .collect();
        assert!(
            starts.windows(2).all(|pair| pair[0] > pair[1]),
            "{group} reverses authored order"
        );
        for (member, start) in members.iter().zip(starts) {
            let matching: Vec<_> = entries
                .iter()
                .filter(|entry| {
                    entry.role == CheckedOccurrenceRole::Generated && entry.ordinal == ordinal
                })
                .collect();
            let [entry] = matching.as_slice() else {
                panic!("one Generated/{ordinal}: {matching:?}");
            };
            let [region] = entry.regions.as_slice() else {
                panic!("one authored region: {entry:?}");
            };
            assert_eq!(region.source, whole.source, "{group}/{ordinal}");
            assert_eq!(
                region.start,
                u64::try_from(start).expect("member start"),
                "{member}"
            );
            assert_eq!(
                region.end,
                u64::try_from(start + member.len()).expect("member end"),
                "{member}"
            );
            assert!(whole.start < region.start && region.end < whole.end);
            ordinal += 1;
        }
    }
    assert_eq!(
        entries.len(),
        usize::try_from(ordinal).expect("member count") + 1
    );
    assert_eq!(
        node["occurrences"].as_array().expect("occurrences").len(),
        entries.len()
    );
    let Some(CheckedTerm::ComponentComposition {
        components,
        guarantees,
        relies,
        liveness_dependencies,
        supplies,
        environment,
        ..
    }) = admitted.term(key)
    else {
        panic!("strict readback retains the typed composition");
    };
    assert_eq!(
        (
            components.len(),
            guarantees.len(),
            relies.len(),
            liveness_dependencies.len(),
            supplies.len(),
            environment.len()
        ),
        (2, 4, 2, 2, 2, 2)
    );
    for (at, (guarantee, (component, identity, name, _, expected_position))) in
        guarantees.iter().zip(&expected_guarantees).enumerate()
    {
        assert_eq!(guarantee.component.as_ref(), *component);
        let owner = admitted
            .direct_dependencies()
            .find(|owner| owner.name() == *identity)
            .expect("the authenticated component owner");
        // Compare the complete raw reference with the independently found
        // dependency document position. Every foreign operand slot must
        // name the same owner's exact document index before expansion.
        let mut raw_target: CheckedNodeId =
            serde_json::from_value(node["body"]["guarantees"][at]["clause"].clone())
                .expect("the emitted raw clause reference");
        assert_eq!(raw_target.package.as_ref(), *identity);
        let mut slots = 0;
        for segment in &mut raw_target.path {
            if let filament_contract_model::CheckedNodePathSegment::Operand(slot) = segment {
                let operand: CheckedOperandEntry = serde_json::from_value(
                    written["semantic_graph"]["operand_rows"][position]
                        [usize::try_from(*slot).expect("operand slot")]
                    .clone(),
                )
                .expect("the raw holder's operand entry");
                let CheckedOperandEntry::Dependency(operand) = operand else {
                    panic!("the foreign clause operand must name its dependency");
                };
                assert_eq!(operand.package.as_ref(), *identity);
                assert!(owner
                    .structural_key_at(
                        usize::try_from(operand.node).expect("foreign operand position")
                    )
                    .is_some());
                *segment = filament_contract_model::CheckedNodePathSegment::Index(operand.node);
                slots += 1;
            }
        }
        assert!(slots > 0, "the raw clause retains foreign operand slots");
        let expected_position = usize::try_from(*expected_position).expect("dependency position");
        assert_eq!(
            Some(&raw_target),
            owner.document_held_id(expected_position),
            "{component}::{name} keeps its exact dependency document target"
        );
        let clause_key = owner
            .structural_key_at(expected_position)
            .expect("the foreign clause exists in its own graph");
        assert!(owner.term(clause_key).is_some());
        let reference = admitted
            .resolve_node_held(position, &guarantee.clause)
            .expect("resolve the composition's foreign operand slots");
        assert_eq!(reference.package.as_ref(), *identity);
        assert_eq!(
            owner.structural_key(&reference),
            Some(clause_key),
            "the expanded reference resolves in its authenticated owner"
        );
        let names: Vec<_> = owner
            .temporal_clause_names()
            .expect("authenticated temporal names")
            .iter()
            .filter(|row| row.name.as_ref() == *name)
            .collect();
        let [named_clause] = names.as_slice() else {
            panic!("one actual declaration of {identity}::{name}");
        };
        assert_eq!(
            owner.structural_key(&named_clause.declaration),
            Some(clause_key),
            "the foreign position resolves to this owner's named clause"
        );
    }
    let owners: BTreeSet<_> = admitted
        .direct_dependencies()
        .map(|owner| owner.name())
        .collect();
    assert_eq!(owners, BTreeSet::from(["test/consumer", "test/producer"]));
    let operand_owners: BTreeSet<_> = written["semantic_graph"]["operand_rows"][position]
        .as_array()
        .expect("operand row")
        .iter()
        .filter_map(|operand| operand["package"].as_str())
        .collect();
    assert_eq!(
        operand_owners, owners,
        "foreign operands retain both authenticated owners"
    );

    let mut evidence = emission.evidence.clone();
    for (identity, document) in &models {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .expect("the same authentic model evidence");
    }
    for dependency in admitted.direct_dependencies() {
        evidence
            .insert_dependency_package(dependency.name(), dependency.clone())
            .expect("the actual admitted dependency");
    }
    let mut reordered = written.clone();
    reordered["source_map"]
        .as_array_mut()
        .expect("source map")
        .reverse();
    let bytes = serde_json::to_vec(&reordered).expect("reordered artifact");
    let Read::Verified {
        admitted: reordered,
        ..
    } = crate::checked_v2::read_v2(
        &bytes,
        library(),
        crate::checked_v2::V2ReadLimits::fixture(),
        &evidence,
    )
    else {
        panic!("source-map entry order does not change composition admission");
    };
    assert_eq!(reordered.claim_name_for_path(&["Sys"]), Some(named));
    assert_eq!(reordered.term(key), admitted.term(key));
    for entry in entries {
        let matching: Vec<_> = reordered
            .source_map()
            .iter()
            .filter(|other| {
                other.node_id == entry.node_id
                    && other.role == entry.role
                    && other.ordinal == entry.ordinal
            })
            .collect();
        assert_eq!(
            matching,
            [entry],
            "member attribution survives source-map reordering"
        );
    }
}

/// Empty body arrays consume no member ordinals. A nonempty region outside
/// the declaration is a composition defect; an empty region is refused by
/// the preceding generic source-map validation.
#[trace("FR-530-AC-1", "FSpec-FR-491-AC-7", "FSpec-FR-491-AC-8")]
#[test]
fn composition_empty_arrays_and_region_refusal_precedence_are_preserved() {
    let world = World::new();
    let declaration = world.declare(
        "interface",
        "environment Sched by \"scheduler\" fair weak each M::Producer::send; ",
    );
    let package = world
        .compile(&declaration, None)
        .expect("the source checks");
    let emission = emit_checked(&package).expect("the source emits");
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the actual model package");
    let read = crate::checked_v2::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &models,
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::checked_v2::SourceWitnessSupplier::new(&emission),
    )
    .expect("the authentic sparse composition admits before mutation");
    let admitted = read.admitted();
    let named = admitted
        .claim_name_for_path(&["Sys"])
        .expect("Sys is named");
    let source = unit(&declaration);
    let generated: Vec<_> = admitted
        .source_map()
        .iter()
        .filter(|entry| {
            entry.node_id == named.declaration && entry.role == CheckedOccurrenceRole::Generated
        })
        .collect();
    assert_eq!(
        generated.len(),
        3,
        "only two components and one environment member"
    );
    for (ordinal, member) in (0_u64..).zip([
        "component Consumer from Q using M;",
        "component Producer from P using M;",
        "environment Sched by \"scheduler\" fair weak each M::Producer::send;",
    ]) {
        let entry = generated
            .iter()
            .find(|entry| entry.ordinal == ordinal)
            .expect("no gaps for empty intermediate arrays");
        let [region] = entry.regions.as_slice() else {
            panic!("one member region");
        };
        let start = source.find(member).expect("authored member");
        assert_eq!(region.start, u64::try_from(start).expect("member start"));
        assert_eq!(
            region.end,
            u64::try_from(start + member.len()).expect("member end")
        );
    }
    let mut evidence = emission.evidence.clone();
    for (identity, document) in &models {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .expect("the same authentic model evidence");
    }
    for dependency in admitted.direct_dependencies() {
        evidence
            .insert_dependency_package(dependency.name(), dependency.clone())
            .expect("the actual admitted dependency");
    }
    let original = raw_wire(&emission);
    let position = original["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .position(|entry| {
            entry["role"] == "generated"
                && entry["ordinal"] == 2
                && entry["node_id"]
                    == serde_json::to_value(&named.declaration).expect("declaration key")
        })
        .expect("the environment member coordinate");
    for (empty, expected_code, expected_path) in [
        (
            false,
            filament_contract_model::CheckedPackageRefusalCode::InvalidPackage,
            format!("/source_map/{position}"),
        ),
        (
            true,
            filament_contract_model::CheckedPackageRefusalCode::InvalidSourceMap,
            format!("/source_map/{position}/regions/0"),
        ),
    ] {
        let mut changed = original.clone();
        // The whole declaration is a valid generic region, but cannot stand
        // in for an authored member strictly inside it.
        changed["source_map"][position]["regions"][0] =
            serde_json::to_value(&named.declaration_region).expect("whole declaration region");
        if empty {
            changed["source_map"][position]["regions"][0]["end"] =
                changed["source_map"][position]["regions"][0]["start"].clone();
        }
        let bytes = serde_json::to_vec(&changed).expect("mutated artifact");
        let result = crate::checked_v2::read_v2(
            &bytes,
            library(),
            crate::checked_v2::V2ReadLimits::fixture(),
            &evidence,
        );
        let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) = result
        else {
            panic!("the malformed coordinate refuses: {result:?}");
        };
        assert_eq!(refusal.code, expected_code, "empty={empty}");
        if !empty {
            assert_eq!(
                refusal.cause,
                Some(filament_contract_model::CheckedPackageRefusalCause::InvalidValue)
            );
        }
        assert!(
            refusal.loci.iter().any(|locus| locus
                .path
                .as_ref()
                .is_some_and(|path| path.as_str() == expected_path)),
            "empty={empty}: {refusal:?}"
        );
    }
}

/// FR-491-AC-6: the composition's bytes read without the component packages
/// supplied are refused, not admitted on the strength of a position alone.
#[trace("FR-530-AC-1", "FSpec-TC-985", "FSpec-FR-491-AC-6")]
#[test]
fn a_composition_without_its_component_packages_is_not_admitted() {
    let world = World::new();
    let declaration = world.declare("assume-guarantee", CASE_ONE_MEMBERS);
    let source = unit(&declaration);
    let package = compile_with(&source, &[&world.producer, &world.consumer])
        .expect("the local marker keeps the composition package nonempty");
    let emission = emit_checked(&package).expect("the composition emits");
    match read_back(&emission) {
        Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(
                refusal.code,
                filament_contract_model::CheckedPackageRefusalCode::MalformedWire
            );
            assert!(refusal.loci[0]
                .path
                .as_ref()
                .is_some_and(|path| path.as_str().starts_with("/semantic_graph/operand_rows/")));
        }
        other => panic!("the component packages are unsupplied: {other:?}"),
    }
}

/// FR-491 / FR-530-AC-1: a key the importer holds stays the importer's own
/// node when a dependency holds the same key; only a key the importer does
/// not hold is named by a dependency position.
#[trace("FR-530-AC-1", "FSpec-FR-491-AC-6")]
#[test]
fn an_importers_own_key_is_not_taken_for_a_dependency_node() {
    let (_, package) =
        package_from_text("function add using v(x: Integer): Integer pure { (x + 1) + 2 }\n");
    let graph = package.graph();
    let ids = StructuralIds::default();
    let names = |foreign: &ForeignNodes| -> Vec<Vec<NodeKey>> {
        candidates_of(&ids, foreign, graph, None, &[])
            .expect("candidates")
            .values()
            .map(|candidate| {
                let mut names: Vec<NodeKey> = candidate.names.iter().cloned().collect();
                names.sort();
                names
            })
            .collect()
    };
    let own = names(&ForeignNodes::default());
    assert!(
        own.iter().any(|names| !names.is_empty()),
        "the unit has operands"
    );
    // A dependency holds every key the importer holds, at its own positions.
    let mut shadowing = ForeignNodes::default();
    for (at, node) in (0_u64..).zip(graph.semantic_graph().nodes()) {
        shadowing.positions.insert(node.key(), ("dep".into(), at));
    }
    assert_eq!(names(&shadowing), own, "the importer's own nodes win");
}
