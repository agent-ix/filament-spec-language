// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-507 (TC-1302): the family of a request item is read from its
//! `item_kind` and from the checked package's claim node (FSpec FR-290's
//! node-to-family table), and `analyze` routes the item to an engine or
//! refuses it `inapplicable-kind` by that family.
//!
//! Each case settles a probe item (`AnalyzeItem::probe`, an item whose work
//! only records that it ran) on `Engine::probe`, which advertises every
//! kind, so the only thing that can refuse a probe item is its family.

use std::cell::Cell;

use crate::graph_world::{config_version, World, CONFIG_CONTRACT, REACHES_TWO};
use crate::provider_test_support::CountingUnknownProvider;
use filament_exact::{Cancel, NodeKey, Origin, Role};
use filament_semantic_value::catalog::CatalogCode;
use fsl_analyze::analyze::{
    analyze, analyze_items, AnalyzeItem, AnalyzeLimits, AnalyzeRecord, AnalyzeRequest, BackendTool,
    ClaimItem, Diagnostic, DispositionKind, Engine, Engines, ItemForm, ItemOptions, NegotiatedItem,
    Payload, Settlement, SubjectBinding, TemporalItem,
};
use fsl_analyze::claim_family::claim_family;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{DeclineCode, ProofRefusalCause, TerminalValue};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::routing::Disposition;
use fsl_route::{Candidate, ManifestMembers, Mode, ProviderOrigin};
use fsl_semantics::check::{Capability, DeclarationFamily, NegotiationOutcome};
use ix_trace_rs::trace;

const PROBE: &str = "probe";

/// The occurrence of `node` at `role`, ordinal `ordinal`.
fn at(node: &NodeKey, role: &str, ordinal: u64) -> OccurrenceKey {
    OccurrenceKey::new(node.clone(), Origin::new(Role::new(role), ordinal))
}

/// The key of the one node of `package` with `semantic_form` `form`.
fn node(package: &CheckedPackage, form: &str) -> NodeKey {
    let mut found = package
        .graph()
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == form);
    let key = found
        .next()
        .unwrap_or_else(|| panic!("the package holds a {form} node"))
        .key();
    assert!(found.next().is_none(), "one {form} node");
    key
}

/// What `analyze` did with one probe item.
struct Settled {
    record: AnalyzeRecord,
    ran: bool,
}

/// Settle a probe item of `kind` at `claim` with `item_kind`, at request
/// index `index`.
fn settle(
    package: &CheckedPackage,
    index: usize,
    kind: Capability,
    item_kind: ItemKind,
    claim: &OccurrenceKey,
) -> Settled {
    let ran = Cell::new(false);
    let item = AnalyzeItem::probe(RequestIndex::new(index), kind, PROBE, &ran)
        .at_claim(package, item_kind, claim);
    let mut engines = Engines::default();
    engines.with(Engine::probe(PROBE)).expect("one engine");
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    Settled {
        record,
        ran: ran.get(),
    }
}

/// Settle one probe item per `(kind, item_kind, claim)` of `requests` in a
/// single `analyze_items` call, the item at position `n` at request index
/// `n`; the settlements come back in caller order.
fn settle_many(
    package: &CheckedPackage,
    requests: &[(Capability, ItemKind, &OccurrenceKey)],
) -> Vec<Settled> {
    let ran: Vec<Cell<bool>> = requests.iter().map(|_| Cell::new(false)).collect();
    let items = requests
        .iter()
        .zip(&ran)
        .enumerate()
        .map(|(index, ((kind, item_kind, claim), ran))| {
            AnalyzeItem::probe(RequestIndex::new(index), *kind, PROBE, ran)
                .at_claim(package, *item_kind, claim)
        })
        .collect();
    let mut engines = Engines::default();
    engines.with(Engine::probe(PROBE)).expect("one engine");
    let records = analyze_items(&engines, items, &Cancel::new());
    assert_eq!(records.len(), requests.len(), "one record per item");
    records
        .into_iter()
        .zip(&ran)
        .map(|(record, ran)| Settled {
            record,
            ran: ran.get(),
        })
        .collect()
}

