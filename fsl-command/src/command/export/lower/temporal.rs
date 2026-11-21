// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813: the lowering of temporal clauses and fairness over the
//! exported state to TLA+ and Lean 4 text.
//!
//! The Lean text is the direct definition by recursion over positions for
//! every operator; the TLA+ text is in [`tla`]. A construct the specification
//! covers and this build does not yet lower is a typed refusal that names it.

mod tla;

pub(super) use self::tla::{tla, Extra};
use super::{
    defer_at, define, mangle, parameters, refusal, refuse, Definition, Graph, Input, Lower, Target,
    E,
};
use filament_contract_model::{CheckedOperationInterval, CheckedProtocolActivation};
use fsl_package::checked_views::{
    temporal_subject, Activation, FormulaNodeUse, FormulaOperation, OperatorDirection,
};
use fsl_package::{CheckedTimeSourceKind, CheckedTimedModel};
use fsl_semantics::check::TemporalProfile;

pub(super) const CONSTRUCT: &str = "temporal/temporal_clause";

/// An operator's interval: `None` for an unbounded operator, otherwise the
/// inclusive lower bound and the inclusive upper bound when there is one.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Iv(Option<(u64, Option<u64>)>);

impl Iv {
    /// No interval.
    pub(super) const fn none() -> Self {
        Self(None)
    }
    /// The inclusive lower bound and the inclusive upper bound, if any.
    fn bounds(self) -> (u64, Option<u64>) {
        self.0.unwrap_or_default()
    }
    fn lower(self) -> u64 {
        self.0.map_or(0, |(lower, _)| lower)
    }
    fn upper(self) -> Option<u64> {
        self.0.and_then(|(_, upper)| upper)
    }
    pub(super) fn is_some(self) -> bool {
        self.0.is_some()
    }
}

/// A temporal formula over the clause's atoms.
#[derive(Clone, PartialEq)]
pub(super) enum Tf {
    Const(bool),
    Atom(usize),
    /// A history variable that stands for a lowered past operator.
    Hist(usize),
    Not(Box<Tf>),
    And(Box<Tf>, Box<Tf>),
    Or(Box<Tf>, Box<Tf>),
    Implies(Box<Tf>, Box<Tf>),
    Always(Iv, Box<Tf>),
    Eventually(Iv, Box<Tf>),
    Once(Iv, Box<Tf>),
    Historically(Iv, Box<Tf>),
    Until(Iv, Box<Tf>, Box<Tf>),
    Release(Iv, Box<Tf>, Box<Tf>),
    Since(Iv, Box<Tf>, Box<Tf>),
    Triggered(Iv, Box<Tf>, Box<Tf>),
    Timed(Box<super::dense::Window>),
}

/// The checked node and operands corresponding to a use in the original formula.
pub(super) struct FormulaSite {
    position: usize,
    operands: Vec<usize>,
}

/// One fairness constraint of a clause, as read from the package.
pub(super) struct FairRead {
    pub(super) strong: bool,
    pub(super) each: bool,
    /// The object type that declares the operation.
    pub(super) declaration: String,
    pub(super) operation: String,
    /// The fairness node.
    pub(super) node: usize,
}

/// A temporal clause as read from the package.
pub(super) struct ClauseRead {
    pub(super) name: String,
    pub(super) position: usize,
    /// The object type the `over` binder ranges over.
    pub(super) context: String,
    pub(super) context_node: usize,
    /// The mangled name of the `over` binder.
    pub(super) binder: String,
    /// The `over` binder node.
    pub(super) over: usize,
    pub(super) tf: Tf,
    pub(super) formula_sites: Vec<FormulaSite>,
    /// The first future operator the clause's TLA+ text has no deterministic
    /// monitor for, when one exists.
    pub(super) refuse_at: Option<usize>,
    /// The node of each atom's Boolean operand, in formula order.
    pub(super) atoms: Vec<usize>,
    pub(super) fairness: Vec<FairRead>,
    /// The admitted dense model root, when the clause reads model time.
    pub(super) timed_root: Option<usize>,
}

