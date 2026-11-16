// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::BTreeMap;

use filament_exact::{Meter, Value, ValueType};
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::temporal::{
    evaluate_observed_decisive, evaluate_sampled, DecisiveAtom, DecisiveSearch, DecisiveStop,
    FirstUndefined, HoldsStop, ObservedPositions, PositionRefusal, ProgressedRefusal,
    TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace, TimestampedTrace,
    UndefinedCause, UndefinedEvaluation, WindowRefusal,
};
use fsl_foundation::diagnostic::Code;
use fsl_foundation::source::Source;
use fsl_semantics::check::{CheckedGraph, FormulaNode, TemporalProfile};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, AdmissionRecord,
    AdmissionUsage, DocumentProvision, DocumentRef, Provisions,
};

use super::progress::{
    ContradictedDimension, LedgerBinding, ObservedPosition, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressContradiction, ProgressLedger, ProgressLowerBoundary,
    ProgressValue, Refusal,
};
use super::trace_refusal::ClauseSite;
use super::{
    convert_call_failure, convert_outcome, ClauseDisposition, ClauseRunReport, ClauseRunUsage,
    CompiledRun, ConsequentEventRefusal, ConsequentSourceEvents, RunRefusal,
    TemporalIndeterminateReason, TemporalPositionRefusal, TemporalTraceRefusal,
    TemporalUndefinedCause, TemporalVerdict, TemporalWindowRefusal,
};
use crate::proof_result::SettlementBasis;

/// FR-330 Inputs: the observations a temporal clause is run over, each a
/// snapshot named by its labels (FR-106).
#[derive(Clone, Debug)]
pub enum TraceInput {
    /// A finite trace: a closed trace under a bounded profile, a finite
    /// prefix under the infinite-trace profile.
    Finite(Vec<DocumentRef>),
    /// A finite event-position or fixed-sample trace with progress (FR-504).
    Progressed {
        /// The admitted observations.
        trace: Vec<DocumentRef>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
        /// Whether the first position is an authoritative origin.
        lower_boundary: ProgressLowerBoundary,
        /// The profile, clock binding name and clock parameters the trace
        /// asserts (FR-522), compared with the clause's before any position
        /// is visited.
        binding: TraceBinding,
    },
    /// A timestamped-event trace with progress (FR-501).
    Timestamped {
        /// The observations, in any order: the evaluator reads them in
        /// canonical order (FR-501 "Canonical order") and never consults the
        /// listing.
        observations: Vec<TimestampedObservation>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
        /// The rest of the finite-window input (FR-330, FR-501 Inputs).
        window: Box<TimestampedWindow>,
    },
    /// A lasso: `prefix`, then `repeat` forever. Its last position re-enters
    /// the first position of `repeat`.
    Lasso {
        /// The positions before the loop.
        prefix: Vec<DocumentRef>,
        /// The positions of the loop; the loop is empty only in a malformed
        /// lasso, which the evaluator refuses.
        repeat: Vec<DocumentRef>,
    },
    /// A model-arm behaviour (FR-641 `model`, FSpec FR-364): an initial state,
    /// the universes, and the steps of its prefix and loop; FR-330 reconstructs
    /// its positions as FR-329 states for `Lasso::Model`.
    Model {
        /// The initial-state snapshot.
        initial: DocumentRef,
        /// The snapshots whose complete populations are the universes.
        universes: Vec<DocumentRef>,
        /// The steps before the loop.
        prefix: Vec<crate::ModelStep>,
        /// The steps of the loop, a terminal stutter step excluded.
        repeat: Vec<crate::ModelStep>,
        /// Whether the loop is the one terminal stutter step.
        terminal_stutter: bool,
        /// FR-641, FR-255 `expansion.max_candidates`: the most candidates one
        /// step reconstruction expands before the run stops `incomplete`.
        max_candidates: u64,
    },
}

impl TraceInput {
    /// The wire member that names this trace's kind (FR-641 "Trace kinds"):
    /// the kind FR-645 reports as `supplied`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Finite(_) => "finite",
            Self::Progressed { .. } => "progressed",
            Self::Timestamped { .. } => "timestamped",
            Self::Lasso { .. } => "lasso",
            Self::Model { .. } => "model",
        }
    }

    /// FR-645: the member path, from `clause`, of the part of a request-file
    /// trace the refusal `reason` is about.
    pub fn request_pointer(&self, reason: &str) -> String {
        let kind = self.kind();
        match (reason, self) {
            ("ledger-without-progress", _) => "selection.temporal.ledger".to_owned(),
            ("empty-lasso-loop", Self::Model { .. }) => {
                "selection.temporal.trace.model.loop".to_owned()
            }
            ("empty-lasso-loop", _) => "selection.temporal.trace.lasso.loop".to_owned(),
            ("empty-finite-trace", Self::Progressed { .. }) => {
                "selection.temporal.trace.progressed.positions".to_owned()
            }
            _ => format!("selection.temporal.trace.{kind}"),
        }
    }

    /// Whether the trace carries a progress assertion a ledger can retain
    /// (FR-502 Inputs). A `Finite` trace and a `Lasso` carry none: a lasso is
    /// the whole infinite behavior and a finite trace is its own closure.
    fn carries_progress(&self) -> bool {
        match self {
            Self::Progressed { .. } | Self::Timestamped { .. } => true,
            Self::Finite(_) | Self::Lasso { .. } | Self::Model { .. } => false,
        }
    }

    /// The progress assertion and position identities carried by this trace.
    fn progress(&self) -> Option<(ProgressAssertion, Vec<ObservedPosition>)> {
        match self {
            Self::Progressed {
                trace, progress, ..
            } => {
                let positions = trace
                    .iter()
                    .enumerate()
                    .map(|(index, document)| {
                        Some(ObservedPosition {
                            identity: document_identity(document),
                            stamp: u64::try_from(index).ok()?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some((*progress, positions))
            }
            Self::Timestamped {
                observations,
                progress,
                ..
            } => Some((
                *progress,
                observations
                    .iter()
                    .map(|observation| ObservedPosition {
                        identity: document_identity(&observation.document),
                        stamp: observation.stamp,
                    })
                    .collect(),
            )),
            Self::Finite(_) | Self::Lasso { .. } | Self::Model { .. } => None,
        }
    }

    /// Every observation, in position order.
    pub fn documents(&self) -> Box<dyn Iterator<Item = &DocumentRef> + '_> {
        match self {
            Self::Finite(positions)
            | Self::Progressed {
                trace: positions, ..
            } => Box::new(positions.iter()),
            Self::Timestamped {
                observations,
                progress,
                window,
            } => Box::new(
                ordered_timestamped_positions(&Timestamped {
                    observations,
                    progress: *progress,
                    window,
                })
                .into_iter()
                .map(|observation| &observation.document),
            ),
            Self::Lasso { prefix, repeat } => Box::new(prefix.iter().chain(repeat)),
            Self::Model {
                initial, universes, ..
            } => Box::new(std::iter::once(initial).chain(universes)),
        }
    }

    /// The trace's shape, or `None` when its length exceeds `u64`.
    fn shape(&self) -> Option<TemporalTrace> {
        match self {
            Self::Finite(positions)
            | Self::Progressed {
                trace: positions, ..
            } => Some(TemporalTrace::Finite {
                len: u64::try_from(positions.len()).ok()?,
            }),
            Self::Timestamped { observations, .. } => Some(TemporalTrace::Finite {
                len: u64::try_from(observations.len()).ok()?,
            }),
            Self::Lasso { prefix, repeat } => Some(TemporalTrace::Lasso {
                prefix: u64::try_from(prefix.len()).ok()?,
                loop_len: u64::try_from(repeat.len()).ok()?,
            }),
            // A model behaviour is reconstructed from its steps, not read as
            // observations.
            Self::Model { .. } => None,
        }
    }
}

/// FR-641: the per-expansion candidate bound of a `model` trace that names
/// none.
pub const DEFAULT_MODEL_MAX_CANDIDATES: u64 = 1_000_000;

/// The authority label of a position that carries a record sample.
const SAMPLE_AUTHORITY: &str = "record-sample";

/// FR-641: a record sample as one position of a
/// [`ClauseRunSelection::SampledTemporal`](super::ClauseRunSelection)
/// trace. `canonical` is the sample's canonical JSON object text; it is the
/// position's identity, so the progress ledger and the canonical order of a
/// timestamped trace read a sample as they read a snapshot's labels.
pub fn sample_position(canonical: &str) -> DocumentRef {
    DocumentRef {
        authority: SAMPLE_AUTHORITY.to_owned(),
        identity: canonical.to_owned(),
    }
}

fn document_identity(document: &DocumentRef) -> String {
    format!(
        "{}:{}{}:{}",
        document.authority.len(),
        document.authority,
        document.identity.len(),
        document.identity,
    )
}

/// FR-522 `TraceBinding`: what a trace of tick-valued quantities asserts
/// about the clause it is run under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBinding {
    /// The profile identity the trace asserts.
    pub profile: String,
    /// The clock binding name the trace asserts.
    pub clock_name: String,
    /// The clock configuration the trace asserts.
    pub parameters: fsl_package::ClockConfiguration,
}

/// One timestamped atom valuation, absent from its observation, named by
/// position and atom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampedMissingValuation {
    /// Zero-based trace position.
    pub position: u64,
    /// Zero-based checked formula node of the `holds` atom.
    pub atom: u64,
}

/// An admitted order key comparable only within one authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampedOrderKey {
    /// Identity of the authority that assigned the key.
    pub authority: String,
    /// Key within that authority's ordered domain.
    pub key: i64,
}

