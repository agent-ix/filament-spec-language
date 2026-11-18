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
    evaluate_observed_decisive, DecisiveAtom, DecisiveSearch, DecisiveStop, FirstUndefined,
    HoldsStop, ObservedPositions, PositionRefusal, ProgressedRefusal, TemporalOutcome,
    TemporalPosition, TemporalRefusal, TemporalTrace, TimestampedTrace, UndefinedCause,
    UndefinedEvaluation, WindowRefusal,
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
    ProgressCompleteness, ProgressContradiction, ProgressLedger, ProgressLedgerEntry,
    ProgressLowerBoundary, ProgressValue, Refusal,
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
        self.progress_from(0)
    }

    /// The progress assertion, and the identities of the positions listed
    /// from index `from` on (FR-820: a carried state lists only the ones its
    /// ledger has not admitted).
    fn progress_from(&self, from: usize) -> Option<(ProgressAssertion, Vec<ObservedPosition>)> {
        match self {
            Self::Progressed {
                trace, progress, ..
            } => Some((
                *progress,
                (from..trace.len())
                    .map(|index| self.position_at(index))
                    .collect::<Option<Vec<_>>>()?,
            )),
            Self::Timestamped {
                observations,
                progress,
                ..
            } => Some((
                *progress,
                (from..observations.len())
                    .map(|index| self.position_at(index))
                    .collect::<Option<Vec<_>>>()?,
            )),
            Self::Finite(_) | Self::Lasso { .. } | Self::Model { .. } => None,
        }
    }

    /// The identity and stamp of the position listed at `index` of a
    /// progress-carrying trace.
    fn position_at(&self, index: usize) -> Option<ObservedPosition> {
        match self {
            Self::Progressed { trace, .. } => {
                let document = trace.get(index)?;
                Some(ObservedPosition {
                    identity: document_identity(document),
                    stamp: u64::try_from(index).ok()?,
                    fingerprint: document_fingerprint(document),
                })
            }
            Self::Timestamped { observations, .. } => {
                let observation = observations.get(index)?;
                Some(ObservedPosition {
                    identity: document_identity(&observation.document),
                    stamp: observation.stamp,
                    fingerprint: document_fingerprint(&observation.document),
                })
            }
            Self::Finite(_) | Self::Lasso { .. } | Self::Model { .. } => None,
        }
    }

    /// The number of positions a progress-carrying trace lists.
    fn listed(&self) -> usize {
        match self {
            Self::Progressed { trace, .. } => trace.len(),
            Self::Timestamped { observations, .. } => observations.len(),
            Self::Finite(positions) => positions.len(),
            Self::Lasso { prefix, repeat } => prefix.len().saturating_add(repeat.len()),
            Self::Model { .. } => 0,
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
                .map(|(_, observation)| &observation.document),
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
pub(crate) const SAMPLE_AUTHORITY: &str = "record-sample";

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

/// FR-993: the content fingerprint of the record sample whose canonical JSON
/// text is `text`: `sha256:` and the 64 lowercase hexadecimal digits of the
/// SHA-256 digest of its canonical value bytes, encoded by ADR-013's one
/// RFC 8785 encoder (`quire-canonical`). It is integrity data beside the
/// sample's index identity, never an identity (ADR-013 §2).
///
/// # Errors
///
/// The internal fault `content-fingerprint-failed` when the canonical bytes
/// or their digest cannot be produced; no empty fingerprint is returned.
pub(crate) fn sample_fingerprint(
    text: &str,
) -> Result<String, filament_semantic_value::catalog::InternalFault> {
    let failed = || {
        filament_semantic_value::catalog::InternalFault::new(
            "temporal",
            "content-fingerprint-failed",
        )
    };
    let document = quire_canonical::read(text.as_bytes(), u64::MAX).map_err(|_| failed())?;
    let digest = quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| failed())?;
    let mut fingerprint = String::with_capacity(71);
    fingerprint.push_str("sha256:");
    for byte in digest.as_bytes() {
        fingerprint.push_str(&format!("{byte:02x}"));
    }
    Ok(fingerprint)
}

/// FR-330-AC-18, FR-643 Decision, FR-993: the record sample at `index` of a
/// trace, cited by its index, never by its value. A progress-carrying trace
/// also carries the sample's content `fingerprint` ([`sample_fingerprint`])
/// for its ledger entry: the run holds it after the index (`<index>#<fp>`),
/// and every identity, citation and order reads the index alone
/// ([`indexed_sample`]).
pub(crate) fn sample_index(index: u64, fingerprint: Option<&str>) -> DocumentRef {
    DocumentRef {
        authority: SAMPLE_AUTHORITY.to_owned(),
        identity: match fingerprint {
            Some(fingerprint) => format!("{index}#{fingerprint}"),
            None => index.to_string(),
        },
    }
}

/// The index of a record sample [`sample_index`] names, and the content
/// fingerprint it carries, if any. `None` for a sample named by its text and
/// for every other document.
pub(crate) fn indexed_sample(document: &DocumentRef) -> Option<(u64, Option<&str>)> {
    if document.authority != SAMPLE_AUTHORITY {
        return None;
    }
    match document.identity.split_once('#') {
        Some((index, fingerprint)) => Some((index.parse().ok()?, Some(fingerprint))),
        None => Some((document.identity.parse().ok()?, None)),
    }
}

/// FR-330-AC-18, FR-643 Decision: a progress-carrying trace of record samples
/// as the run reads it past admission, each sample named by its index in the
/// trace with its content fingerprint (FR-993), so the ledger and the
/// finite-window trace hold no sample's value. A `Timestamped` sample's index
/// is its place in FR-501's canonical order, which its value decides among
/// positions of one stamp and order key, so the listing order still decides
/// nothing. `Ok(None)` for a trace that needs no renaming.
fn indexed_samples(
    trace: &TraceInput,
) -> Result<Option<TraceInput>, filament_semantic_value::catalog::InternalFault> {
    let sample = |document: &DocumentRef| document.authority == SAMPLE_AUTHORITY;
    let cite = |index: usize, document: &DocumentRef| {
        sample_fingerprint(&document.identity).map(|fingerprint| {
            sample_index(u64::try_from(index).unwrap_or(u64::MAX), Some(&fingerprint))
        })
    };
    match trace {
        TraceInput::Progressed {
            trace: documents,
            progress,
            lower_boundary,
            binding,
        } if documents.iter().any(sample) => Ok(Some(TraceInput::Progressed {
            trace: documents
                .iter()
                .enumerate()
                .map(|(index, document)| {
                    if sample(document) {
                        cite(index, document)
                    } else {
                        Ok(document.clone())
                    }
                })
                .collect::<Result<_, _>>()?,
            progress: *progress,
            lower_boundary: *lower_boundary,
            binding: binding.clone(),
        })),
        TraceInput::Timestamped {
            observations,
            progress,
            window,
        } if observations.iter().any(|o| sample(&o.document)) => {
            let view = Timestamped {
                observations,
                progress: *progress,
                window,
            };
            // The canonical place of each listed observation, by its index
            // in the listing.
            let mut canonical = vec![None; observations.len()];
            for (place, (listed, _)) in ordered_timestamped_positions(&view).iter().enumerate() {
                if let Some(slot) = canonical.get_mut(*listed) {
                    *slot = Some(place);
                }
            }
            let renamed = observations
                .iter()
                .zip(canonical)
                .map(|(observation, canonical)| match canonical {
                    Some(index) if sample(&observation.document) => {
                        cite(index, &observation.document).map(|document| TimestampedObservation {
                            document,
                            ..observation.clone()
                        })
                    }
                    _ => Ok(observation.clone()),
                })
                .collect::<Result<_, _>>()?;
            Ok(Some(TraceInput::Timestamped {
                observations: renamed,
                progress: *progress,
                window: window.clone(),
            }))
        }
        TraceInput::Finite(_)
        | TraceInput::Progressed { .. }
        | TraceInput::Timestamped { .. }
        | TraceInput::Lasso { .. }
        | TraceInput::Model { .. } => Ok(None),
    }
}

/// The ledger identity of a position's document: its labels, and for a
/// record sample named by its index, the index alone (FR-330, FR-993): the
/// content fingerprint is the covered entry's own member, never part of the
/// identity.
pub(super) fn document_identity(document: &DocumentRef) -> String {
    let index = indexed_sample(document).map(|(index, _)| index.to_string());
    let identity = index.as_deref().unwrap_or(document.identity.as_str());
    format!(
        "{}:{}{}:{}",
        document.authority.len(),
        document.authority,
        identity.len(),
        identity,
    )
}

/// The content fingerprint a record sample named by its index carries
/// (FR-993), for its covered ledger entry.
fn document_fingerprint(document: &DocumentRef) -> Option<String> {
    indexed_sample(document).and_then(|(_, fingerprint)| fingerprint.map(str::to_owned))
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
    /// The interval stamps, arrival watermarks and late-data cutoff, and
    /// the late observations a repair admits (FR-501, FR-526, FR-966).
    pub late: LateInputs,
}

/// FR-501 "Bounded cover and interval stamps" and "Late-data cutoff",
/// FR-526: the per-observation uncertainty and arrival of a timestamped
/// trace, each list by the observation's index in `observations` (an empty
/// list gives no observation the member).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LateInputs {
    /// Each observation's interval stamp, when it has one.
    pub intervals: Vec<Option<fsl_eval::temporal::StampInterval>>,
    /// Each observation's arrival watermark, when it states one.
    pub arrivals: Vec<Option<u64>>,
    /// The late-data cutoff, when one is selected.
    pub cutoff: Option<fsl_eval::temporal::LateCutoff>,
    /// The late observations a superseding repair admits, by observation
    /// identity (FR-526).
    pub admit_late: Vec<String>,
    /// FR-526: the relation of the run's result to an earlier one.
    pub relation: super::correction::RunRelation,
    /// FR-526: the evidence ledger a prior run wrote, which a correction
    /// reads.
    pub evidence_ledger: Option<super::correction::EvidenceLedgerFile>,
}

