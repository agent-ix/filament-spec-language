// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-757, FR-931: the `tl_lowering` request member. A native temporal
//! clause is classified under FR-527's table, lowered to a `tl-syntax`
//! formula under the selected versioned correspondence, and, when asked,
//! cross-evaluated by `tl-mltl` against the native evaluator's verdict at
//! every activation position. This module holds ADR-011's only edges to the
//! TL crates.

use std::collections::BTreeSet;

use fsl_replay::spine::{TemporalValuation, TlClosure, TlSupport, TlTarget, TlUnmatched};
use fsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, FormulaNode, FormulaNodeId, UnaryTemporal,
};
use serde_json::{json, Map, Value};
use tl_syntax::{
    Formula, InfiniteClock, InfiniteFormulaDocument, InfiniteNode, InfiniteNodeKind, Interval,
    LassoTraceDocument, Node, NodeId, NodeKind, PartialValuation, PartialValue, PropositionId,
    SemanticProfile, TemporalFamily, TemporalInterval, TraceObservation, ValuationEntry,
};

use super::{Result, RunCause};

mod canonical;

/// The four correspondence identities a request may name (FR-931).
const CORRESPONDENCES: [TlTarget; 4] = [
    TlTarget::ClosedTrace,
    TlTarget::OnlinePrefix,
    TlTarget::OriginCompleteHistory,
    TlTarget::InfiniteTrace,
];

/// A decoded `tl_lowering` member.
#[derive(Clone, Debug)]
pub(super) struct TlLowering {
    /// The temporal clause's declared name.
    pub(super) clause: String,
    /// The named correspondence, if any.
    correspondence: Option<TlTarget>,
    /// What to cross-evaluate over, if anything.
    pub(super) cross: Option<CrossEvaluate>,
}

/// The `cross_evaluate` member's form.
#[derive(Clone, Debug)]
pub(super) enum CrossEvaluate {
    /// A closed finite trace or a lasso, read as FR-312 reads a `temporal`
    /// selection's `trace` and `over`.
    Trace {
        /// The `trace` member.
        trace: Value,
        /// The `over` member, if any.
        over: Option<Value>,
        /// Whether the trace is a `lasso` (an infinite trace) rather than a
        /// `finite` one.
        lasso: bool,
    },
    /// Every finite behaviour of the subject's model from the snapshot
    /// `initial`, of 1 to `max_steps` steps.
    Initial {
        /// The snapshot file.
        initial: String,
        /// The longest behaviour, in steps.
        max_steps: u64,
    },
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: format!("tl_lowering.{member}"),
        reason: reason.into(),
    }
}

/// Decode `value`, the request's `tl_lowering` member, under FR-026's
/// closed-envelope rules (FR-931 "The member").
pub(super) fn decode(value: &Value) -> Result<TlLowering> {
    let Value::Object(members) = value else {
        return Err(RunCause::ClauseRequest {
            member: "tl_lowering".to_owned(),
            reason: "expected an object".to_owned(),
        });
    };
    if let Some(unknown) = members
        .keys()
        .find(|key| !matches!(key.as_str(), "clause" | "correspondence" | "cross_evaluate"))
    {
        return Err(invalid(unknown, "unknown member"));
    }
    let clause = match members.get("clause") {
        Some(Value::String(name)) => name.clone(),
        Some(_) => return Err(invalid("clause", "expected a string")),
        None => return Err(invalid("clause", "missing member")),
    };
    let correspondence = match members.get("correspondence") {
        None => None,
        Some(Value::String(identity)) => Some(
            CORRESPONDENCES
                .into_iter()
                .find(|target| target.identity() == identity)
                .ok_or_else(|| RunCause::TlCorrespondence {
                    value: identity.clone(),
                })?,
        ),
        Some(_) => return Err(invalid("correspondence", "expected a string")),
    };
    let cross = match members.get("cross_evaluate") {
        None => None,
        Some(Value::Object(cross)) => Some(decode_cross(cross)?),
        Some(_) => return Err(invalid("cross_evaluate", "expected an object")),
    };
    // FR-931 "Cross-evaluation": the trace form matches the correspondence
    // when one is named; an unnamed one is checked after classification.
    if let (Some(target), Some(cross)) = (correspondence, &cross) {
        if let Some(member) = TraceForm::of_request(cross).mismatch(target) {
            return Err(invalid(member, TraceForm::reason(target)));
        }
    }
    Ok(TlLowering {
        clause,
        correspondence,
        cross,
    })
}

/// The trace form a lowering cross-evaluates over.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceForm {
    /// No cross-evaluation.
    Absent,
    /// A closed finite trace.
    Finite,
    /// A lasso.
    Lasso,
    /// Every finite behaviour of a model subject.
    Initial,
}

impl TraceForm {
    fn of_request(cross: &CrossEvaluate) -> Self {
        match cross {
            CrossEvaluate::Trace { lasso: true, .. } => Self::Lasso,
            CrossEvaluate::Trace { lasso: false, .. } => Self::Finite,
            CrossEvaluate::Initial { .. } => Self::Initial,
        }
    }