/// FR-330 Inputs: the finite-window input beyond its observations and
/// progress.
#[derive(Clone, Debug)]
pub struct TimestampedWindow {
    /// The profile, clock binding name and clock parameters the trace asserts
    /// (FR-522), required: a trace with no binding is refused at stage
    /// `admit` (FR-330-AC-11).
    pub binding: Option<TraceBinding>,
    /// Whether the first represented position is an authoritative origin
    /// (the lower-boundary kind).
    pub authoritative_origin: bool,
    /// The cover the trace declares for its stamps (FR-501 Inputs); `None` is
    /// `Exact`. The checked clause's clock configuration names the timestamp
    /// unit only, so the cover has no other source.
    pub cover: Option<TimestampedCover>,
    /// Atom valuations absent from an admitted observation (FSpec FR-160):
    /// each is a position, by its index in `observations`, and the zero-based
    /// `holds` formula node of the checked clause.
    pub missing_valuations: Vec<TimestampedMissingValuation>,
}

/// The finite-window input as the evaluator reads it.
struct Timestamped<'a> {
    observations: &'a [TimestampedObservation],
    progress: ProgressAssertion,
    window: &'a TimestampedWindow,
}

/// One timestamped observation in a progress-carrying trace.
#[derive(Clone, Debug)]
pub struct TimestampedObservation {
    /// The admitted snapshot identity.
    pub document: DocumentRef,
    /// The non-negative tick stamp.
    pub stamp: u64,
    /// The order key, when the observation has one: an order-authority
    /// identity and a key within it.
    pub order_key: Option<TimestampedOrderKey>,
}

/// The identity of an observation's document for ordering (FR-106): its two
/// labels joined by NUL, which orders as the labels do field by field.
pub(super) fn observation_identity(document: &DocumentRef) -> String {
    format!("{}\0{}", document.authority, document.identity)
}

/// FR-501 "Canonical order": ascending stamp, then ascending order-authority
/// identity (an observation without an order key after every keyed one), then
/// ascending key within the authority, then ascending observation identity.
/// A total order over distinct observations: the listing order decides
/// nothing.
fn ordered_timestamped_positions<'a>(input: &Timestamped<'a>) -> Vec<&'a TimestampedObservation> {
    let mut positions: Vec<_> = input.observations.iter().collect();
    positions.sort_by(|left, right| {
        fn order(position: &TimestampedObservation) -> Option<(&str, i64)> {
            position
                .order_key
                .as_ref()
                .map(|order| (order.authority.as_str(), order.key))
        }
        left.stamp
            .cmp(&right.stamp)
            .then_with(|| match (order(left), order(right)) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left.document.cmp(&right.document))
    });
    positions
}

/// FR-330 Inputs: the `{population, key}` object the clause's `over`
/// parameter names, resolved in the observation at position 0.
#[derive(Clone, Debug)]
pub struct OverObject {
    /// The population identity.
    pub population: String,
    /// The object's declared key.
    pub key: String,
}

/// A progress contradiction as the spine reports it (FR-330, FR-502 Outputs):
/// the typed contradiction and the `invalid_runtime_input`/`invalid-value`
/// record that carries `contradicted_dimension`, `clock`, `retained` and
/// `asserted` as fields.
#[derive(Clone, Debug)]
pub struct ProgressRefusal {
    /// The contradiction.
    pub contradiction: ProgressContradiction,
    /// Its record.
    pub record: AdmissionRecord,
}

/// One value of a contradiction as a record field.
fn progress_field(value: &ProgressValue) -> String {
    let closure = |closure: &ProgressClosure| match closure {
        ProgressClosure::Closed => "Closed",
        ProgressClosure::Open => "Open",
    };
    match value {
        ProgressValue::Watermark(watermark) => watermark.to_string(),
        ProgressValue::Completeness(ProgressCompleteness::Complete { through }) => {
            format!("Complete{{through: {through}}}")
        }
        ProgressValue::Completeness(ProgressCompleteness::Incomplete) => "Incomplete".to_owned(),
        ProgressValue::DecisionScope(value) => format!("scope {}", closure(value)),
        ProgressValue::SurroundingExecution(value) => format!("execution {}", closure(value)),
        ProgressValue::Position(position) => {
            format!("position {} stamped {}", position.identity, position.stamp)
        }
    }
}

impl From<ProgressContradiction> for ProgressRefusal {
    fn from(contradiction: ProgressContradiction) -> Self {
        let dimension = match contradiction.dimension {
            ContradictedDimension::Watermark => "Watermark",
            ContradictedDimension::Completeness => "Completeness",
            ContradictedDimension::Closure => "Closure",
        };
        let fields = BTreeMap::from([
            ("contradicted_dimension", dimension.to_owned()),
            ("clock", contradiction.clock.clone()),
            ("retained", progress_field(&contradiction.retained)),
            ("asserted", progress_field(&contradiction.asserted)),
        ]);
        let record = AdmissionRecord {
            catalog: CatalogCode::new(Code::InvalidRuntimeInput.as_str(), "invalid-value"),
            fields,
            path: Vec::new(),
            kernel: None,
        };
        Self {
            contradiction,
            record,
        }
    }
}

/// FR-502: admit one request's progress into its ledger. FR-501 and FR-504
/// call this once per accepted request, with the binding the request runs
/// under, the progress its trace asserts, the identities and stamps of its
/// positions and the request's `progress.covered_positions` setting.
///
/// # Errors
///
/// The disposition the request reports: `invalid_runtime_input`/
/// `invalid-value` carrying the contradiction, or an `Incomplete` verdict when
/// the covered positions outgrew the setting. The ledger is unchanged either
/// way.
pub fn admit_progress(
    ledger: &ProgressLedger,
    binding: &LedgerBinding,
    assertion: ProgressAssertion,
    positions: &[ObservedPosition],
    covered_positions: u64,
) -> Result<ProgressLedger, Box<ClauseDisposition>> {
    ledger
        .admit(binding, assertion, positions, covered_positions)
        .map_err(|refusal| match refusal {
            Refusal::Contradiction(contradiction) => Box::new(
                ClauseDisposition::ProgressContradiction(Box::new(contradiction.into())),
            ),
            Refusal::LimitReached(reached) => Box::new(ClauseDisposition::Temporal(
                TemporalVerdict::ProgressLimitReached(reached),
            )),
        })
}

/// FR-330 Inputs: one `Temporal` selection.
pub(super) struct TemporalRun<'a> {
    /// The clause's declared name.
    pub(super) name: &'a str,
    /// The supplied trace.
    pub(super) trace: &'a TraceInput,
    /// The clause's `over` binding.
    pub(super) over: Option<&'a OverObject>,
    /// The caller's ledger, when it carries one.
    pub(super) ledger: Option<&'a ProgressLedger>,
    /// What FR-522 authenticates a clock-bearing trace against; `None` for a
    /// run that holds no emission, which accepts only a `Finite` or `Lasso`
    /// trace.
    pub(super) authentication: Option<TemporalAuthentication<'a>>,
    /// The caller's handle: every charge of the run's meter polls it.
    pub(super) cancel: Option<&'a filament_exact::Cancel>,
}

/// The emitted package a clock-bearing trace is authenticated against.
#[derive(Clone, Copy)]
pub struct TemporalAuthentication<'a> {
    /// The compiled package's `temporal_bindings` rows, keyed by clause
    /// identity (FR-522 "Authentication").
    pub clock_rows: &'a fsl_package::TemporalBindingRows,
    /// The emitted bytes after strict checked-package admission.
    pub admitted_package: &'a filament_contract_model::CheckedPackageV2,
}

/// FR-522 "Authentication": compare a `TraceBinding` against the profile and
/// clock rows recovered from the strictly admitted package. A mismatch is
/// refused at stage `admit`, `invalid_runtime_input`/`invalid-value`, with the
/// first differing dimension and asserted/expected values. The clock row is
/// the one keyed by the declaration's `occurrence`. The checked
/// declaration identity comes from emission's exact `(NodeKey, occurrence)`
/// mapping; the profile and clock are read from that declaration's admitted
/// temporal subject.
fn authenticate_binding(
    binding: Option<&TraceBinding>,
    subject: &fsl_package::checked_views::TemporalSubjectView<'_>,
    occurrence: u64,
) -> Result<crate::spine::AuthenticatedTemporalSelection, Box<ClauseDisposition>> {
    let Some(binding) = binding else {
        return Err(Box::new(refused("invalid-value", BTreeMap::new())));
    };
    let binding = fsl_eval::temporal::evidence::TraceBinding {
        profile: binding.profile.clone().into_boxed_str(),
        clock_name: binding.clock_name.clone().into_boxed_str(),
        parameters: binding.parameters.clone(),
    };
    let authenticated = binding
        .authenticate(&fsl_eval::temporal::evidence::ClauseSelection {
            declaration: subject.declaration(),
            profile: subject.profile(),
            // The clock row is the one keyed by this declaration's occurrence:
            // a node several clauses share has a row per occurrence.
            clock: subject
                .occurrences()
                .iter()
                .find(|found| found.ordinal == occurrence)
                .and_then(|found| subject.clock_of(found)),
        })
        .map_err(|mismatch| {
            let mut fields = BTreeMap::from([
                ("mismatched_dimension", mismatch.dimension.name().to_owned()),
                ("asserted", mismatch.asserted),
            ]);
            if let Some(expected) = mismatch.expected {
                fields.insert("expected", expected);
            }
            Box::new(refused("invalid-value", fields))
        })?;
    Ok(crate::spine::AuthenticatedTemporalSelection {
        declaration: authenticated.declaration,
        source_regions: subject.source_span().regions.clone(),
        profile: authenticated.profile,
        clock_name: authenticated.clock_name,
        clock: authenticated.clock,
    })
}

pub(super) fn refused(
    cause: &'static str,
    fields: BTreeMap<&'static str, String>,
) -> ClauseDisposition {
    ClauseDisposition::Admit(AdmissionFailure::Refused(Box::new(AdmissionRecord {
        catalog: CatalogCode::new(Code::InvalidRuntimeInput.as_str(), cause),
        fields,
        path: Vec::new(),
        kernel: None,
    })))
}

