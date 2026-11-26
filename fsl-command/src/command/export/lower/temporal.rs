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
    defer_at, define, mangle, parameters, refusal, refuse, render, Definition, ExprId, Graph,
    Input, Lower, Piece, Refusal, Target,
};
use filament_contract_model::{CheckedOperationInterval, CheckedProtocolActivation};
use filament_walk::{walk, Arena, Children, Id, Walk};
use fsl_package::checked_views::{
    temporal_subject, Activation, FormulaNodeUse, FormulaOperation, OperatorDirection,
};
use fsl_package::{CheckedTimeSourceKind, CheckedTimedModel};
use fsl_semantics::check::TemporalProfile;
use std::ops::ControlFlow;
use std::rc::Rc;

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

pub(super) type TfId = Id<Tf>;

/// A temporal formula over the clause's atoms. Its operands are ids of the
/// [`Formulas`] that holds it.
#[derive(Clone)]
pub(super) enum Tf {
    Const(bool),
    Atom(usize),
    /// A history variable that stands for a lowered past operator.
    Hist(usize),
    Not(TfId),
    And(TfId, TfId),
    Or(TfId, TfId),
    Implies(TfId, TfId),
    Always(Iv, TfId),
    Eventually(Iv, TfId),
    Once(Iv, TfId),
    Historically(Iv, TfId),
    Until(Iv, TfId, TfId),
    Release(Iv, TfId, TfId),
    Since(Iv, TfId, TfId),
    Triggered(Iv, TfId, TfId),
    Timed(Box<super::dense::Window>),
}

/// What a formula node is, computed from its operands when it is added.
#[derive(Clone, Copy, Default)]
pub(super) struct Facts {
    /// No temporal operator.
    pub(super) state: bool,
    /// A formula `[]` and `<>` state directly.
    pub(super) direct: bool,
    /// Boolean connectives over atoms and constants only.
    pub(super) atoms_only: bool,
    /// A timed window reached through conjunctions only.
    pub(super) window: bool,
}

/// The formulas of one clause. A formula is as deep as its source, so nodes
/// live in one arena, each after its operands, and no walk of them recurses
/// (ADR-030 D-1).
#[derive(Default)]
pub(super) struct Formulas {
    nodes: Arena<Tf>,
    /// Each node's facts, by its arena position.
    facts: Vec<Facts>,
}

impl Formulas {
    /// Add `tf`, whose operands are already ids of this arena.
    pub(super) fn add(&mut self, tf: Tf) -> TfId {
        let of = |id: &TfId| self.facts(*id);
        let facts = match &tf {
            Tf::Const(_) | Tf::Atom(_) => Facts {
                state: true,
                direct: true,
                atoms_only: true,
                window: false,
            },
            Tf::Hist(_) => Facts {
                state: true,
                direct: true,
                atoms_only: false,
                window: false,
            },
            Tf::Not(inner) => Facts {
                window: false,
                ..of(inner)
            },
            Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
                let (left, right) = (of(left), of(right));
                Facts {
                    state: left.state && right.state,
                    direct: left.direct && right.direct,
                    atoms_only: left.atoms_only && right.atoms_only,
                    window: matches!(tf, Tf::And(..)) && (left.window || right.window),
                }
            }
            Tf::Always(iv, inner) | Tf::Eventually(iv, inner) => Facts {
                direct: !iv.is_some() && of(inner).direct,
                ..Facts::default()
            },
            Tf::Timed(_) => Facts {
                window: true,
                ..Facts::default()
            },
            Tf::Once(..)
            | Tf::Historically(..)
            | Tf::Until(..)
            | Tf::Release(..)
            | Tf::Since(..)
            | Tf::Triggered(..) => Facts::default(),
        };
        self.facts.push(facts);
        self.nodes.push(tf)
    }

    pub(super) fn get(&self, id: TfId) -> Lower<&Tf> {
        self.nodes
            .get(id)
            .ok_or_else(|| refusal(CONSTRUCT, "a formula operand is not in the formula"))
    }

    /// Every node with its id, operands before the nodes that use them.
    pub(super) fn iter(&self) -> impl Iterator<Item = (TfId, &Tf)> {
        self.nodes.iter()
    }

    pub(super) fn facts(&self, id: TfId) -> Facts {
        self.facts.get(id.index()).copied().unwrap_or_default()
    }
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
    pub(super) formulas: Formulas,
    pub(super) tf: TfId,
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
    pub(super) formulas: Formulas,
    pub(super) tf: TfId,
    pub(super) formula_sites: Vec<FormulaSite>,
    /// The node a clause without a deterministic monitor is refused at.
    pub(super) refuse_at: Option<usize>,
    /// The FR-812 definedness conditions of the atoms, which `ok_<clause>`
    /// states.
    pub(super) defs: Vec<ExprId>,
    /// Each atom in the current state and in the next state.
    pub(super) cur: Vec<ExprId>,
    pub(super) next: Vec<ExprId>,
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