/// A temporal clause with its atoms lowered.
pub(super) struct ClauseLow {
    pub(super) name: String,
    pub(super) position: usize,
    pub(super) object: usize,
    pub(super) binder: String,
    pub(super) tf: Tf,
    pub(super) formula_sites: Vec<FormulaSite>,
    /// The node a clause without a deterministic monitor is refused at.
    pub(super) refuse_at: Option<usize>,
    /// The FR-812 definedness conditions of the atoms, which `ok_<clause>`
    /// states.
    pub(super) defs: Vec<E>,
    /// Each atom in the current state and in the next state.
    pub(super) cur: Vec<E>,
    pub(super) next: Vec<E>,
    pub(super) fairness: Vec<FairLow>,
}

/// A fairness constraint over an exported operation.
pub(super) struct FairLow {
    pub(super) strong: bool,
    pub(super) each: bool,
    pub(super) object: usize,
    pub(super) operation: usize,
}

// ---- Reading -------------------------------------------------------------

/// The position of the package node `id` names.
fn at_node(graph: &Graph<'_>, id: &filament_contract_model::CheckedNodeId) -> Lower<usize> {
    graph.position(id).ok_or_else(|| {
        refusal(
            "checked package",
            "a node of the clause is not in the package",
        )
    })
}

fn interval(graph: &Graph<'_>, node: &FormulaNodeUse<'_>) -> Lower<Iv> {
    let bound = |text: String| {
        text.parse::<u64>()
            .map_err(|_| refusal("temporal/interval", "an interval bound beyond 64 bits"))
    };
    match &node.interval {
        None => Ok(Iv::none()),
        Some(CheckedOperationInterval::Integer { lower, upper }) => {
            let lower = bound(lower.to_string())?;
            let upper = upper
                .as_ref()
                .map(|upper| bound(upper.to_string()))
                .transpose()?;
            Ok(if (lower, upper) == (0, None) {
                Iv::none()
            } else {
                Iv(Some((lower, upper)))
            })
        }
        Some(CheckedOperationInterval::Timed { .. }) => {
            let at = at_node(graph, node.node)?;
            defer_at(
                at,
                &graph.construct(at),
                "a timed interval this build does not lower",
            )
        }
    }
}

fn boxed(items: &[Tf], index: usize) -> Lower<Box<Tf>> {
    items
        .get(index)
        .cloned()
        .map(Box::new)
        .ok_or_else(|| refusal(CONSTRUCT, "a temporal operator is missing an operand"))
}

/// Where `build` is in the formula.
#[derive(Clone, Copy)]
struct Scope {
    /// Inside a future operator.
    future: bool,
    /// Inside a past operator.
    past: bool,
    /// Reached from the root through conjunctions only.
    conj: bool,
    window: WindowScope,
}

#[derive(Clone, Copy)]
enum WindowScope {
    Unsupported,
    Origin,
    Always,
}

impl WindowScope {
    fn through(self, operation: FormulaOperation, interval: Iv) -> Self {
        match (self, operation, interval.is_some()) {
            (scope, FormulaOperation::And, false) => scope,
            (Self::Origin, FormulaOperation::Always, false) => Self::Always,
            _ => Self::Unsupported,
        }
    }
}