/// One observation the late-data cutoff excluded, as the result's `late`
/// list writes it (FR-501 "Request-file spelling").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LateEntry {
    /// The observation's index in input order.
    pub position: u64,
    /// The observation's identity (FR-106), by which a repair admits it.
    pub observation: String,
    /// The classified event instant: the upper end of its uncertainty.
    pub stamp: u64,
    /// Its arrival watermark.
    pub arrival: u64,
    /// The canonical positions of the activations whose windows it falls in.
    pub anchors: Vec<u64>,
}

/// One late observation a superseding repair admitted (FR-526).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedLate {
    /// The observation's index in input order.
    pub position: u64,
    /// The observation's identity.
    pub observation: String,
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
/// A record sample named by its index (FR-330-AC-18) spells the index after
/// its two-digit length, so the identities order as the indices do.
pub(super) fn observation_identity(document: &DocumentRef) -> String {
    if let Some((index, _)) = indexed_sample(document) {
        // The index alone, after its length: the identities order as the
        // indices do, and the content fingerprint is no part of it (FR-993).
        let index = index.to_string();
        return format!("{}\0{:02}:{}", document.authority, index.len(), index);
    }
    format!("{}\0{}", document.authority, document.identity)
}

/// FR-501 "Canonical order": ascending earliest and then latest instant (an
/// observation's interval stamp, or the cover of its stamp), then ascending
/// stamp, then ascending order-authority identity (an observation without an
/// order key after every keyed one), then ascending key within the
/// authority, then ascending observation identity. A total order over
/// distinct observations: the listing order decides nothing. Each entry is
/// the observation's index in `observations` and the observation.
fn ordered_timestamped_positions<'a>(
    input: &Timestamped<'a>,
) -> Vec<(usize, &'a TimestampedObservation)> {
    fn order(position: &TimestampedObservation) -> Option<(&str, i64)> {
        position
            .order_key
            .as_ref()
            .map(|order| (order.authority.as_str(), order.key))
    }
    let cover = super::late_data::eval_cover(input.window.cover);
    let span = |index: usize, position: &TimestampedObservation| {
        cover.order_span(
            position.stamp,
            super::late_data::interval_at(&input.window.late, index),
        )
    };
    let mut positions: Vec<_> = input.observations.iter().enumerate().collect();
    positions.sort_by(|(left_index, left), (right_index, right)| {
        span(*left_index, left)
            .cmp(&span(*right_index, right))
            .then_with(|| left.stamp.cmp(&right.stamp))
            .then_with(|| match (order(left), order(right)) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| {
                observation_identity(&left.document).cmp(&observation_identity(&right.document))
            })
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
                ClauseDisposition::ProgressContradiction(Box::new((*contradiction).into())),
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
    /// Whether the run reports its valuation (FR-757) in place of a verdict.
    pub(super) valuation: bool,
    /// FR-820: the letter tape a carried clause state reads in place of the
    /// record samples the trace names; `None` for a batch run.
    pub(super) tape: Option<TapeInput<'a>>,
}

/// FR-820: what a carried clause state hands the run in place of its samples.
#[derive(Clone, Copy)]
pub(crate) struct TapeInput<'a> {
    /// The letters read at admission, one per position.
    pub(crate) tape: &'a fsl_eval::temporal::LetterTape,
    /// The samples the state holds by value, by position.
    pub(crate) values: &'a BTreeMap<u64, Value>,
    /// The captures fixed at the origin activation.
    pub(crate) captures: &'a [Value],
    /// Whether window aggregates are read from the tape on every route: FR-504.s
    /// open-scope reading of a closed-trace clause reads the aggregate letters
    /// the closed route read, so it needs no window of values.
    pub(crate) open_aggregates: bool,
    /// FR-820: the carried finite-window evaluation the run continues, in
    /// place of evaluating every position of a `Progressed` or `Timestamped`
    /// trace again.
    pub(crate) window: Option<&'a std::cell::RefCell<fsl_eval::temporal::CarriedWindow>>,
    /// FR-820: a snapshot trace's tape (`over` names a model object). Its
    /// letters stand in for the object environments, each snapshot having
    /// been admitted once when it arrived.
    pub(crate) snapshot: Option<SnapshotTape<'a>>,
    /// FR-820: what the state's earlier runs admitted of the trace and the
    /// ledger, which this run continues.
    pub(crate) admission: Option<&'a std::cell::RefCell<CarriedAdmission>>,
}

/// The decisive descent of a snapshot tape: the violated activation, the
/// trace's length and its first silent sample.
pub(crate) type SnapshotDescend<'a> =
    &'a dyn Fn(u64, u64, Option<u64>, &mut Meter) -> Option<fsl_eval::temporal::ObservedRun>;

/// The decisive record of an atom of a snapshot tape.
pub(crate) type SnapshotRecord<'a> =
    &'a dyn Fn(
        &CompiledRun<'_>,
        DecisiveAtom,
        &mut Meter,
    ) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>>;