    fn of_valuation(valuation: &TemporalValuation) -> Self {
        match (&valuation.positions, &valuation.behaviours) {
            (Some(positions), _) => match positions.trace {
                fsl_eval::temporal::TemporalTrace::Lasso { .. } => Self::Lasso,
                fsl_eval::temporal::TemporalTrace::Finite { .. } => Self::Finite,
            },
            (None, Some(_)) => Self::Initial,
            (None, None) => Self::Absent,
        }
    }

    /// The request member this form is refused at beside `target`, when the
    /// correspondence does not read it: `mltl.infinite-trace/v1` reads a
    /// lasso only, the other three a finite trace or the `initial` form.
    fn mismatch(self, target: TlTarget) -> Option<&'static str> {
        match (target, self) {
            (_, Self::Absent) => None,
            (TlTarget::InfiniteTrace, Self::Lasso) => None,
            (TlTarget::InfiniteTrace, Self::Finite) => Some("cross_evaluate.trace"),
            (TlTarget::InfiniteTrace, Self::Initial) => Some("cross_evaluate.initial"),
            (_, Self::Lasso) => Some("cross_evaluate.trace"),
            (_, Self::Finite | Self::Initial) => None,
        }
    }

    fn reason(target: TlTarget) -> &'static str {
        match target {
            TlTarget::InfiniteTrace => "mltl.infinite-trace/v1 reads a lasso trace only",
            TlTarget::ClosedTrace | TlTarget::OnlinePrefix | TlTarget::OriginCompleteHistory => {
                "the correspondence reads a finite trace, not a lasso"
            }
        }
    }
}

fn decode_cross(cross: &Map<String, Value>) -> Result<CrossEvaluate> {
    if let Some(unknown) = cross
        .keys()
        .find(|key| !matches!(key.as_str(), "trace" | "over" | "initial" | "max_steps"))
    {
        return Err(invalid(
            &format!("cross_evaluate.{unknown}"),
            "unknown member",
        ));
    }
    match (cross.get("trace"), cross.get("initial")) {
        (Some(_), Some(_)) => Err(invalid(
            "cross_evaluate",
            "give exactly one of trace and initial, not both",
        )),
        (None, None) => Err(invalid(
            "cross_evaluate",
            "give exactly one of trace and initial",
        )),
        (Some(trace), None) => {
            if cross.contains_key("max_steps") {
                return Err(invalid(
                    "cross_evaluate.max_steps",
                    "max_steps is given with initial only",
                ));
            }
            let lasso = match trace {
                Value::Object(kinds) if kinds.len() == 1 && kinds.contains_key("finite") => false,
                Value::Object(kinds) if kinds.len() == 1 && kinds.contains_key("lasso") => true,
                _ => {
                    return Err(invalid(
                        "cross_evaluate.trace",
                        "only a finite trace or a lasso is cross-evaluated",
                    ));
                }
            };
            Ok(CrossEvaluate::Trace {
                trace: trace.clone(),
                over: cross.get("over").cloned(),
                lasso,
            })
        }
        (None, Some(initial)) => {
            if cross.contains_key("over") {
                return Err(invalid(
                    "cross_evaluate.over",
                    "over is given with trace only",
                ));
            }
            let Value::String(file) = initial.clone() else {
                return Err(invalid(
                    "cross_evaluate.initial",
                    "expected a snapshot file name",
                ));
            };
            if file.is_empty() {
                return Err(invalid(
                    "cross_evaluate.initial",
                    "expected a snapshot file name",
                ));
            }
            match cross.get("max_steps").and_then(Value::as_u64) {
                Some(steps) if steps >= 1 => Ok(CrossEvaluate::Initial {
                    initial: file,
                    max_steps: steps,
                }),
                _ => Err(invalid(
                    "cross_evaluate.max_steps",
                    "expected an integer of at least 1",
                )),
            }
        }
    }
}

/// The observations key the `initial` snapshot is read under.
pub(super) const INITIAL_KEY: &str = "initial";

/// The FR-312 clause selection a lowering reads its clause and valuation
/// through: the trace to cross-evaluate over, or an empty finite trace,
/// which asks for the checked clause alone.
pub(super) fn selection(lowering: &TlLowering) -> Value {
    let mut temporal = json!({"name": lowering.clause, "trace": {"finite": []}});
    if let Some(CrossEvaluate::Trace { trace, over, .. }) = &lowering.cross {
        temporal["trace"] = trace.clone();
        if let Some(over) = over {
            temporal["over"] = over.clone();
        }
    }
    let observations = match &lowering.cross {
        Some(CrossEvaluate::Initial { initial, .. }) => json!({INITIAL_KEY: initial}),
        _ => json!({}),
    };
    json!({"selection": {"temporal": temporal}, "observations": observations})
}

