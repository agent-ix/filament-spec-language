// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283/501/504: package window evaluation and caller-source rendering.

use std::collections::BTreeMap;

use filament_exact::{Meter, NodeKey, Origin, Role};
use fsl_eval::temporal::{HoldsStop, PackagePositions, TimestampedTrace};
use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef, SourceRegion};
use fsl_foundation::{Source, SourceIdentity, Span};
use fsl_package::occurrence_source::OccurrenceSourceRefusal;
use fsl_semantics::check::{CheckedTemporalClause, FormulaNode, FormulaNodeId};

use super::package_selection::PackageSelectionNotRead;
use super::{ClauseDisposition, RunRefusal, TemporalVerdict, TraceInput, WindowRun};

/// A mandatory source attribution unavailable beside an original result.
#[derive(Debug)]
pub struct PackageAtomAttributionFailure {
    /// The actual selected claim occurrence.
    pub selector: OccurrenceKey,
    /// The formula use that required attribution, when one was selected.
    pub atom: Option<usize>,
    /// The helper's untouched typed reason.
    pub reason: OccurrenceSourceRefusal,
    /// The exact owning artifact row or member.
    pub pointer: String,
    /// The original budget denial, when the owning reader supplied one.
    pub budget_context: Option<fsl_foundation::diagnostic::BudgetDenial>,
    /// The original finite-window result, before any label was rendered.
    pub original_window: Option<Box<crate::PackageWindowEvaluation>>,
    /// The original condition refusal, before any locus was rendered.
    pub original_refusal: Option<super::super::CallOutcome>,
}

pub(super) struct Context<'a, 'p> {
    pub package: &'a fsl_package::BarePackage<'p>,
    pub evaluator: &'a dyn fsl_eval::simulation::model::ModelEvaluator,
    pub clause: &'a CheckedTemporalClause,
    pub key: &'a NodeKey,
    pub name: &'a str,
    pub sources: &'a [Source],
    pub claim: OccurrenceKey,
    pub declaration: SourceRegion,
}

impl<'a, 'p> Context<'a, 'p> {
    pub(super) fn new(
        package: &'a fsl_package::BarePackage<'p>,
        evaluator: &'a dyn fsl_eval::simulation::model::ModelEvaluator,
        clause: &'a CheckedTemporalClause,
        key: &'a NodeKey,
        name: &'a str,
        sources: &'a [Source],
    ) -> Result<Self, PackageSelectionNotRead> {
        let claim =
            |ordinal| OccurrenceKey::new(key.clone(), Origin::new(Role::new("claim"), ordinal));
        let row = package
            .package()
            .temporal_clause_names()
            .unwrap_or_default()
            .iter()
            .find(|row| row.name.as_ref() == name)
            .ok_or_else(|| PackageSelectionNotRead::Selection {
                member: "clause.selection.name",
                reason: "the selected clause has no admitted name row".to_owned(),
            })?;
        let selected = claim(row.occurrence);
        let region = &row.declaration_region;
        let source = RawSourceRef::new(
            region.source.authority.as_ref(),
            region.source.identity.as_ref(),
        )
        .map_err(|_| {
            Self::failure(
                &selected,
                None,
                OccurrenceSourceRefusal::MissingRegion,
                "/request/package/temporal_clause_names".to_owned(),
            )
        })?;
        let declaration = SourceRegion::new(source, region.start, region.end).map_err(|_| {
            Self::failure(
                &selected,
                None,
                OccurrenceSourceRefusal::MissingRegion,
                "/request/package/temporal_clause_names".to_owned(),
            )
        })?;
        Ok(Self {
            package,
            evaluator,
            clause,
            key,
            name,
            sources,
            claim: selected,
            declaration,
        })
    }

    fn failure(
        claim: &OccurrenceKey,
        atom: Option<usize>,
        reason: OccurrenceSourceRefusal,
        pointer: String,
    ) -> PackageSelectionNotRead {
        PackageSelectionNotRead::Attribution(Box::new(PackageAtomAttributionFailure {
            selector: claim.clone(),
            atom,
            reason,
            pointer,
            budget_context: None,
            original_window: None,
            original_refusal: None,
        }))
    }

