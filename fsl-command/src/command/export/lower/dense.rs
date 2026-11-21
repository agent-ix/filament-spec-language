// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813's timed transition relation for a model with scalar clocks.

mod formula;

pub(super) use self::formula::tla_clause;
use super::temporal::ClauseRead;
use super::{
    bullets, defer_at, define, mangle, operation_anchor, parameters, refusal, rename_param, str_of,
    tla_domain, Definition, Input, Low, Lower, Lowerer, E,
};
use filament_contract_model::CheckedNodeId;
use fsl_package::{
    CheckedClockConstraint, CheckedConstraintTarget, CheckedResetTarget, CheckedTimedModel,
    CheckedUrgency, ClockComparison, ClockPathStep, TimedNodeMetadata,
};
use std::collections::BTreeMap;

const MODEL: &str = "timed_model/timed_model";

#[derive(Clone, PartialEq)]
struct Quantity {
    numerator: String,
    denominator: String,
}

impl Quantity {
    fn read(spelling: &str) -> Lower<Self> {
        let (numerator, denominator) = spelling
            .split_once('/')
            .ok_or_else(|| refusal(MODEL, "a clock bound with no denominator"))?;
        Ok(Self {
            numerator: numerator.to_owned(),
            denominator: denominator.to_owned(),
        })
    }

    fn tla(&self) -> String {
        if self.denominator == "1" {
            self.numerator.clone()
        } else {
            format!("({} / {})", self.numerator, self.denominator)
        }
    }

    fn lean(&self) -> String {
        format!("(Q.mk {} {})", self.numerator, self.denominator)
    }
}

struct Clock {
    name: String,
    node: usize,
}

struct Bound {
    clock: usize,
    op: ClockComparison,
    value: Quantity,
}

struct Restriction {
    predicate: E,
    bounds: Vec<Bound>,
}

#[derive(Default)]
struct Action {
    guards: Vec<Restriction>,
    resets: BTreeMap<usize, Quantity>,
    urgent: bool,
}

pub(super) struct Model {
    pub(super) node: usize,
    clocks: Vec<Clock>,
    invariants: Vec<Restriction>,
    urgency: Vec<E>,
    actions: Vec<Action>,
}