/// How the lowering maps TL atoms to native `holds` leaves. The request path
/// always maps each atom to its own leaf; a library caller may name atoms to
/// map to the negated leaf, which FR-757-AC-2 uses to show that a mis-mapped
/// atom table is reported as a correspondence loss.
#[derive(Clone, Debug, Default)]
pub struct AtomMapping {
    /// The TL atom names (`p0`, ...) read as the negation of their leaf.
    pub negated: BTreeSet<String>,
    /// A correspondence identity to stamp on the lowered formula in place of
    /// the one the lowering chose, so that FR-931-AC-8 can show that a
    /// lowered formula whose classification differs from the clause's is a
    /// classification loss.
    pub lowered_correspondence: Option<String>,
}

/// A lowering record and the exit code FR-285 gives it.
#[derive(Clone, Debug)]
pub struct TlRecord {
    /// The `tl_lowering` record of the run result document.
    pub record: Value,
    /// 0 for a supported lowering with no fault, 21 for an unsupported one,
    /// 30 for a correspondence loss.
    pub exit_code: u8,
}

/// The FR-931 record of `valuation`, a valuation run's checked clause and
/// native valuation, lowered under `correspondence` (or the classification's
/// default target), its atoms named by the `holds` leaves' text in `source`
/// and mapped by `mapping`. Cross-evaluation runs when the valuation holds
/// one.
pub fn record(
    valuation: &TemporalValuation,
    source: &str,
    correspondence: Option<&str>,
    mapping: &AtomMapping,
) -> std::result::Result<TlRecord, String> {
    let form = TraceForm::of_valuation(valuation);
    let named = match correspondence {
        None => None,
        Some(identity) => Some(
            CORRESPONDENCES
                .into_iter()
                .find(|target| target.identity() == identity)
                .ok_or_else(|| format!("unknown correspondence {identity}"))?,
        ),
    };
    lowering_record(valuation, source, named, mapping, form).map_err(|failure| match failure {
        Failure::Fault(message) => message,
        Failure::Form { member, reason } => format!("tl_lowering.{member}: {reason}"),
    })
}

/// Why a lowering produced no record.
enum Failure {
    /// A lowering or cross-evaluation fault.
    Fault(String),
    /// The trace form the request cross-evaluates over is not the one the
    /// classified correspondence reads (FR-931), refused at `member`.
    Form {
        /// The `tl_lowering` member, below `tl_lowering.`.
        member: &'static str,
        /// Why it is refused.
        reason: &'static str,
    },
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Fault(message)
    }
}

/// The FR-931-AC-8 fragment of a formula by the direction of its operators.
fn fragment_of(past: bool) -> &'static str {
    if past {
        "bounded-past"
    } else {
        "bounded-future"
    }
}

fn classification(correspondence: &str, fragment: &str) -> Value {
    json!({"correspondence": correspondence, "fragment": fragment})
}

fn direction(target: TlTarget) -> bool {
    matches!(
        target,
        TlTarget::OriginCompleteHistory | TlTarget::InfiniteTrace
    )
}

fn dimension(unmatched: TlUnmatched) -> &'static str {
    match unmatched {
        TlUnmatched::FiniteWindow | TlUnmatched::TimedProfile => "profile",
        TlUnmatched::InfiniteTrace => "infinite-trace",
        TlUnmatched::MixedDirection => "direction",
    }
}

/// Whether `interval` is a closed step interval `[a, b]`.
fn is_closed(interval: Option<&CheckedInterval>) -> bool {
    matches!(
        interval,
        Some(CheckedInterval::Steps(steps))
            if matches!(steps.upper(), fsl_foundation::bound::TemporalUpper::Finite(_))
    )
}