    fn attribution(&self, atom: usize, reason: OccurrenceSourceRefusal) -> PackageSelectionNotRead {
        let pointer = self
            .package
            .temporal_clause_node(self.key)
            .ok()
            .and_then(|id| {
                fsl_package::checked_views::temporal_subject(self.package.package(), id).ok()
            })
            .and_then(|view| {
                view.formula_graph()
                    .get(atom)
                    .map(|entry| entry.node.clone())
            })
            .map(|id| {
                let rows = self.package.package().source_map();
                match rows.iter().position(|entry| {
                    entry.node_id == id
                        && entry.role == filament_contract_model::CheckedOccurrenceRole::Expression
                }) {
                    Some(index) => format!("/request/package/source_map/{index}/regions"),
                    None => format!("/request/package/source_map/{}", rows.len()),
                }
            })
            .unwrap_or_else(|| "/request/package/temporal_clause_names".to_owned());
        Self::failure(&self.claim, Some(atom), reason, pointer)
    }

    fn source_text(&self, region: &SourceRegion) -> Result<String, PackageSelectionNotRead> {
        let missing = || {
            PackageSelectionNotRead::Run(Box::new(RunRefusal::SourceNotSupplied {
                reference: SourceIdentity::new(
                    region.source().authority(),
                    region.source().identity(),
                ),
                clause: Some(self.name.to_owned()),
            }))
        };
        let source = self
            .sources
            .iter()
            .find(|source| source.reference() == region.source())
            .ok_or_else(missing)?;
        let span = Span {
            start: usize::try_from(region.start()).map_err(|_| missing())?,
            end: usize::try_from(region.end()).map_err(|_| missing())?,
        };
        source.slice(span).map(str::to_owned).ok_or_else(missing)
    }

    /// Compare caller text only after reading every actual candidate region.
    pub fn atom_name(&self, atom: FormulaNodeId) -> Result<String, PackageSelectionNotRead> {
        self.render_atom(atom).map(|rendered| rendered.name)
    }

    fn render_atom(
        &self,
        atom: FormulaNodeId,
    ) -> Result<super::RenderedClauseAtom, PackageSelectionNotRead> {
        let candidates = self
            .package
            .temporal_atom_sources(&self.claim, atom.index())
            .map_err(|reason| self.attribution(atom.index(), reason))?;
        let mut common = None;
        let mut differing = false;
        for candidate in &candidates {
            let text = self.source_text(&candidate.region)?;
            differing |= common.as_ref().is_some_and(|held| held != &text);
            common.get_or_insert(text);
        }
        let text = common.ok_or_else(|| {
            self.attribution(atom.index(), OccurrenceSourceRefusal::MissingRegion)
        })?;
        if candidates.len() > 1 {
            self.source_text(&self.declaration)?;
        }
        if !differing {
            let region = if candidates.len() == 1 {
                candidates[0].region.clone()
            } else {
                self.declaration.clone()
            };
            return Ok(super::RenderedClauseAtom { name: text, region });
        }
        let term = self
            .package
            .project_holds(self.key, atom.index())
            .map_err(|_| self.attribution(atom.index(), OccurrenceSourceRefusal::UnknownNode))?;
        let rows = crate::canonical_atom::CanonicalRows::from_projection(
            term.projection(),
            &self.package.declared_composites(),
        )
        .map_err(|_| self.attribution(atom.index(), OccurrenceSourceRefusal::UnknownNode))?;
        let name = crate::canonical_atom::package_label(&term, &rows)
            .map_err(|_| self.attribution(atom.index(), OccurrenceSourceRefusal::UnknownNode))?;
        Ok(super::RenderedClauseAtom {
            name,
            region: self.declaration.clone(),
        })
    }