impl Model {
    pub(super) fn read(
        lowerer: &mut Lowerer<'_>,
        clauses: &[ClauseRead],
        models: &[CheckedTimedModel],
        wanted: &[(String, usize)],
    ) -> Lower<Option<Self>> {
        let Some(clause) = clauses.iter().find(|clause| clause.timed_root.is_some()) else {
            for object in lowerer.objects {
                if let Some(model) = models
                    .iter()
                    .find(|model| model.model == lowerer.graph.node(object.node).node_id)
                {
                    return defer_at(
                        position(lowerer, &model.metadata.node_id)?,
                        MODEL,
                        "a timed state export without a model-time temporal obligation",
                    );
                }
                if !object.clocks.is_empty() {
                    return defer_at(
                        object.node,
                        "model/field",
                        "a clock field without a lowered timed model",
                    );
                }
            }
            return Ok(None);
        };
        let root = clause.timed_root.unwrap_or(clause.position);
        let result = (|| {
            if clauses.len() != wanted.len()
                || clauses.iter().any(|other| other.timed_root != Some(root))
            {
                return defer_at(
                    root,
                    MODEL,
                    "a request mixing separate timed or untimed subjects",
                );
            }
            if let Some(fair) = clauses.iter().find_map(|row| row.fairness.first()) {
                return defer_at(
                    fair.node,
                    "temporal/fairness",
                    "a fairness constraint over dense time",
                );
            }
            let [object] = lowerer.objects else {
                return defer_at(root, MODEL, "a timed export with several object types");
            };
            if object.fields.is_empty() {
                return defer_at(root, MODEL, "a timed object without any data fields");
            }
            let model = models
                .iter()
                .find(|candidate| candidate.metadata.node_id == lowerer.graph.node(root).node_id)
                .ok_or_else(|| refusal("checked package", "a timed subject has no model root"))?;
            let mut clocks = Vec::new();
            let mut places = Vec::new();
            for place in &model.clock_places {
                let at = position(lowerer, &place.metadata.node_id)?;
                let [ClockPathStep::Field(field)] = place.path.as_slice() else {
                    return defer_at(
                        at,
                        "timed_model/clock_place",
                        "a clock inside a structured field",
                    );
                };
                if !object.clocks.contains(field) || place.model != model.model {
                    return defer_at(
                        at,
                        "timed_model/clock_place",
                        "a clock of another object type",
                    );
                }
                places.push(&place.metadata.node_id);
                clocks.push(Clock {
                    name: format!("{}_{}", mangle("k_", &object.name), mangle("", field)),
                    node: at,
                });
            }
            if clocks.len() != object.clocks.len() {
                return defer_at(
                    root,
                    MODEL,
                    "the admitted place list does not cover the clock fields",
                );
            }
            let mut actions: Vec<Action> = object
                .operations
                .iter()
                .map(|_| Action::default())
                .collect();
            let graph = lowerer.graph;
            let operation_index = |id: &CheckedNodeId| {
                object.operations.iter().position(|operation| {
                    operation_anchor(graph, object, operation)
                        .is_some_and(|at| graph.node(at).node_id == *id)
                })
            };
            for guard in &model.guards {
                let at = position(lowerer, &guard.metadata.node_id)?;
                let Some(index) = operation_index(&guard.operation) else {
                    return defer_at(
                        at,
                        "timed_model/clock_guard",
                        "a guard for an unexported operation",
                    );
                };
                if guard.point_anchor.is_some()
                    || guard.parts.iter().any(|part| !part.binders.is_empty())
                {
                    return defer_at(
                        at,
                        "timed_model/clock_guard",
                        "a guard over a point or slot binder",
                    );
                }
                for part in &guard.parts {
                    actions[index].guards.push(restriction(
                        lowerer,
                        model,
                        &places,
                        &part.predicate,
                        &part.constraints,
                    )?);
                }
            }
            for reset in &model.resets {
                let at = position(lowerer, &reset.metadata.node_id)?;
                let CheckedResetTarget::Place {
                    place,
                    owner,
                    slots,
                    binders,
                    condition,
                } = &reset.target
                else {
                    return defer_at(
                        at,
                        "timed_model/clock_reset",
                        "a reset through a clock parameter",
                    );
                };
                let Some(index) = operation_index(&reset.operation) else {
                    return defer_at(
                        at,
                        "timed_model/clock_reset",
                        "a reset for an unexported operation",
                    );
                };
                if reset.point_anchor.is_some()
                    || !slots.is_empty()
                    || !binders.is_empty()
                    || !receiver(lowerer, owner)?
                    || predicate(lowerer, condition)? != E::Bool(true)
                {
                    return defer_at(
                        at,
                        "timed_model/clock_reset",
                        "a conditional, point, slot or nonreceiver reset",
                    );
                }
                let clock = clock_index(&places, place)?;
                if actions[index]
                    .resets
                    .insert(clock, Quantity::read(&reset.value)?)
                    .is_some()
                {
                    return defer_at(
                        at,
                        "timed_model/clock_reset",
                        "two resets of one clock by the same operation",
                    );
                }
            }
            let mut invariants = Vec::new();
            for invariant in &model.invariants {
                if !invariant.binders.is_empty() {
                    return unsupported(
                        lowerer,
                        &invariant.metadata,
                        "time_invariant",
                        "a time invariant with slot binders",
                    );
                }
                invariants.push(restriction(
                    lowerer,
                    model,
                    &places,
                    &invariant.predicate,
                    &invariant.bounds,
                )?);
            }
            let mut urgency = Vec::new();
            for rule in &model.urgencies {
                match rule {
                    CheckedUrgency::When {
                        predicate: term, ..
                    } => urgency.push(predicate(lowerer, term)?),
                    CheckedUrgency::Operation {
                        operation_name,
                        metadata,
                        ..
                    } => {
                        let name = operation_name.rsplit("::").next().unwrap_or(operation_name);
                        let Some(index) = object
                            .operations
                            .iter()
                            .position(|operation| operation.name == name)
                        else {
                            return unsupported(
                                lowerer,
                                metadata,
                                "urgency",
                                "an urgent operation outside the exported model",
                            );
                        };
                        actions[index].urgent = true;
                    }
                }
            }
            Ok(Self {
                node: root,
                clocks,
                invariants,
                urgency,
                actions,
            })
        })();
        result.map(Some).map_err(|mut error| {
            error.obligation = Some(clause.name.clone());
            error.node.get_or_insert(root);
            error
        })
    }

