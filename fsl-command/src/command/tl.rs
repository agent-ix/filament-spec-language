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
use tl_syntax::{Formula, Interval, Node, NodeId, NodeKind, PropositionId, SemanticProfile};

use super::{Result, RunCause};

/// The three correspondence identities a request may name (FR-931).
const CORRESPONDENCES: [TlTarget; 3] = [
    TlTarget::ClosedTrace,
    TlTarget::OnlinePrefix,
    TlTarget::OriginCompleteHistory,
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
    /// A closed finite trace, read as FR-312 reads a `temporal` selection's
    /// `trace` and `over`.
    Trace {
        /// The `trace` member.
        trace: Value,
        /// The `over` member, if any.
        over: Option<Value>,
    },
    /// Every finite behaviour of the subject's model from the snapshot
    /// `initial`, of 1 to `max_steps` steps.
    Initial,
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
    Ok(TlLowering {
        clause,
        correspondence,
        cross,
    })
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
            match trace {
                Value::Object(kinds) if kinds.len() == 1 && kinds.contains_key("finite") => {}
                _ => {
                    return Err(invalid(
                        "cross_evaluate.trace",
                        "only a finite trace is cross-evaluated",
                    ))
                }
            }
            Ok(CrossEvaluate::Trace {
                trace: trace.clone(),
                over: cross.get("over").cloned(),
            })
        }
        (None, Some(initial)) => {
            if cross.contains_key("over") {
                return Err(invalid(
                    "cross_evaluate.over",
                    "over is given with trace only",
                ));
            }
            if !matches!(initial, Value::String(file) if !file.is_empty()) {
                return Err(invalid(
                    "cross_evaluate.initial",
                    "expected a snapshot file name",
                ));
            }
            match cross.get("max_steps").and_then(Value::as_u64) {
                Some(steps) if steps >= 1 => Ok(CrossEvaluate::Initial),
                _ => Err(invalid(
                    "cross_evaluate.max_steps",
                    "expected an integer of at least 1",
                )),
            }
        }
    }
}

/// The FR-312 clause selection a lowering reads its clause and valuation
/// through: the trace to cross-evaluate over, or an empty finite trace,
/// which asks for the checked clause alone.
pub(super) fn selection(lowering: &TlLowering) -> Value {
    let mut temporal = json!({"name": lowering.clause, "trace": {"finite": []}});
    if let Some(CrossEvaluate::Trace { trace, over }) = &lowering.cross {
        temporal["trace"] = trace.clone();
        if let Some(over) = over {
            temporal["over"] = over.clone();
        }
    }
    json!({"selection": {"temporal": temporal}, "observations": {}})
}

/// How the lowering maps TL atoms to native `holds` leaves. The request path
/// always maps each atom to its own leaf; a library caller may name atoms to
/// map to the negated leaf, which FR-757-AC-2 uses to show that a mis-mapped
/// atom table is reported as a correspondence loss.
#[derive(Clone, Debug, Default)]
pub struct AtomMapping {
    /// The TL atom names (`p0`, ...) read as the negation of their leaf.
    pub negated: BTreeSet<String>,
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
    let named = match correspondence {
        None => None,
        Some(identity) => Some(
            CORRESPONDENCES
                .into_iter()
                .find(|target| target.identity() == identity)
                .ok_or_else(|| format!("unknown correspondence {identity}"))?,
        ),
    };
    lowering_record(valuation, source, named, mapping)
}

fn direction(target: TlTarget) -> bool {
    matches!(target, TlTarget::OriginCompleteHistory)
}

fn dimension(unmatched: TlUnmatched) -> &'static str {
    match unmatched {
        TlUnmatched::FiniteWindow | TlUnmatched::TimedProfile => "profile",
        TlUnmatched::InfiniteTrace => "infinite-trace",
        TlUnmatched::MixedDirection => "direction",
    }
}

