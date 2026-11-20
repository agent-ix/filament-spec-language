// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813: the lowering of temporal clauses and fairness over the
//! exported state to TLA+ and Lean 4 text.
//!
//! A clause built only from `always`, `eventually` and Boolean connectives
//! over state formulas is `[]` and `<>` in TLA+. A past operator becomes a
//! history variable, `until`, `release` and an interval operator at the root
//! of a clause become a deterministic monitor composed with the
//! specification, and `eventually always` over a state formula becomes the
//! deterministic co-Buchi monitor. The Lean text is the direct definition by
//! recursion over positions for every operator. A clause shape the TLA+
//! encoding does not cover is a refusal that names its operator.

use super::{
    bullets, define, mangle, parameters, refusal, refuse, tla_domain, Definition, Graph, Input,
    Lower, Object, Operation, E,
};
use filament_contract_model::CheckedOperationInterval;
use fsl_package::checked_views::{temporal_subject, Activation, FormulaNodeUse, FormulaOperation};
use fsl_semantics::check::TemporalProfile;

const CONSTRUCT: &str = "temporal/temporal_clause";

/// An operator's interval: `None` for an unbounded operator, otherwise the
/// inclusive lower bound and the inclusive upper bound when there is one.
#[derive(Clone, Copy)]
pub(super) struct Iv(Option<(u64, Option<u64>)>);

impl Iv {
    fn lower(self) -> u64 {
        self.0.map_or(0, |(lower, _)| lower)
    }
    fn upper(self) -> Option<u64> {
        self.0.and_then(|(_, upper)| upper)
    }
    fn is_some(self) -> bool {
        self.0.is_some()
    }
}

/// A temporal formula over the clause's atoms.
#[derive(Clone)]
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
}

/// One fairness constraint of a clause, as read from the package.
pub(super) struct FairRead {
    pub(super) strong: bool,
    pub(super) each: bool,
    /// The object type that declares the operation.
    pub(super) declaration: String,
    pub(super) operation: String,
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
    pub(super) tf: Tf,
    /// The node of each atom's Boolean operand, in formula order.
    pub(super) atoms: Vec<usize>,
    pub(super) fairness: Vec<FairRead>,
}