/// FR-330 Inputs and FR-522: the profile `node`'s clause reads `trace`
/// under, and the authentication of a clock-bearing trace's binding against
/// the emitted rows. The authentication precedes every position of the trace
/// (FR-330-AC-11). The selection is `None` for a `Finite` or `Lasso` trace.
fn authenticate_trace(
    site: ClauseSite<'_>,
    trace: &TraceInput,
    authentication: Option<TemporalAuthentication<'_>>,
) -> Result<Option<crate::spine::AuthenticatedTemporalSelection>, Box<ClauseDisposition>> {
    let node = site.node;
    let clause = node.clause();
    // FR-645: a trace kind the clause's profile does not read refuses at stage
    // `admit` before any position is admitted.
    if !super::trace_refusal::profile_reads(clause.profile()).contains(&trace.kind()) {
        return Err(Box::new(ClauseDisposition::TraceRefused(Box::new(
            site.kind_mismatch(trace.kind()),
        ))));
    }
    let binding = match trace {
        TraceInput::Progressed { binding, .. } => Some(binding),
        TraceInput::Timestamped { window, .. } => window.binding.as_ref(),
        TraceInput::Finite(_) | TraceInput::Lasso { .. } | TraceInput::Model { .. } => {
            return Ok(None)
        }
    };
    // A caller that holds no emission asserts none (FR-820: the decided reading
    // of a `Finite` trace needs no binding), so a clock-bearing trace is read
    // unauthenticated. With rows, the temporal subject is read to authenticate
    // the trace's binding.
    let Some(TemporalAuthentication {
        clock_rows,
        admitted_package,
    }) = authentication
    else {
        return Ok(None);
    };
    let fault = |what: &'static str| {
        Box::new(ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new("FR-050", what),
        ))
    };
    let Some(declaration) = clock_rows.declaration(&node.identity(), node.claim().ordinal()) else {
        return Err(fault("emitted-temporal-declaration-retains-checked-id"));
    };
    let subject = fsl_package::checked_views::temporal_subject(admitted_package, declaration)
        .map_err(|_| fault("admitted-temporal-subject-resolves"))?;
    authenticate_binding(binding, &subject, node.claim().ordinal()).map(Some)
}

/// FR-283: the profile fit and FR-522 authentication of `trace` for the
/// temporal clause `name` of `package`, before any of its positions is
/// admitted. A clause `package` does not declare, and a `Finite` or `Lasso`
/// trace, authenticate nothing.
/// `authentication` is `None` for a caller that holds no emission
///
/// # Errors
///
/// The `admit`-stage disposition [`run_package_temporal`] would report for
/// the same request.
pub fn authenticate_package_trace(
    package: &fsl_package::CheckedPackage,
    name: &str,
    trace: &TraceInput,
    authentication: Option<TemporalAuthentication<'_>>,
) -> Result<(), Box<ClauseDisposition>> {
    let Some(node) = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
    else {
        return Ok(());
    };
    let site = ClauseSite { package, node };
    authenticate_trace(site, trace, authentication).map(drop)
}

/// The `TraceBinding` a trace of the clause `name` asserts to agree with the
/// emitted package: its profile, clock name and clock configuration, read from
/// the admitted `temporal_bindings` row. `None` for a clause the package does
/// not declare or that has no clock row.
pub fn clause_trace_binding(
    package: &fsl_package::CheckedPackage,
    name: &str,
    authentication: TemporalAuthentication<'_>,
) -> Option<TraceBinding> {
    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)?;
    let declaration = authentication
        .clock_rows
        .declaration(&node.identity(), node.claim().ordinal())?;
    let subject =
        fsl_package::checked_views::temporal_subject(authentication.admitted_package, declaration)
            .ok()?;
    let occurrence = node.claim().ordinal();
    let row = subject
        .occurrences()
        .iter()
        .find(|found| found.ordinal == occurrence)
        .and_then(|found| subject.clock_of(found))?;
    Some(TraceBinding {
        profile: subject.profile().definition.identity.to_string(),
        clock_name: row.clock_name.to_string(),
        parameters: row.clock.clone(),
    })
}

/// FR-283: one temporal clause run over a checked package, with no source to
/// compile: the caller supplies the sources the checked package was built
/// from and, for a clock-bearing trace, the emission it authenticates
/// against.
pub struct PackageTemporalRequest<'a> {
    /// The checked package the clause is declared in.
    pub package: &'a fsl_package::CheckedPackage,
    /// FR-056's package input, read by admission.
    pub packages: &'a fsl_semantics::model::intake::PackageInput,
    /// The limits admission re-normalizes the domain packages under.
    pub model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    /// The snapshot documents the trace's positions name.
    pub snapshots: &'a fsl_semantics::model::observation::DocumentProvision,
    /// FR-106's observation limits.
    pub observation_limits: fsl_semantics::model::observation::ObservationLimits,
    /// The evaluation meter's accounting limits.
    pub accounting: filament_exact::ScalarLimits,
    /// The temporal clause's declared name.
    pub name: &'a str,
    /// The supplied trace.
    pub trace: &'a TraceInput,
    /// The object the clause's `over` parameter names; `None` for a clause
    /// over a declared record trace subject, whose positions are record
    /// samples (FR-641).
    pub over: Option<&'a OverObject>,
    /// The progress ledger carried into the run (FR-502); `None` starts a
    /// fresh one for a trace that asserts progress.
    pub ledger: Option<&'a ProgressLedger>,
    /// The unit's and each library's source the package was checked from;
    /// a locus and an atom name are rendered from them.
    pub sources: &'a [Source],
    /// What FR-522 authenticates a `Progressed` or `Timestamped` trace
    /// against; `None` for a request that holds no emission, which reads a
    /// `Progressed` or `Timestamped` trace unauthenticated (FR-820), with no
    /// `authenticated_temporal_selection` in its report.
    pub authentication: Option<TemporalAuthentication<'a>>,
    /// The caller's handle, polled at every charge of the run's meter.
    pub cancel: &'a filament_exact::Cancel,
}

/// FR-283: run one temporal clause over a supplied finite or lasso trace
/// through the evaluator [`super::run_clause`]'s `Temporal` selection uses.
/// A progressed or timestamped trace without `authentication` is read
/// unauthenticated: its binding is not checked and its report carries no
/// authenticated selection.
pub fn run_package_temporal(request: &PackageTemporalRequest<'_>) -> ClauseRunReport {
    let provisions = Provisions {
        snapshots: request.snapshots,
        invocations: &DocumentProvision::default(),
    };
    let run = CompiledRun {
        packages: request.packages,
        model_limits: request.model_limits,
        provisions,
        observation_limits: request.observation_limits,
        accounting: request.accounting,
        package: request.package,
        sources: request.sources,
        // FR-516: the capacity is the system setting
        // `progress.covered_positions` (FR-255), not a request member, so the
        // package route takes its default.
        progress_capacity: fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
    };
    run_temporal(
        &run,
        &TemporalRun {
            name: request.name,
            trace: request.trace,
            over: request.over,
            ledger: request.ledger,
            authentication: request.authentication,
            cancel: Some(request.cancel),
        },
    )
}