/// FR-820: what a carried snapshot state read of its trace once, in place of
/// admitting every snapshot again.
#[derive(Clone, Copy)]
pub(crate) struct SnapshotTape<'a> {
    /// The first snapshot whose admission failed, in trace order.
    pub(crate) failure: Option<&'a AdmissionFailure>,
    /// The object `over` names, resolved in the observation at position 0.
    pub(crate) over: Result<&'a filament_exact::ObjectReference, &'a OverMiss>,
    /// The descent to a violation's decisive atom.
    pub(crate) descend: SnapshotDescend<'a>,
    /// The decisive record of the atom the descent found.
    pub(crate) record: SnapshotRecord<'a>,
}

/// FR-820: what a carried clause state's runs admitted of its trace and its
/// ledger, so each run admits only the positions added since. One is kept
/// for the report's run and one for FR-504's open-scope reading.
#[derive(Clone, Debug, Default)]
pub(crate) struct CarriedAdmission {
    /// The finite-window trace of a `Progressed` or `Timestamped` trace, as
    /// the last run admitted it.
    trace: Option<TimestampedTrace>,
    /// The ledger the last admission returned.
    ledger: Option<LedgerCarry>,
}

/// The ledger a carried admission returned, and what it was admitted from.
#[derive(Clone, Debug)]
struct LedgerCarry {
    /// The binding's entry in the ledger the run started from.
    start: Option<ProgressLedgerEntry>,
    /// The binding's entry in the ledger the run returned: `start`'s covered
    /// positions with the trace's first `admitted` positions added.
    output: ProgressLedgerEntry,
    /// How many of the trace's positions `output` covers.
    admitted: usize,
    /// The last of them.
    last: Option<ObservedPosition>,
}

/// FR-820 "No partial state": a [`CarriedAdmission`] before one call, kept
/// as its count of admitted positions.
#[derive(Clone, Debug)]
pub(crate) struct AdmissionMark {
    positions: Option<usize>,
    ledger: Option<LedgerCarry>,
}

/// Whether `left` and `right` are one version of one entry.
fn same_entry(left: Option<&ProgressLedgerEntry>, right: Option<&ProgressLedgerEntry>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.assertion() == right.assertion()
                && left.covered_list().shares(right.covered_list())
        }
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

impl CarriedAdmission {
    /// The admission as it is now, for [`Self::restore`].
    pub(crate) fn mark(&self) -> AdmissionMark {
        AdmissionMark {
            positions: self.trace.as_ref().map(TimestampedTrace::observed),
            ledger: self.ledger.clone(),
        }
    }

    /// Put back the admission `mark` was taken of.
    pub(crate) fn restore(&mut self, mark: AdmissionMark) {
        match (mark.positions, self.trace.as_mut()) {
            (Some(positions), Some(trace)) => trace.truncate(positions),
            (None, _) => self.trace = None,
            (Some(_), None) => {}
        }
        self.ledger = mark.ledger;
    }

