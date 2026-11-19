// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411, FR-940, FR-124: a unit that selects two models, each with its own
//! `terminal` declaration, classifies terminal states and replays an
//! undefined `terminal when` as one declaration on the source route and on
//! the package route. Both routes order the declarations by the model
//! selection's identity (`test/vault` before `test/vault-other`), the
//! reverse of their aliases (`V`, `N`), so an `any` that comes first
//! intends a state before a later undefined `when` is read.

use filament_exact::Cancel;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::analyze::{
    deadlock_model_item, AnalyzeLimits, PackageDeadlockSubject, RunInput, SourceProvision,
    SubjectBinding, TemporalItem,
};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PackageModelInput, PopulationUniverse,
    TerminalClass,
};
use fsl_eval::simulation::TransitionSystem;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_route::request::ItemKind;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{
    declarations, document, key, other_document, other_key, snapshot, unit_text_with_functions,
    Vault,
};

/// Each vault steps once to `l = i` while `l = 0`, so it is terminal at `l = 1`.
const CONTRACTS: &str = "pre Fresh using v on V::Vault::step { self.l = 0 }\n\
     post Step using v on V::Vault::step { self.l = i }\n\
     pre FreshN using v on N::Vault::step { self.l = 0 }\n\
     post StepN using v on N::Vault::step { self.l = i }\n";

/// `q` divides 6 by its argument, so it is undefined at zero (Kernel).
const FUNCTIONS: &str = "function q using v(x: Integer): Integer pure { 6 mod x }\n\
     function b using v(x: Integer): Int[0, 3] pure { x + 3 }\n";

/// The declaration on `test/vault` (alias `V`) that is undefined at the
/// terminal state `l = 1`.
const UNDEFINED_WHEN: &str = "terminal Done using v on V when { q(1 - self.l) = 0 };\n";
/// The declaration on `test/vault-other` (alias `N`).
const OTHER_ANY: &str = "terminal Halt using v on N any;\n";
/// The declaration on `test/vault` that intends every terminal state.
const ANY: &str = "terminal Halt using v on V any;\n";
/// The declaration on `test/vault-other` that is undefined at `l = 1`.
const OTHER_UNDEFINED_WHEN: &str = "terminal Done using v on N when { q(1 - self.l) = 0 };\n";

/// A two-model unit, its package and its initial snapshot: each vault at
/// `(h, l) = (0, 0)`; a `step` sets `l`, and a vault is terminal once `l` is 1.
struct Two {
    text: String,
    source: SourceProvision,
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
}

fn two_models(terminals: &str) -> Two {
    let first = document(false, false);
    let second = other_document(&first);
    let documents: [&[u8]; 2] = [&first, &second];
    let text = unit_text_with_functions(
        Some(&second),
        Vault::Custom(CONTRACTS),
        false,
        FUNCTIONS,
        terminals,
    );
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Kernel,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let source = SourceProvision::single(graph.source(), text.as_bytes());
    let mut snapshots = DocumentProvision::new();
    let start = snapshot(
        &mut snapshots,
        "s0",
        &[("v", (0, 0))],
        None,
        Some(&[("v", (0, 0))]),
    );
    Two {
        text,
        source,
        package: CheckedPackage::link(graph),
        packages: package_input(documents.iter().copied()).unwrap(),
        snapshots,
        initial: vec![start],
    }
}

impl Two {
    fn universes(&self) -> Vec<PopulationUniverse> {
        [key("vaults"), other_key("vaults")]
            .into_iter()
            .map(|population| PopulationUniverse {
                population,
                keys: vec!["v".to_owned()],
            })
            .collect()
    }

    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes(),
        }
    }

    fn simulation<'a>(
        &'a self,
        input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    ) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            emit: EmitLimits::fixture(),
            input,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        }
    }
}

/// The package written to its handoff bytes, with the rows of its supply.
struct Handoff {
    bytes: Vec<u8>,
    admitted: fsl_package::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: fsl_package::ReadEqualityRows,
    identity_bytes: Vec<u8>,
    equality_bytes: Vec<u8>,
}

fn handoff(world: &Two) -> Handoff {
    let emission = fsl_package::emit_checked(&world.package, EmitLimits::fixture()).expect("emits");
    let bytes = emission.package().bytes().to_vec();
    let admitted =
        fsl_package::admit_checked_package(&bytes, &world.packages, V2ReadLimits::fixture())
            .expect("the strict reader admits the bytes");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    Handoff {
        identity_bytes: identities.to_bytes(&admitted).expect("identities.json"),
        equality_bytes: equality.to_bytes(&admitted).expect("equality.json"),
        bytes,
        admitted,
        identities,
        equality,
    }
}

impl Handoff {
    fn bare(&self) -> BarePackage<'_> {
        BarePackage::new(&self.admitted)
            .with_supply(
                BareSupply {
                    identities: &self.identities,
                    equality: Some(&self.equality),
                },
                &mut V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the supply joins")
    }

    fn supply(&self) -> fsl_replay::HandoffSupply<'_> {
        fsl_replay::HandoffSupply {
            identities: &self.identity_bytes,
            equality: Some(&self.equality_bytes),
        }
    }
}

/// The class of every state `system` reaches, breadth first.
fn classes(system: &ModelSystem<'_>) -> Vec<(Vec<u8>, TerminalClass)> {
    let mut queue: std::collections::VecDeque<_> = system.initial().into_iter().collect();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut classes = Vec::new();
    while let Some(state) = queue.pop_front() {
        let key = system.state_key(&state).expect("a state key").0;
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let expansion = system.successors(&state).expect("expands");
        let class = system
            .terminal_class(&state, !expansion.successors.is_empty())
            .expect("the class reads");
        classes.push((key, class));
        queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
    }
    classes
}