/// FR-330: run the temporal clause `name` over `trace`.
pub(super) fn run_temporal(run: &CompiledRun<'_>, request: &TemporalRun<'_>) -> ClauseRunReport {
    let TemporalRun {
        name,
        trace,
        over,
        ledger,
        authentication,
        cancel,
    } = *request;
    // A trace is evidence for that trace only: it settles no item and
    // derives no witness (FR-332).
    let report = |disposition: ClauseDisposition,
                  usage: ClauseRunUsage,
                  authenticated_temporal_selection| ClauseRunReport {
        // FR-266's Temporal row.
        basis: match &disposition {
            ClauseDisposition::Temporal(verdict) => verdict.basis(),
            _ => SettlementBasis::Unavailable,
        },
        witness: None,
        // A completed temporal verdict names no consequent events: it
        // refuses typed rather than reporting an empty list.
        consequent_source_events: if disposition.truth().is_some() {
            ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
        } else {
            ConsequentSourceEvents::Unavailable
        },
        disposition,
        usage,
        // A refused request returns the caller's ledger as it was; a refused
        // request that supplied none writes none (FSpec FR-352).
        ledger: ledger.cloned(),
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection,
        refused_at: None,
    };
    let early = |disposition| report(disposition, ClauseRunUsage::default(), None);

    let graph = run.package.graph();
    let Some(node) = graph
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
    else {
        return early(ClauseDisposition::MissingName {
            name: name.to_owned(),
        });
    };
    let clause = node.clause();
    let site = ClauseSite {
        package: run.package,
        node,
    };
    let authenticated_temporal_selection = match authenticate_trace(site, trace, authentication) {
        Ok(selection) => selection,
        Err(disposition) => return early(*disposition),
    };

    if let TraceInput::Timestamped {
        observations,
        window,
        ..
    } = trace
    {
        // FR-645 keeps each refusal's existing stage: an empty timestamped
        // trace was refused at admit before the trace refusals were named.
        if observations.is_empty() {
            return early(refused("invalid-value", BTreeMap::new()));
        }
        if window.missing_valuations.iter().any(|missing| {
            usize::try_from(missing.position)
                .ok()
                .is_none_or(|position| position >= observations.len())
                || usize::try_from(missing.atom).ok().is_none_or(|atom| {
                    !matches!(
                        clause.formula().nodes().get(atom),
                        Some(FormulaNode::Holds { .. })
                    )
                })
        }) {
            return early(refused("invalid-value", BTreeMap::new()));
        }
    }

    // A ledger retains progress; a trace that asserts none has nothing to
    // retain (FR-502 Behavior).
    if ledger.is_some() && !trace.carries_progress() {
        return early(ClauseDisposition::TraceRefused(Box::new(
            site.ledger_without_progress(),
        )));
    }
    // FR-330: a model-arm behaviour is re-executed, not read as snapshots.
    if matches!(trace, TraceInput::Model { .. }) {
        let model = super::model_trace::run_model(run, node, trace, over);
        let mut finished = report(
            model.disposition,
            model.usage,
            authenticated_temporal_selection,
        );
        finished.decisive = model.decisive;
        return finished;
    }
    let Some(shape) = trace.shape() else {
        return early(refused("invalid-value", BTreeMap::new()));
    };

    // FR-645: a trace with no position, or a lasso with no loop position, has
    // no observation to evaluate over.
    let empty = match trace {
        TraceInput::Lasso { repeat, .. } if repeat.is_empty() => Some(site.empty_loop()),
        TraceInput::Finite(positions)
        | TraceInput::Progressed {
            trace: positions, ..
        } if positions.is_empty() => Some(site.empty_trace(trace.kind())),
        TraceInput::Finite(_)
        | TraceInput::Progressed { .. }
        | TraceInput::Timestamped { .. }
        | TraceInput::Lasso { .. }
        | TraceInput::Model { .. } => None,
    };
    if let Some(refusal) = empty {
        return early(ClauseDisposition::TraceRefused(Box::new(refusal)));
    }

    // FR-504 "Consistency": an assertion the trace contradicts is refused at
    // admit before any snapshot is read, and is never shortened.
    let mut meter = Meter::new(run.accounting);
    if let Some(cancel) = cancel {
        meter = meter.with_cancel(cancel.clone());
    }
    let progressed = match trace {
        TraceInput::Progressed {
            trace: documents,
            progress,
            lower_boundary,
            ..
        } => {
            let assertion = fsl_eval::temporal::SampleProgress {
                watermark: progress.watermark,
                completeness: progress.completeness.into(),
                scope: progress.scope.into(),
                execution: progress.execution.into(),
                lower_boundary: match lower_boundary {
                    ProgressLowerBoundary::AuthoritativeOrigin => {
                        fsl_eval::temporal::LowerBoundary::AuthoritativeOrigin
                    }
                    ProgressLowerBoundary::HistoryCutoff => {
                        fsl_eval::temporal::LowerBoundary::HistoryCutoff
                    }
                },
            };
            let identities = documents.iter().map(observation_identity).collect();
            match TimestampedTrace::admit_progressed(
                &fsl_eval::temporal::evidence::ClauseFacts::of_checked(clause),
                identities,
                assertion,
                &mut meter,
            ) {
                Ok(Ok(admitted)) => Some(admitted),
                Ok(Err(inconsistent)) => {
                    let member = match inconsistent {
                        ProgressedRefusal::WatermarkBelowLastPosition => "watermark",
                        ProgressedRefusal::CompletenessBelowLastPosition => "completeness",
                        ProgressedRefusal::NotSampleProfile { .. } => "profile",
                        ProgressedRefusal::Incomplete(incomplete) => {
                            return early(ClauseDisposition::Temporal(
                                TemporalVerdict::Incomplete(incomplete),
                            ));
                        }
                    };
                    return early(refused(
                        "invalid-value",
                        BTreeMap::from([("member", member.to_owned())]),
                    ));
                }
                Err(refusal) => {
                    return early(ClauseDisposition::TemporalRefusal(trace_refusal(refusal)))
                }
            }
        }
        TraceInput::Finite(_)
        | TraceInput::Timestamped { .. }
        | TraceInput::Lasso { .. }
        | TraceInput::Model { .. } => None,
    };

    let mut admitted = AdmissionUsage::default();
    let mut environments: Vec<ObjectEnvironment> = Vec::new();
    let mut samples: Vec<Value> = Vec::new();
    let named = node.named_populations();

    // FR-641: a clause over a declared record trace subject reads record
    // samples, a clause over a model object reads snapshots.
    let (_, over_type) = clause.over();
    let sampled = over.is_none();
    let fits = if sampled {
        matches!(over_type, ValueType::Composite(_))
    } else {
        matches!(over_type, ValueType::Reference(_))
    };
    if !fits {
        return early(refused(
            "invalid-value",
            BTreeMap::from([(
                "member",
                (if sampled { "trace" } else { "over" }).to_owned(),
            )]),
        ));
    }
    if let (true, ValueType::Composite(record)) = (sampled, over_type) {
        for (index, document) in trace.documents().enumerate() {
            match fsl_semantics::model::observation::admit_record_sample(
                graph.scope().types(),
                record,
                document.identity.as_bytes(),
            ) {
                Ok(sample) => samples.push(sample),
                Err(record) => {
                    let mut record = record;
                    record.fields.insert("position", index.to_string());
                    return report(
                        ClauseDisposition::Admit(AdmissionFailure::Refused(Box::new(record))),
                        ClauseRunUsage::default(),
                        authenticated_temporal_selection.clone(),
                    );
                }
            }
        }
    }
    for document in trace.documents().filter(|_| !sampled) {
        match admit_trace_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            run.packages,
            run.model_limits,
            run.provisions.snapshots,
            document,
            run.observation_limits,
            &named,
        ) {
            Ok((environment, usage)) => {
                admitted.document_bytes =
                    admitted.document_bytes.saturating_add(usage.document_bytes);
                admitted.objects = admitted.objects.saturating_add(usage.objects);
                admitted.values = admitted.values.saturating_add(usage.values);
                environments.push(environment);
            }
            Err(failure) => {
                return report(
                    ClauseDisposition::Admit(failure),
                    ClauseRunUsage {
                        admission_consumed: admitted,
                        ..ClauseRunUsage::default()
                    },
                    authenticated_temporal_selection.clone(),
                )
            }
        }
    }

    // From here on admission has run: every report keeps what it consumed.
    let late = |disposition| {
        report(
            disposition,
            ClauseRunUsage {
                admission_consumed: admitted,
                ..ClauseRunUsage::default()
            },
            authenticated_temporal_selection.clone(),
        )
    };

    // `over` resolves in the observation at position 0 (FR-330 Inputs).
    let observing = match over {
        None => ObservedPositions::Samples(&samples),
        Some(over) => {
            let ValueType::Reference(type_identity) = over_type else {
                return late(ClauseDisposition::EvaluateFault(
                    filament_semantic_value::catalog::InternalFault::new(
                        "temporal",
                        "over-is-a-reference",
                    ),
                ));
            };
            let universe = match population_universe_for(
                graph.model_inputs(),
                run.packages,
                run.model_limits,
                type_identity.clone(),
            ) {
                Ok(universe) => universe,
                Err(failure) => return late(ClauseDisposition::Admit(failure)),
            };
            let resolved = environments
                .first()
                .and_then(|environment| environment.objects().find(universe, &over.key));
            let Some(reference) = resolved else {
                return late(refused(
                    "wrong-role-mapping",
                    BTreeMap::from([
                        ("population", over.population.clone()),
                        ("object", over.key.clone()),
                    ]),
                ));
            };
            ObservedPositions::Objects {
                environments: &environments,
                over: reference,
            }
        }
    };
    // The snapshot positions a counterexample and a decisive record read; a
    // record sample has neither.
    let objects = match observing {
        ObservedPositions::Objects { environments, over } => Some((environments, over)),
        ObservedPositions::Samples(_) => None,
    };

    // QSL-2138: only the closed finite route below reads window aggregates.
    if sampled
        && !(matches!(trace, TraceInput::Finite(_))
            && clause.profile() == TemporalProfile::EventPositionFalseExtension)
        && clause
            .formula()
            .nodes()
            .iter()
            .any(|formula| matches!(formula, FormulaNode::Aggregate { .. }))
    {
        return early(ClauseDisposition::TraceRefused(Box::new(
            site.window_aggregate(trace.kind()),
        )));
    }

    // FR-330-AC-17: a closed finite trace of samples under the event-position
    // profile is the trace-subject route's, which also reads window
    // aggregates.
    if sampled
        && matches!(trace, TraceInput::Finite(_))
        && clause.profile() == TemporalProfile::EventPositionFalseExtension
    {
        let mut first_undefined = None;
        let mut delegated = super::trace_subject::run_subject_observing(
            run,
            name,
            over_type,
            &samples,
            fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
            cancel,
            &mut first_undefined,
        );
        // A violation, and an `Undefined` that is a violation, carries its
        // counterexample (FR-330 Outputs): the samples are its steps, as for
        // the other sample routes.
        let needs = match &delegated.disposition {
            ClauseDisposition::Temporal(TemporalVerdict::Violated { .. }) => true,
            ClauseDisposition::Temporal(TemporalVerdict::Undefined { cause, .. }) => {
                !matches!(cause, TemporalUndefinedCause::InsufficientData { .. })
            }
            _ => false,
        };
        if needs {
            match counterexample(node, trace, &delegated.disposition, first_undefined) {
                Some(built) => delegated.counterexample = Some(built),
                None => {
                    delegated.disposition = ClauseDisposition::EvaluateFault(
                        filament_semantic_value::catalog::InternalFault::new(
                            "temporal",
                            "violation-has-its-counterexample",
                        ),
                    );
                }
            }
        }
        return delegated;
    }

    // FR-352: a run over a trace that asserts progress returns its ledger,
    // the one it was given or a fresh one.
    let fresh = ProgressLedger::new();
    let mut accepted_ledger = None;
    // A fresh ledger needs the package's source identity to bind to; a run
    // with no sources has none, and its atom naming then refuses with
    // `SourceNotSupplied`, so the fresh admission is skipped.
    let fresh_ledger = trace.carries_progress() && !run.sources.is_empty();
    if let Some(ledger) = ledger.or_else(|| fresh_ledger.then_some(&fresh)) {
        let Some((assertion, positions)) = trace.progress() else {
            return early(refused("invalid-value", BTreeMap::new()));
        };
        let Some(package_identity) = run.sources.first().map(|source| source.identity().clone())
        else {
            return late(ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal-progress",
                    "missing-package-source-identity",
                ),
            ));
        };
        let binding = LedgerBinding {
            package: package_identity.identity.clone(),
            clause: node.identity(),
            occurrence: node.claim().ordinal(),
            clock: clause.clock().to_owned(),
            profile: clause.profile().identity().to_owned(),
        };
        accepted_ledger = match admit_progress(
            ledger,
            &binding,
            assertion,
            &positions,
            run.progress_capacity,
        ) {
            Ok(updated) => Some(updated),
            Err(disposition) => return late(*disposition),
        };
    }

    if let Some(window_trace) = progressed {
        let silent_from = u64::try_from(window_trace.observed()).ok();
        let window = run_window(
            run,
            node,
            &window_trace,
            (&[], silent_from),
            observing,
            &mut meter,
        );
        // A violation carries its counterexample (FR-330 Outputs) over record
        // samples as over snapshots: the samples are its steps.
        let (disposition, counterexample) =
            with_counterexample(node, trace, window.disposition, window.undefined);
        // FR-504: a progressed trace's violation reads its decisive atom
        // through the finite-trace evaluator, over the supplied trace. A
        // record sample has none.
        let violated = matches!(
            disposition,
            ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
        );
        let (disposition, counterexample, decisive) = match objects {
            Some((environments, reference)) => {
                let seeking = violated.then(|| {
                    progressed_search(evaluate_observed_decisive(
                        run.package,
                        node,
                        environments,
                        shape,
                        reference,
                        silent_from,
                        &mut meter,
                    ))
                });
                with_decisive(
                    &DecisiveRun {
                        run,
                        node,
                        trace,
                        environments,
                        over: reference,
                        silent_from,
                    },
                    seeking,
                    disposition,
                    counterexample,
                    &mut meter,
                )
            }
            None => (disposition, counterexample, None),
        };
        let accepted = !matches!(
            disposition.category(),
            filament_semantic_value::catalog::Category::Refusal
                | filament_semantic_value::catalog::Category::InternalFailure
        );
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = window.visits;
        let mut finished = report(disposition, usage, authenticated_temporal_selection.clone());
        finished.finite_window = window.report;
        finished.refused_at = window.refused_at;
        finished.counterexample = counterexample;
        finished.decisive = decisive;
        if accepted {
            finished.ledger = accepted_ledger.or_else(|| ledger.cloned());
        }
        return finished;
    }
    if let TraceInput::Timestamped {
        observations,
        progress,
        window,
    } = trace
    {
        let view = Timestamped {
            observations,
            progress: *progress,
            window,
        };
        let window = run_finite_window(run, node, &view, observing, &mut meter);
        // A violation carries its counterexample (FR-330 Outputs) over record
        // samples as over snapshots: the samples are its steps.
        let (disposition, counterexample) =
            with_counterexample(node, trace, window.disposition, window.undefined);
        let accepted = !matches!(
            disposition.category(),
            filament_semantic_value::catalog::Category::Refusal
                | filament_semantic_value::catalog::Category::InternalFailure
        );
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = window.visits;
        // FR-330: a `Timestamped` run carries no `decisive`; FR-501's
        // decision support is its record.
        let mut finished = report(disposition, usage, authenticated_temporal_selection.clone());
        finished.finite_window = window.report;
        finished.refused_at = window.refused_at;
        finished.counterexample = counterexample;
        if accepted {
            finished.ledger = accepted_ledger.or_else(|| ledger.cloned());
        }
        return finished;
    }
    let observed = match observing {
        ObservedPositions::Objects { environments, over } => evaluate_observed_decisive(
            run.package,
            node,
            environments,
            shape,
            over,
            None,
            &mut meter,
        ),
        ObservedPositions::Samples(samples) => {
            evaluate_sampled(run.package, node, samples, shape, None, &mut meter)
        }
    };
    let (disposition, refused_at) = observed_disposition(
        observed.stopped,
        observed.result,
        run.package.graph(),
        run.sources,
    );
    let (disposition, counterexample, decisive) = match objects {
        Some((environments, reference)) => {
            let (disposition, counterexample) =
                with_counterexample(node, trace, disposition, observed.undefined);
            with_decisive(
                &DecisiveRun {
                    run,
                    node,
                    trace,
                    environments,
                    over: reference,
                    silent_from: None,
                },
                observed.decisive,
                disposition,
                counterexample,
                &mut meter,
            )
        }
        None => {
            let (disposition, counterexample) =
                with_counterexample(node, trace, disposition, observed.undefined);
            (disposition, counterexample, None)
        }
    };
    let accepted = !matches!(
        disposition.category(),
        filament_semantic_value::catalog::Category::Refusal
            | filament_semantic_value::catalog::Category::InternalFailure
    );
    let mut report = report(
        disposition,
        ClauseRunUsage::from_meter(&meter, admitted),
        authenticated_temporal_selection,
    );
    report.counterexample = counterexample;
    report.decisive = decisive;
    report.refused_at = refused_at;
    if accepted {
        report.ledger = accepted_ledger.or_else(|| ledger.cloned());
    }
    report
}