/// A temporal clause with its atoms lowered.
pub(super) struct ClauseLow {
    pub(super) name: String,
    pub(super) position: usize,
    pub(super) object: usize,
    pub(super) binder: String,
    pub(super) tf: Tf,
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

fn interval(value: &Option<CheckedOperationInterval>) -> Lower<Iv> {
    let bound = |text: String| {
        text.parse::<u64>()
            .map_err(|_| refusal("temporal/interval", "an interval bound beyond 64 bits"))
    };
    match value {
        None => Ok(Iv(None)),
        Some(CheckedOperationInterval::Integer { lower, upper }) => Ok(Iv(Some((
            bound(lower.to_string())?,
            upper
                .as_ref()
                .map(|upper| bound(upper.to_string()))
                .transpose()?,
        )))),
        Some(CheckedOperationInterval::Timed { .. }) => refuse(
            "temporal/interval",
            "an interval over time stamps, which this build does not lower",
        ),
    }
}

fn boxed(items: &[Tf], index: usize) -> Lower<Box<Tf>> {
    items
        .get(index)
        .cloned()
        .map(Box::new)
        .ok_or_else(|| refusal(CONSTRUCT, "a temporal operator is missing an operand"))
}

fn build(uses: &[FormulaNodeUse<'_>], at: usize, atom: &mut usize) -> Lower<Tf> {
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
    let mut operands = Vec::new();
    for &operand in &node.operands {
        operands.push(build(uses, operand, atom)?);
    }
    let iv = interval(&node.interval)?;
    Ok(match kind {
        FormulaOperation::True => Tf::Const(true),
        FormulaOperation::False => Tf::Const(false),
        FormulaOperation::Holds => unreachable!("read above"),
        FormulaOperation::Aggregate => {
            return refuse(
                "temporal/aggregate",
                "an aggregate atom over an accumulation of the trace",
            )
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
pub(super) fn read(graph: &Graph<'_>, name: &str, position: usize) -> Lower<ClauseRead> {
    let node = graph.node(position);
    let view = temporal_subject(graph.package, &node.node_id).map_err(|_| {
        refusal(
            CONSTRUCT,
            "a clause the package does not read as a temporal clause",
        )
    })?;
    let profile = TemporalProfile::from_identity(&view.profile().definition.identity);
    if profile != Some(TemporalProfile::InfiniteTrace) {
        return refuse(
            "temporal/profile",
            "a profile other than filament.temporal.infinite-trace/v1",
        );
    }
    if !matches!(view.activation(), Activation::Origin) || !view.captures().is_empty() {
        return refuse(CONSTRUCT, "an activation other than `on origin`");
    }
    let mut atom = 0;
    let tf = build(view.formula_graph(), 0, &mut atom)?;
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
    let over = graph
        .position(view.over())
        .ok_or_else(|| refusal(CONSTRUCT, "the `over` binder names no package node"))?;
    let binder = super::parameter_name(graph, over)
        .ok_or_else(|| refusal(CONSTRUCT, "the `over` binder has no name"))?;
    let (context, context_node) = super::context_of(graph, over)?;
    Ok(ClauseRead {
        name: name.to_owned(),
        position,
        context,
        context_node,
        binder: mangle("p_", &binder),
        tf,
        atoms,
        fairness: super::fairness_of(graph, position)?,
    })
}

// ---- Shared analysis -----------------------------------------------------

fn is_state(tf: &Tf) -> bool {
    match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => true,
        Tf::Not(inner) => is_state(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            is_state(left) && is_state(right)
        }
        _ => false,
    }
}

/// A formula `[]` and `<>` state directly.
fn is_direct(tf: &Tf) -> bool {
    match tf {
        Tf::Not(inner) => is_direct(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            is_direct(left) && is_direct(right)
        }
        Tf::Always(iv, inner) | Tf::Eventually(iv, inner) => !iv.is_some() && is_direct(inner),
        _ => is_state(tf),
    }
}

fn operator_name(tf: &Tf) -> &'static str {
    match tf {
        Tf::Always(..) => "always",
        Tf::Eventually(..) => "eventually",
        Tf::Once(..) => "once",
        Tf::Historically(..) => "historically",
        Tf::Until(..) => "until",
        Tf::Release(..) => "release",
        Tf::Since(..) => "since",
        Tf::Triggered(..) => "triggered",
        _ => "formula",
    }
}

/// The first temporal operator of `tf` that no encoding below covers.
fn first_operator(tf: &Tf) -> &'static str {
    match tf {
        Tf::Not(inner) => first_operator(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            if is_direct(left) {
                first_operator(right)
            } else {
                first_operator(left)
            }
        }
        Tf::Always(_, inner) | Tf::Eventually(_, inner) if !is_state(inner) => {
            if is_direct(inner) {
                operator_name(tf)
            } else {
                first_operator(inner)
            }
        }
        other => operator_name(other),
    }
}

/// A past operator replaced by a history variable.
struct History {
    kind: HistoryKind,
    operand: Tf,
    other: Option<Tf>,
    /// The value of the operator before the origin.
    before: bool,
}

#[derive(Clone, Copy)]
enum HistoryKind {
    Once,
    Historically,
    Since,
    Triggered,
}

/// The value `tf` has at every position before the origin, where each atom
/// is false.
fn before_origin(tf: &Tf, histories: &[History]) -> bool {
    match tf {
        Tf::Const(value) => *value,
        Tf::Atom(_) => false,
        Tf::Hist(index) => histories.get(*index).is_some_and(|history| history.before),
        Tf::Not(inner) => !before_origin(inner, histories),
        Tf::And(left, right) => before_origin(left, histories) && before_origin(right, histories),
        Tf::Or(left, right) => before_origin(left, histories) || before_origin(right, histories),
        Tf::Implies(left, right) => {
            !before_origin(left, histories) || before_origin(right, histories)
        }
        _ => false,
    }
}

/// Replace each past operator of `tf` by a history variable, innermost
/// first.
fn extract(tf: &Tf, histories: &mut Vec<History>) -> Lower<Tf> {
    let push = |kind, operand: Tf, other: Option<Tf>, histories: &mut Vec<History>| {
        // At every position before the origin the operand has the value
        // `before_origin` computes, so each past operator has the value of
        // its deciding operand there.
        let before = before_origin(other.as_ref().unwrap_or(&operand), histories);
        histories.push(History {
            kind,
            operand,
            other,
            before,
        });
        Tf::Hist(histories.len() - 1)
    };
    let bounded = |iv: &Iv| {
        if iv.is_some() {
            refuse(
                "temporal/interval",
                "an interval on a past operator in a TLA+ export",
            )
        } else {
            Ok(())
        }
    };
    Ok(match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => tf.clone(),
        Tf::Not(inner) => Tf::Not(Box::new(extract(inner, histories)?)),
        Tf::And(left, right) => Tf::And(
            Box::new(extract(left, histories)?),
            Box::new(extract(right, histories)?),
        ),
        Tf::Or(left, right) => Tf::Or(
            Box::new(extract(left, histories)?),
            Box::new(extract(right, histories)?),
        ),
        Tf::Implies(left, right) => Tf::Implies(
            Box::new(extract(left, histories)?),
            Box::new(extract(right, histories)?),
        ),
        Tf::Always(iv, inner) => Tf::Always(*iv, Box::new(extract(inner, histories)?)),
        Tf::Eventually(iv, inner) => Tf::Eventually(*iv, Box::new(extract(inner, histories)?)),
        Tf::Until(iv, left, right) => Tf::Until(
            *iv,
            Box::new(extract(left, histories)?),
            Box::new(extract(right, histories)?),
        ),
        Tf::Release(iv, left, right) => Tf::Release(
            *iv,
            Box::new(extract(left, histories)?),
            Box::new(extract(right, histories)?),
        ),
        Tf::Once(iv, inner) => {
            bounded(iv)?;
            let operand = extract(inner, histories)?;
            push(HistoryKind::Once, operand, None, histories)
        }
        Tf::Historically(iv, inner) => {
            bounded(iv)?;
            let operand = extract(inner, histories)?;
            push(HistoryKind::Historically, operand, None, histories)
        }
        Tf::Since(iv, left, right) => {
            bounded(iv)?;
            let (left, right) = (extract(left, histories)?, extract(right, histories)?);
            push(HistoryKind::Since, left, Some(right), histories)
        }
        Tf::Triggered(iv, left, right) => {
            bounded(iv)?;
            let (left, right) = (extract(left, histories)?, extract(right, histories)?);
            push(HistoryKind::Triggered, left, Some(right), histories)
        }
    })
}

// ---- TLA+ ----------------------------------------------------------------

/// A clause's TLA+ naming.
struct Names {
    /// `m_<clause>`: the monitor variable.
    monitor: String,
    counter: String,
    vars: String,
    type_ok: String,
    init: String,
    mon: String,
    next: String,
    spec: String,
    accept: String,
    theorem: String,
}

impl Names {
    fn new(name: &str) -> Self {
        Self {
            monitor: mangle("m_", name),
            counter: mangle("n_", name),
            vars: mangle("vars_", name),
            type_ok: mangle("TypeOK_", name),
            init: mangle("Init_", name),
            mon: mangle("Mon_", name),
            next: mangle("Next_", name),
            spec: mangle("Spec_", name),
            accept: mangle("accept_", name),
            theorem: mangle("Th_", name),
        }
    }

    fn history(&self, index: usize) -> String {
        format!("{}_h{index}", self.monitor)
    }
}

/// How a root operator is monitored.
#[derive(Clone, Copy, Eq, PartialEq)]
enum MonitorKind {
    /// `until` and `eventually` with an interval: accepting once satisfied.
    Reach,
    /// `release` and `always` with an interval: rejecting once violated.
    Safety,
}

struct Monitor {
    kind: MonitorKind,
    operator: &'static str,
    iv: Iv,
    left: Tf,
    right: Option<Tf>,
}

/// The shape a clause's formula lowers by.
enum Shape {
    /// `[]` and `<>` directly.
    Direct,
    Monitor(Monitor),
    /// `eventually always` over a state formula, as a deterministic
    /// co-Buchi monitor.
    CoBuchi(Tf),
}

fn shape_of(tf: &Tf) -> Lower<Shape> {
    if is_direct(tf) {
        if let Tf::Eventually(iv, inner) = tf {
            if let (false, Tf::Always(inner_iv, state)) = (iv.is_some(), &**inner) {
                if !inner_iv.is_some() && is_state(state) {
                    return Ok(Shape::CoBuchi((**state).clone()));
                }
            }
        }
        return Ok(Shape::Direct);
    }
    let (kind, left, right, iv) = match tf {
        Tf::Until(iv, left, right) => (MonitorKind::Reach, left, Some(right), *iv),
        Tf::Release(iv, left, right) => (MonitorKind::Safety, left, Some(right), *iv),
        Tf::Always(iv, inner) if iv.is_some() => (MonitorKind::Safety, inner, None, *iv),
        Tf::Eventually(iv, inner) if iv.is_some() => (MonitorKind::Reach, inner, None, *iv),
        other => {
            return refuse(
                &format!("temporal/{}", first_operator(other)),
                "a future operator nested in another future operator has no deterministic monitor in this build",
            )
        }
    };
    if !is_state(left) || !right.is_none_or(|right| is_state(right)) {
        return refuse(
            &format!("temporal/{}", operator_name(tf)),
            "a future operator over a formula that holds another future operator has no deterministic monitor in this build",
        );
    }
    Ok(Shape::Monitor(Monitor {
        kind,
        operator: operator_name(tf),
        iv,
        left: (**left).clone(),
        right: right.map(|right| (**right).clone()),
    }))
}

struct TlaClause<'a> {
    input: &'a Input<'a>,
    clause: &'a ClauseLow,
    object: &'a Object,
    names: Names,
}

impl TlaClause<'_> {
    fn key(&self) -> &str {
        &self.clause.binder
    }