fn build(
    graph: &Graph<'_>,
    uses: &[FormulaNodeUse<'_>],
    at: usize,
    atom: &mut usize,
    scope: Scope,
    refuse_at: &mut Option<usize>,
) -> Lower<Tf> {
    let node = uses
        .get(at)
        .ok_or_else(|| refusal(CONSTRUCT, "a formula operand is not in the formula"))?;
    let kind = node
        .kind()
        .ok_or_else(|| refusal(CONSTRUCT, "an operation that is no temporal operation"))?;
    if kind == FormulaOperation::Holds {
        *atom += 1;
        return Ok(Tf::Atom(*atom - 1));
    }
    let here = at_node(graph, node.node)?;
    let direction = kind.direction();
    if let Some(CheckedOperationInterval::Timed { .. }) = &node.interval {
        if matches!(scope.window, WindowScope::Unsupported) {
            return defer_at(
                here,
                &graph.construct(here),
                "a timed window outside the origin or its unbounded always",
            );
        }
        return super::dense::Window::read(graph, kind, node, here, |operand| {
            build(
                graph,
                uses,
                operand,
                atom,
                Scope {
                    future: true,
                    past: false,
                    conj: false,
                    window: WindowScope::Unsupported,
                },
                refuse_at,
            )
        })
        .map(|window| Tf::Timed(Box::new(window)));
    }
    let iv = interval(graph, node)?;
    if direction == Some(OperatorDirection::Future) {
        if scope.past {
            return defer_at(
                here,
                "temporal/future-in-past",
                "a future operator inside the operand of a past operator",
            );
        }
        let monitored =
            matches!(kind, FormulaOperation::Until | FormulaOperation::Release) || iv.is_some();
        if scope.future || (!scope.conj && monitored) {
            refuse_at.get_or_insert(here);
        }
    }
    let inner = Scope {
        future: scope.future || direction == Some(OperatorDirection::Future),
        past: scope.past || direction == Some(OperatorDirection::Past),
        conj: scope.conj && kind == FormulaOperation::And,
        window: scope.window.through(kind, iv),
    };
    let mut operands = Vec::new();
    for &operand in &node.operands {
        operands.push(build(graph, uses, operand, atom, inner, refuse_at)?);
    }
    Ok(match kind {
        FormulaOperation::True => Tf::Const(true),
        FormulaOperation::False => Tf::Const(false),
        FormulaOperation::Holds => unreachable!("read above"),
        FormulaOperation::Aggregate => {
            return defer_at(
                here,
                "temporal/aggregate",
                "an aggregate atom over an accumulation of the trace",
            );
        }
        FormulaOperation::Not => Tf::Not(boxed(&operands, 0)?),
        FormulaOperation::And => Tf::And(boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Or => Tf::Or(boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Implies => Tf::Implies(boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Always => Tf::Always(iv, boxed(&operands, 0)?),
        FormulaOperation::Eventually => Tf::Eventually(iv, boxed(&operands, 0)?),
        FormulaOperation::Once => Tf::Once(iv, boxed(&operands, 0)?),
        FormulaOperation::Historically => Tf::Historically(iv, boxed(&operands, 0)?),
        FormulaOperation::Until => Tf::Until(iv, boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Release => Tf::Release(iv, boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Since => Tf::Since(iv, boxed(&operands, 0)?, boxed(&operands, 1)?),
        FormulaOperation::Triggered => {
            Tf::Triggered(iv, boxed(&operands, 0)?, boxed(&operands, 1)?)
        }
    })
}

/// Read the temporal clause at `position` of the package.
///
/// # Errors
///
/// A [`super::Refusal`] naming the clause shape this build does not lower.
pub(super) fn read(
    graph: &Graph<'_>,
    name: &str,
    position: usize,
    timed_models: &[CheckedTimedModel],
    target: Target,
) -> Lower<ClauseRead> {
    let node = graph.node(position);
    let view = temporal_subject(graph.package, &node.node_id).map_err(|_| {
        refusal(
            CONSTRUCT,
            "a clause the package does not read as a temporal clause",
        )
    })?;
    let activation_rows = graph.package.protocol_activation_relation();
    for protocol in view.requirements() {
        if let Some(row) = activation_rows
            .iter()
            .find(|row| &row.clause == *protocol && row.required_temporal == node.node_id)
        {
            if let CheckedProtocolActivation::Each { trigger } = &row.activation {
                let trigger_position = at_node(graph, &trigger)?;
                let trigger_node = graph.node(trigger_position);
                let construct = format!("{}/{}", trigger_node.node_tag, trigger_node.semantic_form);
                return defer_at(
                    trigger_position,
                    &construct,
                    "a required temporal clause on a protocol `on each` activation row",
                );
            }
        }
    }
    if let Some(protocol) = view.requirements().first() {
        let protocol_position = at_node(graph, protocol)?;
        let protocol_node = graph.node(protocol_position);
        let construct = format!("{}/{}", protocol_node.node_tag, protocol_node.semantic_form);
        return defer_at(
            protocol_position,
            &construct,
            "a protocol-required temporal clause that this build does not lower",
        );
    }
    if let Activation::Each { trigger, .. } = view.activation() {
        return defer_at(
            at_node(graph, trigger)?,
            "temporal/activation",
            "an activation other than `on origin`",
        );
    }
    if let Some(capture) = view.captures().first() {
        return defer_at(
            at_node(graph, capture)?,
            "temporal/capture",
            "a capture of the activation",
        );
    }
    if !view.capture_names().is_empty() {
        // The capture is read as a name, with no node of its own.
        return defer_at(position, "temporal/capture", "a capture of the activation");
    }
    let over = graph
        .position(view.over())
        .ok_or_else(|| refusal(CONSTRUCT, "the `over` binder names no package node"))?;
    let (context, context_node) = super::context_of(graph, over)?;
    let profile = TemporalProfile::from_identity(&view.profile().definition.identity);
    let timed_model = timed_models
        .iter()
        .find(|model| model.model == graph.node(context_node).node_id);
    let timed_root = match (profile, timed_model) {
        (Some(TemporalProfile::Timed), Some(model))
            if model.source.kind == CheckedTimeSourceKind::Dense =>
        {
            Some(at_node(graph, &model.metadata.node_id)?)
        }
        (Some(TemporalProfile::InfiniteTrace), None) => None,
        (_, Some(model)) => {
            return defer_at(
                at_node(graph, &model.metadata.node_id)?,
                "timed_model/timed_model",
                "a timed model outside the dense model-time profile",
            );
        }
        _ => {
            // A profile selection is a lock row, so name the clause node.
            return defer_at(
                position,
                "temporal/profile",
                "a profile outside the supported infinite-trace and dense model-time forms",
            );
        }
    };
    if timed_root.is_some() && target == Target::Tla {
        check_dense_tla_operators(graph, view.formula_graph())?;
    }
    let mut atom = 0;
    let mut refuse_at = None;
    let start = Scope {
        future: false,
        past: false,
        conj: true,
        window: if timed_root.is_some() {
            WindowScope::Origin
        } else {
            WindowScope::Unsupported
        },
    };
    let tf = build(
        graph,
        view.formula_graph(),
        0,
        &mut atom,
        start,
        &mut refuse_at,
    )?;
    let leaves = view.predicate_leaves();
    if atom != leaves.len() {
        return refuse(CONSTRUCT, "a formula whose atoms the package does not list");
    }
    let atoms = leaves
        .iter()
        .map(|leaf| {
            graph
                .position(leaf)
                .ok_or_else(|| refusal(CONSTRUCT, "an atom names no package node"))
        })
        .collect::<Lower<Vec<_>>>()?;
    let binder = super::parameter_name(graph, over)
        .ok_or_else(|| refusal(CONSTRUCT, "the `over` binder has no name"))?;
    let formula_sites = if target == Target::Tla && timed_root.is_none() {
        view.formula_graph()
            .iter()
            .map(|usage| {
                Ok(FormulaSite {
                    position: at_node(graph, usage.node)?,
                    operands: usage.operands.clone(),
                })
            })
            .collect::<Lower<Vec<_>>>()?
    } else {
        Vec::new()
    };
    Ok(ClauseRead {
        name: name.to_owned(),
        position,
        context,
        context_node,
        binder: mangle("p_", &binder),
        over,
        tf,
        formula_sites,
        refuse_at,
        atoms,
        fairness: super::fairness_of(graph, position)?,
        timed_root,
    })
}

/// Locate unsupported operators while their checked node IDs are still
/// available. In particular, an unbounded root `until` has no nested
/// future operator to supply the generic monitor refusal location.
/// The dense TLA+ renderer emits one window monitor per clause.
fn check_dense_tla_operators(graph: &Graph<'_>, uses: &[FormulaNodeUse<'_>]) -> Lower<()> {
    let mut has_window = false;
    for usage in uses {
        let supported = matches!(
            usage.kind(),
            Some(
                FormulaOperation::True
                    | FormulaOperation::False
                    | FormulaOperation::Holds
                    | FormulaOperation::And
                    | FormulaOperation::Or
                    | FormulaOperation::Not
                    | FormulaOperation::Implies
                    | FormulaOperation::Always
                    | FormulaOperation::Eventually
            )
        ) && !matches!(
            &usage.interval,
            Some(CheckedOperationInterval::Integer { .. })
        );
        if !supported {
            let at = at_node(graph, usage.node)?;
            return defer_at(
                at,
                &graph.construct(at),
                "an operator outside the dense TLA+ lowering",
            );
        }
        if matches!(
            &usage.interval,
            Some(CheckedOperationInterval::Timed { .. })
        ) {
            if has_window {
                let at = at_node(graph, usage.node)?;
                return defer_at(
                    at,
                    &graph.construct(at),
                    "more than one timed window in a dense TLA+ clause",
                );
            }
            has_window = true;
        }
    }
    Ok(())
}

// ---- Lean 4 --------------------------------------------------------------

struct LeanClause<'a> {
    clause: &'a ClauseLow,
    atom: String,
    next: usize,
}

impl LeanClause<'_> {
    fn atom_name(&self, index: usize) -> String {
        format!("{}_{index}", self.atom)
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}{}", self.next)
    }

    /// The conditions that put the offset `d` in `iv`. A past operator
    /// without an upper bound ranges over the positions from the origin
    /// through `position - lower` only (FSpec FR-092, FR-367), so it adds
    /// `0 <= position - d`.
    fn window(iv: Iv, d: &str, past: Option<&str>) -> Vec<String> {
        let mut conditions = match (iv.lower(), iv.upper()) {
            (0, None) => vec![format!("0 <= {d}")],
            (lower, None) => vec![format!("{lower} <= {d}")],
            (lower, Some(upper)) => vec![format!("{lower} <= {d}"), format!("{d} <= {upper}")],
        };
        if let (Some(position), None) = (past, iv.upper()) {
            conditions.push(format!("0 <= ({position} - {d})"));
        }
        conditions
    }

    /// `tf` at the position `position` of the behaviour `b`.
    fn at(&mut self, tf: &Tf, position: &str) -> Lower<String> {
        Ok(match tf {
            Tf::Const(true) => "True".to_owned(),
            Tf::Const(false) => "False".to_owned(),
            Tf::Hist(_) => {
                return refuse(CONSTRUCT, "a TLA+ monitor construct in a Lean definition");
            }
            Tf::Atom(index) => format!(
                "(0 <= {position} /\\ {} (b.st (Int.toNat {position})) {})",
                self.atom_name(*index),
                self.clause.binder
            ),
            Tf::Not(inner) => format!("Not ({})", self.at(inner, position)?),
            Tf::And(left, right) => {
                format!(
                    "({} /\\ {})",
                    self.at(left, position)?,
                    self.at(right, position)?
                )
            }
            Tf::Or(left, right) => {
                format!(
                    "({} \\/ {})",
                    self.at(left, position)?,
                    self.at(right, position)?
                )
            }
            Tf::Implies(left, right) => {
                format!(
                    "({} -> {})",
                    self.at(left, position)?,
                    self.at(right, position)?
                )
            }
            Tf::Always(iv, inner) => {
                let d = self.fresh("d");
                let next_position = format!("({position} + {d})");
                let body = self.at(inner, &next_position)?;
                let window = Self::window(*iv, &d, None).join(" -> ");
                format!("(forall {d} : Int, {window} -> {body})")
            }
            Tf::Eventually(iv, inner) => {
                let d = self.fresh("d");
                let next_position = format!("({position} + {d})");
                let body = self.at(inner, &next_position)?;
                let window = Self::window(*iv, &d, None).join(" /\\ ");
                format!("(exists {d} : Int, {window} /\\ {body})")
            }
            Tf::Historically(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} - {d})"))?;
                let window = Self::window(*iv, &d, Some(position)).join(" -> ");
                format!("(forall {d} : Int, {window} -> {body})")
            }
            Tf::Once(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} - {d})"))?;
                let window = Self::window(*iv, &d, Some(position)).join(" /\\ ");
                format!("(exists {d} : Int, {window} /\\ {body})")
            }
            Tf::Until(iv, left, right) => self.relation(*iv, left, right, position, "+")?,
            Tf::Since(iv, left, right) => self.relation(*iv, left, right, position, "-")?,
            Tf::Release(iv, left, right) => {
                let (left, right) = (negate(left), negate(right));
                format!("Not {}", self.relation(*iv, &left, &right, position, "+")?)
            }
            Tf::Triggered(iv, left, right) => {
                let (left, right) = (negate(left), negate(right));
                format!("Not {}", self.relation(*iv, &left, &right, position, "-")?)
            }
            Tf::Timed(window) => {
                let d = self.fresh("d");
                let next = format!("({position} + {d})");
                let points =
                    format!("Q_sub (b.ts (Int.toNat {next})) (b.ts (Int.toNat {position}))");
                let bounds = window.lean_bounds(&format!("({points})"));
                let inner = match &window.predicate {
                    Tf::Atom(index) => format!(
                        "{} (b.st (Int.toNat {next})) {}",
                        self.atom_name(*index),
                        self.clause.binder,
                    ),
                    formula => self.at(formula, &next)?,
                };
                match window.operator {
                    super::dense::Operator::Always => format!(
                        "(forall {d} : Int, 0 <= {d} -> 0 <= {next} -> ({bounds}) -> {inner})"
                    ),
                    super::dense::Operator::Eventually => format!(
                        "(exists {d} : Int, 0 <= {d} /\\ (0 <= {next} /\\ {bounds} /\\ {inner}))"
                    ),
                }
            }
        })
    }

    /// `left until right` or, reading backwards, `left since right`:
    /// some offset `d` in the interval makes `right` hold and every offset
    /// `e` from the interval's lower bound up to `d` makes `left` hold.
    fn relation(
        &mut self,
        iv: Iv,
        left: &Tf,
        right: &Tf,
        position: &str,
        sign: &str,
    ) -> Lower<String> {
        let d = self.fresh("d");
        let e = self.fresh("e");
        let right = self.at(right, &format!("({position} {sign} {d})"))?;
        let left = self.at(left, &format!("({position} {sign} {e})"))?;
        let past = (sign == "-").then_some(position);
        let window = Self::window(iv, &d, past).join(" /\\ ");
        Ok(format!(
            "(exists {d} : Int, {window} /\\ {right} /\\ (forall {e} : Int, {} <= {e} -> {e} < {d} -> {left}))",
            iv.lower()
        ))
    }
}