    /// Keep only the first `len` positions: the positions after them take
    /// another order (FR-501 "Canonical order").
    pub(crate) fn truncate(&mut self, len: usize) {
        if let Some(trace) = self.trace.as_mut() {
            trace.truncate(len);
        }
        if self.ledger.as_ref().is_some_and(|held| held.admitted > len) {
            self.ledger = None;
        }
    }

    /// The carried trace with every position of `listed` admitted, the first
    /// `held` of which it holds already: the ones after them, or all when the
    /// carried trace is not a prefix of the listing. `open` makes the trace the
    /// first time. Returns how many positions are to be admitted.
    fn continue_trace(
        &mut self,
        listed: usize,
        held: impl Fn(&TimestampedTrace, usize) -> bool,
        open: impl FnOnce() -> TimestampedTrace,
    ) -> (&mut TimestampedTrace, usize) {
        let from = match self.trace.as_ref() {
            Some(trace) if trace.observed() <= listed && held(trace, trace.observed()) => {
                trace.observed()
            }
            _ => 0,
        };
        let trace = self.trace.get_or_insert_with(open);
        trace.truncate(from);
        (trace, listed.saturating_sub(from))
    }
}

/// FR-502, FR-820: [`admit_progress`] for a carried state. When the ledger
/// the run starts from is the one the carried admission returned, or the
/// one it started from, only the positions after the ones it admitted are
/// checked and added; the result equals the whole trace's admission.
/// Returns the ledger and the number of positions admitted.
fn admit_carried_progress(
    ledger: &ProgressLedger,
    binding: &LedgerBinding,
    trace: &TraceInput,
    carry: &mut CarriedAdmission,
    covered_positions: u64,
) -> Result<(ProgressLedger, usize), Box<ClauseDisposition>> {
    let retained = ledger.entry(binding);
    let listed = trace.listed();
    let (from, covered) = match carry.ledger.as_ref() {
        Some(held)
            if (same_entry(retained, Some(&held.output))
                || same_entry(retained, held.start.as_ref()))
                && held.admitted <= listed
                && held
                    .admitted
                    .checked_sub(1)
                    .map_or(held.last.is_none(), |at| {
                        trace.position_at(at).as_ref() == held.last.as_ref()
                    }) =>
        {
            (held.admitted, held.output.covered_list().clone())
        }
        _ => (
            0,
            retained
                .map(|entry| entry.covered_list().clone())
                .unwrap_or_default(),
        ),
    };
    let Some((assertion, positions)) = trace.progress_from(from) else {
        return Err(Box::new(refused("invalid-value", BTreeMap::new())));
    };
    let updated = ledger
        .admit_continued(binding, assertion, covered, &positions, covered_positions)
        .map_err(|refusal| match refusal {
            Refusal::Contradiction(contradiction) => Box::new(
                ClauseDisposition::ProgressContradiction(Box::new((*contradiction).into())),
            ),
            Refusal::LimitReached(reached) => Box::new(ClauseDisposition::Temporal(
                TemporalVerdict::ProgressLimitReached(reached),
            )),
        })?;
    carry.ledger = updated.entry(binding).map(|output| LedgerCarry {
        start: retained.cloned(),
        output: output.clone(),
        admitted: listed,
        last: listed.checked_sub(1).and_then(|at| trace.position_at(at)),
    });
    Ok((updated, positions.len()))
}

/// Why `over` did not resolve in the observation at position 0.
#[derive(Clone, Debug)]
pub(crate) enum OverMiss {
    /// The population universe of the `over` type did not resolve.
    Universe(AdmissionFailure),
    /// The observation holds no object of that key.
    Missing,
    /// The clause's `over` parameter is not of a reference type.
    NotReference,
}

impl OverMiss {
    /// The disposition FR-330 reports for the miss of `over`.
    pub(crate) fn disposition(&self, over: &OverObject) -> ClauseDisposition {
        match self {
            Self::Universe(failure) => ClauseDisposition::Admit(failure.clone()),
            Self::NotReference => ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal",
                    "over-is-a-reference",
                ),
            ),
            Self::Missing => refused(
                "wrong-role-mapping",
                BTreeMap::from([
                    ("population", over.population.clone()),
                    ("object", over.key.clone()),
                ]),
            ),
        }
    }
}

/// FR-330 Inputs: `over`, the object of the reference type `over_type`,
/// resolved in `environment`, the observation at position 0.
pub(crate) fn resolve_over(
    run: &CompiledRun<'_>,
    over_type: &ValueType,
    over: &OverObject,
    environment: Option<&ObjectEnvironment>,
) -> Result<filament_exact::ObjectReference, OverMiss> {
    let ValueType::Reference(type_identity) = over_type else {
        return Err(OverMiss::NotReference);
    };
    let universe = population_universe_for(
        run.package.graph().model_inputs(),
        run.packages,
        run.model_limits,
        type_identity.clone(),
    )
    .map_err(OverMiss::Universe)?;
    environment
        .and_then(|environment| environment.objects().find(universe, &over.key))
        .cloned()
        .ok_or(OverMiss::Missing)
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
    run_package_temporal_bounded(request, fsl_eval::temporal::DEFAULT_COVERED_POSITIONS)
}

/// [`run_package_temporal`] under the system setting
/// `progress.covered_positions` (FR-255): the most positions, each record
/// sample's content fingerprint with it (FR-993), the ledger covers.
pub fn run_package_temporal_bounded(
    request: &PackageTemporalRequest<'_>,
    covered_positions: u64,
) -> ClauseRunReport {
    run_package_temporal_over(request, None, covered_positions)
}

/// [`run_package_temporal`] reading `tape` in place of the record samples the
/// trace names (FR-820).
pub(crate) fn run_package_temporal_over(
    request: &PackageTemporalRequest<'_>,
    tape: Option<TapeInput<'_>>,
    covered_positions: u64,
) -> ClauseRunReport {
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
        package_name: request.package.emitted_package_name().unwrap_or_default(),
        sources: request.sources,
        // FR-516: the capacity is the system setting
        // `progress.covered_positions` (FR-255), not a request member.
        progress_capacity: covered_positions,
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
            valuation: false,
            tape,
        },
    )
}

