// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411, FR-940, FR-124: a unit that selects two models, each with its own
//! `terminal` declaration, classifies terminal states and replays an
//! undefined `terminal when` as one declaration on the source route and on
//! the package route, which agree.

use filament_exact::Cancel;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::analyze::{
    AnalyzeLimits, PackageDeadlockSubject, RunInput, SourceProvision, SubjectBinding, TemporalItem,
    deadlock_model_item,
};
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PackageModelInput, PopulationUniverse,
    TerminalClass,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_route::request::ItemKind;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{
    Vault, declarations, document, key, other_document, other_key, snapshot,
    unit_text_with_functions,
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
/// The declaration on `test/vault-other` that never intends a state.
const OTHER_FALSE: &str = "terminal Stop using v on N when { false };\n";
/// The declaration on `test/vault-other` that is undefined at `l = 1`.
const OTHER_UNDEFINED_WHEN: &str = "terminal Done using v on N when { q(1 - self.l) = 0 };\n";

const ONE_IDENTITY_CONTRACTS: &str = "pre Fresh using v on M::Vault::step { self.l = 0 }\n\
     post Step using v on M::Vault::step { self.l = i }\n";
const ONE_IDENTITY_DISABLED_STEP: &str = "pre Disabled using v on M::Vault::step { false }\n";

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

/// One selected vault identity under two aliases, used by FR-124-AC-11.
struct OneIdentity {
    text: String,
    source: SourceProvision,
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
    vault_keys: Vec<String>,
}

fn one_identity(terminals: &str) -> OneIdentity {
    one_identity_with(terminals, ONE_IDENTITY_CONTRACTS, &[("v", (0, 0))], "")
}

fn one_identity_with(
    terminals: &str,
    contracts: &str,
    vaults: &[(&str, (i64, i64))],
    declarations: &str,
) -> OneIdentity {
    let document = document(false, false);
    let text = unit_text_with_functions(
        None,
        Vault::Custom(contracts),
        false,
        FUNCTIONS,
        &format!("{declarations}{terminals}"),
    )
    .replace(
        "model V = \"test/vault\";",
        "model M = \"test/vault\";\nmodel N = \"test/vault\";",
    );
    let documents: [&[u8]; 1] = [&document];
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Kernel,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let source = SourceProvision::single(graph.source(), text.as_bytes());
    let mut snapshots = DocumentProvision::new();
    let start = snapshot(&mut snapshots, "one-identity-start", vaults, None, None);
    OneIdentity {
        text,
        source,
        package: CheckedPackage::link(graph),
        packages: package_input(documents.iter().copied()).unwrap(),
        snapshots,
        initial: vec![start],
        vault_keys: vaults.iter().map(|(key, _)| (*key).to_owned()).collect(),
    }
}

impl OneIdentity {
    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: vec![PopulationUniverse {
                population: key("vaults"),
                keys: self.vault_keys.clone(),
            }],
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
            universes: vec![PopulationUniverse {
                population: key("vaults"),
                keys: self.vault_keys.clone(),
            }],
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
    handoff_package(&world.package, &world.packages)
}

fn handoff_package(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
) -> Handoff {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let bytes = emission.package().bytes().to_vec();
    let admitted = fsl_package::admit_checked_package(&bytes, packages, V2ReadLimits::fixture())
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

/// FR-411, FR-940-AC-6: over a unit that selects two model identities, the
/// source route and the package route classify every state alike, and their
/// terminal declarations join by selection identity. FR-124 reads one model's
/// declarations for a subject and FSpec FR-368-AC-22 gives a package with
/// several model roots no derived deadlock-freedom item, so what a joint
/// state classifies as is not asserted here, only that both routes agree.
#[trace("TC-2958", "FR-940-AC-6", "TC-519", "FR-124-AC-11")]
#[test]
fn two_models_read_their_terminals_in_selection_identity_order_on_both_routes() {
    for terminals in [
        String::new(),
        "terminal Done using v on V when { self.l = 1 };\n\
         terminal Fin using v on N when { self.l = 1 };\n"
            .to_owned(),
        format!("{ANY}{OTHER_UNDEFINED_WHEN}"),
        format!("{UNDEFINED_WHEN}{OTHER_ANY}"),
        format!("{UNDEFINED_WHEN}terminal Fin using v on N when {{ self.l = 1 }};\n"),
        format!("terminal Done using v on V when {{ b(self.l) = 0 }};\n{OTHER_UNDEFINED_WHEN}"),
        UNDEFINED_WHEN.to_owned(),
        format!("{ANY}terminal Done using v on N when {{ b(self.l) = 0 }};\n"),
        format!("terminal Done using v on V when {{ b(self.l) = 0 }};\n{OTHER_ANY}"),
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
        assert!(
            from_source
                .iter()
                .any(|(_, class)| *class != TerminalClass::NotTerminal),
            "{terminals}: a terminal state is reached"
        );
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
    let world = two_models(&format!("{UNDEFINED_WHEN}{OTHER_FALSE}"));
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

/// The Counter unit selecting `test/counter` as `M` and as `N`, with `body`.
fn two_aliases(body: &str) -> (CheckedPackage, fsl_semantics::model::intake::PackageInput) {
    use crate::counter::{PACKAGE, document, unit_text_in};
    let doc = document(false);
    let text = unit_text_in(PACKAGE, body).replace(
        "model M = \"test/counter\";",
        "model M = \"test/counter\";\nmodel N = \"test/counter\";",
    );
    let documents: [&[u8]; 1] = [&doc];
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    (
        CheckedPackage::link(graph),
        package_input(documents.iter().copied()).unwrap(),
    )
}

fn bare_terminal_keys(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
) -> fsl_semantics::check::TerminalKeys {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        packages,
        V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the package");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the package witnesses join");
    fsl_semantics::check::TerminalKeys::of(
        bare.terminal_declarations()
            .expect("the bare terminal rows read")
            .into_iter()
            .map(|terminal| ((terminal.model, terminal.ordinal), terminal.node)),
    )
}

/// FR-411 "Deadlock-freedom verdict join", FR-124: two aliases that select one
/// package identity each declare a terminal. Both routes key the declarations
/// by (identity, ordinal in terminal node key order), so neither replaces the
/// other when source placement changes.
#[trace("TC-1142", "FR-411-AC-5")]
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn two_aliases_of_one_identity_keep_their_terminals_apart() {
    let when = "terminal Done using v on M when { self.value = 3 };\n";
    let any = "terminal Halt using v on N any;\n";
    let (both, packages) = two_aliases(&format!("{when}{any}"));
    let (only_first, _) = two_aliases(when);
    let (only_second, _) = two_aliases(any);

    let keys = both.graph().terminal_keys();
    let first = only_first.graph().terminal_keys();
    let second = only_second.graph().terminal_keys();
    assert_ne!(keys, first, "the second alias's declaration is not dropped");
    assert_ne!(keys, second, "the first alias's declaration is not dropped");
    let refused = keys
        .mismatch(&first)
        .expect("the second declaration is missing");
    let resolved = fsl_package::resolve_terminals(&both).expect("the source nodes resolve");
    let missing_second = resolved
        .iter()
        .find(|terminal| terminal.name == "Halt")
        .expect("the second terminal resolves");
    assert_eq!(refused.requested, Some(missing_second.node.clone()));
    assert_eq!(
        refused.model,
        if missing_second.ordinal == 0 {
            "test/counter".to_owned()
        } else {
            format!("test/counter#{}", missing_second.ordinal)
        }
    );
    assert_eq!(refused.recompiled, None);
    let missing_first = resolved
        .iter()
        .find(|terminal| terminal.name == "Done")
        .expect("the first terminal resolves");
    assert_eq!(
        keys.mismatch(&second)
            .expect("the first declaration is missing")
            .model,
        if missing_first.ordinal == 0 {
            "test/counter".to_owned()
        } else {
            format!("test/counter#{}", missing_first.ordinal)
        },
        "the ordinals count the identity's declarations in terminal node key order"
    );
    assert_eq!(
        fsl_eval::simulation::model::ModelCheckInput::source(&both).terminal_keys(),
        keys,
        "the run's input keys its declarations as the graph does"
    );

    let emission = fsl_package::emit_checked(&both, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &packages,
        V2ReadLimits::fixture(),
    );
    let admitted = admitted.expect("a package carries two terminal nodes on one model node");
    let rows = fsl_package::read_terminals(&admitted).expect("the terminal rows read");
    let [first_row, second_row] = &rows[..] else {
        panic!("two terminal rows: {rows:?}");
    };
    assert_eq!(first_row.model, second_row.model);
    assert_eq!((first_row.ordinal, second_row.ordinal), (0, 1));
    assert!(first_row.node < second_row.node);
    assert_ne!(first_row.node, second_row.node);
    assert_eq!(
        fsl_package::read_terminal(&admitted, &second_row.model, 1),
        Ok(second_row.declaration.clone())
    );
    assert_ne!(first_row.declaration, second_row.declaration);
}

/// FR-124-AC-11: swapping the source placement of two aliases leaves the
/// source route, package resolver and bare package reader on the same
/// (identity, terminal node key order) ordinal assignment.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn terminal_ordinals_are_stable_when_source_placements_swap() {
    let when = "terminal Done using v on M when { self.value = 3 };\n";
    let any = "terminal Halt using v on N any;\n";
    let (first, packages) = two_aliases(&format!("{when}{any}"));
    let (swapped, swapped_packages) = two_aliases(&format!("{any}{when}"));

    let keys = |package: &CheckedPackage, packages: &fsl_semantics::model::intake::PackageInput| {
        let source = package.graph().terminal_keys();
        let resolved = fsl_package::resolve_terminals(package)
            .expect("the source terminal nodes resolve")
            .into_iter()
            .map(|terminal| {
                (
                    (terminal.package.to_owned(), terminal.ordinal),
                    terminal.node,
                )
            });
        assert_eq!(source, fsl_semantics::check::TerminalKeys::of(resolved));
        assert_eq!(source, bare_terminal_keys(package, packages));
        source
    };

    assert_eq!(keys(&first, &packages), keys(&swapped, &swapped_packages));
}

/// FR-124-AC-11: the same identity selected by two aliases keeps the
/// undefined declaration's identity+ordinal key while the source label
/// follows the alias that owns `Done`; a false sibling does not make it
/// intended.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn one_identity_undefined_terminal_keeps_key_and_source_label_across_routes() {
    for (done_alias, stop_alias) in [("M", "N"), ("N", "M")] {
        let terminals = format!(
            "terminal Done using v on {done_alias} when {{ q(1 - self.l) = 0 }};\n\
             terminal Stop using v on {stop_alias} when {{ false }};\n"
        );
        let world = one_identity(&terminals);
        let selection = world
            .package
            .graph()
            .model_selections()
            .first()
            .expect("the shared model identity is selected")
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
        let handoff = handoff_package(&world.package, &world.packages);
        let bare = handoff.bare();
        let input =
            PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
                .expect("the package route reads both terminal nodes");
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
        let fsl_replay::CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind
        else {
            panic!("the counterexample names undefined evaluation: {counterexample:?}");
        };
        let fsl_replay::UndefinedLocus::Terminal(model) = &evaluation.site.locus else {
            panic!("the undefined site names a terminal declaration: {evaluation:?}");
        };
        let done = fsl_package::resolve_terminals(&world.package)
            .expect("the source terminal nodes resolve")
            .into_iter()
            .find(|terminal| terminal.name == "Done")
            .expect("Done resolves");
        assert_eq!(model.identity(), "test/vault");
        assert_eq!(model.ordinal(), done.ordinal);
        assert_eq!(model.label(), done_alias);
        assert!(world.text.contains(&format!("model {done_alias}")));
    }
}

/// FR-124-AC-11: a refused `Done` predicate paired with false `Stop` is
/// undecided, retaining Done's identity, ordinal and source alias in the
/// source model class, and settles inconclusive on both routes.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn one_identity_refused_terminal_is_inconclusive_and_names_done() {
    let world = one_identity_with(
        "terminal Done using v on M when { exists(p in M::vaults : true) };\n\
         terminal Stop using v on N when { false };\n",
        ONE_IDENTITY_DISABLED_STEP,
        &[("a", (0, 0)), ("b", (0, 0))],
        "bound OnePerSet on M::vaults [1]\n",
    );
    let selection = world
        .package
        .graph()
        .model_selections()
        .first()
        .expect("the shared model identity is selected")
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
    let handoff = handoff_package(&world.package, &world.packages);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
        .expect("the package route reads both terminal nodes");
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
    let from_source = check(source_item()).expect("the source check completes");
    let from_package = check(package_item()).expect("the package check completes");
    assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
    assert!(matches!(
        from_source,
        fsl_replay::ModelCheckOutcome::Undecided {
            cause: fsl_replay::UndecidedCause::UndecidedSuccessor,
            ..
        }
    ));

    let mut source_system = ModelSystem::new(world.simulation(
        fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
    ))
    .expect("the source admits");
    let mut package_system =
        ModelSystem::new(world.simulation(input.input())).expect("the package admits");
    let classify = |system: &ModelSystem<'_>| {
        let state = system.initial().into_iter().next().expect("initial state");
        let expansion = system.successors(&state).expect("expands");
        system
            .terminal_class(&state, !expansion.successors.is_empty())
            .expect("the terminal predicates read")
    };
    let fsl_eval::simulation::model::TerminalClass::Undecided {
        model: source_model,
    } = classify(&source_system)
    else {
        panic!("the source class is undecided");
    };
    let fsl_eval::simulation::model::TerminalClass::Undecided {
        model: package_model,
    } = classify(&package_system)
    else {
        panic!("the package class is undecided");
    };
    let done = fsl_package::resolve_terminals(&world.package)
        .expect("the source terminal nodes resolve")
        .into_iter()
        .find(|terminal| terminal.name == "Done")
        .expect("Done resolves");
    assert_eq!(source_model.identity(), "test/vault");
    assert_eq!(source_model.ordinal(), done.ordinal);
    assert_eq!(source_model.label(), "M");
    assert_eq!(package_model.identity(), source_model.identity());
    assert_eq!(package_model.ordinal(), source_model.ordinal());
    assert_eq!(source_model, package_model);
}
