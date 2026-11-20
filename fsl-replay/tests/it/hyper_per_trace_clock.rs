// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2277 (FR-640-AC-12): compile a two-model hyper clause with one
//! event-position clock row per trace.

use filament_contract_model::ClockConfiguration;
use fsl_package::{emit_checked_with_temporal_selections, TemporalClockSelection};
use ix_trace_rs::trace;

use crate::vault_world::World;

/// Compile emits independent event-position rows for both model identities.
#[trace("TC-2243", "FR-640-AC-12", "FSpec-TC-1680", "FSpec-FR-322-AC-453")]
#[test]
fn a_two_model_event_position_hyper_clause_emits_rows_per_trace() {
    let declaration = "hyper Saves using ep over behaviours (d: V::Vault, e: N::Vault) \
         forall trace s of V forall trace n of N \
         { always holds(d.l @ s != e.l @ n) }";
    let world = World::two_aliases(declaration, &crate::vault_world::SECURE);
    let clause = world.clause();
    let graph = world.package().graph();
    let source = graph.source().clone();
    let selections: Vec<_> = clause
        .quantifiers()
        .iter()
        .map(|quantifier| {
            let mut selection = TemporalClockSelection::new(
                source.clone(),
                clause.span(),
                ClockConfiguration::EventPosition {
                    sequence_authority: match quantifier.variable.as_str() {
                        "s" => "test/vault",
                        "n" => "test/vault-other",
                        other => panic!("unexpected trace variable {other}"),
                    }
                    .into(),
                },
            );
            selection.trace = Some(quantifier.variable.as_str().into());
            selection
        })
        .collect();
    let emitted = emit_checked_with_temporal_selections(
        world.package(),
        &selections,
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the two-model hyper clause compiles");
    let occurrence = clause
        .assembled_occurrence()
        .expect("the clause is assembled");
    let rows = emitted.temporal_binding_rows();
    let s = rows
        .get_for_trace(occurrence.node(), occurrence.origin().ordinal(), "s")
        .expect("trace s has its own clock row");
    let n = rows
        .get_for_trace(occurrence.node(), occurrence.origin().ordinal(), "n")
        .expect("trace n has its own clock row");
    assert_eq!(s.trace.as_deref(), Some("s"));
    assert_eq!(n.trace.as_deref(), Some("n"));
    assert_eq!(s.clock_name.as_ref(), "model-steps");
    assert_eq!(n.clock_name.as_ref(), "model-steps");
    assert_eq!(
        s.clock,
        ClockConfiguration::EventPosition {
            sequence_authority: "test/vault".into()
        }
    );
    assert_eq!(
        n.clock,
        ClockConfiguration::EventPosition {
            sequence_authority: "test/vault-other".into()
        }
    );
}