    pub(super) fn variables(&self) -> impl Iterator<Item = String> + '_ {
        ["now", "tick", "idle"]
            .into_iter()
            .map(str::to_owned)
            .chain(self.clocks.iter().map(|clock| clock.name.clone()))
    }

    pub(super) fn tla_types(&self, variable: &str) -> Vec<String> {
        let mut types = vec![
            "now \\in Real".to_owned(),
            "now >= 0".to_owned(),
            "tick \\in BOOLEAN".to_owned(),
            "idle \\in BOOLEAN".to_owned(),
        ];
        types.extend(self.clocks.iter().flat_map(|clock| {
            let name = &clock.name;
            [
                format!("{name} \\in [DOMAIN {variable} -> Real]"),
                format!("\\A k \\in DOMAIN {variable} : {name}[k] >= 0"),
            ]
        }));
        types
    }

    pub(super) fn tla_init(&self, variable: &str) -> Vec<String> {
        let mut init = vec![
            "now = 0".to_owned(),
            "tick = FALSE".to_owned(),
            "idle = FALSE".to_owned(),
        ];
        init.extend(
            self.clocks
                .iter()
                .map(|clock| format!("{} = [k \\in DOMAIN {variable} |-> 0]", clock.name)),
        );
        init
    }

    pub(super) fn tla_action(&self, input: &Input<'_>, index: usize) -> Lower<Vec<String>> {
        let action = &self.actions[index];
        let mut action_lines = vec!["~idle".to_owned()];
        if !action.guards.is_empty() {
            let mut guards = Vec::new();
            for guard in &action.guards {
                let mut terms = vec![input.show.expr(&guard.predicate)?];
                terms.extend(
                    guard
                        .bounds
                        .iter()
                        .map(|bound| self.tla_bound(bound, "r_self", None)),
                );
                guards.push(format!("({})", terms.join(" /\\ ")));
            }
            action_lines.push(format!("({})", guards.join(" \\/ ")));
        }
        for (index, clock) in self.clocks.iter().enumerate() {
            let name = &clock.name;
            if let Some(value) = action.resets.get(&index) {
                action_lines.push(format!(
                    "{name}' = [{name} EXCEPT ![r_self] = {}]",
                    value.tla()
                ));
            } else {
                action_lines.push(format!("UNCHANGED {name}"));
            }
        }
        action_lines.extend(["now' = now", "tick' = FALSE", "UNCHANGED idle"].map(str::to_owned));
        action_lines.extend(["TimeInv(0)", "TimeInv(0)'"].map(str::to_owned));
        Ok(action_lines)
    }

    fn tla_bound(&self, bound: &Bound, key: &str, delay: Option<&str>) -> String {
        let name = &self.clocks[bound.clock].name;
        let clock = match delay {
            Some(delay) => format!("{name}[{key}] + {delay}"),
            None => format!("{name}[{key}]"),
        };
        let op = match bound.op {
            ClockComparison::Lt => "<",
            ClockComparison::Le => "<=",
            ClockComparison::Eq => "=",
            ClockComparison::Ge => ">=",
            ClockComparison::Gt => ">",
        };
        format!("{clock} {op} {}", bound.value.tla())
    }

    pub(super) fn tla_delay_bounds(
        &self,
        input: &Input<'_>,
        out: &mut Vec<Definition>,
    ) -> Lower<()> {
        let v = input.objects[0].variable();
        let mut invariants = Vec::new();
        for invariant in &self.invariants {
            let predicate =
                input
                    .show
                    .expr(&rename_param(invariant.predicate.clone(), "r_self", "k"))?;
            let bounds: Vec<_> = invariant
                .bounds
                .iter()
                .map(|bound| self.tla_bound(bound, "k", Some("d")))
                .collect();
            invariants.push(format!(
                "\\A k \\in DOMAIN {v} : {predicate} => ({})",
                bounds.join(" /\\ ")
            ));
        }
        let body = if invariants.is_empty() {
            " TRUE".to_owned()
        } else {
            format!("\n{}", bullets(&invariants))
        };
        out.push(define(
            self.node,
            None,
            format!("TimeInv(d) =={body}"),
            None,
        ));
        Ok(())
    }

    pub(super) fn tla_steps(&self, input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
        let object = &input.objects[0];
        let v = object.variable();
        let mut push = |body: String| {
            out.push(define(self.node, None, body, None));
        };
        let mut urgent = Vec::new();
        for term in &self.urgency {
            let renamed = rename_param(term.clone(), "r_self", "k");
            let predicate = input.show.expr(&renamed)?;
            urgent.push(if renamed == *term {
                predicate
            } else {
                format!("(\\E k \\in DOMAIN {v} : {predicate})")
            });
        }
        for (op, timing) in object.operations.iter().zip(&self.actions) {
            if timing.urgent {
                let params = parameters(op);
                let names: Vec<_> = params.iter().map(|(name, _)| name.as_str()).collect();
                let binders: Vec<_> = params
                    .iter()
                    .map(|(name, kind)| match kind {
                        None => format!("{name} \\in DOMAIN {v}"),
                        Some(kind) => format!("{name} \\in {}", tla_domain(kind)),
                    })
                    .collect();
                urgent.push(format!(
                    "(\\E {} : ENABLED {}({}))",
                    binders.join(", "),
                    object.operation(op),
                    names.join(", ")
                ));
            }
        }
        push(format!(
            "Urgent == {}",
            disjunction(&urgent, "FALSE", " \\/ ")
        ));
        let mut applying = Vec::new();
        for invariant in &self.invariants {
            let predicate =
                input
                    .show
                    .expr(&rename_param(invariant.predicate.clone(), "r_self", "k"))?;
            applying.push(format!("\\E k \\in DOMAIN {v} : {predicate}"));
        }
        let applying = match applying.as_slice() {
            [] => "FALSE".to_owned(),
            [one] => one.clone(),
            several => format!("({})", several.join(") \\/ (")),
        };
        push(format!("Quiescent == ~Urgent /\\ ~({applying})"));
        let advance = |lines: &mut Vec<String>| {
            lines.push("now' = now + d".to_owned());
            lines.extend(self.clocks.iter().map(|clock| {
                let name = &clock.name;
                format!("{name}' = [k \\in DOMAIN {v} |-> {name}[k] + d]")
            }));
        };
        let mut tick = ["~idle", "d \\in Real", "d > 0", "~Urgent", "TimeInv(d)"]
            .map(str::to_owned)
            .to_vec();
        advance(&mut tick);
        tick.extend([
            "tick' = TRUE".to_owned(),
            format!("UNCHANGED <<{v}, idle>>"),
        ]);
        push(format!("Tick(d) ==\n{}", bullets(&tick)));
        let mut idle = ["Quiescent", "d \\in Real", "d > 0"]
            .map(str::to_owned)
            .to_vec();
        advance(&mut idle);
        idle.extend([
            "tick' = FALSE".to_owned(),
            "idle' = TRUE".to_owned(),
            format!("UNCHANGED <<{v}>>"),
        ]);
        push(format!("Idle(d) ==\n{}", bullets(&idle)));
        push(
            "TNext == Next \\/ (\\E d \\in Real : Tick(d)) \\/ (\\E d \\in Real : Idle(d))"
                .to_owned(),
        );
        push("NZ == \\A r \\in Real : <>(now > r)".to_owned());
        Ok(())
    }

    pub(super) fn clock_names(&self) -> impl Iterator<Item = (String, usize)> + '_ {
        self.clocks
            .iter()
            .map(|clock| (clock.name.clone(), clock.node))
    }

    pub(super) fn lean_clock_types(&self, input: &Input<'_>, out: &mut Vec<Definition>) {
        let mut push = |name: Option<&str>, body: String| {
            out.push(define(self.node, name.map(str::to_owned), body, None));
        };
        push(
            None,
            "structure Q where\n  num : Int\n  den : Nat\nderiving DecidableEq".to_owned(),
        );
        push(Some("Q_normalize"), "def Q_normalize (n : Int) (d : Nat) : Q :=\n  let g := n.natAbs.gcd d\n  Q.mk (n / Int.ofNat g) (d / g)".to_owned());
        for (name, op) in [("Q_add", "+"), ("Q_sub", "-")] {
            push(Some(name), format!("def {name} (a b : Q) : Q :=\n  Q_normalize (a.num * Int.ofNat b.den {op} b.num * Int.ofNat a.den) (a.den * b.den)"));
        }
        for (name, op) in [("Q_le", "<="), ("Q_lt", "<"), ("Q_eq", "=")] {
            push(Some(name), format!("def {name} (a b : Q) : Prop := a.num * Int.ofNat b.den {op} b.num * Int.ofNat a.den"));
        }
        push(
            Some("Q_wf"),
            "def Q_wf (x : Q) : Prop := 0 < x.den /\\ x.num.natAbs.gcd x.den = 1".to_owned(),
        );
        let fields: String = self
            .clocks
            .iter()
            .map(|clock| format!("\n  {} : List Nat -> Q", clock.name))
            .collect();
        push(None, format!("structure Clocks where{fields}"));
        let object = &input.objects[0];
        let v = object.variable();
        let has = mangle("F_has_", &object.name);
        let all_clocks = |name: &str, lhs: &str, rhs: &str, body: &dyn Fn(&str) -> String| {
            let terms: Vec<_> = self
                .clocks
                .iter()
                .map(|clock| format!("(forall k, {has} k s.{v} -> {})", body(&clock.name)))
                .collect();
            format!(
                "def {name} {lhs} : Prop := {}",
                if terms.is_empty() {
                    rhs.to_owned()
                } else {
                    terms.join(" /\\\n  ")
                }
            )
        };
        push(
            Some("F_clockWf"),
            all_clocks("F_clockWf", "(s : State) (c : Clocks)", "True", &|clock| {
                format!("Q_wf (c.{clock} k) /\\ Q_le (Q.mk 0 1) (c.{clock} k)")
            }),
        );
        push(
            Some("F_clockInit"),
            all_clocks(
                "F_clockInit",
                "(s : State) (c : Clocks)",
                "True",
                &|clock| format!("c.{clock} k = Q.mk 0 1"),
            ),
        );
        push(
            Some("F_clockAdvance"),
            all_clocks(
                "F_clockAdvance",
                "(s : State) (c c' : Clocks) (d : Q)",
                "True",
                &|clock| format!("c'.{clock} k = Q_add (c.{clock} k) d"),
            ),
        );
    }

    pub(super) fn lean_action(&self, input: &Input<'_>, index: usize) -> Lower<Vec<String>> {
        let action = &self.actions[index];
        let object = &input.objects[0];
        let has = mangle("F_has_", &object.name);
        let v = object.variable();
        let mut lines = Vec::new();
        if !action.guards.is_empty() {
            let mut cases = Vec::new();
            for guard in &action.guards {
                let mut terms = vec![input.show.expr(&guard.predicate)?];
                terms.extend(
                    guard
                        .bounds
                        .iter()
                        .map(|bound| self.lean_bound(bound, "r_self", None)),
                );
                cases.push(format!("({})", terms.join(" /\\ ")));
            }
            lines.push(cases.join(" \\/ "));
        }
        for (index, clock) in self.clocks.iter().enumerate() {
            let name = &clock.name;
            let value = match action.resets.get(&index) {
                Some(reset) => format!("if k = r_self then {} else c.{name} k", reset.lean()),
                None => format!("c.{name} k"),
            };
            lines.push(format!(
                "forall k, {has} k s.{v} -> c'.{name} k = ({value})"
            ));
        }
        Ok(lines)
    }

    fn lean_bound(&self, bound: &Bound, key: &str, delay: Option<&str>) -> String {
        let name = &self.clocks[bound.clock].name;
        let val = match delay {
            Some(delay) => format!("(Q_add (c.{name} {key}) {delay})"),
            None => format!("(c.{name} {key})"),
        };
        let bound_text = bound.value.lean();
        match bound.op {
            ClockComparison::Lt => format!("Q_lt {val} {bound_text}"),
            ClockComparison::Le => format!("Q_le {val} {bound_text}"),
            ClockComparison::Eq => format!("Q_eq {val} {bound_text}"),
            ClockComparison::Ge => format!("Q_le {bound_text} {val}"),
            ClockComparison::Gt => format!("Q_lt {bound_text} {val}"),
        }
    }

    pub(super) fn lean_steps(&self, input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
        let object = &input.objects[0];
        let v = object.variable();
        let has = mangle("F_has_", &object.name);
        let mut push = |body: String| {
            out.push(define(self.node, None, body, None));
        };
        let mut urgent = Vec::new();
        for term in &self.urgency {
            let renamed = rename_param(term.clone(), "r_self", "k");
            let predicate = input.show.expr(&renamed)?;
            urgent.push(if renamed == *term {
                predicate
            } else {
                format!("(exists k, {has} k s.{v} /\\ {predicate})")
            });
        }
        for (op, timing) in object.operations.iter().zip(&self.actions) {
            if timing.urgent {
                let params = parameters(op);
                let binders = params
                    .iter()
                    .map(|(name, _)| format!("exists {name}, "))
                    .collect::<String>();
                let names = params
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                urgent.push(format!(
                    "(exists s' c', {binders}{} s s' c c' {names} /\\ TimeInv s' c' (Q.mk 0 1))",
                    object.operation(op)
                ));
            }
        }
        let mut applying = Vec::new();
        let mut invariants = Vec::new();
        for invariant in &self.invariants {
            let predicate =
                input
                    .show
                    .expr(&rename_param(invariant.predicate.clone(), "r_self", "k"))?;
            applying.push(format!("(exists k, {has} k s.{v} /\\ {predicate})"));
            let bounds: Vec<_> = invariant
                .bounds
                .iter()
                .map(|bound| self.lean_bound(bound, "k", Some("d")))
                .collect();
            invariants.push(format!(
                "(forall k, {has} k s.{v} -> {predicate} -> {})",
                bounds.join(" /\\ ")
            ));
        }
        push(format!(
            "def TimeInv (s : State) (c : Clocks) (d : Q) : Prop := {}",
            disjunction(&invariants, "True", " /\\ ")
        ));
        push(format!(
            "def Urgent (s : State) (c : Clocks) : Prop := {}",
            disjunction(&urgent, "False", " \\/ ")
        ));
        push(format!(
            "def Quiescent (s : State) (c : Clocks) : Prop := Not (Urgent s c) /\\ Not ({})",
            disjunction(&applying, "False", " \\/ ")
        ));
        push(r#"structure TBehaviour where
  st : Nat -> State
  ts : Nat -> Q
  ck : Nat -> Clocks
  idle : Nat -> Bool
  init : Init (st 0) /\ ts 0 = Q.mk 0 1 /\ F_clockInit (st 0) (ck 0) /\ TimeInv (st 0) (ck 0) (Q.mk 0 1) /\ idle 0 = false
  wf : forall n, Q_wf (ts n) /\ F_clockWf (st n) (ck n)
  next : forall n,
    let d := Q_sub (ts (n + 1)) (ts n)
    (idle n = false /\ idle (n + 1) = false /\ Q_le (Q.mk 0 1) d /\
      (Q_lt (Q.mk 0 1) d -> Not (Urgent (st n) (ck n)) /\ TimeInv (st n) (ck n) d) /\
      (exists a, F_clockAdvance (st n) (ck n) a d /\ Step (st n) (st (n + 1)) a (ck (n + 1))) /\
      TimeInv (st (n + 1)) (ck (n + 1)) (Q.mk 0 1)) \/
    (Quiescent (st n) (ck n) /\ Q_lt (Q.mk 0 1) d /\ st (n + 1) = st n /\
      F_clockAdvance (st n) (ck n) (ck (n + 1)) d /\ idle (n + 1) = true)
  nz : forall r : Q, Q_wf r -> exists n : Nat, Q_lt r (ts n)"#.to_owned());
        Ok(())
    }
}