fn operand_at(items: &[TfId], index: usize) -> Lower<TfId> {
    items
        .get(index)
        .copied()
        .ok_or_else(|| refusal(CONSTRUCT, "a temporal operator is missing an operand"))
}

/// Where a formula use is in the formula.
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

/// What an entered use builds once its operands are built.
enum Built {
    /// A leaf, already built.
    Leaf,
    Window(super::dense::Pending),
    Operator {
        kind: FormulaOperation,
        interval: Iv,
        here: usize,
        arity: usize,
    },
}

/// The walk that reads the formula uses into [`Formulas`]; `done` holds the
/// operands not yet taken.
struct Build<'a, 'u> {
    graph: &'a Graph<'a>,
    uses: &'a [FormulaNodeUse<'u>],
    formulas: Formulas,
    done: Vec<TfId>,
    atom: usize,
    refuse_at: Option<usize>,
}

impl Build<'_, '_> {
    fn leaf(&mut self, tf: Tf) -> Built {
        let id = self.formulas.add(tf);
        self.done.push(id);
        Built::Leaf
    }

    fn enter_use(
        &mut self,
        (at, scope): (usize, Scope),
        children: &mut Children<'_, (usize, Scope)>,
    ) -> Lower<Built> {
        let node = self
            .uses
            .get(at)
            .ok_or_else(|| refusal(CONSTRUCT, "a formula operand is not in the formula"))?;
        let kind = node
            .kind()
            .ok_or_else(|| refusal(CONSTRUCT, "an operation that is no temporal operation"))?;
        if kind == FormulaOperation::Holds {
            self.atom += 1;
            return Ok(self.leaf(Tf::Atom(self.atom - 1)));
        }
        let here = at_node(self.graph, node.node)?;
        let direction = kind.direction();
        if let Some(CheckedOperationInterval::Timed { .. }) = &node.interval {
            if matches!(scope.window, WindowScope::Unsupported) {
                return defer_at(
                    here,
                    &self.graph.construct(here),
                    "a timed window outside the origin or its unbounded always",
                );
            }
            let (window, child) = super::dense::Window::read(self.graph, kind, node, here)?;
            children.push((
                child,
                Scope {
                    future: true,
                    past: false,
                    conj: false,
                    window: WindowScope::Unsupported,
                },
            ));
            return Ok(Built::Window(window));
        }
        let iv = interval(self.graph, node)?;
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
                self.refuse_at.get_or_insert(here);
            }
        }
        let inner = Scope {
            future: scope.future || direction == Some(OperatorDirection::Future),
            past: scope.past || direction == Some(OperatorDirection::Past),
            conj: scope.conj && kind == FormulaOperation::And,
            window: scope.window.through(kind, iv),
        };
        for &operand in &node.operands {
            children.push((operand, inner));
        }
        Ok(Built::Operator {
            kind,
            interval: iv,
            here,
            arity: node.operands.len(),
        })
    }

    fn finish(&mut self, built: Built) -> Lower<()> {
        let tf = match built {
            Built::Leaf => return Ok(()),
            Built::Window(window) => {
                let predicate = self
                    .done
                    .pop()
                    .ok_or_else(|| refusal(CONSTRUCT, "a timed operator lost its operand"))?;
                Tf::Timed(Box::new(window.finish(
                    self.graph,
                    &self.formulas,
                    predicate,
                )?))
            }
            Built::Operator {
                kind,
                interval: iv,
                here,
                arity,
            } => {
                let first = self
                    .done
                    .len()
                    .checked_sub(arity)
                    .ok_or_else(|| refusal(CONSTRUCT, "a temporal operator lost an operand"))?;
                let operands = self.done.split_off(first);
                let one = || operand_at(&operands, 0);
                let two = || operand_at(&operands, 1);
                match kind {
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
                    FormulaOperation::Not => Tf::Not(one()?),
                    FormulaOperation::And => Tf::And(one()?, two()?),
                    FormulaOperation::Or => Tf::Or(one()?, two()?),
                    FormulaOperation::Implies => Tf::Implies(one()?, two()?),
                    FormulaOperation::Always => Tf::Always(iv, one()?),
                    FormulaOperation::Eventually => Tf::Eventually(iv, one()?),
                    FormulaOperation::Once => Tf::Once(iv, one()?),
                    FormulaOperation::Historically => Tf::Historically(iv, one()?),
                    FormulaOperation::Until => Tf::Until(iv, one()?, two()?),
                    FormulaOperation::Release => Tf::Release(iv, one()?, two()?),
                    FormulaOperation::Since => Tf::Since(iv, one()?, two()?),
                    FormulaOperation::Triggered => Tf::Triggered(iv, one()?, two()?),
                }
            }
        };
        let id = self.formulas.add(tf);
        self.done.push(id);
        Ok(())
    }
}

