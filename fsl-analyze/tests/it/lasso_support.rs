// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared bounded-lasso fixture constructors.

use crate::counter::{document, package_from_text, unit_text, Documents, CONTRACT};
use crate::liveness_support::Fixture;

pub(crate) fn bounded_fixture(formula: &str) -> Fixture {
    bounded_fixture_on("on origin", formula)
}

pub(crate) fn bounded_fixture_on(activation: &str, formula: &str) -> Fixture {
    bounded_fixture_over(document(false), CONTRACT, activation, formula)
}

pub(crate) fn bounded_fixture_over(
    doc: Vec<u8>,
    contract: &str,
    activation: &str,
    formula: &str,
) -> Fixture {
    let body = contract.to_owned()
        + &format!(
            "temporal C using tb over (c: M::Counter) clock \"model-steps\" {activation} {{ {formula} }}\n"
        );
    let text = unit_text(&body).replace(
        "profile t0 = \"filament.temporal.infinite-trace/v1\";\n",
        "profile tb = \"filament.temporal.event-position.false-extension/v1\";\n",
    );
    let package = package_from_text(&doc, &text);
    let clauses = package
        .graph()
        .temporal_clauses()
        .iter()
        .map(|node| node.clause().clone())
        .collect();
    let mut documents = Documents::new(doc);
    let initial = vec![documents.start("s0", &[("c", 0)])];
    let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&package);
    Fixture {
        package,
        temporal_clock_selections,
        clauses,
        documents,
        initial,
        keys: vec!["c"],
        unit: text.into_bytes(),
    }
}