/// The item ran on the probe engine: its record is the engine's, naming the
/// engine's `BackendId` (`settled_by` is the engine's result).
fn assert_engine_settled(settled: &Settled, index: usize) {
    assert!(settled.ran, "an applicable item runs its engine");
    assert_eq!(settled.record.request_index, RequestIndex::new(index));
    let Settlement::Terminal(terminal) = &settled.record.settlement else {
        panic!(
            "an engine settles the item: {:?}",
            settled.record.settlement
        );
    };
    assert_eq!(terminal.backend().map(|b| b.as_str()), Some(PROBE));
}

/// The item settled `invalid-request` by its disposition with
/// `invalid_capability`/`inapplicable-kind` and the payload `(index, kind,
/// family)`, and no engine ran.
fn assert_inapplicable(
    settled: &Settled,
    index: usize,
    kind: Capability,
    family: Option<DeclarationFamily>,
) {
    assert!(!settled.ran, "no engine runs an inapplicable item");
    let Settlement::Disposed(disposed) = &settled.record.settlement else {
        panic!("settled by disposition: {:?}", settled.record.settlement);
    };
    assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
    assert!(disposed.backend.is_none());
    let diagnostic = disposed.diagnostic.as_ref().expect("a diagnostic");
    assert_eq!(
        diagnostic.code,
        CatalogCode::new("invalid_capability", "inapplicable-kind")
    );
    assert_eq!(
        diagnostic.payload,
        Payload::Inapplicable {
            request_index: RequestIndex::new(index),
            capability: kind,
            family,
        }
    );
}

/// An item of `kind` at `claim` is applicable to the claim's family and
/// runs; a `schedulability` item at the same claim settles
/// `inapplicable-kind` with `family`, which shows the family read.
fn assert_family_read(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    kind: Capability,
    family: DeclarationFamily,
) {
    assert_eq!(
        claim_family(package, ItemKind::Formula, claim),
        Some(family),
        "{claim}"
    );
    assert_engine_settled(&settle(package, 0, kind, ItemKind::Formula, claim), 0);
    let schedulability = settle(
        package,
        1,
        Capability::Schedulability,
        ItemKind::Formula,
        claim,
    );
    assert_inapplicable(&schedulability, 1, Capability::Schedulability, Some(family));
}

const ABSTRACTION: &str = "abstraction A using v {\n\
     population Config::config_history to \"store::history\";\n\
     operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
     }\n";

const DETERMINISTIC: &str = "relation Det using t over (x: Config::ConfigVersion::attemptUpdate, \
     y: Config::ConfigVersion::attemptUpdate) \
     { pre(x.self.versionNumber) = pre(y.self.versionNumber) \
     implies x.self.versionNumber = y.self.versionNumber }\n";

/// ADR-023 §8's `Det` relation with its operation bound, so FSL writes its
/// model claim and its code claim, beside a state clause, a frame, a
/// state-graph claim, a `hyper` clause, a temporal clause and a hybrid
/// temporal clause over the `ConfigVersion` model.
fn claims_world() -> World {
    let profiles = "profile m = \"filament.model.complete/v1\";\n\
        profile t = \"filament.temporal.event-position.false-extension/v1\";\n";
    let temporal = "temporal TemporalProgress using t over (c: Config::ConfigVersion) \
        clock \"model-steps\" on origin { eventually[0,2] holds(true) }\n";
    let hyper = "hyper HyperProgress using t over behaviours (c: Config::ConfigVersion) \
        exists trace a of Config { always [0, 2] holds(true) }\n";
    let hybrid = "hybrid Braking using t {\n\
        mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
          flow { v' = -a; a' = 0; t' = 1; }\n\
          transition to Stopped when { v = 0 } reset { v = 0 };\n\
        mode Stopped invariant { v = 0 }\n\
          flow { v' = 0; a' = 0; t' = 1; }\n\
        }\n\
        temporal SlowEnough using t over (h: Braking) clock \"model-time\" \
        on origin { always [0, 10] holds(h.v < 30) }\n";
    World::with_profiles(
        config_version(),
        profiles,
        &format!(
            "{CONFIG_CONTRACT}{REACHES_TWO}{temporal}{hyper}{hybrid}{DETERMINISTIC}{ABSTRACTION}"
        ),
        &[&[("a", &[("versionNumber", 0)])]],
    )
}