/// The disposition of an observed run that `stopped` at a condition or
/// returned `result`, and the position of the refused condition it stopped
/// at. A condition's refusal is reported as FR-109 reports a state clause's
/// (`convert_outcome`); a failed call as `check_clause` does.
pub(super) fn observed_disposition(
    stopped: Option<HoldsStop>,
    result: Result<fsl_eval::temporal::TemporalOutcome, TemporalRefusal>,
    graph: &CheckedGraph,
    sources: &[Source],
) -> (ClauseDisposition, Option<u64>) {
    match (stopped, result) {
        (Some(HoldsStop::Refused { at, evaluation }), _) => (
            refused_disposition(evaluation, graph, sources),
            Some(at.index()),
        ),
        (Some(HoldsStop::Failed(failure)), _) => {
            (refusal_disposition(*convert_call_failure(failure)), None)
        }
        (None, Ok(outcome)) => (ClauseDisposition::Temporal(verdict(outcome)), None),
        (None, Err(TemporalRefusal::Fault(fault))) => {
            (ClauseDisposition::EvaluateFault(fault), None)
        }
        (None, Err(refusal)) => (
            ClauseDisposition::TemporalRefusal(trace_refusal(refusal)),
            None,
        ),
    }
}

/// The disposition of a refused `holds` condition's S6a `evaluation`, as
/// FR-109 reports the same result of a state clause (`convert_outcome`):
/// FR-100's `refused` outcome with the record, or, for the kernel
/// `CheckedInvariant`, the internal failure FR-100 maps it to.
pub(super) fn refused_disposition(
    evaluation: filament_eval::Evaluation,
    graph: &CheckedGraph,
    sources: &[Source],
) -> ClauseDisposition {
    match convert_outcome(evaluation, graph, sources) {
        Ok(outcome) => ClauseDisposition::Evaluate(outcome),
        Err(refusal) => refusal_disposition(*refusal),
    }
}