/// What the terminal state of the two-model unit classifies as.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Expected {
    Deadlocked,
    Intended,
    Undefined,
    Undecided,
}

/// FR-124, FR-411, FR-940: over two models, the declarations are read in
/// selection-identity order on both routes (`test/vault` before
/// `test/vault-other`, the reverse of the aliases `V`, `N`). An `any` read
/// first intends a state before a later undefined or refused `when`; the
/// `when` read first decides the state: undefined (`6 mod 0`) or undecided
/// (a function result outside its `Int[0, 3]`). No declaration deadlocks the
/// terminal state, and a plain `when` that holds intends it.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn two_models_read_their_terminals_in_selection_identity_order_on_both_routes() {
    for (terminals, expected) in [
        (String::new(), Expected::Deadlocked),
        (
            "terminal Done using v on V when { self.l = 1 };\n\
             terminal Fin using v on N when { self.l = 1 };\n"
                .to_owned(),
            Expected::Intended,
        ),
        (format!("{ANY}{OTHER_UNDEFINED_WHEN}"), Expected::Intended),
        (format!("{UNDEFINED_WHEN}{OTHER_ANY}"), Expected::Undefined),
        (
            format!("{ANY}terminal Done using v on N when {{ b(self.l) = 0 }};\n"),
            Expected::Intended,
        ),
        (
            format!("terminal Done using v on V when {{ b(self.l) = 0 }};\n{OTHER_ANY}"),
            Expected::Undecided,
        ),
    ] {
        let world = two_models(&terminals);
        let handoff = handoff(&world);
        let bare = handoff.bare();
        let input =
            PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
                .expect("the package route reads both terminals");
        let over_source = ModelSystem::new(world.simulation(
            fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
        ))
        .expect("the source admits");
        let over_bytes =
            ModelSystem::new(world.simulation(input.input())).expect("the package admits");
        let (from_source, from_bytes) = (classes(&over_source), classes(&over_bytes));
        assert_eq!(from_bytes, from_source, "{terminals}");
        let found = from_source
            .iter()
            .find_map(|(_, class)| match class {
                TerminalClass::NotTerminal => None,
                TerminalClass::Deadlocked => Some((Expected::Deadlocked, None)),
                TerminalClass::Intended => Some((Expected::Intended, None)),
                TerminalClass::Undefined { model, .. } => {
                    Some((Expected::Undefined, Some(model.clone())))
                }
                TerminalClass::Undecided { model } => {
                    Some((Expected::Undecided, Some(model.clone())))
                }
            })
            .expect("a terminal state is reached");
        assert_eq!(found.0, expected, "{terminals}");
        if let Some(model) = found.1 {
            assert_eq!(model.identity(), "test/vault");
            assert_eq!(model.label(), "V", "the source route labels by alias");
        }
        assert_eq!(
            world_keys(&world),
            input.input().terminal_keys(),
            "{terminals}: the declarations join by selection identity"
        );
    }
}

fn world_keys(world: &Two) -> fsl_semantics::check::TerminalKeys {
    fsl_eval::simulation::model::ModelCheckInput::source(&world.package).terminal_keys()
}

/// FR-411, FR-940, FR-128-AC-6: the deadlock-freedom item over the two-model
/// unit whose first declaration is an undefined `when` is refuted at the
/// terminal predicate, and the package route's run and its replay from the
/// package bytes give the source route's outcome and settlement.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-539", "FR-128-AC-6")]
#[test]
fn an_undefined_terminal_of_two_models_runs_and_replays_as_on_the_source_route() {
    let world = two_models(&format!("{UNDEFINED_WHEN}{OTHER_ANY}"));
    let selection = world
        .package
        .graph()
        .model_selections()
        .first()
        .expect("a selection")
        .clone();
    let claim = OccurrenceKey::generated(
        fsl_semantics::check::model_root_key(&selection).expect("the model root"),
    );
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let run = RunInput::source(&world.package);
    let source_item = || {
        deadlock_model_item(
            &run,
            &world.binding(),
            AnalyzeLimits::unbounded(),
            &world.source,
            &claim,
            &form,
        )
        .expect("the derived target is formed")
    };
    let handoff = handoff(&world);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
        .expect("the package route reads both terminals");
    let package_item = || {
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(),
            AnalyzeLimits::unbounded(),
            &claim,
            &form,
        )
        .model_item()
    };
    let check = |item: fsl_analyze::analyze::ModelItem<'_>| {
        fsl_analyze::model_check::check_model(item.request, &Cancel::new())
    };
    let from_source = check(source_item()).expect("the source checks");
    let from_package = check(package_item()).expect("the package checks");
    assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = from_source else {
        panic!("the undefined terminal refutes the item: {from_source:?}");
    };
    assert!(
        matches!(
            counterexample.kind,
            fsl_replay::CounterexampleKind::UndefinedEvaluation(_)
        ),
        "{counterexample:?}"
    );
    let source_replay = (source_item().replay)(&counterexample).expect("the source replays");
    let package_replay = (package_item().replay)(&counterexample).expect("the bytes replay");
    assert!(
        format!("{:?}", source_replay.settlement).contains("Reproduced"),
        "{:?}",
        source_replay.settlement
    );
    assert_eq!(
        format!("{:?}", package_replay.settlement),
        format!("{:?}", source_replay.settlement)
    );
    assert_eq!(
        source_replay.terminals.mismatch(&package_replay.terminals),
        None
    );
    assert!(world.text.contains("model N"));
}