fn negate(tf: &Tf) -> Tf {
    Tf::Not(Box::new(tf.clone()))
}

/// The Lean text of a fairness premise over the behaviour `b`.
fn lean_fair(input: &Input<'_>, clause: &ClauseLow) -> Lower<Option<String>> {
    let mut premises = Vec::new();
    for constraint in &clause.fairness {
        let object = input
            .objects
            .get(constraint.object)
            .ok_or_else(|| refusal("temporal/fairness", "a fairness target outside the state"))?;
        let operation = object
            .operations
            .get(constraint.operation)
            .ok_or_else(|| refusal("temporal/fairness", "an operation outside the state"))?;
        let name = object.operation(operation);
        let params = parameters(operation);
        let names: Vec<&str> = params.iter().map(|(param, _)| param.as_str()).collect();
        let chosen: Vec<&str> = if constraint.each {
            names.iter().skip(1).copied().collect()
        } else {
            names.clone()
        };
        let exists = |state: &str, next: &str| {
            let binders: String = chosen
                .iter()
                .map(|param| format!("exists {param}, "))
                .collect();
            format!("{binders}{name} {state} {next} {}", names.join(" "))
        };
        let enabled = format!("(exists t, {})", exists("(b.st j)", "t"));
        let taken = format!("({})", exists("(b.st j)", "(b.st (j + 1))"));
        let premise = if constraint.strong {
            format!(
                "((forall i : Nat, exists j : Nat, i <= j /\\ {enabled}) -> (forall i : Nat, exists j : Nat, i <= j /\\ {taken}))"
            )
        } else {
            format!(
                "(forall i : Nat, (forall j : Nat, i <= j -> {enabled}) -> (exists j : Nat, i <= j /\\ {taken}))"
            )
        };
        premises.push(if constraint.each {
            format!(
                "(forall r_self, {} r_self -> {premise})",
                mangle("Ids_", &object.name)
            )
        } else {
            premise
        });
    }
    Ok((!premises.is_empty()).then(|| premises.join(" /\\\n  ")))
}