    pub fn classes(&self) -> Result<Vec<usize>, PackageSelectionNotRead> {
        let id = self
            .package
            .temporal_clause_node(self.key)
            .map_err(|_| self.attribution(0, OccurrenceSourceRefusal::UnknownNode))?;
        let view = fsl_package::checked_views::temporal_subject(self.package.package(), id)
            .map_err(|_| self.attribution(0, OccurrenceSourceRefusal::UnknownNode))?;
        let uses = view.formula_graph();
        Ok(uses
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                uses.iter()
                    .take(index)
                    .position(|earlier| earlier.node == entry.node)
                    .unwrap_or(index)
            })
            .collect())
    }

    pub fn refused(
        &self,
        evaluation: filament_eval::Evaluation,
    ) -> Result<ClauseDisposition, PackageSelectionNotRead> {
        let mut selected = None;
        if let Some(location) = &evaluation.location {
            for (index, node) in self.clause.formula().nodes().iter().enumerate() {
                if !matches!(node, FormulaNode::Holds { .. }) {
                    continue;
                }
                let term = self
                    .package
                    .project_holds(self.key, index)
                    .map_err(|_| self.attribution(index, OccurrenceSourceRefusal::UnknownNode))?;
                match term.projection().source_at_location_within(
                    self.package,
                    location,
                    &self.declaration,
                ) {
                    Ok(Some(region)) => {
                        if selected.as_ref().is_some_and(|held| held != &region) {
                            return Err(
                                self.attribution(index, OccurrenceSourceRefusal::AmbiguousRegion)
                            );
                        }
                        selected = Some(region);
                    }
                    Ok(None) => {}
                    Err(reason) => {
                        let candidates = self
                            .package
                            .temporal_atom_sources(&self.claim, index)
                            .map_err(|reason| self.attribution(index, reason))?;
                        if candidates.len() > 1 {
                            // Naming verifies caller provisioning; the declaration
                            // is a true coarser region, never an arbitrary use.
                            self.atom_name(FormulaNodeId::from_arena_index(index))?;
                            selected = Some(self.declaration.clone());
                        } else {
                            let mut failure = self.attribution(index, reason);
                            if let PackageSelectionNotRead::Attribution(attribution) = &mut failure
                            {
                                attribution.original_refusal =
                                    super::super::super::call::convert_outcome_resolving(
                                        evaluation,
                                        |_| None,
                                        &[],
                                    )
                                    .ok();
                            }
                            return Err(failure);
                        }
                    }
                }
            }
        }
        match super::super::super::call::convert_outcome_resolving(
            evaluation,
            |_| selected.clone(),
            self.sources,
        ) {
            Ok(outcome) => Ok(ClauseDisposition::Evaluate(outcome)),
            Err(mut refusal) => {
                if let RunRefusal::SourceNotSupplied { clause, .. } = refusal.as_mut() {
                    *clause = Some(self.name.to_owned());
                }
                Err(PackageSelectionNotRead::Run(refusal))
            }
        }
    }

    pub(super) fn evaluate(
        &self,
        positions: PackagePositions<'_>,
        trace: &TimestampedTrace,
        absent: &[(u64, u64)],
        silent_from: Option<u64>,
        classes: &[usize],
        meter: &mut Meter,
    ) -> Result<WindowRun, PackageSelectionNotRead> {
        let observed = fsl_eval::temporal::evaluate_package_finite_window(
            self.evaluator,
            self.clause,
            self.key,
            positions,
            trace,
            fsl_eval::temporal::WindowValuations {
                absent,
                classes,
                silent_from,
            },
            meter,
        );
        let undefined = observed.undefined;
        if let Some(stop) = observed.stopped {
            return match stop {
                HoldsStop::Refused { at, evaluation } => Ok(WindowRun {
                    refused_at: Some(at.index()),
                    ..WindowRun::stopped(self.refused(evaluation)?)
                }),
                HoldsStop::Failed(failure) => {
                    Ok(WindowRun::stopped(super::package_call_disposition(failure)))
                }
            };
        }
        let evaluation = match observed.result {
            Ok(evaluation) => evaluation,
            Err(fsl_eval::temporal::TemporalRefusal::Fault(fault)) => {
                return Ok(WindowRun::stopped(ClauseDisposition::EvaluateFault(fault)));
            }
            Err(refusal) => {
                return Ok(WindowRun::stopped(ClauseDisposition::TemporalRefusal(
                    super::trace_refusal(refusal),
                )));
            }
        };
        let mut requested: Vec<FormulaNodeId> =
            evaluation.support.iter().map(|pair| pair.atom).collect();
        let mut named_atom = None;
        if let fsl_eval::temporal::WindowVerdict::Indeterminate {
            reason:
                fsl_eval::temporal::IndeterminateReason::MissingValuation { atom, .. }
                | fsl_eval::temporal::IndeterminateReason::ConflictingValuation { atom, .. },
            ..
        } = &evaluation.verdict
        {
            named_atom = Some(*atom);
            requested.push(*atom)
        }
        if let fsl_eval::temporal::WindowVerdict::Undefined(
            fsl_eval::temporal::UndefinedEvaluation {
                cause: fsl_eval::temporal::UndefinedCause::InsufficientData(_),
                ..
            },
        ) = &evaluation.verdict
        {
            if let Some(atom) = evaluation.undefined_atom {
                named_atom = Some(atom);
                requested.push(atom);
            }
        }
        let mut labels = BTreeMap::new();
        let mut regions = BTreeMap::new();
        for atom in requested {
            if labels.contains_key(&atom) {
                continue;
            }
            match self.render_atom(atom) {
                Ok(rendered) => {
                    regions.insert(atom, rendered.region);
                    labels.insert(atom, rendered.name);
                }
                Err(mut failure) => {
                    if let PackageSelectionNotRead::Attribution(attribution) = &mut failure {
                        attribution.original_window = Some(Box::new(evaluation.into()));
                    }
                    return Err(failure);
                }
            }
        }
        let names = |atom: FormulaNodeId| {
            labels.get(&atom).cloned().ok_or_else(|| {
                self.attribution(atom.index(), OccurrenceSourceRefusal::UnknownOccurrence)
            })
        };
        let guard = self
            .package
            .project_guard(self.key)
            .ok()
            .flatten()
            .and_then(|term| {
                term.projection()
                    .location(term.projection().root())
                    .cloned()
            });
        let Some(verdict) =
            super::window_verdict(evaluation.verdict, &names, guard, evaluation.undefined_atom)?
        else {
            return Ok(WindowRun::stopped(ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal",
                    "named-window-evaluation-retains-its-attribution",
                ),
            )));
        };
        let key = fsl_foundation::node_wire::node_key_to_text(self.key)
            .map_err(|_| self.attribution(0, OccurrenceSourceRefusal::UnknownNode))?;
        let mut report = super::finite_window_report_named(
            (
                evaluation.basis,
                evaluation.support,
                evaluation.premises,
                evaluation.identity,
            ),
            super::super::correction::EvidenceKey::new(
                self.package.package().name().to_owned(),
                key,
                self.claim.origin().role().as_str().to_owned(),
                self.claim.origin().ordinal(),
            ),
            &names,
        )?;
        report.named_atom_region = named_atom.and_then(|atom| regions.get(&atom).cloned());
        report.atom_regions = regions;
        Ok(WindowRun {
            disposition: ClauseDisposition::Temporal(verdict),
            visits: evaluation.visits,
            report: Some(report),
            undefined,
            refused_at: None,
        })
    }
}

