// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-940: the model-check input of a model system, built on the
//! source route from the source package and on the package route once from
//! a `BarePackage`'s admitted nodes and handoff supply.

use fsl_eval::simulation::model::{
    ModelCheckInput, ModelSystem, PackageModelInput, PendingPackageForm,
};
use fsl_eval::simulation::TransitionSystem;
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use ix_trace_rs::trace;
use serde_json::Value as Json;

use crate::counter::{document, package_with_clauses, subject, Documents, CONTRACT, UNLIMITED};

/// The Counter domain package with no operation: its object type keeps its
/// field and population and declares nothing a step applies.
fn still_document() -> Vec<u8> {
    let mut document: Json =
        serde_json::from_slice(&document(false)).expect("the Counter document is JSON");
    for declared in document["types"]
        .as_array_mut()
        .expect("the document declares types")
    {
        if declared.get("operations").is_some() {
            declared["operations"] = Json::Array(Vec::new());
        }
    }
    document.to_string().into_bytes()
}

/// The admitted bytes `package` was written to, with the identity and
/// equality rows of its handoff.
struct Handoff {
    admitted: fsl_package::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: fsl_package::ReadEqualityRows,
}

fn handoff(package: &CheckedPackage, documents: &Documents) -> Handoff {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &documents.packages,
        V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the written bytes");
    let identities = emission
        .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("the equality rows");
    Handoff {
        admitted,
        identities,
        equality,
    }
}

fn bare(handoff: &Handoff) -> BarePackage<'_> {
    BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: Some(&handoff.equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply")
}

/// The state keys of `system`'s initial states, in order.
fn initial_keys(system: &ModelSystem<'_>) -> Vec<Vec<u8>> {
    system
        .initial()
        .iter()
        .map(|state| system.state_key(state).expect("a state key").0)
        .collect()
}

/// FR-940-AC-6 (model-check input half), FR-531: over a Counter package whose
/// model declares no operation, the package route builds the model-check
/// input once from the admitted bytes and handoff rows, and the model system
/// over it has the source route's populations, requires-bound roots and
/// initial state keys.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_package_route_input_builds_the_source_route_model_system() {
    let model = still_document();
    let (package, _) = package_with_clauses(&model, "", &[]);
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 1), ("b", 2)]);
    let source = subject(&package, &documents, vec![start], &["a", "b"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package route reads every form");
    let mut request = source.simulation.clone();
    request.input = input.input();
    assert!(request.source_package().is_none());
    assert!(source.simulation.source_package().is_some());

    let over_source = ModelSystem::new(source.simulation.clone()).expect("the source admits");
    let over_bytes = ModelSystem::new(request).expect("the package route admits");
    assert_eq!(
        format!("{:?}", over_source.populations()),
        format!("{:?}", over_bytes.populations())
    );
    assert_eq!(over_source.rooted_domains(), over_bytes.rooted_domains());
    assert_eq!(initial_keys(&over_source), initial_keys(&over_bytes));
    assert_eq!(initial_keys(&over_bytes).len(), 1);
    assert!(over_bytes.source_package().is_none());
}

/// FR-940: a package with a state clause and an operation frame refuses the
/// package-route input at its first such node, naming the form; no input
/// that leaves the clause out is built.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_input_refuses_a_form_it_does_not_read() {
    let model = document(false);
    let (package, _) = package_with_clauses(&model, CONTRACT, &[]);
    let documents = Documents::new(model);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let Err(PendingPackageForm::Form { form, node }) = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    ) else {
        panic!("the Counter contract's clauses are not read on the package route");
    };
    assert_eq!(form, fsl_package::UnreadModelForm::State);
    assert!(
        handoff
            .admitted
            .resolved_graph()
            .nodes
            .iter()
            .any(|admitted| admitted.node_id == node && admitted.node_tag.as_ref() == "state"),
        "{node:?}"
    );
}

/// The canonical keys of every state `system` reaches from its initial
/// states, in breadth-first order, with each state's successor keys.
fn reachable(system: &ModelSystem<'_>) -> Vec<(Vec<u8>, Vec<Vec<u8>>)> {
    let key = |state: &<ModelSystem<'_> as TransitionSystem>::State| {
        system.state_key(state).expect("a state key").0
    };
    let mut queue: std::collections::VecDeque<_> = system.initial().into_iter().collect();
    let mut seen = Vec::new();
    while let Some(state) = queue.pop_front() {
        let at = key(&state);
        if seen.iter().any(|(known, _)| *known == at) {
            continue;
        }
        let expansion = system.successors(&state).expect("expands");
        let next: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(_, next)| next)
            .collect();
        seen.push((at, next.iter().map(key).collect()));
        queue.extend(next);
    }
    seen
}

/// FR-940, FR-850: a model whose domain package declares operations is built
/// over a package-route input with the operation table S3 declares for its
/// selected domain packages, so its steps reach the source route's states
/// with the source route's successors, rather than being refused.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_system_steps_the_operations_of_its_domain_packages() {
    let model = document(false);
    let (package, _) = package_with_clauses(&model, "", &[]);
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 1)]);
    let source = subject(&package, &documents, vec![start], &["a"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package holds no unread node");
    assert_eq!(
        input.input().operations,
        package.graph().scope().operations(),
        "the package route declares the source route's operations"
    );
    assert_ne!(
        *package.graph().scope().operations(),
        fsl_semantics::model::operation::OperationTable::default(),
        "the Counter declares operations"
    );
    let mut request = source.simulation.clone();
    request.input = input.input();
    let over_bytes = ModelSystem::new(request).expect("the package route admits operations");
    let over_source = ModelSystem::new(source.simulation).expect("the source admits");
    let (from_bytes, from_source) = (reachable(&over_bytes), reachable(&over_source));
    assert_eq!(from_bytes, from_source);
    assert!(
        from_source.iter().any(|(_, next)| !next.is_empty()),
        "the operations step the counter"
    );
}

/// FR-531: the source route's input reads the source package's tables and
/// evaluates through the source package.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_source_route_input_reads_the_source_package() {
    let model = document(false);
    let (package, _) = package_with_clauses(&model, CONTRACT, &[]);
    let input = ModelCheckInput::source(&package);
    let graph = package.graph();
    assert!(std::ptr::eq(input.types, graph.scope().types()));
    assert!(std::ptr::eq(input.operations, graph.scope().operations()));
    let names: Vec<&str> = input
        .state_clauses
        .iter()
        .map(|clause| clause.name.as_str())
        .collect();
    let checked: Vec<&str> = graph
        .state_clauses()
        .iter()
        .map(|clause| clause.name())
        .collect();
    assert_eq!(names, checked);
    assert_eq!(names.len(), 2);
    assert!(input
        .source_package()
        .is_some_and(|source| std::ptr::eq(source, &package)));
}