fn lowering_record(
    valuation: &TemporalValuation,
    _source: &str,
    named: Option<TlTarget>,
    mapping: &AtomMapping,
    form: TraceForm,
) -> std::result::Result<TlRecord, Failure> {
    let clause = &valuation.clause;
    let formula = clause.formula();
    let (mut future, mut past, mut unclosed) = (false, false, false);
    for node in formula.nodes() {
        let interval = match node {
            FormulaNode::Unary {
                operator: UnaryTemporal::Eventually | UnaryTemporal::Always,
                interval,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Until | BinaryTemporal::Release,
                interval,
                ..
            } => {
                future = true;
                interval
            }
            FormulaNode::Unary {
                operator: UnaryTemporal::Once | UnaryTemporal::Historically,
                interval,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Since | BinaryTemporal::Triggered,
                interval,
                ..
            } => {
                past = true;
                interval
            }
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. }
            | FormulaNode::Not(_)
            | FormulaNode::And(..)
            | FormulaNode::Or(..)
            | FormulaNode::Implies(..) => continue,
        };
        unclosed |= !is_closed(interval.as_ref());
    }
    // FR-931 "The correspondence": the member fixes the closure.
    let surrounding = match named {
        Some(TlTarget::OnlinePrefix) => TlClosure::Open,
        Some(TlTarget::ClosedTrace | TlTarget::OriginCompleteHistory | TlTarget::InfiniteTrace)
        | None => TlClosure::Complete,
    };
    let support =
        fsl_replay::spine::tl_support_of(clause.profile(), future, past, unclosed, surrounding);
    let mut record = Map::new();
    record.insert("clause".to_owned(), json!(clause.name()));
    let classified = match support {
        TlSupport::Unsupported { dimensions } => {
            record.insert("disposition".to_owned(), json!("unsupported"));
            if let Some(target) = named {
                record.insert("correspondence".to_owned(), json!(target.identity()));
            }
            let dimensions: Vec<&str> = dimensions.into_iter().map(dimension).collect();
            record.insert("dimensions".to_owned(), json!(dimensions));
            return Ok(TlRecord {
                record: Value::Object(record),
                exit_code: 21,
            });
        }
        TlSupport::Supported { target, .. } => target,
    };
    if let Some(target) = named.filter(|target| *target != classified) {
        record.insert("disposition".to_owned(), json!("unsupported"));
        record.insert("correspondence".to_owned(), json!(target.identity()));
        record.insert("classified".to_owned(), json!(classified.identity()));
        // A correspondence that reads the other kind of trace differs by
        // the profile; otherwise by the operator direction or the closure.
        let dimension =
            if (target == TlTarget::InfiniteTrace) != (classified == TlTarget::InfiniteTrace) {
                "profile"
            } else if direction(target) == direction(classified) {
                "closure"
            } else {
                "direction"
            };
        record.insert("dimensions".to_owned(), json!([dimension]));
        return Ok(TlRecord {
            record: Value::Object(record),
            exit_code: 21,
        });
    }
    // FR-931 "Cross-evaluation": the trace form matches the target the
    // classification names, checked here when the request named none.
    if let Some(member) = form.mismatch(classified) {
        return Err(Failure::Form {
            member,
            reason: TraceForm::reason(classified),
        });
    }
    let labels = canonical::source_labels(valuation)?;
    let mut lowered = Lowered::default();
    let root = lowered.lower(formula, formula.root(), &labels)?;
    let tl = lowered.document(classified, root)?;
    record.insert("disposition".to_owned(), json!("supported"));
    record.insert("correspondence".to_owned(), json!(classified.identity()));
    record.insert("formula".to_owned(), json!(display(&lowered.nodes, root)));
    record.insert(
        "atoms".to_owned(),
        Value::Object(
            lowered
                .atoms
                .iter()
                .enumerate()
                .map(|(index, (_, text))| (format!("p{index}"), json!(text)))
                .collect(),
        ),
    );
    let mut exit_code = 0;
    // FR-931-AC-8: the native side is the FR-527 target plus its fragment;
    // the TL side is read from the lowered formula alone, by its profile and
    // the direction of its temporal nodes.
    let native = classification(classified.identity(), fragment_of(past));
    let (lowered_profile, lowered_past) = tl.classification();
    let lowered_profile = mapping
        .lowered_correspondence
        .as_deref()
        .unwrap_or(lowered_profile);
    let lowered_class = classification(lowered_profile, fragment_of(lowered_past));
    if lowered_class == native {
        record.insert("classification".to_owned(), native);
    } else {
        record.insert(
            "classification_fault".to_owned(),
            json!({"kind": "classification-loss", "clause": clause.name(),
                "native": native, "lowered": lowered_class}),
        );
        exit_code = 30;
    }
    let traces: Option<(Vec<&fsl_eval::temporal::ObservedValuation>, bool)> =
        match (&valuation.positions, &valuation.behaviours) {
            (Some(positions), _) => Some((vec![positions], true)),
            (None, Some(behaviours)) => Some((
                behaviours.traces.iter().collect(),
                matches!(behaviours.end, fsl_replay::BehaviourValuationEnd::Complete),
            )),
            (None, None) => None,
        };
    if let Some((traces, complete)) = traces {
        let cross = cross_evaluate(clause.name(), &tl, &lowered, &traces, mapping, classified)?;
        if cross["agree"] != json!(true) {
            exit_code = 30;
        } else if !complete && exit_code == 0 {
            // FR-931: a work-budget stop during enumeration reports the
            // counts reached and exits incomplete.
            exit_code = 22;
        }
        record.insert("cross_evaluation".to_owned(), cross);
    }
    Ok(TlRecord {
        record: Value::Object(record),
        exit_code,
    })
}

/// The lowered formula in the edition its correspondence names.
enum LoweredFormula<'a> {
    /// A bounded-edition formula over a finite trace.
    Bounded(Formula<'a>),
    /// An unbounded-edition formula over a lasso trace.
    Unbounded(InfiniteFormulaDocument),
}

impl LoweredFormula<'_> {
    /// The profile and past-ness the lowered formula carries, read from the
    /// formula alone: its semantic profile, and whether every temporal node
    /// is a past node (with a closed interval in the unbounded edition).
    fn classification(&self) -> (&str, bool) {
        match self {
            Self::Bounded(formula) => (
                formula.profile().as_str(),
                formula
                    .nodes()
                    .iter()
                    .any(|node| node.kind.temporal_family() == Some(TemporalFamily::Past)),
            ),
            Self::Unbounded(document) => (
                document.semantic_profile().as_str(),
                document.nodes().iter().all(|node| {
                    use InfiniteNodeKind as K;
                    match node.kind {
                        K::Once { interval, .. }
                        | K::Historically { interval, .. }
                        | K::Since { interval, .. }
                        | K::Triggered { interval, .. } => {
                            matches!(interval, TemporalInterval::Closed(_))
                        }
                        K::StrongPrevious { .. }
                        | K::False
                        | K::True
                        | K::Proposition { .. }
                        | K::Not { .. }
                        | K::And { .. }
                        | K::Or { .. }
                        | K::Implies { .. }
                        | K::Equivalent { .. } => true,
                        K::Future { .. }
                        | K::Globally { .. }
                        | K::Until { .. }
                        | K::Release { .. } => false,
                    }
                }),
            ),
        }
    }
}