/// The relation's model claim and its code claim: two occurrences of its one
/// `claim`/`step_relation` node.
///
/// FSpec FR-477 and FR-507-AC-7 put both claims on one `claim` occurrence,
/// told apart by capability kind.
fn relation_claims(package: &CheckedPackage) -> (OccurrenceKey, OccurrenceKey) {
    let graph = package.graph();
    let [relation] = graph.step_relations() else {
        panic!("one step relation");
    };
    let model = relation.occurrence().expect("the model claim").clone();
    let node = relation.identity().expect("assembled");
    assert_eq!(model.node(), &node);
    assert_eq!(
        graph
            .requirements()
            .get_kind(&model, Capability::TemporalSatisfaction)
            .expect("the model claim has a temporal-satisfaction record")
            .requirements()
            .kind(),
        Capability::TemporalSatisfaction
    );
    let code_records: Vec<_> = graph
        .requirements()
        .iter()
        .filter(|(occurrence, record)| {
            occurrence.node() == &node
                && record.requirements().kind() == Capability::OperationContract
        })
        .map(|(occurrence, _)| occurrence.clone())
        .collect();
    assert_eq!(code_records.len(), 1, "one operation-contract record");
    let code = code_records.into_iter().next().expect("the code claim");
    assert_eq!(model, code);
    (model, code)
}

/// The refinement's node, its `declaration` occurrence, and the occurrences
/// of its initial-state and per-step obligations in ordinal order.
fn refinement_claims(
    world: &crate::refinement_fixture::World,
) -> (OccurrenceKey, Vec<OccurrenceKey>) {
    let refinement = world.refinement();
    assert!(refinement.proof_eligible());
    let node = refinement.v2_emission().expect("emits").identity;
    let obligations = refinement
        .step_simulations()
        .iter()
        .map(|entry| entry.occurrence().clone())
        .collect::<Vec<_>>();
    assert!(obligations.iter().all(|claim| claim.node() == &node));
    (at(&node, "declaration", 0), obligations)
}

/// FR-507-AC-7, a `relation` clause with an implementation binding: its
/// model claim and its code claim are read as `relation-claim`, each item
/// is settled by an engine, `global-conformance` on the claim settles
/// `inapplicable-kind` with `relation-claim`, and `schedulability` on either
/// occurrence does too.
#[trace("TC-1302", "TC-115", "FR-507-AC-7", "FR-057-AC-11")]
#[test]
fn a_bound_relations_model_and_code_claims_are_read_as_relation_claims() {
    let world = claims_world();
    let (model, code) = relation_claims(&world.package);
    assert_eq!(model.node(), code.node());
    let relation = DeclarationFamily::RelationClaim;

    // All six items in one request, in caller order.
    let settled = settle_many(
        &world.package,
        &[
            (Capability::TemporalSatisfaction, ItemKind::Formula, &model),
            (Capability::OperationContract, ItemKind::Formula, &code),
            (Capability::GlobalConformance, ItemKind::Formula, &model),
            (Capability::Schedulability, ItemKind::Formula, &model),
            (Capability::GlobalConformance, ItemKind::Formula, &code),
            (Capability::Schedulability, ItemKind::Formula, &code),
        ],
    );
    assert_engine_settled(&settled[0], 0);
    assert_engine_settled(&settled[1], 1);
    for claim in [&model, &code] {
        assert_eq!(
            claim_family(&world.package, ItemKind::Formula, claim),
            Some(relation)
        );
    }
    assert_inapplicable(
        &settled[2],
        2,
        Capability::GlobalConformance,
        Some(relation),
    );
    assert_inapplicable(&settled[3], 3, Capability::Schedulability, Some(relation));
    assert_inapplicable(
        &settled[4],
        4,
        Capability::GlobalConformance,
        Some(relation),
    );
    assert_inapplicable(&settled[5], 5, Capability::Schedulability, Some(relation));
}