/// FR-501: evaluate the finite-window clause `node` over the admitted
/// `environments`, one per observation in canonical order, and report its
/// verdict with its basis, decision support, premises and result identity.
fn run_finite_window(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
    input: &Timestamped<'_>,
    observing: ObservedPositions<'_>,
    meter: &mut Meter,
) -> WindowRun {
    use fsl_eval::temporal::{
        LowerBoundary, TimestampedObservation as WindowObservation, Watermark, WindowClock,
        WindowProgress,
    };
    let fault = |invariant: &'static str| {
        WindowRun::stopped(ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
        ))
    };
    let clause = node.clause();
    let ordered = ordered_timestamped_positions(input);
    let trace = match TimestampedTrace::admit(
        clause.name(),
        WindowClock {
            unit: clause.clock().to_owned(),
            cover: match input.window.cover.unwrap_or(TimestampedCover::Exact) {
                TimestampedCover::Exact => fsl_eval::temporal::ClockCover::Exact,
                TimestampedCover::Counter { uncertainty } => {
                    fsl_eval::temporal::ClockCover::Counter { uncertainty }
                }
            },
        },
        ordered
            .iter()
            .map(|position| WindowObservation {
                stamp: position.stamp,
                order_key: position
                    .order_key
                    .as_ref()
                    .map(|order| fsl_eval::temporal::OrderKey {
                        authority: order.authority.clone(),
                        key: order.key,
                    }),
                observation: observation_identity(&position.document),
            })
            .collect(),
        WindowProgress {
            watermark: Watermark::Inclusive(input.progress.watermark),
            completeness: input.progress.completeness.into(),
            scope: input.progress.scope.into(),
            execution: input.progress.execution.into(),
            lower_boundary: if input.window.authoritative_origin {
                LowerBoundary::AuthoritativeOrigin
            } else {
                LowerBoundary::HistoryCutoff
            },
        },
    ) {
        Ok(trace) => trace,
        Err(_) => return fault("finite-window-trace-admits"),
    };
    // The valuation lists name positions by listing index; the evaluator reads
    // canonical positions.
    let canonical = |valuations: &[TimestampedMissingValuation]| -> Vec<(u64, u64)> {
        valuations
            .iter()
            .filter_map(|valuation| {
                let original = input
                    .observations
                    .get(usize::try_from(valuation.position).ok()?)?;
                let position = ordered
                    .iter()
                    .position(|candidate| std::ptr::eq(*candidate, original))?;
                Some((u64::try_from(position).ok()?, valuation.atom))
            })
            .collect()
    };
    // Nodes whose `holds` conditions read the same in the unit are one atom
    // and share one valuation at a position.
    let classes = atom_classes(run, node);
    let same_atom = |valuations: Vec<(u64, u64)>| -> Vec<(u64, u64)> {
        valuations
            .iter()
            .flat_map(|(position, atom)| {
                let class = usize::try_from(*atom)
                    .ok()
                    .and_then(|atom| classes.get(atom).copied());
                classes
                    .iter()
                    .enumerate()
                    .filter(move |(_, candidate)| Some(**candidate) == class)
                    .filter_map(|(index, _)| Some((*position, u64::try_from(index).ok()?)))
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let absent = same_atom(canonical(&input.window.missing_valuations));
    run_window(run, node, &trace, (&absent, None), observing, meter)
}

/// FR-501, FR-504: evaluate `clause` over the admitted `trace` and report its
/// verdict with its basis, decision support, premises and result identity.
/// `valuations` are the `(position, atom)` valuations absent from the
/// observations and the first silent sample.
fn run_window(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
    trace: &fsl_eval::temporal::TimestampedTrace,
    (absent, silent_from): (&[(u64, u64)], Option<u64>),
    observing: ObservedPositions<'_>,
    meter: &mut Meter,
) -> WindowRun {
    let classes = atom_classes(run, node);
    let valuations = fsl_eval::temporal::WindowValuations {
        absent,
        classes: &classes,
        silent_from,
    };
    let observed = match observing {
        ObservedPositions::Objects { environments, over } => {
            fsl_eval::temporal::evaluate_observed_finite_window(
                run.package,
                node,
                environments,
                trace,
                over,
                valuations,
                meter,
            )
        }
        ObservedPositions::Samples(samples) => fsl_eval::temporal::evaluate_sampled_finite_window(
            run.package,
            node,
            samples,
            trace,
            valuations,
            meter,
        ),
    };
    let undefined = observed.undefined;
    match (observed.stopped, observed.result) {
        (Some(HoldsStop::Refused { at, evaluation }), _) => WindowRun {
            refused_at: Some(at.index()),
            ..WindowRun::stopped(refused_disposition(
                evaluation,
                run.package.graph(),
                run.sources,
            ))
        },
        (Some(HoldsStop::Failed(failure)), _) => {
            WindowRun::stopped(refusal_disposition(*convert_call_failure(failure)))
        }
        (None, Ok(evaluation)) => {
            let names = |atom: fsl_semantics::check::FormulaNodeId| {
                atom_name(run.package, run.sources, node, atom)
            };
            let fsl_eval::temporal::WindowEvaluation {
                verdict,
                basis,
                support,
                premises,
                identity,
                visits,
            } = evaluation;
            let verdict = match window_verdict(verdict, &names, guard_location(node)) {
                Ok(Some(verdict)) => verdict,
                Ok(None) => {
                    return WindowRun::stopped(ClauseDisposition::EvaluateFault(
                        filament_semantic_value::catalog::InternalFault::new(
                            "temporal",
                            "unresolved-guard-has-a-guard",
                        ),
                    ));
                }
                Err(refusal) => return WindowRun::stopped(refusal_disposition(*refusal)),
            };
            let report = match finite_window_report(run, node, (basis, support, premises, identity))
            {
                Ok(report) => report,
                Err(refusal) => return WindowRun::stopped(refusal_disposition(*refusal)),
            };
            WindowRun {
                disposition: ClauseDisposition::Temporal(verdict),
                visits,
                report: Some(report),
                undefined,
                refused_at: None,
            }
        }
        (None, Err(TemporalRefusal::Fault(internal))) => {
            WindowRun::stopped(ClauseDisposition::EvaluateFault(internal))
        }
        (None, Err(refusal)) => {
            WindowRun::stopped(ClauseDisposition::TemporalRefusal(trace_refusal(refusal)))
        }
    }
}

/// The disposition a spine refusal stands for: an internal failure for a
/// fault, an `admit`-stage argument refusal for any other.
fn refusal_disposition(refusal: RunRefusal) -> ClauseDisposition {
    match refusal {
        RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
        other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
    }
}

/// What a finite-window or progressed run returns.
struct WindowRun {
    /// The run's disposition.
    disposition: ClauseDisposition,
    /// The `temporal.visit` charges made.
    visits: u64,
    /// FR-501's result report, for a verdict.
    report: Option<FiniteWindowReport>,
    /// The first undefined reading.
    undefined: Option<FirstUndefined>,
    /// The position of the refused condition the run stopped at.
    refused_at: Option<u64>,
}

impl WindowRun {
    /// A run that stopped with `disposition` before any verdict.
    fn stopped(disposition: ClauseDisposition) -> Self {
        Self {
            disposition,
            visits: 0,
            report: None,
            undefined: None,
            refused_at: None,
        }
    }
}

/// The decisive search of a progressed violation's finite-trace run. A run
/// that stopped before it reached its own violation is that stop, never a
/// violation without a record.
fn progressed_search(observed: fsl_eval::temporal::ObservedRun) -> DecisiveSearch {
    if let Some(search) = observed.decisive {
        return search;
    }
    Err(match observed.result {
        Ok(TemporalOutcome::Incomplete(incomplete)) => DecisiveStop::Incomplete(incomplete),
        Err(TemporalRefusal::Fault(fault)) => DecisiveStop::Fault(fault),
        _ => DecisiveStop::Fault(filament_semantic_value::catalog::InternalFault::new(
            "temporal-progress",
            "decisive-run-reaches-the-violation",
        )),
    })
}

/// What the decisive record of a violation is read from.
struct DecisiveRun<'a> {
    run: &'a CompiledRun<'a>,
    node: &'a fsl_semantics::check::TemporalClauseNode,
    trace: &'a TraceInput,
    environments: &'a [ObjectEnvironment],
    over: &'a filament_exact::ObjectReference,
    /// The first silent sample of a progressed trace: it has no observation.
    silent_from: Option<u64>,
}

/// FR-330 Outputs, FR-283 `decisive`: the report of a temporal violation with
/// its decisive information. `search` is the evaluator's descent to the
/// decisive atom (FR-330 "The decisive atom"), found on the run's own
/// evaluator and meter; `None` for a run that did not seek one.
///
/// FR-107's evaluator, its witness channel kept, evaluates the atom over the
/// observation at `decisive.position` on the run's meter and gives the
/// quantifier that decided it, its element, index and value path, or, when no
/// quantifier decides it, the atom's value `false` at the empty path at the
/// observation's root. No decisive record when the clause has no decisive
/// atom. A descent or an atom evaluation that stops is not passed over: the
/// run reports the stop as its own disposition.
fn with_decisive(
    source: &DecisiveRun<'_>,
    search: Option<DecisiveSearch>,
    disposition: ClauseDisposition,
    counterexample: Option<crate::ObservedTemporalCounterexample>,
    meter: &mut Meter,
) -> (
    ClauseDisposition,
    Option<crate::ObservedTemporalCounterexample>,
    Option<crate::TemporalDecisive>,
) {
    let stopped = |disposition| (disposition, None, None);
    let atom = match search {
        None | Some(Ok(None)) => return (disposition, counterexample, None),
        Some(Ok(Some(atom))) => atom,
        Some(Err(DecisiveStop::Incomplete(incomplete))) => {
            return stopped(ClauseDisposition::Temporal(TemporalVerdict::Incomplete(
                incomplete,
            )))
        }
        Some(Err(DecisiveStop::Fault(fault))) => {
            return stopped(ClauseDisposition::EvaluateFault(fault))
        }
    };
    match decisive_record(source, atom, meter) {
        Ok(decisive) => (disposition, counterexample, decisive),
        Err(failure) => stopped(*failure),
    }
}

/// The decisive record of `atom`, read over the observation at its position.
fn decisive_record(
    source: &DecisiveRun<'_>,
    atom: DecisiveAtom,
    meter: &mut Meter,
) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>> {
    let DecisiveRun {
        run,
        node,
        trace,
        environments,
        over,
        silent_from,
    } = *source;
    let fault = |invariant: &'static str| {
        Box::new(ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
        ))
    };
    let position = atom.position.index();
    // A silent sample and an aggregate atom have no `holds` observation.
    if silent_from.is_some_and(|first| position >= first)
        || !matches!(
            node.clause().formula().nodes().get(atom.atom.index()),
            Some(FormulaNode::Holds { .. })
        )
    {
        return Ok(None);
    }
    let slot = usize::try_from(position)
        .ok()
        .and_then(|at| observation_slot(trace, at))
        .ok_or_else(|| fault("decisive-position-has-an-observation"))?;
    let document = trace
        .documents()
        .nth(slot)
        .ok_or_else(|| fault("decisive-position-has-a-document"))?;
    let objects = environments
        .get(slot)
        .ok_or_else(|| fault("decisive-position-has-an-environment"))?;
    decisive_over(
        run,
        node,
        atom,
        over,
        objects,
        fsl_eval::value::ObservationIdentity {
            authority: document.authority.clone(),
            identity: document.identity.clone(),
        },
        meter,
    )
}

/// The decisive record of `atom`, evaluated over `objects`, the observation
/// `observation` names, with the clause's `over` parameter bound to `over`.
#[deny(clippy::wildcard_enum_match_arm)]
#[allow(clippy::too_many_arguments)]
pub(super) fn decisive_over(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
    atom: DecisiveAtom,
    over: &filament_exact::ObjectReference,
    objects: &ObjectEnvironment,
    observation: fsl_eval::value::ObservationIdentity,
    meter: &mut Meter,
) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>> {
    use filament_eval::FamilyOutcome;
    use filament_exact::{Outcome, Value};
    use fsl_eval::value::CheckedPackageEvaluation as _;
    let fault = |invariant: &'static str| {
        Box::new(ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
        ))
    };
    let position = atom.position.index();
    let evaluation = run
        .package
        .evaluate_holds_traced(node, atom.atom, over, objects, observation, meter)
        .map_err(|failure| {
            let refusal = *convert_call_failure(failure);
            Box::new(if let RunRefusal::Fault(internal) = refusal {
                ClauseDisposition::EvaluateFault(internal)
            } else {
                ClauseDisposition::ArgumentRefusal(Box::new(refusal))
            })
        })?;
    match &evaluation.evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {}
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            return Err(Box::new(ClauseDisposition::Temporal(
                TemporalVerdict::Incomplete(incomplete.clone()),
            )))
        }
        FamilyOutcome::Evaluated(_) | FamilyOutcome::FamilyEvaluated(_) => {
            return Err(fault("decisive-atom-reads-false-again"))
        }
    }
    let (condition, _) = node
        .condition(atom.atom)
        .ok_or_else(|| fault("decisive-atom-has-a-condition"))?;
    let stop = crate::witness::decisive_stop(condition, &evaluation)
        .map_err(|internal| Box::new(ClauseDisposition::EvaluateFault(internal)))?;
    let decisive = match stop {
        Some(report) => crate::TemporalDecisive::from_stop(report),
        None => crate::TemporalDecisive::undecided(
            evaluation
                .receiver_path()
                .cloned()
                .ok_or_else(|| fault("decisive-atom-has-a-root-path"))?,
        ),
    };
    Ok(Some(decisive.at_position(position)))
}