/// A native formula lowered to a `tl-syntax` arena, operands first.
#[derive(Default)]
struct Lowered {
    nodes: Vec<Node>,
    /// Each TL proposition's native `holds` node and its source text, in
    /// proposition order.
    atoms: Vec<(FormulaNodeId, String)>,
}

impl Lowered {
    /// The lowered arena as a `tl-syntax` document of the edition `target`
    /// reads: the bounded edition for the finite-trace targets, the unbounded
    /// edition for the infinite-trace one.
    fn document(
        &self,
        target: TlTarget,
        root: NodeId,
    ) -> std::result::Result<LoweredFormula<'_>, String> {
        let profile = match target {
            TlTarget::ClosedTrace => SemanticProfile::ClosedTraceV1,
            TlTarget::OnlinePrefix => SemanticProfile::OnlinePrefixV1,
            TlTarget::OriginCompleteHistory => SemanticProfile::OriginCompleteHistoryV1,
            TlTarget::InfiniteTrace => {
                return InfiniteFormulaDocument::new(
                    SemanticProfile::InfiniteTraceV1,
                    InfiniteClock::EventPosition,
                    root,
                    self.nodes.iter().map(infinite_node).collect(),
                )
                .map(LoweredFormula::Unbounded)
                .map_err(|error| error.to_string());
            }
        };
        Formula::new(profile, root, &self.nodes)
            .map(LoweredFormula::Bounded)
            .map_err(|error| error.to_string())
    }

    fn push(&mut self, kind: NodeKind) -> std::result::Result<NodeId, String> {
        let id = u32::try_from(self.nodes.len()).map_err(|_| "formula too large".to_owned())?;
        self.nodes.push(Node::new(kind));
        Ok(NodeId(id))
    }

    fn interval(interval: Option<&CheckedInterval>) -> std::result::Result<Interval, String> {
        let Some(CheckedInterval::Steps(steps)) = interval else {
            return Err("a lowered operator carries a closed step interval".to_owned());
        };
        let fsl_foundation::bound::TemporalUpper::Finite(upper) = steps.upper() else {
            return Err("a lowered operator carries a closed step interval".to_owned());
        };
        let bound = |value: u64| u32::try_from(value).map_err(|_| "interval bound too large");
        Interval::new(bound(steps.lower())?, bound(upper)?).map_err(|error| error.to_string())
    }

    /// Lower node `id` of `formula`, each `holds` leaf one proposition
    /// named by its canonical checked-expression label.
    fn lower(
        &mut self,
        formula: &CheckedFormula,
        id: FormulaNodeId,
        labels: &std::collections::BTreeMap<FormulaNodeId, String>,
    ) -> std::result::Result<NodeId, String> {
        let node = formula
            .nodes()
            .get(id.index())
            .ok_or_else(|| "formula node in range".to_owned())?;
        let kind = match node {
            FormulaNode::Constant(true) => NodeKind::True,
            FormulaNode::Constant(false) => NodeKind::False,
            FormulaNode::Holds { .. } => {
                let index = match self.atoms.iter().position(|(atom, _)| *atom == id) {
                    Some(index) => index,
                    None => {
                        let text = labels
                            .get(&id)
                            .ok_or_else(|| "holds atom has a checked condition label".to_owned())?;
                        self.atoms.push((id, text.clone()));
                        self.atoms.len() - 1
                    }
                };
                NodeKind::Proposition {
                    proposition: PropositionId(
                        u32::try_from(index).map_err(|_| "too many atoms".to_owned())?,
                    ),
                }
            }
            FormulaNode::Aggregate { .. } => {
                return Err("a window aggregate atom has no TL atom".to_owned());
            }
            FormulaNode::Not(operand) => NodeKind::Not {
                operand: self.lower(formula, *operand, labels)?,
            },
            FormulaNode::And(left, right) => NodeKind::And {
                left: self.lower(formula, *left, labels)?,
                right: self.lower(formula, *right, labels)?,
            },
            FormulaNode::Or(left, right) => NodeKind::Or {
                left: self.lower(formula, *left, labels)?,
                right: self.lower(formula, *right, labels)?,
            },
            FormulaNode::Implies(left, right) => NodeKind::Implies {
                left: self.lower(formula, *left, labels)?,
                right: self.lower(formula, *right, labels)?,
            },
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let interval = Self::interval(interval.as_ref())?;
                let operand = self.lower(formula, *operand, labels)?;
                match operator {
                    UnaryTemporal::Eventually => NodeKind::Future { interval, operand },
                    UnaryTemporal::Always => NodeKind::Globally { interval, operand },
                    UnaryTemporal::Once => NodeKind::Once { interval, operand },
                    UnaryTemporal::Historically => NodeKind::Historically { interval, operand },
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let interval = Self::interval(interval.as_ref())?;
                let left = self.lower(formula, *left, labels)?;
                let right = self.lower(formula, *right, labels)?;
                match operator {
                    BinaryTemporal::Until => NodeKind::Until {
                        interval,
                        left,
                        right,
                    },
                    BinaryTemporal::Release => NodeKind::Release {
                        interval,
                        left,
                        right,
                    },
                    BinaryTemporal::Since => NodeKind::Since {
                        interval,
                        left,
                        right,
                    },
                    BinaryTemporal::Triggered => NodeKind::Triggered {
                        interval,
                        left,
                        right,
                    },
                }
            }
        };
        self.push(kind)
    }
}