/// The code claim of the bound relation runs on the real
/// `operation-contract` arm: its adapter reads `relation-claim` from the
/// claim's node and the arm settles the record, naming its own `BackendId`.
#[test]
fn the_operation_contract_arm_runs_a_relations_code_claim() {
    use fsl_analyze::analyze::contract_item;
    use fsl_analyze::contract_check::{manifest, ContractRequest};
    use fsl_replay::contract_check::select_contract;
    use fsl_replay::{TerminalValue, UnavailabilityCause};

    let world = claims_world();
    let (_, code) = relation_claims(&world.package);
    let request = select_contract(&world.package, &code).map(|selected| ContractRequest {
        package: &world.package,
        claim: selected,
        subject: world.subject(&["a"], crate::counter::UNLIMITED).simulation,
        abstract_subject: None,
        max_cases: None,
    });
    let record = analyze_items(
        &Engines::fsl().expect("the linked engines register"),
        vec![contract_item(
            RequestIndex::new(0),
            &world.package,
            &code,
            request,
        )],
        &Cancel::new(),
    )
    .remove(0);
    let terminal = record.terminal().expect("the arm settles the item");
    assert_eq!(
        terminal.backend().map(|backend| backend.as_str()),
        Some(manifest::BACKEND_ID)
    );
    assert!(
        matches!(
            terminal.value(),
            TerminalValue::Unsupported(UnavailabilityCause::ContractCheck { .. })
        ),
        "{:?}",
        terminal.value()
    );
}

/// FR-507-AC-7, a proof-eligible model refinement with step rows `R0` and
/// `R1`: the whole-refinement item (`declaration` occurrence) and the three
/// `operation-contract` obligations (`generated` ordinals 0 to 2) are four
/// distinct occurrences of one node, each read as `relation-claim` and
/// settled by an engine; `schedulability` on each settles `inapplicable-kind`
/// with `relation-claim`.
#[trace("TC-1302", "FR-507-AC-7")]
#[test]
fn a_proof_eligible_refinements_four_occurrences_are_relation_claims() {
    use crate::refinement_fixture::{lost_update_refinement, Variant, World as Cas};

    let world = Cas::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let (declaration, obligations) = refinement_claims(&world);
    // The initial-state obligation at ordinal 0 and one obligation per step
    // row after it: this refinement's rows are `R0` to `R4`, of which the
    // criterion names the first two.
    assert_eq!(obligations.len(), 6);
    for (ordinal, claim) in obligations.iter().enumerate() {
        assert_eq!(claim.origin().role().as_str(), "generated");
        assert_eq!(claim.origin().ordinal(), ordinal as u64);
        assert_eq!(
            claim_family(world.package(), ItemKind::Formula, claim),
            Some(DeclarationFamily::RelationClaim)
        );
    }
    let obligations: Vec<_> = obligations.into_iter().take(3).collect();
    let package = world.package();
    let relation = DeclarationFamily::RelationClaim;
    let mut items = vec![(declaration, Capability::TemporalSatisfaction)];
    items.extend(
        obligations
            .into_iter()
            .map(|claim| (claim, Capability::OperationContract)),
    );
    assert_eq!(items.len(), 4);
    // The four items, then `schedulability` on each, in one request.
    let requests: Vec<_> = items
        .iter()
        .map(|(claim, kind)| (*kind, ItemKind::Formula, claim))
        .chain(
            items
                .iter()
                .map(|(claim, _)| (Capability::Schedulability, ItemKind::Formula, claim)),
        )
        .collect();
    let settled = settle_many(package, &requests);
    for (position, settled) in settled.iter().enumerate() {
        if position < 4 {
            assert_engine_settled(settled, position);
        } else {
            assert_inapplicable(
                settled,
                position,
                Capability::Schedulability,
                Some(relation),
            );
        }
    }
}