/// FR-820: FR-330's report of the sampled clause `node` over a `Finite` trace
/// of `len` positions, from `observed`, the run a carried stream reached in
/// place of the batch evaluation, and `usage`, the work it charged. `trace`
/// builds the trace's positions, read only for a counterexample.
pub(crate) fn streamed_sample_report(
    package: &fsl_package::CheckedPackage,
    sources: &[Source],
    node: &fsl_semantics::check::TemporalClauseNode,
    observed: fsl_eval::temporal::ObservedRun,
    usage: ClauseRunUsage,
    trace: impl FnOnce() -> TraceInput,
) -> ClauseRunReport {
    let stopped = match (observed.stopped, &observed.result) {
        (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
        (stopped, _) => stopped,
    };
    let (disposition, refused_at) =
        observed_disposition(stopped, observed.result, package.graph(), sources);
    let needs_positions = matches!(
        &disposition,
        ClauseDisposition::Temporal(
            TemporalVerdict::Violated { .. } | TemporalVerdict::Undefined { .. }
        )
    );
    let positions = if needs_positions {
        trace()
    } else {
        TraceInput::Finite(Vec::new())
    };
    let (disposition, counterexample) =
        with_counterexample(node, &positions, disposition, observed.undefined);
    ClauseRunReport {
        basis: match &disposition {
            ClauseDisposition::Temporal(verdict) => verdict.basis(),
            _ => SettlementBasis::Unavailable,
        },
        witness: None,
        state_clause_counterexample: None,
        frame_field_values: None,
        consequent_source_events: if disposition.truth().is_some() {
            ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
        } else {
            ConsequentSourceEvents::Unavailable
        },
        disposition,
        usage,
        ledger: None,
        finite_window: None,
        counterexample,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at,
    }
}

/// FR-330 Outputs: a trace-subject violation, and an `Undefined` that is a
/// violation, carries its counterexample: the samples of `trace` are its
/// steps, as for the other sample routes.
pub(crate) fn with_subject_counterexample(
    node: &fsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    report: &mut ClauseRunReport,
    first_undefined: Option<FirstUndefined>,
) {
    let needs = match &report.disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. }) => true,
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { cause, .. }) => {
            !matches!(cause, TemporalUndefinedCause::InsufficientData { .. })
        }
        _ => false,
    };
    if needs {
        match counterexample(node, trace, &report.disposition, first_undefined) {
            Some(built) => report.counterexample = Some(built),
            None => {
                report.disposition = ClauseDisposition::EvaluateFault(
                    filament_semantic_value::catalog::InternalFault::new(
                        "temporal",
                        "violation-has-its-counterexample",
                    ),
                );
            }
        }
    }
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
        valuation,
        tape,
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
        state_clause_counterexample: None,
        frame_field_values: None,
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
    // FR-931: a valuation run with an empty finite trace asks for the
    // checked clause alone.
    if valuation && matches!(trace, TraceInput::Finite(positions) if positions.is_empty()) {
        return early(ClauseDisposition::TemporalValuation(Box::new(
            super::TemporalValuation {
                clause: clause.clone(),
                positions: None,
            },
        )));
    }
    let site = ClauseSite {
        package: run.package,
        node,
    };
    let authenticated_temporal_selection = match authenticate_trace(site, trace, authentication) {
        Ok(selection) => selection,
        Err(disposition) => return early(*disposition),
    };
    // FR-330-AC-18: past admission a record sample is its index. A carried
    // state names its samples by index already.
    let listed = trace;
    let renamed =
        (tape.is_none() && over.is_none() && matches!(clause.over().1, ValueType::Composite(_)))
            .then(|| indexed_samples(trace))
            .transpose();
    let renamed = match renamed {
        Ok(renamed) => renamed.flatten(),
        Err(fault) => return early(ClauseDisposition::EvaluateFault(fault)),
    };
    let trace = renamed.as_ref().unwrap_or(listed);

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
        if let Err(refusal) = super::late_data::admit_intervals(observations, &window.late) {
            return early(*refusal);
        }
    }
    // FR-501 "Late-data cutoff": the evaluator reads the on-time
    // observations only.
    let screened = match trace {
        TraceInput::Timestamped {
            observations,
            progress,
            window,
        } => match super::late_data::screen(observations, *progress, window) {
            Ok(screened) => screened,
            Err(refusal) => return early(*refusal),
        },
        _ => None,
    };
    let trace = screened.as_ref().map_or(trace, |screened| &screened.trace);

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
    // FR-820: the positions this run admitted into its trace and its ledger.
    let mut trace_positions = 0usize;
    let mut ledger_positions = 0usize;
    // `Some(None)`: a carried state's trace, held in its admission.
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
            let facts = if tape.is_some_and(|tape| tape.open_aggregates) {
                fsl_eval::temporal::evidence::ClauseFacts::of_checked_letters(clause)
            } else {
                fsl_eval::temporal::evidence::ClauseFacts::of_checked(clause)
            };
            // FR-820: a carried state's trace admits only the positions added
            // since its last run.
            let admission = tape.and_then(|tape| tape.admission);
            let mut carried = admission.map(std::cell::RefCell::borrow_mut);
            let admitted_trace = match carried.as_deref_mut() {
                Some(carried) => {
                    let (held, count) = carried.continue_trace(
                        documents.len(),
                        |held, observed| {
                            observed.checked_sub(1).is_none_or(|last| {
                                held.positions().get(last).map(|p| p.observation.as_str())
                                    == documents.get(last).map(observation_identity).as_deref()
                            })
                        },
                        || TimestampedTrace::open_progressed(&facts),
                    );
                    trace_positions = count;
                    let from = documents.len().saturating_sub(count);
                    let identities = documents
                        .get(from..)
                        .unwrap_or(&[])
                        .iter()
                        .map(observation_identity)
                        .collect();
                    held.extend_progressed(&facts, identities, assertion, &mut meter)
                        .map(|extended| extended.map(|()| None))
                }
                None => {
                    trace_positions = documents.len();
                    let identities = documents.iter().map(observation_identity).collect();
                    TimestampedTrace::admit_progressed(&facts, identities, assertion, &mut meter)
                        .map(|admitted| admitted.map(Some))
                }
            };
            drop(carried);
            match admitted_trace {
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
    // FR-501 "Late-data cutoff": the samples are the evaluated positions',
    // in their canonical order, so an excluded late observation is not one.
    let kept_listed;
    let sampled_from = match (listed, screened.as_ref().map(|screened| &screened.trace)) {
        (
            TraceInput::Timestamped { observations, .. },
            Some(TraceInput::Timestamped {
                progress, window, ..
            }),
        ) => {
            let kept = screened
                .as_ref()
                .map_or(&[][..], |screened| &screened.kept[..]);
            kept_listed = TraceInput::Timestamped {
                observations: kept
                    .iter()
                    .filter_map(|position| observations.get(*position).cloned())
                    .collect(),
                progress: *progress,
                window: window.clone(),
            };
            &kept_listed
        }
        _ => listed,
    };
    if let (true, ValueType::Composite(record), None) = (sampled, over_type, tape) {
        for (index, document) in sampled_from.documents().enumerate() {
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
    // FR-820: a carried snapshot state admitted each snapshot once; the
    // first that failed is the report, as the batch admission's.
    if let Some(failure) = tape
        .and_then(|tape| tape.snapshot)
        .and_then(|tape| tape.failure)
    {
        return report(
            ClauseDisposition::Admit(failure.clone()),
            ClauseRunUsage::default(),
            authenticated_temporal_selection.clone(),
        );
    }
    let carried_snapshots = tape.is_some_and(|tape| tape.snapshot.is_some());
    for document in trace.documents().filter(|_| !sampled && !carried_snapshots) {
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
    let resolved_over;
    let observing = match (over, tape) {
        (None, Some(tape)) => ObservedPositions::Tape(tape.tape),
        (None, None) => ObservedPositions::Samples(&samples),
        (
            Some(over),
            Some(TapeInput {
                tape,
                snapshot: Some(snapshot),
                ..
            }),
        ) => match snapshot.over {
            Ok(reference) => ObservedPositions::ObjectTape {
                tape,
                over: reference,
                from: 0,
            },
            Err(miss) => return late(miss.disposition(over)),
        },
        (Some(over), _) => {
            resolved_over = match resolve_over(run, over_type, over, environments.first()) {
                Ok(reference) => reference,
                Err(miss) => return late(miss.disposition(over)),
            };
            ObservedPositions::Objects {
                environments: &environments,
                over: &resolved_over,
            }
        }
    };
    // The snapshot positions a counterexample and a decisive record read; a
    // record sample has neither.
    let objects = match observing {
        ObservedPositions::Objects { environments, over } => Some((environments, over)),
        ObservedPositions::Samples(_)
        | ObservedPositions::Tape(_)
        | ObservedPositions::ObjectTape { .. } => None,
    };
    let snapshot = tape.and_then(|tape| tape.snapshot);

    // FR-757 "Cross-evaluation": a valuation run reads every atom and
    // activation of the admitted closed finite trace through the one clause
    // evaluator, in place of the verdict.
    if valuation {
        return match fsl_eval::temporal::observed_valuation(
            run.package,
            node,
            observing,
            shape,
            &mut meter,
        ) {
            Ok(positions) => late(ClauseDisposition::TemporalValuation(Box::new(
                super::TemporalValuation {
                    clause: clause.clone(),
                    positions: Some(positions),
                },
            ))),
            Err(fsl_eval::temporal::ValuationStop::NotFinite) => early(refused(
                "invalid-value",
                BTreeMap::from([("member", "trace".to_owned())]),
            )),
            Err(_) => late(ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal",
                    "valuation-reads-every-atom-and-activation",
                ),
            )),
        };
    }

    // QSL-2138: only the closed finite route below reads window aggregates.
    if sampled
        && !tape.is_some_and(|tape| tape.open_aggregates)
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
            match tape {
                Some(tape) => super::trace_subject::SubjectSamples::Tape {
                    tape: tape.tape,
                    values: tape.values,
                    captures: tape.captures,
                    len: u64::try_from(tape.tape.len()).unwrap_or(u64::MAX),
                },
                None => super::trace_subject::SubjectSamples::Values(&samples),
            },
            fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
            cancel,
            &mut first_undefined,
        );
        // A violation, and an `Undefined` that is a violation, carries its
        // counterexample (FR-330 Outputs): the samples are its steps, as for
        // the other sample routes.
        with_subject_counterexample(node, trace, &mut delegated, first_undefined);
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
        // A trace whose positions have no stamp in `u64` lists no progress.
        if !trace.carries_progress() || u64::try_from(trace.listed()).is_err() {
            return early(refused("invalid-value", BTreeMap::new()));
        }
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
        accepted_ledger = match tape.and_then(|tape| tape.admission) {
            Some(admission) => match admit_carried_progress(
                ledger,
                &binding,
                trace,
                &mut admission.borrow_mut(),
                run.progress_capacity,
            ) {
                Ok((updated, count)) => {
                    ledger_positions = count;
                    Some(updated)
                }
                Err(disposition) => return late(*disposition),
            },
            None => {
                let Some((assertion, positions)) = trace.progress() else {
                    return early(refused("invalid-value", BTreeMap::new()));
                };
                ledger_positions = positions.len();
                match admit_progress(
                    ledger,
                    &binding,
                    assertion,
                    &positions,
                    run.progress_capacity,
                ) {
                    Ok(updated) => Some(updated),
                    Err(disposition) => return late(*disposition),
                }
            }
        };
    }

    if let Some(owned) = progressed {
        let held = tape
            .and_then(|tape| tape.admission)
            .map(std::cell::RefCell::borrow);
        let Some(window_trace) = owned
            .as_ref()
            .or_else(|| held.as_ref().and_then(|held| held.trace.as_ref()))
        else {
            return late(ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal",
                    "carried-progressed-trace-is-held",
                ),
            ));
        };
        let silent_from = u64::try_from(window_trace.observed()).ok();
        let window = run_window(
            run,
            node,
            window_trace,
            (&[], silent_from),
            observing,
            tape.and_then(|tape| tape.window),
            &mut meter,
        );
        drop(held);
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
            None => match (snapshot, &disposition) {
                (Some(snapshot), ClauseDisposition::Temporal(TemporalVerdict::Violated { at })) => {
                    let len = match shape {
                        TemporalTrace::Finite { len } => len,
                        TemporalTrace::Lasso { prefix, loop_len } => {
                            prefix.saturating_add(loop_len)
                        }
                    };
                    let seeking = (snapshot.descend)(*at, len, silent_from, &mut meter)
                        .map(progressed_search);
                    with_decisive_by(
                        seeking,
                        disposition,
                        counterexample,
                        |atom, meter| (snapshot.record)(run, atom, meter),
                        &mut meter,
                    )
                }
                _ => (disposition, counterexample, None),
            },
        };
        let accepted = !matches!(
            disposition.category(),
            filament_semantic_value::catalog::Category::Refusal
                | filament_semantic_value::catalog::Category::InternalFailure
        );
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = window.visits;
        usage.trace_positions = u64::try_from(trace_positions).unwrap_or(u64::MAX);
        usage.ledger_positions = u64::try_from(ledger_positions).unwrap_or(u64::MAX);
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
        let window = run_finite_window(
            run,
            node,
            (&view, screened.as_ref()),
            observing,
            (
                tape.and_then(|tape| tape.window),
                tape.and_then(|tape| tape.admission),
            ),
            &mut trace_positions,
            &mut meter,
        );
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
        usage.trace_positions = u64::try_from(trace_positions).unwrap_or(u64::MAX);
        usage.ledger_positions = u64::try_from(ledger_positions).unwrap_or(u64::MAX);
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
        positions @ (ObservedPositions::Samples(_)
        | ObservedPositions::Tape(_)
        | ObservedPositions::ObjectTape { .. }) => fsl_eval::temporal::evaluate_positions(
            run.package,
            node,
            positions,
            shape,
            None,
            &mut meter,
        ),
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
    let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
    usage.ledger_positions = u64::try_from(ledger_positions).unwrap_or(u64::MAX);
    let mut report = report(disposition, usage, authenticated_temporal_selection);
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
    (input, screened): (&Timestamped<'_>, Option<&super::late_data::Screened>),
    observing: ObservedPositions<'_>,
    (carried, admission): (
        Option<&std::cell::RefCell<fsl_eval::temporal::CarriedWindow>>,
        Option<&std::cell::RefCell<CarriedAdmission>>,
    ),
    trace_positions: &mut usize,
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
    let clock = WindowClock {
        unit: clause.clock().to_owned(),
        cover: match input.window.cover.unwrap_or(TimestampedCover::Exact) {
            TimestampedCover::Exact => fsl_eval::temporal::ClockCover::Exact,
            TimestampedCover::Counter { uncertainty } => {
                fsl_eval::temporal::ClockCover::Counter { uncertainty }
            }
            TimestampedCover::Bounded { uncertainty } => {
                fsl_eval::temporal::ClockCover::Bounded { uncertainty }
            }
        },
    };
    let window_observation = |position: &TimestampedObservation| WindowObservation {
        stamp: position.stamp,
        order_key: position
            .order_key
            .as_ref()
            .map(|order| fsl_eval::temporal::OrderKey {
                authority: order.authority.clone(),
                key: order.key,
            }),
        observation: observation_identity(&position.document),
    };
    let progress = WindowProgress {
        watermark: Watermark::Inclusive(input.progress.watermark),
        completeness: input.progress.completeness.into(),
        scope: input.progress.scope.into(),
        execution: input.progress.execution.into(),
        lower_boundary: if input.window.authoritative_origin {
            LowerBoundary::AuthoritativeOrigin
        } else {
            LowerBoundary::HistoryCutoff
        },
    };
    // FR-522 "Result identity": the checked clause identity, the package
    // identity and the declaration, both of which its node key spells.
    let Ok(clause_identity) = fsl_foundation::node_wire::node_key_to_text(&node.identity()) else {
        return fault("clause-node-key-has-a-text-spelling");
    };
    // FR-820: a carried state lists its positions in canonical order, each
    // stamped after the ones before (FR-643 "Order"), and supplies no absent
    // valuation; its trace admits only the positions added since its last
    // run.
    // Interval stamps and a late-data cutoff take the whole-trace
    // evaluation: a carried state supplies neither.
    let plain = input.window.missing_valuations.is_empty()
        && screened.is_none()
        && input.window.late.intervals.iter().all(Option::is_none)
        && input.window.late.arrivals.iter().all(Option::is_none);
    if let (Some(admission), true) = (admission, plain) {
        let observations = input.observations;
        let mut held = admission.borrow_mut();
        let (trace, count) = held.continue_trace(
            observations.len(),
            |held, observed| {
                observed.checked_sub(1).is_none_or(|last| {
                    match (held.positions().get(last), observations.get(last)) {
                        (Some(held), Some(listed)) => *held == window_observation(listed),
                        _ => false,
                    }
                })
            },
            || TimestampedTrace::open(clause_identity.clone(), clock.clone(), progress),
        );
        *trace_positions = count;
        let from = observations.len().saturating_sub(count);
        trace.extend(
            observations
                .get(from..)
                .unwrap_or(&[])
                .iter()
                .map(window_observation)
                .collect(),
            progress,
        );
        drop(held);
        let held = admission.borrow();
        let Some(trace) = held.trace.as_ref() else {
            return fault("carried-timestamped-trace-is-held");
        };
        return run_window(run, node, trace, (&[], None), observing, carried, meter);
    }
    *trace_positions = input.observations.len();
    let ordered = ordered_timestamped_positions(input);
    let late = &input.window.late;
    let trace = match TimestampedTrace::admit_uncertain(
        clause_identity,
        clock,
        ordered
            .iter()
            .map(|(index, position)| {
                (
                    window_observation(position),
                    super::late_data::interval_at(late, *index),
                )
            })
            .collect(),
        progress,
    ) {
        Ok(trace) => trace,
        Err(_) => return fault("finite-window-trace-admits"),
    };
    // FR-966 "Identity": each read observation's arrival watermark, in
    // canonical order, and the late-data cutoff and classification.
    let mut lateness = screened.map_or_else(Default::default, |screened| screened.lateness.clone());
    lateness.arrivals = ordered
        .iter()
        .map(|(index, _)| late.arrivals.get(*index).copied().flatten())
        .collect();
    let trace = trace.with_lateness(lateness);
    // The valuation lists name positions by listing index; the evaluator reads
    // canonical positions.
    let canonical = |valuations: &[TimestampedMissingValuation]| -> Vec<(u64, u64)> {
        valuations
            .iter()
            .filter_map(|valuation| {
                let listed = usize::try_from(valuation.position).ok()?;
                let position = ordered.iter().position(|(index, _)| *index == listed)?;
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
    let mut window = run_window(
        run,
        node,
        &trace,
        (&absent, None),
        observing,
        carried,
        meter,
    );
    if let (Some(screened), Some(report)) = (screened, window.report.as_mut()) {
        let reach = match fsl_eval::temporal::window_reach(
            &fsl_eval::temporal::evidence::ClauseFacts::of_checked(clause),
        ) {
            Ok(reach) => reach,
            Err(_) => return fault("finite-window-reach-fits"),
        };
        let stamps: Vec<u64> = trace
            .positions()
            .iter()
            .map(|position| position.stamp)
            .collect();
        report.late = screened.late.as_ref().map(|entries| {
            entries
                .iter()
                .map(|entry| LateEntry {
                    anchors: super::late_data::anchors(
                        &clause.activation(),
                        &stamps,
                        reach,
                        entry.stamp,
                    ),
                    ..entry.clone()
                })
                .collect()
        });
        report.admitted_late.clone_from(&screened.admitted);
        report.unmatched_admissions.clone_from(&screened.unmatched);
    }
    window
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
    carried: Option<&std::cell::RefCell<fsl_eval::temporal::CarriedWindow>>,
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
        positions @ (ObservedPositions::Samples(_)
        | ObservedPositions::Tape(_)
        | ObservedPositions::ObjectTape { .. }) => match carried {
            Some(carried) => carried.borrow_mut().evaluate(
                run.package,
                node,
                positions,
                trace,
                valuations,
                meter,
            ),
            None => fsl_eval::temporal::evaluate_positions_finite_window(
                run.package,
                node,
                positions,
                trace,
                valuations,
                meter,
            ),
        },
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
    with_decisive_by(
        search,
        disposition,
        counterexample,
        |atom, meter| decisive_record(source, atom, meter),
        meter,
    )
}

/// [`with_decisive`] reading the decisive record of the atom the search
/// found through `record` (FR-820: a carried snapshot state reads it from the
/// snapshot it holds).
pub(crate) fn with_decisive_by(
    search: Option<DecisiveSearch>,
    disposition: ClauseDisposition,
    counterexample: Option<crate::ObservedTemporalCounterexample>,
    record: impl FnOnce(
        DecisiveAtom,
        &mut Meter,
    ) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>>,
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
    match record(atom, meter) {
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
    // FR-330-AC-18: a record sample is cited by its index in the trace, a
    // snapshot by its labels.
    let step = |index: usize, document: &DocumentRef| ObservedTemporalStep {
        document: if document.authority == SAMPLE_AUTHORITY {
            DocumentRef {
                authority: SAMPLE_AUTHORITY.to_owned(),
                identity: index.to_string(),
            }
        } else {
            document.clone()
        },
    };
    match disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) => {
            Some(ObservedTemporalCounterexample {
                prefix: documents
                    .iter()
                    .enumerate()
                    .map(|(index, document)| step(index, document))
                    .collect(),
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
                .map(|index| {
                    let slot = slot(index);
                    documents.get(slot).map(|document| step(slot, document))
                })
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
                ClockCover::Bounded { uncertainty } => TimestampedCover::Bounded { uncertainty },
            },
            unit: premises.unit,
        },
        identity: identity.0,
        late: None,
        admitted_late: Vec::new(),
        unmatched_admissions: Vec::new(),
        correction: None,
        key: evidence_key(run, node)?,
    })
}

/// FR-975: the obligation instance of the clause `node` in a run: the
/// package, the declaration node key's canonical text and the selected
/// occurrence, with no trigger (a run request carries no trigger delivery).
fn evidence_key(
    run: &CompiledRun<'_>,
    node: &fsl_semantics::check::TemporalClauseNode,
) -> Result<super::correction::EvidenceKey, Box<RunRefusal>> {
    let clause = fsl_foundation::node_wire::node_key_to_text(&node.identity()).map_err(|_| {
        Box::new(RunRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "temporal",
                "clause-node-key-has-a-text-spelling",
            ),
        ))
    })?;
    Ok(super::correction::EvidenceKey::new(
        run.package_name.to_owned(),
        clause,
        node.claim().role().as_str().to_owned(),
        node.claim().ordinal(),
    ))
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
    /// A point stamp `t` covers the closed interval `[t-u, t+u]` (FR-501
    /// "Bounded cover and interval stamps").
    Bounded {
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
    /// FR-501 "Late-data cutoff": the observations the cutoff excluded, in
    /// input order; `None` when no cutoff is selected.
    pub late: Option<Vec<LateEntry>>,
    /// FR-526: the late observations a superseding repair admitted.
    pub admitted_late: Vec<AdmittedLate>,
    /// The `admit_late` names that match no late observation, which the
    /// run's correction refuses `admitted-not-late` (FR-526).
    pub unmatched_admissions: Vec<String>,
    /// FR-526: the run's place in a correction chain, once the run settles
    /// its relation.
    pub correction: Option<super::correction::CorrectionRecord>,
    /// FR-975: the obligation instance the result is of, the key of its
    /// evidence ledger entry.
    pub key: super::correction::EvidenceKey,
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