/// The unbounded-edition node of the bounded-edition `node`: each closed
/// interval `[a, b]` lowered to the same closed interval.
fn infinite_node(node: &Node) -> InfiniteNode {
    use InfiniteNodeKind as K;
    let closed = TemporalInterval::Closed;
    InfiniteNode::new(match node.kind {
        NodeKind::False => K::False,
        NodeKind::True => K::True,
        NodeKind::Proposition { proposition } => K::Proposition { proposition },
        NodeKind::Not { operand } => K::Not { operand },
        NodeKind::And { left, right } => K::And { left, right },
        NodeKind::Or { left, right } => K::Or { left, right },
        NodeKind::Implies { left, right } => K::Implies { left, right },
        NodeKind::Equivalent { left, right } => K::Equivalent { left, right },
        NodeKind::Future { interval, operand } => K::Future {
            interval: closed(interval),
            operand,
        },
        NodeKind::Globally { interval, operand } => K::Globally {
            interval: closed(interval),
            operand,
        },
        NodeKind::Once { interval, operand } => K::Once {
            interval: closed(interval),
            operand,
        },
        NodeKind::Historically { interval, operand } => K::Historically {
            interval: closed(interval),
            operand,
        },
        NodeKind::StrongPrevious { operand } => K::StrongPrevious { operand },
        NodeKind::Until {
            interval,
            left,
            right,
        } => K::Until {
            interval: closed(interval),
            left,
            right,
        },
        NodeKind::Release {
            interval,
            left,
            right,
        } => K::Release {
            interval: closed(interval),
            left,
            right,
        },
        NodeKind::Since {
            interval,
            left,
            right,
        } => K::Since {
            interval: closed(interval),
            left,
            right,
        },
        NodeKind::Triggered {
            interval,
            left,
            right,
        } => K::Triggered {
            interval: closed(interval),
            left,
            right,
        },
    })
}

/// The display form of node `id` of `nodes`: `G[0,3] p0`, `(p0 U[0,2] p1)`.
fn display(nodes: &[Node], id: NodeId) -> String {
    let Some(node) = usize::try_from(id.0)
        .ok()
        .and_then(|index| nodes.get(index))
    else {
        return String::new();
    };
    let unary = |symbol: &str, interval: Interval, operand: NodeId| {
        format!(
            "{symbol}{} {}",
            display_interval(interval),
            display(nodes, operand)
        )
    };
    let binary = |symbol: String, left: NodeId, right: NodeId| {
        format!(
            "({} {symbol} {})",
            display(nodes, left),
            display(nodes, right)
        )
    };
    match node.kind {
        NodeKind::False => "false".to_owned(),
        NodeKind::True => "true".to_owned(),
        NodeKind::Proposition { proposition } => format!("p{}", proposition.0),
        NodeKind::Not { operand } => format!("!{}", display(nodes, operand)),
        NodeKind::And { left, right } => binary("&".to_owned(), left, right),
        NodeKind::Or { left, right } => binary("|".to_owned(), left, right),
        NodeKind::Implies { left, right } => binary("->".to_owned(), left, right),
        NodeKind::Equivalent { left, right } => binary("<->".to_owned(), left, right),
        NodeKind::Future {
            interval: at,
            operand,
        } => unary("F", at, operand),
        NodeKind::Globally {
            interval: at,
            operand,
        } => unary("G", at, operand),
        NodeKind::Once {
            interval: at,
            operand,
        } => unary("O", at, operand),
        NodeKind::Historically {
            interval: at,
            operand,
        } => unary("H", at, operand),
        NodeKind::StrongPrevious { operand } => format!("Y {}", display(nodes, operand)),
        NodeKind::Until {
            interval: at,
            left,
            right,
        } => binary(format!("U{}", display_interval(at)), left, right),
        NodeKind::Release {
            interval: at,
            left,
            right,
        } => binary(format!("R{}", display_interval(at)), left, right),
        NodeKind::Since {
            interval: at,
            left,
            right,
        } => binary(format!("S{}", display_interval(at)), left, right),
        NodeKind::Triggered {
            interval: at,
            left,
            right,
        } => binary(format!("T{}", display_interval(at)), left, right),
    }
}