/// FR-507-AC-8: a derived item is read from its `item_kind`, whatever node
/// it is anchored on. The deadlock-freedom and race-freedom items anchored
/// on a `protocol`/`protocol_clause` node run as `temporal-trace`, where a
/// reader that took the family from the anchor node would refuse them with
/// `family` `protocol`; an item with `item_kind` `formula` on the protocol
/// clause node's own claim is read as `protocol`.
#[trace("TC-1302", "FR-507-AC-8")]
#[test]
fn derived_items_take_temporal_trace_from_their_item_kind_on_any_anchor() {
    use crate::protocol_roles::{unit, EACH};
    use crate::protocol_system::touch;

    let temporal = DeclarationFamily::TemporalTrace;
    // The derived items over a model subject and over a timed subject are
    // anchored on a `model_root` node (FSpec FR-368 "Claim and anchor").
    let untimed = claims_world();
    let timed = crate::zone_search::rpc(4);
    for (package, item_kind) in [
        (&untimed.package, ItemKind::DeadlockFreedom),
        (&timed.package, ItemKind::TimeLockFreedom),
    ] {
        let (_, root) = package.graph().model_roots().next().expect("a model_root");
        let anchor = OccurrenceKey::generated(root);
        let settled = settle(
            package,
            0,
            Capability::TemporalSatisfaction,
            item_kind,
            &anchor,
        );
        assert_eq!(claim_family(package, item_kind, &anchor), Some(temporal));
        assert_engine_settled(&settled, 0);
    }

    let body = unit(EACH, "", "", &format!("sequence Main {{ {} }}", touch("T")));
    let package = crate::fill::package_with(crate::litmus::PROTOCOL_PROFILE, &body);
    let clause = at(&node(&package, "protocol_clause"), "declaration", 0);
    for (index, item_kind) in [ItemKind::DeadlockFreedom, ItemKind::RaceFreedom]
        .into_iter()
        .enumerate()
    {
        let derived = settle(
            &package,
            index,
            Capability::TemporalSatisfaction,
            item_kind,
            &clause,
        );
        assert_eq!(claim_family(&package, item_kind, &clause), Some(temporal));
        assert_engine_settled(&derived, index);
    }

    // The anchor node alone would read `protocol`, where `temporal-
    // satisfaction` applies nowhere.
    let anchored = settle(
        &package,
        2,
        Capability::TemporalSatisfaction,
        ItemKind::Formula,
        &clause,
    );
    assert_inapplicable(
        &anchored,
        2,
        Capability::TemporalSatisfaction,
        Some(DeclarationFamily::Protocol),
    );
    // The protocol clause node's own claim is a `protocol` claim and runs.
    let own = settle(
        &package,
        3,
        Capability::GlobalConformance,
        ItemKind::Formula,
        &clause,
    );
    assert_eq!(
        claim_family(&package, ItemKind::Formula, &clause),
        Some(DeclarationFamily::Protocol)
    );
    assert_engine_settled(&own, 3);
}

/// The package of a `Value` function with a Boolean result and a scalar
/// operation application in its body.
fn value_package() -> CheckedPackage {
    crate::value_validity::package_of(
        "function f using v(x: Integer): Integer pure { x }\n\
         function subject using v(x: Integer): Boolean pure { f(x) + 1 > x }\n",
    )
}

/// A Boolean value clause is read as `value` on its Boolean `expression`
/// node, at roles `claim` and `declaration` (FSpec FR-290, FR-484).
#[trace("TC-1302", "FR-507-AC-11")]
#[test]
fn a_boolean_value_clause_is_read_on_its_expression_node() {
    let package = value_package();
    let graph = package.graph();
    let mut claims = graph.occurrences().filter_map(|(node, origin, _)| {
        (origin.role().as_str() == "claim"
            && graph
                .semantic_graph()
                .node(node.clone())
                .is_some_and(|node| node.node_tag() == fsl_semantics::check::NodeTag::Expression))
        .then_some(node)
    });
    let clause = claims.next().expect("a Boolean expression claim");
    assert!(claims.next().is_none(), "one Boolean expression claim");
    let expression = graph
        .semantic_graph()
        .node(clause.clone())
        .expect("the expression node");
    let semantic_type = expression.semantic_type().expect("Boolean type");
    assert_eq!(
        graph
            .semantic_graph()
            .node(semantic_type)
            .expect("the scalar type node")
            .semantic_form(),
        "boolean"
    );
    for role in ["claim", "declaration"] {
        assert_family_read(
            &package,
            &at(&clause, role, 0),
            Capability::ValueValidity,
            DeclarationFamily::Value,
        );
    }
    let function = graph
        .callable("subject")
        .expect("subject is callable")
        .identity;
    assert_eq!(
        claim_family(&package, ItemKind::Formula, &at(&function, "claim", 0)),
        None,
        "the function node does not carry the value claim"
    );
}