/// Lower the clauses of `input` to Lean 4.
pub(super) fn lean(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
    let mut universes = std::collections::BTreeSet::new();
    for clause in input.temporals {
        let object = input
            .objects
            .get(clause.object)
            .ok_or_else(|| refusal(CONSTRUCT, "an unexported clause context"))?;
        let obligation = Some(clause.name.as_str());
        let has = mangle("F_has_", &object.name);
        if universes.insert(clause.object) {
            let ids = mangle("Ids_", &object.name);
            out.push(define(
                clause.position,
                Some(ids.clone()),
                format!(
                    "def {ids} (k : List Nat) : Prop := exists s : State, Init s /\\ {has} k s.{}",
                    object.variable()
                ),
                obligation,
            ));
        }
        let ok = mangle("ok_", &clause.name);
        let mut ok_body = vec![format!("{has} {} s.{}", clause.binder, object.variable())];
        for def in &clause.defs {
            ok_body.push(input.show.expr(def)?);
        }
        out.push(define(
            clause.position,
            Some(ok.clone()),
            format!(
                "def {ok} (s : State) ({} : List Nat) : Prop := {}",
                clause.binder,
                ok_body.join(" /\\ ")
            ),
            obligation,
        ));
        let atom = mangle("at_", &clause.name);
        let holds = mangle("holds_", &clause.name);
        let ob = mangle("Ob_", &clause.name);
        let behavior = if input.dense.is_some() {
            "TBehaviour"
        } else {
            "Behaviour"
        };
        let binder = &clause.binder;
        for (index, e) in clause.cur.iter().enumerate() {
            out.push(define(
                clause.position,
                None,
                format!(
                    "def {atom}_{index} (s : State) ({binder} : List Nat) : Prop :=\n  {}",
                    input.show.expr(e)?
                ),
                obligation,
            ));
        }
        let mut lean = LeanClause {
            clause,
            atom,
            next: 0,
        };
        let formula = lean.at(&clause.tf, "i")?;
        out.push(define(
            clause.position,
            Some(holds.clone()),
            format!(
                "def {holds} (b : {behavior}) ({binder} : List Nat) (i : Int) : Prop :=\n  {formula}"
            ),
            obligation,
        ));
        let fairness = lean_fair(input, clause)?;
        let mut premise = String::new();
        if let Some(text) = fairness {
            let fair = mangle("fair_", &clause.name);
            out.push(define(
                clause.position,
                Some(fair.clone()),
                format!("def {fair} (b : Behaviour) : Prop :=\n  {text}"),
                obligation,
            ));
            premise = format!("{fair} b -> ");
        }
        out.push(define(
            clause.position,
            Some(ob.clone()),
            format!(
                "def {ob} : Prop :=\n  forall b : {behavior}, {premise}forall {binder}, {} {binder} -> (forall j : Nat, {ok} (b.st j) {binder}) /\\ {holds} b {binder} 0",
                mangle("Ids_", &object.name)
            ),
            obligation,
        ));
    }
    Ok(())
}