pub(super) fn screen(
    clause: &CheckedTemporalClause,
    trace: &TraceInput,
) -> Result<Option<super::super::late_data::Screened>, Box<ClauseDisposition>> {
    let TraceInput::Timestamped {
        observations,
        progress,
        window,
    } = trace
    else {
        return Ok(None);
    };
    if observations.is_empty()
        || window.missing_valuations.iter().any(|missing| {
            usize::try_from(missing.position)
                .ok()
                .is_none_or(|position| position >= observations.len())
                || usize::try_from(missing.atom).ok().is_none_or(|atom| {
                    !matches!(
                        clause.formula().nodes().get(atom),
                        Some(FormulaNode::Holds { .. } | FormulaNode::Aggregate { .. })
                    )
                })
        })
    {
        return Err(Box::new(super::refused("invalid-value", BTreeMap::new())));
    }
    super::super::late_data::admit_intervals(observations, &window.late)
        .map_err(|refusal| Box::new(*refusal))?;
    super::super::late_data::screen(observations, *progress, window)
        .map_err(|refusal| Box::new(*refusal))
}

pub(super) fn progressed(
    clause: &CheckedTemporalClause,
    trace: &TraceInput,
    meter: &mut Meter,
) -> Result<Option<TimestampedTrace>, Box<ClauseDisposition>> {
    let TraceInput::Progressed {
        trace,
        progress,
        lower_boundary,
        ..
    } = trace
    else {
        return Ok(None);
    };
    let assertion = fsl_eval::temporal::SampleProgress {
        watermark: progress.watermark,
        completeness: progress.completeness.into(),
        scope: progress.scope.into(),
        execution: progress.execution.into(),
        lower_boundary: match lower_boundary {
            super::ProgressLowerBoundary::AuthoritativeOrigin => {
                fsl_eval::temporal::LowerBoundary::AuthoritativeOrigin
            }
            super::ProgressLowerBoundary::HistoryCutoff => {
                fsl_eval::temporal::LowerBoundary::HistoryCutoff
            }
        },
    };
    match TimestampedTrace::admit_progressed(
        &fsl_eval::temporal::evidence::ClauseFacts::of_checked(clause),
        trace.iter().map(super::observation_identity).collect(),
        assertion,
        meter,
    ) {
        Ok(Ok(trace)) => Ok(Some(trace)),
        Ok(Err(fsl_eval::temporal::ProgressedRefusal::Incomplete(incomplete))) => Err(Box::new(
            ClauseDisposition::Temporal(TemporalVerdict::Incomplete(incomplete)),
        )),
        Ok(Err(refusal)) => Err(Box::new(super::refused(
            "invalid-value",
            BTreeMap::from([(
                "member",
                match refusal {
                    fsl_eval::temporal::ProgressedRefusal::WatermarkBelowLastPosition => {
                        "watermark"
                    }
                    fsl_eval::temporal::ProgressedRefusal::CompletenessBelowLastPosition => {
                        "completeness"
                    }
                    fsl_eval::temporal::ProgressedRefusal::NotSampleProfile { .. } => "profile",
                    fsl_eval::temporal::ProgressedRefusal::Incomplete(incomplete) => {
                        return Err(Box::new(ClauseDisposition::Temporal(
                            TemporalVerdict::Incomplete(incomplete),
                        )));
                    }
                }
                .to_owned(),
            )]),
        ))),
        Err(refusal) => Err(Box::new(ClauseDisposition::TemporalRefusal(
            super::trace_refusal(refusal),
        ))),
    }
}