/// FR-057-AC-11: a state declaration's claim, read from the checked package,
/// admits `operation-contract`, while `finite-replay` and
/// `probabilistic-satisfaction` settle `inapplicable-kind` naming the family
/// the reader produced (`state-model`), without running an engine.
#[trace("TC-115", "FR-057-AC-11", "FR-281-AC-13")]
#[test]
fn a_state_declaration_admits_operation_contract_and_refuses_other_kinds_as_state_model() {
    let world = claims_world();
    let package = &world.package;
    let clause = at(&node(package, "state_clause"), "claim", 0);
    let settled = settle_many(
        package,
        &[
            (Capability::OperationContract, ItemKind::Formula, &clause),
            (Capability::FiniteReplay, ItemKind::Formula, &clause),
            (
                Capability::ProbabilisticSatisfaction,
                ItemKind::Formula,
                &clause,
            ),
        ],
    );
    assert_engine_settled(&settled[0], 0);
    assert_inapplicable(
        &settled[1],
        1,
        Capability::FiniteReplay,
        Some(DeclarationFamily::StateModel),
    );
    assert_inapplicable(
        &settled[2],
        2,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::StateModel),
    );
}

/// FR-281-AC-13: a state claim refuses every inapplicable handoff before
/// backend lookup; applicable temporal claims preserve their routing results.
#[trace("TC-762", "TC-1302", "FR-281-AC-13")]
#[test]
fn handoff_refusals_are_settled_after_claim_family_is_checked() {
    let profile = "profile t = \"filament.temporal.infinite-trace/v1\";\n";
    let state = "constraint StateOk using v on Config::ConfigVersion at current { true }\n";
    let temporal = "temporal TemporalProgress using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { always holds(true) }\n";
    let body = format!("{CONFIG_CONTRACT}{state}{temporal}");
    let objects: [crate::graph_world::Object<'_>; 1] = [("a", &[("versionNumber", 0)])];
    let world = World::with_profiles(config_version(), profile, &body, &[&objects]);
    let package = &world.package;
    let state_clause = package
        .graph()
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == "StateOk")
        .expect("the fixture has its named state clause");
    let state_claim = OccurrenceKey::new(state_clause.identity(), state_clause.claim().clone());
    let temporal_clause = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.clause().name() == "TemporalProgress")
        .expect("the fixture has its named temporal clause");
    let temporal_claim =
        OccurrenceKey::new(temporal_clause.identity(), temporal_clause.claim().clone());
    let test_a = Candidate::new(fsl_foundation::BackendId::new("fsl.test-a"));
    let test_c = Candidate::new(fsl_foundation::BackendId::new("fsl.test-c"));
    let provider = CountingUnknownProvider::new();
    let value_domain = [fsl_foundation::bound::DomainKind::Integer.to_wire()];
    let descriptor = fsl_route::descriptor_from_manifest(
        test_a.clone(),
        ProviderOrigin::Linked,
        [(
            Some(Capability::ValueValidity.to_wire()),
            Some(Mode::Unbounded.to_wire()),
        )],
        ManifestMembers {
            domains: Some(&value_domain),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("test-a advertises only unbounded value-validity");
    let mut engines = Engines::default();
    engines
        .with(
            Engine::external(
                descriptor,
                BackendTool::new("handoff-test", "1").expect("a tool identity"),
                Box::new(provider.clone()),
            )
            .expect("the external test engine is admitted"),
        )
        .expect("one test engine");

    let item = |index: usize,
                claim: &OccurrenceKey,
                capability: Capability,
                item_kind: ItemKind,
                disposition: Disposition,
                backend: &Candidate,
                diagnostic: Option<Diagnostic>| {
        let mut request = ClaimItem {
            request_index: RequestIndex::new(index),
            claim: claim.clone(),
            kind: capability,
            extent: ExtentClassification::Bounded,
            backend: if matches!(&disposition, Disposition::InvalidRequest) {
                None
            } else {
                Some(backend.id().clone())
            },
            candidates: if matches!(&disposition, Disposition::InvalidRequest) {
                vec![test_a.clone(), test_c.clone()]
            } else {
                vec![backend.clone()]
            },
            options: ItemOptions::default(),
            form: None,
        };
        if capability == Capability::TemporalSatisfaction {
            request.form = Some(ItemForm::Temporal(TemporalItem {
                item_kind,
                ..TemporalItem::default()
            }));
        }
        NegotiatedItem {
            item: request,
            disposition,
            diagnostic,
        }
    };
    let invalid_ambiguous = || Some(Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code()));
    let items = vec![
        item(
            0,
            &state_claim,
            Capability::FiniteReplay,
            ItemKind::Formula,
            Disposition::Supported(test_c.clone()),
            &test_c,
            None,
        ),
        item(
            1,
            &state_claim,
            Capability::FiniteReplay,
            ItemKind::Formula,
            Disposition::Supported(test_a.clone()),
            &test_a,
            None,
        ),
        item(
            2,
            &state_claim,
            Capability::FiniteReplay,
            ItemKind::Formula,
            Disposition::InvalidRequest,
            &test_a,
            invalid_ambiguous(),
        ),
        item(
            3,
            &temporal_claim,
            Capability::TemporalSatisfaction,
            ItemKind::Formula,
            Disposition::Supported(test_c.clone()),
            &test_c,
            None,
        ),
        item(
            4,
            &temporal_claim,
            Capability::TemporalSatisfaction,
            ItemKind::Formula,
            Disposition::Supported(test_a.clone()),
            &test_a,
            None,
        ),
        item(
            5,
            &temporal_claim,
            Capability::TemporalSatisfaction,
            ItemKind::Formula,
            Disposition::InvalidRequest,
            &test_a,
            invalid_ambiguous(),
        ),
    ];
    let subject = world.subject(&["a"], crate::counter::UNLIMITED);
    let binding = SubjectBinding {
        packages: subject.simulation.packages,
        snapshots: subject.simulation.snapshots,
        initial: subject.simulation.initial,
        universes: subject.simulation.universes,
    };
    let request = AnalyzeRequest {
        binding,
        provision: world.source_provision(),
        items,
    };
    let outcome = analyze(
        package,
        &request,
        &engines,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the package binding admits")
    .into_value();

    assert_eq!(outcome.records.len(), 6);
    for (index, record) in outcome.records[..3].iter().enumerate() {
        assert_eq!(record.request_index(), RequestIndex::new(index));
        assert_eq!(
            record.value(),
            &TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Fsl(fsl_foundation::diagnostic::Code::InvalidRequest),
            }
        );
        assert_eq!(
            record.diagnostic(),
            Some(filament_semantic_value::catalog::CatalogCode::new(
                "invalid_capability",
                "inapplicable-kind",
            ))
        );
        assert_eq!(
            record
                .disposition_detail()
                .expect("state-clause refusal includes its family detail")["family"],
            "state-model"
        );
        assert_eq!(record.backend(), None);
    }
    let temporal_expectations = [
        ("unknown-backend", Some("fsl.test-c")),
        ("inconsistent-candidates", Some("fsl.test-a")),
        ("ambiguous-backend", None),
    ];
    for (offset, (cause, backend)) in temporal_expectations.into_iter().enumerate() {
        let record = &outcome.records[offset + 3];
        assert_eq!(record.request_index(), RequestIndex::new(offset + 3));
        assert_eq!(
            record.value(),
            &TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Fsl(fsl_foundation::diagnostic::Code::InvalidRequest),
            }
        );
        assert_eq!(
            record.diagnostic(),
            Some(filament_semantic_value::catalog::CatalogCode::new(
                "invalid_capability",
                cause,
            ))
        );
        let detail = record
            .disposition_detail()
            .expect("the disposition carries its FR-272 detail");
        match (offset, backend) {
            (0, Some("fsl.test-c")) => assert_eq!(detail["backend"], "fsl.test-c"),
            (1, Some("fsl.test-a")) => {
                assert_eq!(detail["candidates"], serde_json::json!(["fsl.test-a"]))
            }
            (2, None) => assert!(detail.is_empty()),
            _ => panic!("unexpected temporal expectation at index {offset}"),
        }
    }
    assert_eq!(provider.calls(), 0, "no rejected handoff runs");
}