fn lowering_record(
    valuation: &TemporalValuation,
    source: &str,
    named: Option<TlTarget>,
    mapping: &AtomMapping,
) -> std::result::Result<TlRecord, String> {
    let clause = &valuation.clause;
    let formula = clause.formula();
    let (mut future, mut past) = (false, false);
    for node in formula.nodes() {
        match node {
            FormulaNode::Unary {
                operator: UnaryTemporal::Eventually | UnaryTemporal::Always,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Until | BinaryTemporal::Release,
                ..
            } => future = true,
            FormulaNode::Unary {
                operator: UnaryTemporal::Once | UnaryTemporal::Historically,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Since | BinaryTemporal::Triggered,
                ..
            } => past = true,
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. }
            | FormulaNode::Not(_)
            | FormulaNode::And(..)
            | FormulaNode::Or(..)
            | FormulaNode::Implies(..) => {}
        }
    }
    // FR-931 "The correspondence": the member fixes the closure.
    let surrounding = match named {
        Some(TlTarget::OnlinePrefix) => TlClosure::Open,
        Some(TlTarget::ClosedTrace | TlTarget::OriginCompleteHistory) | None => TlClosure::Complete,
    };
    let support = fsl_replay::spine::tl_support_of(clause.profile(), future, past, surrounding);
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
        let dimension = if direction(target) == direction(classified) {
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
    let profile = match classified {
        TlTarget::ClosedTrace => SemanticProfile::ClosedTraceV1,
        TlTarget::OnlinePrefix => SemanticProfile::OnlinePrefixV1,
        TlTarget::OriginCompleteHistory => SemanticProfile::OriginCompleteHistoryV1,
    };
    let mut lowered = Lowered::default();
    let root = lowered.lower(formula, formula.root(), source)?;
    let tl = Formula::new(profile, root, &lowered.nodes).map_err(|error| error.to_string())?;
    record.insert("disposition".to_owned(), json!("supported"));
    record.insert("correspondence".to_owned(), json!(classified.identity()));
    record.insert("formula".to_owned(), json!(display(tl, root)));
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
    if let Some(positions) = &valuation.positions {
        let cross = cross_evaluate(clause.name(), tl, &lowered, positions, mapping, classified)?;
        if cross["agree"] != json!(true) {
            exit_code = 30;
        }
        record.insert("cross_evaluation".to_owned(), cross);
    }
    Ok(TlRecord {
        record: Value::Object(record),
        exit_code,
    })
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
    /// named by its text in `source`.
    fn lower(
        &mut self,
        formula: &CheckedFormula,
        id: FormulaNodeId,
        source: &str,
    ) -> std::result::Result<NodeId, String> {
        let node = formula
            .nodes()
            .get(id.index())
            .ok_or_else(|| "formula node in range".to_owned())?;
        let kind = match node {
            FormulaNode::Constant(true) => NodeKind::True,
            FormulaNode::Constant(false) => NodeKind::False,
            FormulaNode::Holds { span } => {
                let index = match self.atoms.iter().position(|(atom, _)| *atom == id) {
                    Some(index) => index,
                    None => {
                        let text = source
                            .get(span.start..span.end)
                            .ok_or_else(|| "holds leaf span in the source".to_owned())?;
                        self.atoms.push((id, text.to_owned()));
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
                return Err("a window aggregate atom has no TL atom".to_owned())
            }
            FormulaNode::Not(operand) => NodeKind::Not {
                operand: self.lower(formula, *operand, source)?,
            },
            FormulaNode::And(left, right) => NodeKind::And {
                left: self.lower(formula, *left, source)?,
                right: self.lower(formula, *right, source)?,
            },
            FormulaNode::Or(left, right) => NodeKind::Or {
                left: self.lower(formula, *left, source)?,
                right: self.lower(formula, *right, source)?,
            },
            FormulaNode::Implies(left, right) => NodeKind::Implies {
                left: self.lower(formula, *left, source)?,
                right: self.lower(formula, *right, source)?,
            },
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let interval = Self::interval(interval.as_ref())?;
                let operand = self.lower(formula, *operand, source)?;
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
                let left = self.lower(formula, *left, source)?;
                let right = self.lower(formula, *right, source)?;
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

/// The display form of node `id` of `formula`: `G[0,3] p0`, `(p0 U[0,2] p1)`.
fn display(formula: Formula<'_>, id: NodeId) -> String {
    let Some(node) = formula.node(id) else {
        return String::new();
    };
    let unary = |symbol: &str, interval: Interval, operand: NodeId| {
        format!(
            "{symbol}{} {}",
            display_interval(interval),
            display(formula, operand)
        )
    };
    let binary = |symbol: String, left: NodeId, right: NodeId| {
        format!(
            "({} {symbol} {})",
            display(formula, left),
            display(formula, right)
        )
    };
    match node.kind {
        NodeKind::False => "false".to_owned(),
        NodeKind::True => "true".to_owned(),
        NodeKind::Proposition { proposition } => format!("p{}", proposition.0),
        NodeKind::Not { operand } => format!("!{}", display(formula, operand)),
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
        NodeKind::StrongPrevious { operand } => format!("Y {}", display(formula, operand)),
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
    formula: Formula<'_>,
    lowered: &Lowered,
    positions: &fsl_replay::spine::ObservedValuation,
    mapping: &AtomMapping,
    target: TlTarget,
) -> std::result::Result<Value, String> {
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
    let mut faults = Vec::new();
    for (position, outcome) in &positions.activations {
        let at = position.index();
        let tl = tl_verdict(formula, &trace, at, target)?;
        let native = native_verdict(outcome);
        if native != json!(tl) {
            faults.push(json!({
                "kind": "correspondence-loss",
                "clause": clause,
                "trace": 0,
                "position": at,
                "native": native,
                "tl": tl,
            }));
        }
    }
    Ok(json!({
        "traces": 1,
        "positions": positions.activations.len(),
        "agree": faults.is_empty(),
        "faults": faults,
    }))
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
) -> std::result::Result<TlRecord, String> {
    lowering_record(
        valuation,
        source,
        lowering.correspondence,
        &AtomMapping::default(),
    )
}