/// The trace slot holding the observation at position `at`: a lasso's
/// position past its prefix wraps into its loop.
pub(super) fn observation_slot(trace: &TraceInput, at: usize) -> Option<usize> {
    match trace {
        TraceInput::Lasso { prefix, repeat } if at >= prefix.len() => {
            let offset = (at - prefix.len()).checked_rem(repeat.len())?;
            Some(prefix.len() + offset)
        }
        TraceInput::Lasso { .. }
        | TraceInput::Finite(_)
        | TraceInput::Progressed { .. }
        | TraceInput::Timestamped { .. }
        | TraceInput::Model { .. } => Some(at),
    }
}

/// `disposition` with its FR-330 counterexample. A violation or an
/// `Undefined` that is a violation whose counterexample the run cannot build
/// is an internal failure: FR-267 writes no such verdict without one.
fn with_counterexample(
    node: &fsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    disposition: ClauseDisposition,
    undefined: Option<FirstUndefined>,
) -> (
    ClauseDisposition,
    Option<crate::ObservedTemporalCounterexample>,
) {
    let needed = matches!(
        &disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
    ) || matches!(
        &disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { cause, .. })
            if !matches!(cause, TemporalUndefinedCause::InsufficientData { .. })
    );
    match counterexample(node, trace, &disposition, undefined) {
        Some(counterexample) => (disposition, Some(counterexample)),
        None if needed => (
            ClauseDisposition::EvaluateFault(filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "violation-has-its-counterexample",
            )),
            None,
        ),
        None => (disposition, None),
    }
}

/// The first `Steps` interval of the clause's operators, in operator order.
/// A time-bounded operator's interval names ticks, not trace positions, and
/// FR-364's observed arm carries none.
fn step_interval(
    clause: &fsl_semantics::check::CheckedTemporalClause,
) -> Option<fsl_foundation::bound::TemporalInterval> {
    clause
        .operators()
        .iter()
        .find_map(|operator| match operator.interval.as_ref() {
            Some(fsl_semantics::check::CheckedInterval::Steps(interval)) => Some(*interval),
            _ => None,
        })
}

/// FR-330 Outputs: the observed-arm counterexample (FSpec FR-364) of a
/// `Completed(false)` or of an `Undefined` that is a violation, built from what
/// the entry ran: one `{document}` step per represented position (for an
/// `Undefined` result, the positions 0 to the failing position and no loop),
/// the clause's fairness set, the clause's first `Steps` interval (none for
/// an `Undefined` result) and the trace position. `None` for every other
/// disposition, and for an `Undefined` result whose first undefined reading
/// the evaluator did not record at its position.
fn counterexample(
    node: &fsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    disposition: &ClauseDisposition,
    undefined: Option<FirstUndefined>,
) -> Option<crate::ObservedTemporalCounterexample> {
    use crate::{ObservedTemporalCounterexample, ObservedTemporalKind, ObservedTemporalStep};
    let clause = node.clause();
    let documents: Vec<&DocumentRef> = trace.documents().collect();
    let step = |document: &DocumentRef| ObservedTemporalStep {
        document: document.clone(),
    };
    match disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) => {
            Some(ObservedTemporalCounterexample {
                prefix: documents.iter().map(|document| step(document)).collect(),
                loop_start: match trace {
                    TraceInput::Lasso { prefix, .. } => Some(u64::try_from(prefix.len()).ok()?),
                    _ => None,
                },
                fairness: clause.fairness().to_vec(),
                interval: step_interval(clause),
                kind: ObservedTemporalKind::Formula,
                trace_position: TemporalPosition::new(*at),
            })
        }
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { at, cause })
            if !matches!(cause, TemporalUndefinedCause::InsufficientData { .. }) =>
        {
            let first = undefined.filter(|first| first.position.index() == *at)?;
            // A lasso position past its represented positions re-enters the
            // loop.
            let (prefix_len, loop_len) = match trace {
                TraceInput::Lasso { prefix, repeat } => (prefix.len(), repeat.len()),
                _ => (documents.len(), 0),
            };
            let slot = |index: usize| {
                if index < prefix_len || loop_len == 0 {
                    index
                } else {
                    prefix_len + (index - prefix_len) % loop_len
                }
            };
            let last = usize::try_from(*at).ok()?;
            let prefix = (0..=last)
                .map(|index| documents.get(slot(index)).map(|document| step(document)))
                .collect::<Option<Vec<_>>>()?;
            Some(ObservedTemporalCounterexample {
                prefix,
                loop_start: None,
                fairness: clause.fairness().to_vec(),
                interval: None,
                kind: ObservedTemporalKind::UndefinedEvaluation {
                    locus: first.locus,
                    cause: first.cause,
                },
                trace_position: TemporalPosition::new(*at),
            })
        }
        _ => None,
    }
}

/// Each formula node's atom class: the index of the first `holds` node whose
/// condition reads the same in the unit, or the node's own index.
fn atom_classes(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
) -> Vec<usize> {
    let nodes = node.clause().formula().nodes();
    let source = clause_source(run.package, run.sources, node);
    let text = |index: usize| match nodes.get(index) {
        Some(FormulaNode::Holds { span }) => source.and_then(|source| source.slice(*span)),
        _ => None,
    };
    (0..nodes.len())
        .map(|index| match text(index) {
            Some(own) => (0..index)
                .find(|earlier| text(*earlier) == Some(own))
                .unwrap_or(index),
            None => index,
        })
        .collect()
}

/// The supplied source the clause `node` is declared in: the element of
/// `run.sources` whose reference equals its declaration's region (FR-283
/// "Behavior"), when one is supplied.
fn clause_source<'a>(
    package: &fsl_package::CheckedPackage,
    sources: &'a [Source],
    node: &fsl_semantics::check::TemporalClauseNode,
) -> Option<&'a Source> {
    let reference = clause_source_ref(package, node)?;
    sources
        .iter()
        .find(|source| source.reference() == &reference)
}

/// The source the clause `node` is declared in, as its declaration's region
/// names it.
pub(super) fn clause_source_ref(
    package: &fsl_package::CheckedPackage,
    node: &fsl_semantics::check::TemporalClauseNode,
) -> Option<fsl_foundation::source::provenance::RawSourceRef> {
    let graph = package.graph();
    let location = graph.occurrence(node.identity(), node.claim())?;
    Some(graph.region(location)?.source().clone())
}

/// The checked location of the clause's activation guard.
fn guard_location(
    node: &fsl_semantics::check::TemporalClauseNode,
) -> Option<filament_semantic_value::location::ExpressionLocation> {
    node.guard().map(|(guard, _)| guard.location().clone())
}

/// The spine's own form of the evaluator's result report (FR-100-AC-8).
fn finite_window_report(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
    (basis, support, premises, identity): (
        fsl_eval::temporal::Basis,
        Vec<fsl_eval::temporal::SupportPair>,
        fsl_eval::temporal::Premises,
        fsl_eval::temporal::ResultIdentity,
    ),
) -> Result<FiniteWindowReport, Box<RunRefusal>> {
    use fsl_eval::temporal::{Assessment, Basis, ClockCover, LowerBoundary};
    Ok(FiniteWindowReport {
        basis: match basis {
            Basis::ClosedScope => SettlementBasis::ClosedScope,
            Basis::DecisiveWitness => SettlementBasis::DecisiveWitness,
            Basis::DecisiveCounterexample => SettlementBasis::DecisiveCounterexample,
            Basis::Unsettled => SettlementBasis::Unsettled,
            Basis::Unavailable => SettlementBasis::Unavailable,
        },
        support: support
            .iter()
            .map(|pair| {
                Ok(FiniteWindowSupport {
                    position: pair.position.index(),
                    atom: atom_name(run.package, run.sources, node, pair.atom)?,
                })
            })
            .collect::<Result<_, Box<RunRefusal>>>()?,
        premises: FiniteWindowPremises {
            scope: premises.scope.into(),
            execution: premises.execution.into(),
            assessment: match premises.assessment {
                Assessment::Completed => FiniteWindowAssessment::Completed,
                Assessment::Failed => FiniteWindowAssessment::Failed,
            },
            completeness: premises.completeness.into(),
            watermark: premises.watermark,
            authoritative_origin: premises.lower_boundary == LowerBoundary::AuthoritativeOrigin,
            cover: match premises.cover {
                ClockCover::Exact => TimestampedCover::Exact,
                ClockCover::Counter { uncertainty } => TimestampedCover::Counter { uncertainty },
            },
            unit: premises.unit,
        },
        identity: identity.0,
    })
}

/// The name of the `holds` or window-aggregate atom `atom` of the clause
/// `node`: the source text of its condition, as the unit wrote it, read from
/// the supplied source its region names.
///
/// # Errors
///
/// [`RunRefusal::SourceNotSupplied`] when no element of `sources` has the
/// region's reference, or the matching source does not hold the span on
/// UTF-8 boundaries.
pub(super) fn atom_name(
    package: &fsl_package::CheckedPackage,
    sources: &[Source],
    node: &fsl_semantics::check::TemporalClauseNode,
    atom: fsl_semantics::check::FormulaNodeId,
) -> Result<String, Box<RunRefusal>> {
    let Some(FormulaNode::Holds { span } | FormulaNode::Aggregate { span }) =
        node.clause().formula().nodes().get(atom.index()).cloned()
    else {
        // An atom is a `holds` or aggregate node; FR-283 never names one by
        // its formula node index.
        return Err(Box::new(RunRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "named-atom-is-a-holds-or-aggregate-node",
            ),
        )));
    };
    let Some(reference) = clause_source_ref(package, node) else {
        return Err(Box::new(RunRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "clause-declaration-has-a-region",
            ),
        )));
    };
    let text = clause_source(package, sources, node).and_then(|source| source.slice(span));
    text.map(str::to_owned).ok_or_else(|| {
        Box::new(RunRefusal::SourceNotSupplied {
            reference: fsl_foundation::SourceIdentity::new(
                reference.authority(),
                reference.identity(),
            ),
        })
    })
}