/// FR-507-AC-9 evidence for the reader's remaining rows: an `item_kind` of
/// `refinement` is a `relation-claim` whatever node it names, and the
/// `operation_anchor` and `state_clause` rows answer only at their own roles.
#[test]
fn item_kind_and_occurrence_role_decide_the_family() {
    let world = claims_world();
    let package = &world.package;
    let temporal = at(&node(package, "state_graph"), "claim", 0);
    assert_eq!(
        claim_family(package, ItemKind::Formula, &temporal),
        Some(DeclarationFamily::TemporalTrace)
    );
    assert_eq!(
        claim_family(package, ItemKind::Refinement, &temporal),
        Some(DeclarationFamily::RelationClaim)
    );

    let anchor = node(package, "operation_anchor");
    assert_eq!(
        claim_family(package, ItemKind::Formula, &at(&anchor, "anchor", 0)),
        Some(DeclarationFamily::StateModel)
    );
    assert_eq!(
        claim_family(package, ItemKind::Formula, &at(&anchor, "claim", 0)),
        None
    );

    let clause = node(package, "state_clause");
    assert_eq!(
        claim_family(package, ItemKind::Formula, &at(&clause, "claim", 0)),
        Some(DeclarationFamily::StateModel)
    );
    assert_eq!(
        claim_family(package, ItemKind::Formula, &at(&clause, "anchor", 0)),
        None
    );
}