/// Settle the existing window evaluator's report and shared progress ledger.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    context: &Context<'_, '_>,
    positions: PackagePositions<'_>,
    trace: &TraceInput,
    screened: Option<&super::super::late_data::Screened>,
    progressed: Option<&TimestampedTrace>,
    ledger: Option<&super::ProgressLedger>,
    capacity: u64,
    admitted: fsl_semantics::model::observation::AdmissionUsage,
    meter: &mut Meter,
) -> Result<super::ClauseRunReport, PackageSelectionNotRead> {
    let classes = context.classes()?;
    let mut ledger_positions = 0;
    let mut accepted_ledger = None;
    let fresh = super::ProgressLedger::new();
    if let Some(ledger) = ledger.or_else(|| (!context.sources.is_empty()).then_some(&fresh)) {
        let Some((assertion, positions)) = trace.progress() else {
            return Ok(*super::disposition_report(
                super::refused("invalid-value", BTreeMap::new()),
                super::ClauseRunUsage::from_meter(meter, admitted),
            ));
        };
        ledger_positions = positions.len();
        let binding = super::LedgerBinding {
            package: context.declaration.source().identity().to_owned(),
            clause: context.key.clone(),
            occurrence: context.claim.origin().ordinal(),
            clock: context.clause.clock().to_owned(),
            profile: context.clause.profile().identity().to_owned(),
        };
        match super::admit_progress(ledger, &binding, assertion, &positions, capacity) {
            Ok(updated) => accepted_ledger = Some(updated),
            Err(disposition) => {
                let mut report = super::disposition_report(
                    *disposition,
                    super::ClauseRunUsage::from_meter(meter, admitted),
                );
                report.ledger = Some(ledger.clone());
                return Ok(*report);
            }
        }
    }
    let mut trace_positions = 0;
    let mut window = if let Some(progressed) = progressed {
        trace_positions = trace.listed();
        context.evaluate(
            positions,
            progressed,
            &[],
            Some(u64::try_from(progressed.observed()).unwrap_or(u64::MAX)),
            &classes,
            meter,
        )?
    } else if let TraceInput::Timestamped {
        observations,
        progress,
        window,
    } = trace
    {
        super::run_finite_window_core(
            (context.clause, context.key),
            (
                &super::Timestamped {
                    observations,
                    progress: *progress,
                    window,
                },
                screened,
            ),
            &classes,
            None,
            &mut trace_positions,
            meter,
            |trace, absent, meter| {
                context.evaluate(positions, trace, absent, None, &classes, meter)
            },
        )?
    } else {
        return Err(PackageSelectionNotRead::Selection {
            member: "clause.selection.trace",
            reason: "the window route reads a progressed or timestamped trace".to_owned(),
        });
    };
    let (mut disposition, mut counterexample) =
        super::with_counterexample(context.clause, trace, window.disposition, window.undefined);
    let mut decisive = None;
    if progressed.is_some()
        && matches!(
            disposition,
            ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
        )
    {
        if let PackagePositions::Objects { environments, over } = positions {
            let shape = trace
                .shape()
                .ok_or_else(|| PackageSelectionNotRead::Selection {
                    member: "clause.selection.trace",
                    reason: "the progressed trace has a finite shape".to_owned(),
                })?;
            let observed = fsl_eval::temporal::evaluate_package_positions_decisive(
                context.evaluator,
                context.clause,
                context.key,
                positions,
                shape,
                meter,
            );
            let search = super::progressed_search(observed);
            (disposition, counterexample, decisive) = super::with_decisive_by(
                Some(search),
                disposition,
                counterexample,
                |atom, meter| {
                    let slot = usize::try_from(atom.position.index())
                        .ok()
                        .and_then(|at| super::observation_slot(trace, at));
                    let Some((objects, document)) = slot
                        .and_then(|slot| environments.get(slot).zip(trace.documents().nth(slot)))
                    else {
                        return Ok(None);
                    };
                    super::package_selection::decisive_over(
                        context.package,
                        context.clause,
                        context.key,
                        atom,
                        over,
                        objects,
                        fsl_eval::value::ObservationIdentity {
                            authority: document.authority.clone(),
                            identity: document.identity.clone(),
                        },
                        meter,
                    )
                },
                meter,
            );
        }
    }
    let accepted = !matches!(
        disposition.category(),
        filament_semantic_value::catalog::Category::Refusal
            | filament_semantic_value::catalog::Category::InternalFailure
    );
    let mut usage = super::ClauseRunUsage::from_meter(meter, admitted);
    usage.temporal_visits = window.visits;
    usage.trace_positions = u64::try_from(trace_positions).unwrap_or(u64::MAX);
    usage.ledger_positions = u64::try_from(ledger_positions).unwrap_or(u64::MAX);
    let mut report = super::disposition_report(disposition, usage);
    report.finite_window = window.report.take();
    report.counterexample = counterexample.map(Into::into);
    report.decisive = decisive;
    report.refused_at = window.refused_at;
    report.ledger = if accepted {
        accepted_ledger.or_else(|| ledger.cloned())
    } else {
        ledger.cloned()
    };
    Ok(*report)
}