/// FR-283 "Behavior": the name of the atom with formula occurrence `atom` of
/// the temporal clause `name` of `package`, read from `sources`.
///
/// # Errors
///
/// [`RunRefusal::SourceNotSupplied`] as `atom_name` refuses, and a
/// [`RunRefusal::Fault`] for a clause or atom `package` does not hold.
pub fn name_clause_atom(
    package: &fsl_package::CheckedPackage,
    sources: &[Source],
    name: &str,
    atom: usize,
) -> Result<String, Box<RunRefusal>> {
    let fault = |invariant: &'static str| {
        Box::new(RunRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new("monitor", invariant),
        ))
    };
    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
        .ok_or_else(|| fault("named-atom-clause-is-declared"))?;
    let atom = node
        .clause()
        .formula()
        .node_id(atom)
        .ok_or_else(|| fault("named-atom-is-a-formula-node"))?;
    atom_name(package, sources, node, atom)
}

/// The spine's own form of the finite-window evaluator's verdict
/// (FR-100-AC-8).
fn window_verdict(
    verdict: fsl_eval::temporal::WindowVerdict,
    names: &dyn Fn(fsl_semantics::check::FormulaNodeId) -> Result<String, Box<RunRefusal>>,
    guard: Option<filament_semantic_value::location::ExpressionLocation>,
) -> Result<Option<TemporalVerdict>, Box<RunRefusal>> {
    use fsl_eval::temporal::{IndeterminateReason, WindowVerdict};
    Ok(Some(match verdict {
        WindowVerdict::Holds => TemporalVerdict::Holds,
        WindowVerdict::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        WindowVerdict::Pending { at, deadline } => TemporalVerdict::PendingWindow {
            at: at.index(),
            deadline,
        },
        WindowVerdict::PendingNextEvent { at } => {
            TemporalVerdict::PendingNextEvent { at: at.index() }
        }
        WindowVerdict::Indeterminate { at, reason } => TemporalVerdict::Indeterminate {
            at: at.index(),
            reason: match reason {
                IndeterminateReason::MissingValuation { position, atom } => {
                    TemporalIndeterminateReason::MissingValuation {
                        position: position.index(),
                        atom: names(atom)?,
                        instance: None,
                        also_held_by: Vec::new(),
                    }
                }
                IndeterminateReason::ConflictingValuation { position, atom } => {
                    TemporalIndeterminateReason::ConflictingValuation {
                        position: position.index(),
                        atom: names(atom)?,
                    }
                }
                IndeterminateReason::IncompleteInput => {
                    TemporalIndeterminateReason::IncompleteInput
                }
                IndeterminateReason::MissingHistory => TemporalIndeterminateReason::MissingHistory,
                IndeterminateReason::StraddlesBound => TemporalIndeterminateReason::StraddlesBound,
            },
        },
        WindowVerdict::Undefined(undefined) => verdict_of_undefined(undefined),
        WindowVerdict::Incomplete(incomplete) => TemporalVerdict::Incomplete(incomplete),
        WindowVerdict::WindowLimitReached(reached) => TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: reached.limit,
            setting: reached.setting,
        },
        // An unresolved guard comes from the clause's own guard: a clause
        // without one broke an evaluator invariant.
        WindowVerdict::ActivationIncomplete { at } => {
            let Some(guard) = guard else {
                return Ok(None);
            };
            TemporalVerdict::ActivationIncomplete {
                at: at.index(),
                guard,
            }
        }
    }))
}

fn verdict_of_undefined(undefined: UndefinedEvaluation) -> TemporalVerdict {
    verdict(TemporalOutcome::Undefined(undefined))
}

/// The declared cover of a trace's stamps (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampedCover {
    /// A stamp is the instant itself.
    Exact,
    /// A counter reading `k` covers `[k, k+1)` widened by `uncertainty` ticks
    /// on each side.
    Counter {
        /// The widening `u`, in ticks.
        uncertainty: u64,
    },
}

/// Whether the evaluator produced an outcome (FR-501 "Result report").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteWindowAssessment {
    /// The evaluator produced an outcome.
    Completed,
    /// The evaluator returned `Incomplete` or a refusal.
    Failed,
}

/// One `(position, atom)` pair of a decision support.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FiniteWindowSupport {
    /// The position, in canonical order.
    pub position: u64,
    /// The atom: the source text of its `holds` condition.
    pub atom: String,
}

/// The four independent dimensions and the inputs that qualify a finite-window
/// verdict (FR-501 "Result report").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteWindowPremises {
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The assessment-execution disposition.
    pub assessment: FiniteWindowAssessment,
    /// The input completeness.
    pub completeness: ProgressCompleteness,
    /// The watermark as an inclusive tick, `-1` when progress covers none.
    pub watermark: i128,
    /// Whether the first position is an authoritative origin (the
    /// lower-boundary kind).
    pub authoritative_origin: bool,
    /// The clock cover.
    pub cover: TimestampedCover,
    /// The clock unit.
    pub unit: String,
}

/// FR-330, FR-501 "Result report": the basis, decision support, premises and
/// result identity of a finite-window run, as separate fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteWindowReport {
    /// What the verdict rests on.
    pub basis: SettlementBasis,
    /// The decision support, as `(position, atom)` pairs.
    pub support: Vec<FiniteWindowSupport>,
    /// The premises.
    pub premises: FiniteWindowPremises,
    /// The result identity: a content identity over the inputs.
    pub identity: String,
}

/// The spine's own form of the evaluator's outcome (FR-100-AC-8).
/// The spine's own form of the evaluator's outcome (FR-100-AC-8).
fn verdict(outcome: TemporalOutcome) -> TemporalVerdict {
    match outcome {
        TemporalOutcome::Holds => TemporalVerdict::Holds,
        TemporalOutcome::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        TemporalOutcome::Undefined(UndefinedEvaluation { at, cause }) => {
            TemporalVerdict::Undefined {
                at: at.index(),
                cause: match cause {
                    UndefinedCause::Kernel(undefined) => TemporalUndefinedCause::Kernel(undefined),
                    UndefinedCause::Family(record) => TemporalUndefinedCause::Family(record),
                    UndefinedCause::InsufficientData(data) => {
                        TemporalUndefinedCause::InsufficientData {
                            atom: 0,
                            count: data.count,
                            min_count: data.min_count,
                        }
                    }
                },
            }
        }
        TemporalOutcome::InsufficientData(insufficient) => TemporalVerdict::Undefined {
            at: insufficient.at.index(),
            cause: TemporalUndefinedCause::InsufficientData {
                atom: insufficient.atom.index(),
                count: insufficient.data.count,
                min_count: insufficient.data.min_count,
            },
        },
        TemporalOutcome::Pending => TemporalVerdict::Pending,
        TemporalOutcome::MissingFairnessPremise { constraint } => {
            TemporalVerdict::MissingFairnessPremise { constraint }
        }
        TemporalOutcome::Incomplete(incomplete) => TemporalVerdict::Incomplete(incomplete),
        TemporalOutcome::WindowLimitReached(reached) => TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: reached.limit,
            setting: reached.setting,
        },
    }
}

/// The spine's own form of the evaluator's trace refusal (FR-100-AC-8).
fn trace_refusal(refusal: TemporalRefusal) -> TemporalTraceRefusal {
    match refusal {
        TemporalRefusal::EmptyTrace => TemporalTraceRefusal::EmptyTrace,
        TemporalRefusal::TupleArity { expected, found } => {
            TemporalTraceRefusal::TupleArity { expected, found }
        }
        TemporalRefusal::AlignSkipNotLockstep => TemporalTraceRefusal::AlignSkipNotLockstep,
        TemporalRefusal::NotProjectionAligned => TemporalTraceRefusal::NotProjectionAligned,
        TemporalRefusal::EmptyLoop => TemporalTraceRefusal::EmptyLoop,
        TemporalRefusal::TraceTooLong => TemporalTraceRefusal::TraceTooLong,
        TemporalRefusal::LassoUnderBoundedProfile { profile } => {
            TemporalTraceRefusal::LassoUnderBoundedProfile { profile }
        }
        TemporalRefusal::ProfileNotEvaluated { profile } => {
            TemporalTraceRefusal::ProfileNotEvaluated { profile }
        }
        TemporalRefusal::Position(position) => TemporalTraceRefusal::Position(match position {
            PositionRefusal::NotDecimal => TemporalPositionRefusal::NotDecimal,
            PositionRefusal::LeadingZero => TemporalPositionRefusal::LeadingZero,
            PositionRefusal::Overflow => TemporalPositionRefusal::Overflow,
            PositionRefusal::OutsideTrace { position } => TemporalPositionRefusal::OutsideTrace {
                position: position.index(),
            },
        }),
        TemporalRefusal::Window(window) => TemporalTraceRefusal::Window(match window {
            WindowRefusal::OutsideTrace { position } => {
                TemporalWindowRefusal::OutsideTrace { position }
            }
            WindowRefusal::WindowKindMismatch => TemporalWindowRefusal::WindowKindMismatch,
            WindowRefusal::TimestampsDecrease => TemporalWindowRefusal::TimestampsDecrease,
        }),
        TemporalRefusal::UnorderedTie { position } => {
            TemporalTraceRefusal::SameStampOrderUnavailable {
                position: position.index(),
            }
        }
        TemporalRefusal::Fault(fault) => TemporalTraceRefusal::Fault(fault),
        // The observed letters keep every refused condition whole, and the
        // caller reports that stop before it reads this refusal.
        TemporalRefusal::ConditionRefused { .. } => {
            TemporalTraceRefusal::Fault(filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "refused-condition-keeps-its-evaluation",
            ))
        }
        // The timed evaluator's refusals: the spine reads a clause through
        // the step evaluator only, which never returns them.
        TemporalRefusal::NotTimedProfile { .. }
        | TemporalRefusal::NotFiniteWindowProfile { .. }
        | TemporalRefusal::TraceCarriesNoStamps { .. }
        | TemporalRefusal::NegativeStamp { .. }
        | TemporalRefusal::DecreasingStamp { .. }
        | TemporalRefusal::NonPositiveLoopDelay
        | TemporalRefusal::LoopReentersBeforeLastStamp => {
            TemporalTraceRefusal::Fault(filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "step-evaluator-refusal",
            ))
        }
    }
}