/// FR-507-AC-11: every claim form of "FSL claim forms and their families"
/// that requires a kind is read from its node, tag, `semantic_form` and
/// occurrence role; the Boolean value clause's expression carrier is covered
/// by `a_boolean_value_clause_is_read_on_its_expression_node`.
#[trace("TC-1302", "FR-507-AC-11")]
#[test]
fn each_claim_form_is_read_from_its_node_tag_form_and_role() {
    use DeclarationFamily::{RelationClaim, StateModel, TemporalTrace, Value};

    // `value`: a scalar operation application in a `Value` function body.
    let package = value_package();
    let function = package
        .graph()
        .callable("subject")
        .expect("subject is callable")
        .identity;
    let scalar = package
        .graph()
        .requirements()
        .iter()
        .find(|(_, record)| record.requirements().kind() == Capability::ValueValidity)
        .map(|(occurrence, _)| occurrence.clone())
        .expect("a scalar operation application is a value-validity claim");
    assert_eq!(scalar.origin().role().as_str(), "expression");
    assert_family_read(&package, &scalar, Capability::ValueValidity, Value);

    // The same function node at the role of no value claim has no family.
    assert_eq!(
        claim_family(&package, ItemKind::Formula, &at(&function, "anchor", 0)),
        None
    );

    let world = claims_world();
    let package = &world.package;
    // `state-model`: a frame obligation.
    assert_family_read(
        package,
        &at(&node(package, "operation_anchor"), "anchor", 0),
        Capability::OperationContract,
        StateModel,
    );
    // `temporal-trace`: a state-graph claim, a `hyper` clause and a temporal
    // clause over a timed subject and over a hybrid subject.
    for form in ["state_graph", "hyperproperty"] {
        assert_family_read(
            package,
            &at(&node(package, form), "claim", 0),
            Capability::TemporalSatisfaction,
            TemporalTrace,
        );
    }
    // The claims: a clause with a requirement record. The `hyper` clause's
    // wrapper node is a `temporal_clause` node that is no claim.
    let graph = package.graph();
    let clauses: Vec<NodeKey> = graph
        .requirements()
        .iter()
        .filter(|(occurrence, _)| {
            graph
                .semantic_graph()
                .node(occurrence.node().clone())
                .is_some_and(|node| node.semantic_form() == "temporal_clause")
        })
        .map(|(occurrence, _)| occurrence.node().clone())
        .collect();
    assert_eq!(clauses.len(), 2, "the model-steps and the hybrid clause");
    for clause in &clauses {
        assert_family_read(
            package,
            &at(clause, "claim", 0),
            Capability::TemporalSatisfaction,
            TemporalTrace,
        );
    }
    let timed = crate::zone_search::rpc(4);
    let timed_clause = at(&node(&timed.package, "temporal_clause"), "claim", 0);
    assert_family_read(
        &timed.package,
        &timed_clause,
        Capability::TemporalSatisfaction,
        TemporalTrace,
    );

    // `relation-claim`: a refinement whose abstract side is a protocol.
    let source = crate::refinement_protocol::cas_twice(false, false);
    let world = crate::refinement_fixture::World::with_internals(
        crate::refinement_fixture::Variant {
            peek: false,
            intended_terminal: true,
            ..crate::refinement_fixture::Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let node = world.refinement().v2_emission().expect("emits").identity;
    assert_family_read(
        world.package(),
        &at(&node, "declaration", 0),
        Capability::TemporalSatisfaction,
        RelationClaim,
    );
}