    fn atom(&self, index: usize, primed: bool) -> Lower<String> {
        let atoms = if primed {
            &self.clause.next
        } else {
            &self.clause.cur
        };
        let e = atoms
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "an atom with no lowered operand"))?;
        self.input.show.expr(e)
    }

    /// A state formula at the current or the next state.
    fn state(&self, tf: &Tf, primed: bool) -> Lower<String> {
        let tick = if primed { "'" } else { "" };
        Ok(match tf {
            Tf::Const(true) => "TRUE".to_owned(),
            Tf::Const(false) => "FALSE".to_owned(),
            Tf::Atom(index) => self.atom(*index, primed)?,
            Tf::Hist(index) => {
                format!("{}{tick}[{}]", self.names.history(*index), self.key())
            }
            Tf::Not(inner) => format!("~({})", self.state(inner, primed)?),
            Tf::And(left, right) => format!(
                "({} /\\ {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            Tf::Or(left, right) => format!(
                "({} \\/ {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            Tf::Implies(left, right) => format!(
                "({} => {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            _ => return refuse(CONSTRUCT, "a temporal operator read as a state formula"),
        })
    }

    /// A formula whose temporal operators are `[]` and `<>`.
    fn direct(&self, tf: &Tf) -> Lower<String> {
        if is_state(tf) {
            return self.state(tf, false);
        }
        Ok(match tf {
            Tf::Not(inner) => format!("~({})", self.direct(inner)?),
            Tf::And(left, right) => {
                format!("({} /\\ {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Or(left, right) => {
                format!("({} \\/ {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Implies(left, right) => {
                format!("({} => {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Always(_, inner) => format!("[]({})", self.direct(inner)?),
            Tf::Eventually(_, inner) => format!("<>({})", self.direct(inner)?),
            _ => return refuse(CONSTRUCT, "a temporal operator read as `[]` or `<>`"),
        })
    }
}

fn in_window(iv: Iv, position: &str) -> String {
    match (iv.lower(), iv.upper()) {
        (0, None) => "TRUE".to_owned(),
        (lower, None) => format!("({lower} <= {position})"),
        (lower, Some(upper)) => format!("({lower} <= {position} /\\ {position} <= {upper})"),
    }
}

fn from_lower(iv: Iv, position: &str) -> String {
    match iv.lower() {
        0 => "TRUE".to_owned(),
        lower => format!("({lower} <= {position})"),
    }
}

/// `left /\ right`, dropping a `TRUE` operand.
fn both(left: &str, right: &str) -> String {
    if left == "TRUE" {
        right.to_owned()
    } else {
        format!("{left} /\\ {right}")
    }
}

/// The monitor's next state from its current state `m` (`None` at the
/// origin, where the monitor is running), the position of the state it reads
/// and the operands `left` and `right` there.
fn monitor_step(
    monitor: &Monitor,
    m: Option<&str>,
    position: &str,
    left: &str,
    right: &str,
) -> String {
    let iv = monitor.iv;
    let window = in_window(iv, position);
    let tail = from_lower(iv, position);
    let timeout = match iv.upper() {
        Some(upper) => format!("IF {position} >= {upper} THEN \"rej\" ELSE \"run\""),
        None => "\"run\"".to_owned(),
    };
    let running = m.map_or_else(String::new, |m| format!("{m} = \"run\" /\\ "));
    let (settled, body) = match monitor.operator {
        "until" => (
            format!("{} /= \"run\"", m.unwrap_or_default()),
            format!(
                "IF {} THEN \"acc\" ELSE IF {} THEN \"rej\" ELSE {timeout}",
                both(&window, right),
                both(&tail, &format!("~{left}"))
            ),
        ),
        "eventually" => (
            format!("{} /= \"run\"", m.unwrap_or_default()),
            format!("IF {} THEN \"acc\" ELSE {timeout}", both(&window, left)),
        ),
        "release" => (
            format!("{} = \"rej\"", m.unwrap_or_default()),
            format!(
                "IF {running}{} THEN \"rej\" ELSE IF {running}{} THEN \"rel\" ELSE {}",
                both(&window, &format!("~{right}")),
                both(&tail, left),
                m.unwrap_or("\"run\"")
            ),
        ),
        _ => (
            format!("{} = \"rej\"", m.unwrap_or_default()),
            format!(
                "IF {} THEN \"rej\" ELSE \"run\"",
                both(&window, &format!("~{left}"))
            ),
        ),
    };
    match (m, monitor.operator) {
        (None, _) => body,
        (Some(m), "until" | "eventually") => format!("IF {settled} THEN {m} ELSE {body}"),
        (Some(_), _) => format!("IF {settled} THEN \"rej\" ELSE {body}"),
    }
}

fn history_step(history: &History, operand: &str, other: &str, previous: &str) -> String {
    match history.kind {
        HistoryKind::Once => format!("({operand} \\/ {previous})"),
        HistoryKind::Historically => format!("({operand} /\\ {previous})"),
        HistoryKind::Since => format!("({other} \\/ ({operand} /\\ {previous}))"),
        HistoryKind::Triggered => format!("({other} /\\ ({operand} \\/ {previous}))"),
    }
}

fn boolean(value: bool) -> &'static str {
    if value {
        "TRUE"
    } else {
        "FALSE"
    }
}

/// The action an operation contributes, with its arguments chosen.
fn operation_action(object: &Object, operation: &Operation, receiver: bool) -> String {
    let v = object.variable();
    let name = mangle("op_", &operation.name);
    let params = parameters(operation);
    let names: Vec<&str> = params.iter().map(|(name, _)| name.as_str()).collect();
    let binders: Vec<String> = params
        .iter()
        .skip(usize::from(receiver))
        .map(|(param, kind)| match kind {
            None => format!("{param} \\in DOMAIN {v}"),
            Some(kind) => format!("{param} \\in {}", tla_domain(kind)),
        })
        .collect();
    let call = format!("{name}({})", names.join(", "));
    if binders.is_empty() {
        call
    } else {
        format!("\\E {} : {call}", binders.join(", "))
    }
}

/// Lower the clauses of `input` to TLA+. The result is the variables the
/// clauses add.
pub(super) fn tla(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<Vec<String>> {
    let mut variables = Vec::new();
    for clause in input.temporals {
        let object = input
            .objects
            .get(clause.object)
            .ok_or_else(|| refusal(CONSTRUCT, "an unexported clause context"))?;
        let this = TlaClause {
            input,
            clause,
            object,
            names: Names::new(&clause.name),
        };
        variables.extend(tla_clause(&this, out)?);
    }
    Ok(variables)
}

fn tla_clause(this: &TlaClause<'_>, out: &mut Vec<Definition>) -> Lower<Vec<String>> {
    let clause = this.clause;
    let names = &this.names;
    let v = this.object.variable();
    let key = this.key();
    let obligation = Some(clause.name.as_str());
    let mut histories = Vec::new();
    let tf = extract(&clause.tf, &mut histories)?;
    let shape = shape_of(&tf)?;
    let monitor = match &shape {
        Shape::Monitor(monitor) => Some(monitor),
        _ => None,
    };
    let counted = monitor.is_some_and(|monitor| monitor.iv.is_some());
    let cap = monitor.and_then(|monitor| {
        monitor.iv.is_some().then(|| {
            monitor
                .iv
                .upper()
                .map_or(monitor.iv.lower(), |upper| upper.saturating_add(1))
        })
    });

    // The variables the clause adds, with their types, initial values and
    // updates.
    let mut variables = Vec::new();
    let mut types = Vec::new();
    let mut inits = Vec::new();
    let mut updates = Vec::new();
    for (index, history) in histories.iter().enumerate() {
        let name = names.history(index);
        types.push(format!("{name} \\in [DOMAIN {v} -> BOOLEAN]"));
        let step = |primed: bool, previous: &str| -> Lower<String> {
            let operand = this.state(&history.operand, primed)?;
            let other = match &history.other {
                Some(other) => this.state(other, primed)?,
                None => String::new(),
            };
            Ok(history_step(history, &operand, &other, previous))
        };
        inits.push(format!(
            "{name} = [{key} \\in DOMAIN {v} |-> {}]",
            step(false, boolean(history.before))?
        ));
        updates.push(format!(
            "{name}' = [{key} \\in DOMAIN {v} |-> {}]",
            step(true, &format!("{name}[{key}]"))?
        ));
        variables.push(name);
    }
    if let Some(monitor) = monitor {
        let states = if monitor.operator == "release" {
            "{\"run\", \"rel\", \"rej\"}"
        } else {
            "{\"run\", \"acc\", \"rej\"}"
        };
        let m = &names.monitor;
        types.push(format!("{m} \\in [DOMAIN {v} -> {states}]"));
        let pair = |primed: bool| -> Lower<(String, String)> {
            let left = this.state(&monitor.left, primed)?;
            let right = match &monitor.right {
                Some(right) => this.state(right, primed)?,
                None => String::new(),
            };
            Ok((left, right))
        };
        let (left, right) = pair(false)?;
        inits.push(format!(
            "{m} = [{key} \\in DOMAIN {v} |-> {}]",
            monitor_step(monitor, None, "0", &left, &right)
        ));
        let (left, right) = pair(true)?;
        let position = if counted {
            format!("{}'", names.counter)
        } else {
            "0".to_owned()
        };
        updates.push(format!(
            "{m}' = [{key} \\in DOMAIN {v} |-> {}]",
            monitor_step(
                monitor,
                Some(&format!("{m}[{key}]")),
                &position,
                &left,
                &right
            )
        ));
        variables.push(m.clone());
        if let Some(cap) = cap {
            let n = &names.counter;
            types.push(format!("{n} \\in 0..{cap}"));
            inits.push(format!("{n} = 0"));
            updates.push(format!("{n}' = IF {n} < {cap} THEN {n} + 1 ELSE {cap}"));
            variables.push(n.clone());
        }
    }
    if let Shape::CoBuchi(state) = &shape {
        let m = &names.monitor;
        types.push(format!("{m} \\in [DOMAIN {v} -> BOOLEAN]"));
        inits.push(format!(
            "{m} = [{key} \\in DOMAIN {v} |-> {}]",
            this.state(state, false)?
        ));
        updates.push(format!(
            "{m}' = [{key} \\in DOMAIN {v} |-> {}]",
            this.state(state, true)?
        ));
        variables.push(m.clone());
    }

    let composed = !variables.is_empty();
    let mut fair = Vec::new();
    for constraint in &clause.fairness {
        let object =
            this.input.objects.get(constraint.object).ok_or_else(|| {
                refusal("temporal/fairness", "a fairness target outside the state")
            })?;
        let operation = object
            .operations
            .get(constraint.operation)
            .ok_or_else(|| refusal("temporal/fairness", "an operation outside the state"))?;
        let subscript = if composed { &names.vars } else { "vars" };
        let kind = if constraint.strong { "SF" } else { "WF" };
        let mon = if composed {
            format!(" /\\ {}", names.mon)
        } else {
            String::new()
        };
        let text = if constraint.each {
            let action = operation_action(object, operation, true);
            format!(
                "\\A p_self \\in DOMAIN {} : {kind}_{subscript}(({action}){mon})",
                object.variable()
            )
        } else {
            let action = operation_action(object, operation, false);
            format!("{kind}_{subscript}(({action}){mon})")
        };
        fair.push(text);
    }

    if composed {
        let list: Vec<&str> = std::iter::once("vars")
            .chain(variables.iter().map(String::as_str))
            .collect();
        out.push(define(
            clause.position,
            Some(names.vars.clone()),
            format!("{} == <<{}>>", names.vars, list.join(", ")),
            obligation,
        ));
        out.push(define(
            clause.position,
            Some(names.type_ok.clone()),
            format!("{} ==\n{}", names.type_ok, bullets(&types)),
            obligation,
        ));
        let mut init = vec!["Init".to_owned(), names.type_ok.clone()];
        init.extend(inits);
        out.push(define(
            clause.position,
            Some(names.init.clone()),
            format!("{} ==\n{}", names.init, bullets(&init)),
            obligation,
        ));
        out.push(define(
            clause.position,
            Some(names.mon.clone()),
            format!("{} ==\n{}", names.mon, bullets(&updates)),
            obligation,
        ));
        out.push(define(
            clause.position,
            Some(names.next.clone()),
            format!("{} == Next /\\ {}", names.next, names.mon),
            obligation,
        ));
        let mut spec = format!(
            "{} == {} /\\ [][{}]_{} /\\ WF_{}({})",
            names.spec, names.init, names.next, names.vars, names.vars, names.next
        );
        for text in &fair {
            spec.push_str(&format!(" /\\ {text}"));
        }
        out.push(define(
            clause.position,
            Some(names.spec.clone()),
            spec,
            obligation,
        ));
    } else if !fair.is_empty() {
        out.push(define(
            clause.position,
            Some(names.spec.clone()),
            format!("{} == Spec /\\ {}", names.spec, fair.join(" /\\ ")),
            obligation,
        ));
    }
    let premise = if composed || !fair.is_empty() {
        names.spec.as_str()
    } else {
        "Spec"
    };
    let quantifier = format!("\\A {key} \\in DOMAIN {v} :");
    let statement = match &shape {
        Shape::Direct => format!("{quantifier} {}", this.direct(&tf)?),
        Shape::CoBuchi(_) => {
            let m = &names.monitor;
            out.push(define(
                clause.position,
                Some(names.accept.clone()),
                format!("{} == {quantifier} {m}[{key}]", names.accept),
                obligation,
            ));
            format!("<>[]{}", names.accept)
        }
        Shape::Monitor(monitor) => {
            let m = &names.monitor;
            let (condition, wrap) = match monitor.kind {
                MonitorKind::Reach => (format!("{m}[{key}] = \"acc\""), "[]<>"),
                MonitorKind::Safety => (format!("{m}[{key}] /= \"rej\""), "[]"),
            };
            out.push(define(
                clause.position,
                Some(names.accept.clone()),
                format!("{} == {quantifier} {condition}", names.accept),
                obligation,
            ));
            format!("{wrap}{}", names.accept)
        }
    };
    out.push(define(
        clause.position,
        Some(names.theorem.clone()),
        format!("THEOREM {} == {premise} => {statement}", names.theorem),
        obligation,
    ));
    Ok(variables)
}

// ---- Lean 4 --------------------------------------------------------------

struct LeanClause<'a> {
    clause: &'a ClauseLow,
    holds: String,
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

    fn window(iv: Iv, offset: &str) -> String {
        match (iv.lower(), iv.upper()) {
            (0, None) => format!("0 <= {offset}"),
            (lower, None) => format!("{lower} <= {offset}"),
            (lower, Some(upper)) => format!("{lower} <= {offset} /\\ {offset} <= {upper}"),
        }
    }

    /// `tf` at the position `position` of the behaviour `b`.
    fn at(&mut self, tf: &Tf, position: &str) -> String {
        match tf {
            Tf::Const(true) => "True".to_owned(),
            Tf::Const(false) => "False".to_owned(),
            Tf::Hist(_) => "False".to_owned(),
            Tf::Atom(index) => format!(
                "(0 <= {position} /\\ {} (b.st (Int.toNat {position})) {})",
                self.atom_name(*index),
                self.clause.binder
            ),
            Tf::Not(inner) => format!("Not ({})", self.at(inner, position)),
            Tf::And(left, right) => {
                format!(
                    "({} /\\ {})",
                    self.at(left, position),
                    self.at(right, position)
                )
            }
            Tf::Or(left, right) => {
                format!(
                    "({} \\/ {})",
                    self.at(left, position),
                    self.at(right, position)
                )
            }
            Tf::Implies(left, right) => {
                format!(
                    "({} -> {})",
                    self.at(left, position),
                    self.at(right, position)
                )
            }
            Tf::Always(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} + {d})"));
                format!(
                    "(forall {d} : Int, {} -> {body})",
                    Self::window(*iv, &d).replace(" /\\ ", " -> ")
                )
            }
            Tf::Eventually(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} + {d})"));
                format!("(exists {d} : Int, {} /\\ {body})", Self::window(*iv, &d))
            }
            Tf::Historically(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} - {d})"));
                format!(
                    "(forall {d} : Int, {} -> {body})",
                    Self::window(*iv, &d).replace(" /\\ ", " -> ")
                )
            }
            Tf::Once(iv, inner) => {
                let d = self.fresh("d");
                let body = self.at(inner, &format!("({position} - {d})"));
                format!("(exists {d} : Int, {} /\\ {body})", Self::window(*iv, &d))
            }
            Tf::Until(iv, left, right) => self.relation(*iv, left, right, position, "+"),
            Tf::Since(iv, left, right) => self.relation(*iv, left, right, position, "-"),
            Tf::Release(iv, left, right) => {
                let (left, right) = (negate(left), negate(right));
                format!("Not {}", self.relation(*iv, &left, &right, position, "+"))
            }
            Tf::Triggered(iv, left, right) => {
                let (left, right) = (negate(left), negate(right));
                format!("Not {}", self.relation(*iv, &left, &right, position, "-"))
            }
        }
    }

    /// `left until right` or, reading backwards, `left since right`:
    /// some offset `d` in the interval makes `right` hold and every offset
    /// `e` from the interval's lower bound up to `d` makes `left` hold.
    fn relation(&mut self, iv: Iv, left: &Tf, right: &Tf, position: &str, sign: &str) -> String {
        let d = self.fresh("d");
        let e = self.fresh("e");
        let right = self.at(right, &format!("({position} {sign} {d})"));
        let left = self.at(left, &format!("({position} {sign} {e})"));
        format!(
            "(exists {d} : Int, {} /\\ {right} /\\ (forall {e} : Int, {} <= {e} -> {e} < {d} -> {left}))",
            Self::window(iv, &d),
            iv.lower()
        )
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
        let name = mangle("op_", &operation.name);
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
                "(forall p_self, {} p_self (b.st 0).{} -> {premise})",
                mangle("F_has_", &object.name),
                object.variable()
            )
        } else {
            premise
        });
    }
    Ok((!premises.is_empty()).then(|| premises.join(" /\\\n  ")))
}

/// Lower the clauses of `input` to Lean 4.
pub(super) fn lean(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
    for clause in input.temporals {
        let object = input
            .objects
            .get(clause.object)
            .ok_or_else(|| refusal(CONSTRUCT, "an unexported clause context"))?;
        let obligation = Some(clause.name.as_str());
        let atom = mangle("a_", &clause.name);
        let holds = mangle("holds_", &clause.name);
        let ob = mangle("Ob_", &clause.name);
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
            holds: holds.clone(),
            atom,
            next: 0,
        };
        let formula = lean.at(&clause.tf, "i");
        out.push(define(
            clause.position,
            Some(lean.holds.clone()),
            format!(
                "def {holds} (b : Behaviour) ({binder} : List Nat) (i : Int) : Prop :=\n  {formula}"
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
                "def {ob} : Prop :=\n  forall b : Behaviour, {premise}forall {binder}, {} {binder} (b.st 0).{} -> {holds} b {binder} 0",
                mangle("F_has_", &object.name),
                object.variable()
            ),
            obligation,
        ));
    }
    Ok(())
}
