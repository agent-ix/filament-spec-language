// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared fixture support spans split integration test binaries"
)]
//! Shared derived-request helpers used by integration test targets.

use crate::counter::{clause_text, CONTRACT};
use fsl_analyze::analyze::{derived_model_item, AnalyzeItem, AnalyzeRecord, ModelItem};
use fsl_analyze::model_check::manifest as en1;
use fsl_analyze::model_check::SubjectContext;
use fsl_analyze::zone_check::manifest as en6;
use fsl_eval::model_subject::ModelSubject;
use fsl_replay::SubjectIdentity;
use fsl_replay::TerminalValue;
use fsl_route::deadlock::{
    ItemObligation, ModelRefinementItem, ModelStateGraphItem, ModelTemporalItem,
    ProtocolTemporalItem,
};
use fsl_route::model_request::{ModelRequest, WrittenRequest};
use fsl_route::request::{AbstractSubject, DerivedLimits, ItemKind, RequestItem};
use fsl_route::{ProviderOrigin, Registry};
use fsl_semantics::check::Capability;
use fsl_semantics::family::{ClaimExtent, Requirements};
use fsl_semantics::library::LibraryName;
use std::collections::BTreeSet;

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits"))
        .expect("registered");
    registry
        .register(en6::descriptor(ProviderOrigin::Linked).expect("EN-6 admits"))
        .expect("registered");
    registry
}

pub(crate) fn derived_items<S: Clone + Eq>(
    temporal: &[ModelTemporalItem<'_, S>],
    refinements: &[ModelRefinementItem<'_, S>],
    state_graph: &[ModelStateGraphItem<'_, S>],
    selections: &[&str],
) -> Vec<(RequestItem, ItemObligation<S>)> {
    let selections: BTreeSet<&str> = selections.iter().copied().collect();
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let mut request = ModelRequest::new();
    for item in temporal {
        request.authored(item, &record, &selections);
    }
    for item in refinements {
        let abstract_subject = AbstractSubject {
            library: LibraryName::new("abstract").expect("a library name"),
            initial_states: item.abstract_initial.to_vec(),
        };
        request
            .refinement(item, &record, abstract_subject)
            .expect("the refinement has a record");
    }
    for item in state_graph {
        request.state_graph(item, &selections);
    }
    let written = request.derived(&registry(), None, &DerivedLimits::default());
    written
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.item_kind(),
                Some(ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom)
            )
        })
        .map(|item| {
            let obligation = written
                .obligation(item.index())
                .expect("its obligation")
                .clone();
            (item.clone(), obligation)
        })
        .collect()
}

/// Write derived items for the provided protocol temporal items.
pub(crate) fn protocol_request<S: Clone + Eq>(
    items: &[ProtocolTemporalItem<'_, S>],
) -> WrittenRequest<S> {
    protocol_request_under(items, &DerivedLimits::default())
}

/// Write derived items with explicit limits.
pub(crate) fn protocol_request_under<S: Clone + Eq>(
    items: &[ProtocolTemporalItem<'_, S>],
    limits: &DerivedLimits,
) -> WrittenRequest<S> {
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let mut request = ModelRequest::new();
    for item in items {
        request.protocol(item, &record);
    }
    request.derived(&registry(), None, limits)
}

/// Return written items of `kind` and their obligations.
pub(crate) fn items_of_kind<S: Clone + Eq>(
    written: &WrittenRequest<S>,
    kind: ItemKind,
) -> Vec<(RequestItem, ItemObligation<S>)> {
    written
        .items()
        .iter()
        .filter(|item| item.item_kind() == Some(kind))
        .map(|item| {
            let obligation = written.obligation(item.index()).expect("its obligation");
            (item.clone(), obligation.clone())
        })
        .collect()
}

/// Return subjects of derived items of `kind` in derivation order.
pub(crate) fn derived_subjects<S: Clone + Eq>(
    written: &WrittenRequest<S>,
    kind: ItemKind,
) -> Vec<S> {
    let unanchored = written
        .unanchored()
        .iter()
        .filter(|item| item.kind == kind)
        .map(|item| item.subject.clone());
    items_of_kind(written, kind)
        .into_iter()
        .map(|(_, obligation)| obligation.subject)
        .chain(unanchored)
        .collect()
}

/// Return the backend and terminal value of a settled record.
pub(crate) fn settled(record: &AnalyzeRecord) -> (String, TerminalValue) {
    let terminal = record.terminal().expect("a terminal record");
    (
        terminal
            .backend()
            .map(|backend| backend.as_str().to_owned())
            .expect("a backend"),
        terminal.value().clone(),
    )
}

/// Write an authored model clause followed by its derived items.
pub(crate) fn written(
    subject: &ModelSubject<'_>,
    limits: &DerivedLimits,
) -> WrittenRequest<SubjectIdentity> {
    let context = SubjectContext::new(subject).expect("the package emits");
    let mut request = ModelRequest::new();
    let clause = &subject
        .simulation
        .source_package()
        .expect("a source-route subject")
        .graph()
        .temporal_clauses()[0];
    let authored = context
        .author_clause(&mut request, clause)
        .expect("the clause has a record");
    assert_eq!(authored.get(), 0, "the caller's item comes first");
    request.derived(&registry(), None, limits)
}

pub(crate) fn counter_body(terminal: &str) -> String {
    format!(
        "{CONTRACT}{terminal}{}",
        clause_text("Bounded", "", "always holds(c.value <= 3)")
    )
}

pub(crate) fn derived_deadlock(item: ModelItem<'_>) -> AnalyzeItem<'_> {
    let written = written(&item.request.subject, &DerivedLimits::default());
    let derived = &written.items()[1];
    let obligation = written.obligation(derived.index()).expect("its obligation");
    derived_model_item(derived, obligation, item).expect("the deadlock-freedom run")
}