fn display_interval(interval: Interval) -> String {
    format!("[{},{}]", interval.start(), interval.end())
}

/// The native verdict a cross-evaluation compares: `true`, `false`, or the
/// outcome's own name when it is neither.
fn native_verdict(outcome: &fsl_replay::spine::TemporalOutcome) -> Value {
    match outcome {
        fsl_replay::spine::TemporalOutcome::Holds => json!(true),
        fsl_replay::spine::TemporalOutcome::Violated { .. } => json!(false),
        other => json!(format!("{other:?}")),
    }
}

/// FR-757 "Cross-evaluation": `tl-mltl`'s verdict of `formula` at every
/// activation position of the valuation, against the native one.
fn cross_evaluate(
    clause: &str,
    formula: &LoweredFormula<'_>,
    lowered: &Lowered,
    traces: &[&fsl_replay::spine::ObservedValuation],
    mapping: &AtomMapping,
    target: TlTarget,
) -> std::result::Result<Value, String> {
    let mut faults = Vec::new();
    let mut compared = 0_usize;
    for (index, positions) in traces.iter().enumerate() {
        // Each position's true propositions, in increasing order.
        let mut trace: Vec<Vec<PropositionId>> = Vec::new();
        for row in &positions.readings {
            let mut true_at = Vec::new();
            for (proposition, (atom, _)) in lowered.atoms.iter().enumerate() {
                let column = positions
                    .atoms
                    .iter()
                    .position(|native| native == atom)
                    .ok_or_else(|| "lowered atom is a native atom".to_owned())?;
                let native = matches!(row.get(column), Some(fsl_replay::spine::AtomReading::True));
                let name = format!("p{proposition}");
                if native != mapping.negated.contains(&name) {
                    true_at.push(PropositionId(
                        u32::try_from(proposition).map_err(|_| "too many atoms".to_owned())?,
                    ));
                }
            }
            trace.push(true_at);
        }
        let reading = Reading::of(
            formula,
            &trace,
            positions.trace,
            lowered.atoms.len(),
            target,
        )?;
        for (position, outcome) in &positions.activations {
            let at = position.index();
            let tl = reading.verdict(at)?;
            let native = native_verdict(outcome);
            if native != json!(tl) {
                faults.push(json!({
                    "kind": "correspondence-loss",
                    "clause": clause,
                    "trace": index,
                    "position": at,
                    "native": native,
                    "tl": tl,
                }));
            }
        }
        compared = compared.saturating_add(positions.activations.len());
    }
    Ok(json!({
        "traces": traces.len(),
        "positions": compared,
        "agree": faults.is_empty(),
        "faults": faults,
    }))
}

/// The identity of the proposition map a lasso trace document is over.
const PROPOSITION_MAP: &str = "fsl-atoms";

/// How `tl-mltl` reads one trace of a cross-evaluation.
enum Reading<'a> {
    /// A closed finite trace, under a finite-trace correspondence.
    Finite {
        formula: Formula<'a>,
        trace: &'a [Vec<PropositionId>],
        target: TlTarget,
    },
    /// A lasso trace, under `mltl.infinite-trace/v1`.
    Lasso {
        formula: &'a InfiniteFormulaDocument,
        graph: String,
        trace: LassoTraceDocument,
        identity: String,
    },
}