fn position(lowerer: &Lowerer<'_>, node: &CheckedNodeId) -> Lower<usize> {
    lowerer
        .graph
        .position(node)
        .ok_or_else(|| refusal("checked package", "a timed node is missing"))
}

fn unsupported<T>(
    lowerer: &Lowerer<'_>,
    metadata: &TimedNodeMetadata,
    form: &str,
    why: &str,
) -> Lower<T> {
    defer_at(
        position(lowerer, &metadata.node_id)?,
        &format!("timed_model/{form}"),
        why,
    )
}

fn predicate(lowerer: &mut Lowerer<'_>, term: &CheckedNodeId) -> Lower<E> {
    let low = read_term(lowerer, term)?;
    let at = position(lowerer, term)?;
    total_predicate(low, at, &lowerer.graph.construct(at))
}

fn total_predicate(low: Low, at: usize, construct: &str) -> Lower<E> {
    if low.defined.is_empty() {
        return Ok(low.value);
    }
    // A predicate without a truth value blocks timed execution rather
    // than making its invariant, urgency or guard inactive.
    defer_at(
        at,
        construct,
        "a timed predicate with a definedness condition",
    )
}

fn receiver(lowerer: &mut Lowerer<'_>, term: &CheckedNodeId) -> Lower<bool> {
    let low = read_term(lowerer, term)?;
    Ok(low.defined.is_empty() && low.value == E::Param("r_self".to_owned()))
}

