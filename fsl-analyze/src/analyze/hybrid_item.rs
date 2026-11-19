// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the item of a checked hybrid claim, run on the in-process hybrid
//! provider its negotiated candidate names and settled by FR-250 with FR-450's
//! replay.

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{HybridCounterexample, HybridReplayMeter, HybridSolveRequest, TerminalValue};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::{AnalyzeItem, EngineReport, HybridItem, Refusal};

/// What a hybrid item settled: its value and, for `refuted`, the replayed
/// counterexample with its FR-286 spelling.
#[derive(Debug)]
pub(crate) struct HybridReport {
    pub(crate) value: TerminalValue,
    pub(crate) counterexample: Option<Box<HybridCounterexample>>,
    pub(crate) wire: Result<serde_json::Value, fsl_replay::CounterexampleWireError>,
}

/// The item of the hybrid claim at `claim`, or `None` when the package holds
/// no checked hybrid claim there.
pub(super) fn hybrid_item<'a>(
    package: &'a CheckedPackage,
    request_index: RequestIndex,
    claim: &OccurrenceKey,
    form: &'a HybridItem,
    work_units: u64,
) -> Option<AnalyzeItem<'a>> {
    let graph = package.graph();
    let position = graph
        .hybrid_claim_sites()
        .iter()
        .position(|site| &site.identity == claim.node() && &site.claim == claim.origin())?;
    let checked = graph.hybrid_claims().get(position)?;
    let model = graph.hybrid_models().get(checked.model.get())?;
    Some(
        AnalyzeItem::new_with_engine(
            request_index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            Mode::Bounded,
            fsl_hybrid_reference::BACKEND,
            move |cancel, engine| {
                let Some(provider) = engine.hybrid_provider() else {
                    return EngineReport::Refused(Refusal::Unsupported(CatalogCode::new(
                        "unsupported_projection",
                        "tool-unavailable",
                    )));
                };
                let bounds = engine.descriptor().solver_bounds();
                let budget = form.limits.resolved_budget(
                    bounds
                        .and_then(|bounds| bounds.max_solver_time)
                        .unwrap_or(u64::MAX),
                    bounds
                        .and_then(|bounds| bounds.max_solver_steps)
                        .unwrap_or(u64::MAX),
                );
                let request = HybridSolveRequest {
                    claim: checked,
                    model,
                    budget,
                    options: form.options.clone(),
                    deadline_ms: budget
                        .time_seconds()
                        .map_or(u64::MAX, |seconds| seconds.saturating_mul(1000)),
                };
                let answer = provider.solve(&request, cancel);
                let mut meter = HybridReplayMeter::new(work_units);
                match answer.result.settle(graph, checked, &mut meter) {
                    Ok(settled) => {
                        let wire = match &settled.counterexample {
                            Some(found) => found.to_wire(model).ok_or(
                                fsl_replay::CounterexampleWireError::Unspelled(
                                    "hybrid undefined-evaluation",
                                ),
                            ),
                            None => Ok(serde_json::Value::Null),
                        };
                        EngineReport::Hybrid(Box::new(HybridReport {
                            value: settled.value,
                            counterexample: settled.counterexample.map(Box::new),
                            wire,
                        }))
                    }
                    Err(_) => EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    ))),
                }
            },
        )
        .hybrid(),
    )
}