impl Walk for Build<'_, '_> {
    /// A use of the formula, by its position in `uses`.
    type Node = (usize, Scope);
    type Frame = Built;
    type Stop = Refusal;

    fn enter(
        &mut self,
        node: (usize, Scope),
        children: &mut Children<'_, (usize, Scope)>,
    ) -> ControlFlow<Refusal, Built> {
        match self.enter_use(node, children) {
            Ok(built) => ControlFlow::Continue(built),
            Err(refusal) => ControlFlow::Break(refusal),
        }
    }

    fn exit(&mut self, built: Built) -> ControlFlow<Refusal> {
        match self.finish(built) {
            Ok(()) => ControlFlow::Continue(()),
            Err(refusal) => ControlFlow::Break(refusal),
        }
    }
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
                let trigger_position = at_node(graph, trigger)?;
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
    let mut build = Build {
        graph,
        uses: view.formula_graph(),
        formulas: Formulas::default(),
        done: Vec::new(),
        atom: 0,
        refuse_at: None,
    };
    if let ControlFlow::Break(refusal) = walk(&mut build, (0, start)) {
        return Err(refusal);
    }
    let Build {
        formulas,
        mut done,
        atom,
        refuse_at,
        ..
    } = build;
    let tf = done
        .pop()
        .ok_or_else(|| refusal(CONSTRUCT, "an empty formula"))?;
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
        formulas,
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

/// A formula to write at a position of the behaviour, negated or not.
struct At {
    tf: TfId,
    position: Rc<str>,
    negated: bool,
}

fn at(tf: TfId, position: Rc<str>) -> Piece<At> {
    Piece::Node(At {
        tf,
        position,
        negated: false,
    })
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
    fn at(&mut self, tf: TfId, position: &str) -> Lower<String> {
        render(
            At {
                tf,
                position: position.into(),
                negated: false,
            },
            |node, out| self.pieces(node, out),
        )
    }

    fn pieces(&mut self, node: At, out: &mut Vec<Piece<At>>) -> Lower<()> {
        let At {
            tf,
            position,
            negated,
        } = node;
        if negated {
            out.extend([Piece::text("Not ("), at(tf, position), Piece::text(")")]);
            return Ok(());
        }
        let clause = self.clause;
        match clause.formulas.get(tf)? {
            Tf::Const(true) => out.push(Piece::text("True")),
            Tf::Const(false) => out.push(Piece::text("False")),
            Tf::Hist(_) => {
                return refuse(CONSTRUCT, "a TLA+ monitor construct in a Lean definition");
            }
            Tf::Atom(index) => out.push(Piece::text(format!(
                "(0 <= {position} /\\ {} (b.st (Int.toNat {position})) {})",
                self.atom_name(*index),
                clause.binder
            ))),
            &Tf::Not(inner) => {
                out.extend([Piece::text("Not ("), at(inner, position), Piece::text(")")]);
            }
            &Tf::And(left, right) => Self::binary(out, left, " /\\ ", right, position),
            &Tf::Or(left, right) => Self::binary(out, left, " \\/ ", right, position),
            &Tf::Implies(left, right) => Self::binary(out, left, " -> ", right, position),
            &Tf::Always(iv, inner) => {
                let d = self.fresh("d");
                let window = Self::window(iv, &d, None).join(" -> ");
                out.extend([
                    Piece::text(format!("(forall {d} : Int, {window} -> ")),
                    at(inner, format!("({position} + {d})").into()),
                    Piece::text(")"),
                ]);
            }
            &Tf::Eventually(iv, inner) => {
                let d = self.fresh("d");
                let window = Self::window(iv, &d, None).join(" /\\ ");
                out.extend([
                    Piece::text(format!("(exists {d} : Int, {window} /\\ ")),
                    at(inner, format!("({position} + {d})").into()),
                    Piece::text(")"),
                ]);
            }
            &Tf::Historically(iv, inner) => {
                let d = self.fresh("d");
                let window = Self::window(iv, &d, Some(&position)).join(" -> ");
                out.extend([
                    Piece::text(format!("(forall {d} : Int, {window} -> ")),
                    at(inner, format!("({position} - {d})").into()),
                    Piece::text(")"),
                ]);
            }
            &Tf::Once(iv, inner) => {
                let d = self.fresh("d");
                let window = Self::window(iv, &d, Some(&position)).join(" /\\ ");
                out.extend([
                    Piece::text(format!("(exists {d} : Int, {window} /\\ ")),
                    at(inner, format!("({position} - {d})").into()),
                    Piece::text(")"),
                ]);
            }
            &Tf::Until(iv, left, right) => {
                self.relation(out, iv, (left, right, false), &position, "+");
            }
            &Tf::Since(iv, left, right) => {
                self.relation(out, iv, (left, right, false), &position, "-");
            }
            &Tf::Release(iv, left, right) => {
                out.push(Piece::text("Not "));
                self.relation(out, iv, (left, right, true), &position, "+");
            }
            &Tf::Triggered(iv, left, right) => {
                out.push(Piece::text("Not "));
                self.relation(out, iv, (left, right, true), &position, "-");
            }
            Tf::Timed(window) => {
                let d = self.fresh("d");
                let next = format!("({position} + {d})");
                let points =
                    format!("Q_sub (b.ts (Int.toNat {next})) (b.ts (Int.toNat {position}))");
                let bounds = window.lean_bounds(&format!("({points})"));
                out.push(Piece::text(match window.operator {
                    super::dense::Operator::Always => {
                        format!("(forall {d} : Int, 0 <= {d} -> 0 <= {next} -> ({bounds}) -> ")
                    }
                    super::dense::Operator::Eventually => {
                        format!("(exists {d} : Int, 0 <= {d} /\\ (0 <= {next} /\\ {bounds} /\\ ")
                    }
                }));
                match clause.formulas.get(window.predicate)? {
                    Tf::Atom(index) => out.push(Piece::text(format!(
                        "{} (b.st (Int.toNat {next})) {}",
                        self.atom_name(*index),
                        clause.binder,
                    ))),
                    _ => out.push(at(window.predicate, next.into())),
                }
                out.push(Piece::text(match window.operator {
                    super::dense::Operator::Always => ")",
                    super::dense::Operator::Eventually => "))",
                }));
            }
        }
        Ok(())
    }

    fn binary(
        out: &mut Vec<Piece<At>>,
        left: TfId,
        symbol: &'static str,
        right: TfId,
        position: Rc<str>,
    ) {
        out.extend([
            Piece::text("("),
            at(left, Rc::clone(&position)),
            Piece::text(symbol),
            at(right, position),
            Piece::text(")"),
        ]);
    }

    /// `left until right` or, reading backwards, `left since right`:
    /// some offset `d` in the interval makes `right` hold and every offset
    /// `e` from the interval's lower bound up to `d` makes `left` hold.
    /// `negated` reads both operands negated.
    fn relation(
        &mut self,
        out: &mut Vec<Piece<At>>,
        iv: Iv,
        (left, right, negated): (TfId, TfId, bool),
        position: &str,
        sign: &str,
    ) {
        let d = self.fresh("d");
        let e = self.fresh("e");
        let past = (sign == "-").then_some(position);
        let window = Self::window(iv, &d, past).join(" /\\ ");
        let operand = |tf, offset: &str| {
            Piece::Node(At {
                tf,
                position: format!("({position} {sign} {offset})").into(),
                negated,
            })
        };
        out.extend([
            Piece::text(format!("(exists {d} : Int, {window} /\\ ")),
            operand(right, &d),
            Piece::text(format!(
                " /\\ (forall {e} : Int, {} <= {e} -> {e} < {d} -> ",
                iv.lower()
            )),
            operand(left, &e),
            Piece::text("))"),
        ]);
    }
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
        for &def in &clause.defs {
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
        for (index, &e) in clause.cur.iter().enumerate() {
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
        let formula = lean.at(clause.tf, "i")?;
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