fn read_term(lowerer: &mut Lowerer<'_>, term: &CheckedNodeId) -> Lower<Low> {
    let at = position(lowerer, term)?;
    let body = serde_json::to_value(&lowerer.graph.node(at).body)
        .map_err(|_| refusal("checked package", "a timed expression does not serialize"))?;
    if str_of(&body, "term") == Some("reference") {
        lowerer.term(&body, false)
    } else {
        lowerer.node(at, false)
    }
}

fn restriction(
    lowerer: &mut Lowerer<'_>,
    model: &CheckedTimedModel,
    places: &[&CheckedNodeId],
    term: &CheckedNodeId,
    constraints: &[CheckedNodeId],
) -> Lower<Restriction> {
    let mut bounds = Vec::new();
    for id in constraints {
        let bound = model
            .constraints
            .iter()
            .find(|bound| bound.metadata.node_id == *id)
            .ok_or_else(|| refusal("checked package", "a clock bound is missing"))?;
        bounds.push(read_bound(lowerer, places, bound)?);
    }
    Ok(Restriction {
        predicate: predicate(lowerer, term)?,
        bounds,
    })
}

fn read_bound(
    lowerer: &mut Lowerer<'_>,
    places: &[&CheckedNodeId],
    bound: &CheckedClockConstraint,
) -> Lower<Bound> {
    if let CheckedConstraintTarget::Place {
        place,
        owner,
        slots,
    } = &bound.target
    {
        if slots.is_empty() && receiver(lowerer, owner)? {
            return Ok(Bound {
                clock: clock_index(places, place)?,
                op: bound.comparison,
                value: Quantity::read(&bound.bound)?,
            });
        }
    }
    unsupported(
        lowerer,
        &bound.metadata,
        "clock_constraint",
        "a bound on a parameter, slot or nonreceiver clock",
    )
}