impl<'a> Reading<'a> {
    /// The reading of `trace`, the true propositions at each represented
    /// position, which `shape` splits into a lasso's prefix and loop.
    fn of(
        formula: &'a LoweredFormula<'a>,
        trace: &'a [Vec<PropositionId>],
        shape: fsl_eval::temporal::TemporalTrace,
        atoms: usize,
        target: TlTarget,
    ) -> std::result::Result<Self, String> {
        let (formula, prefix) = match (formula, shape) {
            (LoweredFormula::Bounded(formula), _) => {
                return Ok(Self::Finite {
                    formula: *formula,
                    trace,
                    target,
                });
            }
            (
                LoweredFormula::Unbounded(formula),
                fsl_eval::temporal::TemporalTrace::Lasso { prefix, .. },
            ) => (formula, prefix),
            (LoweredFormula::Unbounded(_), fsl_eval::temporal::TemporalTrace::Finite { .. }) => {
                return Err("an unbounded formula is evaluated over a lasso".to_owned());
            }
        };
        let propositions = (0..atoms)
            .map(|index| {
                u32::try_from(index)
                    .map(PropositionId)
                    .map_err(|_| "too many atoms".to_owned())
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut observations = Vec::with_capacity(trace.len());
        for (position, true_at) in trace.iter().enumerate() {
            let entries = propositions
                .iter()
                .map(|proposition| ValuationEntry {
                    proposition: *proposition,
                    value: if true_at.contains(proposition) {
                        PartialValue::True
                    } else {
                        PartialValue::False
                    },
                })
                .collect();
            observations.push(TraceObservation {
                position: u32::try_from(position).map_err(|_| "trace too long".to_owned())?,
                valuation: PartialValuation::new(
                    PROPOSITION_MAP.to_owned(),
                    &propositions,
                    entries,
                )
                .map_err(|error| format!("{error:?}"))?,
            });
        }
        let prefix = usize::try_from(prefix).map_err(|_| "prefix too long".to_owned())?;
        let loop_observations = observations.split_off(prefix.min(observations.len()));
        let trace = LassoTraceDocument::new(
            SemanticProfile::InfiniteTraceV1,
            InfiniteClock::EventPosition,
            PROPOSITION_MAP.to_owned(),
            propositions,
            observations,
            loop_observations,
        )
        .map_err(|error| format!("{error:?}"))?;
        Ok(Self::Lasso {
            graph: formula
                .content_identity()
                .map_err(|error| error.to_string())?,
            identity: trace
                .content_identity()
                .map_err(|error| error.to_string())?,
            formula,
            trace,
        })
    }

    /// `tl-mltl`'s verdict of the formula at position `at`.
    fn verdict(&self, at: u64) -> std::result::Result<bool, String> {
        match self {
            Self::Finite {
                formula,
                trace,
                target,
            } => tl_verdict(*formula, trace, at, *target),
            Self::Lasso {
                formula,
                graph,
                trace,
                identity,
            } => {
                let result = tl_mltl::infinite::evaluate_lasso(&tl_mltl::infinite::LassoRequest {
                    formula,
                    trace,
                    fairness: None,
                    evidence_closure: tl_mltl::infinite::EvidenceClosure::Closed,
                    graph_id: graph,
                    trace_id: identity,
                    selected_position: at,
                    limit: tl_mltl::infinite::EvaluationLimit::default(),
                })
                .map_err(|error| error.to_string())?;
                match result.disposition {
                    tl_mltl::infinite::Disposition::Proved => Ok(true),
                    tl_mltl::infinite::Disposition::Refuted => Ok(false),
                    other => Err(format!(
                        "an exact lasso settles every verdict, not {other:?}"
                    )),
                }
            }
        }
    }
}

/// `tl-mltl`'s verdict of `formula` at `at` over the closed finite `trace`.
fn tl_verdict(
    formula: Formula<'_>,
    trace: &[Vec<PropositionId>],
    at: u64,
    target: TlTarget,
) -> std::result::Result<bool, String> {
    let truth = |verdict: tl_mltl::TruthValue| match verdict {
        tl_mltl::TruthValue::True => Ok(true),
        tl_mltl::TruthValue::False => Ok(false),
        tl_mltl::TruthValue::Pending => Err("a closed trace settles every verdict".to_owned()),
    };
    match target {
        TlTarget::ClosedTrace => truth(
            tl_mltl::evaluate_closed_at(
                formula,
                "fsl",
                trace,
                "fsl",
                at,
                tl_mltl::EvaluationLimits::default(),
            )
            .map_err(|error| error.to_string())?
            .verdict,
        ),
        TlTarget::OnlinePrefix => truth(
            tl_mltl::evaluate_prefix_at(
                formula,
                "fsl",
                trace,
                "fsl",
                true,
                at,
                tl_mltl::EvaluationLimits::default(),
            )
            .map_err(|error| error.to_string())?
            .verdict,
        ),
        TlTarget::OriginCompleteHistory => {
            let through = u64::try_from(trace.len().saturating_sub(1))
                .map_err(|_| "trace too long".to_owned())?;
            let observations = trace
                .iter()
                .enumerate()
                .map(|(position, true_at)| {
                    u64::try_from(position)
                        .map(|position| {
                            tl_mltl::PositionObservation::new(position, true_at.clone(), None)
                        })
                        .map_err(|_| "trace too long".to_owned())
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let history = tl_mltl::PositionHistoryDocument::new(
                "fsl",
                1,
                0,
                through,
                Some(tl_mltl::ClockBinding::EventPosition),
                observations,
            )
            .map_err(|error| error.to_string())?;
            Ok(tl_mltl::evaluate_past(
                formula,
                "fsl",
                &history,
                at,
                "fsl",
                1,
                tl_mltl::PastEvaluationRelationInput::Original,
                tl_mltl::PastEvaluationLimits::default(),
            )
            .map_err(|error| error.to_string())?
            .verdict)
        }
        TlTarget::InfiniteTrace => Err("an infinite-trace verdict is read over a lasso".to_owned()),
    }
}

/// The run result document of a lowering.
pub(super) fn document(record: Value) -> Value {
    json!({"format": "native-run-result/2", "tl_lowering": record})
}

/// The record of the request path's lowering: each atom mapped to its own
/// leaf.
pub(super) fn lower(
    valuation: &TemporalValuation,
    source: &str,
    lowering: &TlLowering,
) -> Result<TlRecord> {
    let form = lowering
        .cross
        .as_ref()
        .map_or(TraceForm::Absent, TraceForm::of_request);
    lowering_record(
        valuation,
        source,
        lowering.correspondence,
        &AtomMapping::default(),
        form,
    )
    .map_err(|failure| match failure {
        Failure::Fault(message) => RunCause::TlFault(message),
        Failure::Form { member, reason } => invalid(member, reason),
    })
}