fn clock_index(places: &[&CheckedNodeId], id: &CheckedNodeId) -> Lower<usize> {
    places.iter().position(|&place| place == id).ok_or_else(|| {
        refusal(
            "checked package",
            "a constraint names a missing clock place",
        )
    })
}

fn disjunction(terms: &[String], otherwise: &str, separator: &str) -> String {
    if terms.is_empty() {
        otherwise.to_owned()
    } else {
        terms.join(separator)
    }
}

#[cfg(test)]
mod tests {
    use super::{total_predicate, Low, E};
    use ix_trace_rs::trace;

    #[test]
    #[trace("FSpec-FR-813-AC-24", "FSpec-TC-1972")]
    fn dense_time_partial_predicate_stops_export_at_its_node() {
        let construct = "expression/application";
        let partial = Low {
            value: E::Bool(false),
            defined: vec![E::Bool(false)],
        };
        let Err(refusal) = total_predicate(partial, 3, construct) else {
            panic!("a predicate requiring a definedness check must refuse");
        };
        assert_eq!(refusal.node, Some(3));
        assert_eq!(refusal.construct, construct);
        assert_eq!(
            refusal.detail,
            "a timed predicate with a definedness condition"
        );
        let inactive = Low {
            value: E::Bool(false),
            defined: Vec::new(),
        };
        assert!(matches!(
            total_predicate(inactive, 3, construct),
            Ok(E::Bool(false))
        ));
    }
}
