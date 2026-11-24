// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-498: the `claim`/`probabilistic_claim` node each checked probabilistic
//! claim lowers to, with its one declared `claim` occurrence.
//!
//! The producer keys the node structurally under FSpec FR-322's
//! `{package, path, ordinal}` rule: path `["claim", "probabilistic_claim", name, ...]`, the nodes the claim reads
//! following the name (FSpec FR-640), and an ordinal that ranks the claims sharing that path by their
//! written content ([`fsl_forms::SignificantContent`]). No span, source position
//! or declaration order enters the key, so permuting the declarations keys
//! every claim as before; claims written identically are indistinguishable,
//! so which of them takes which rank is unobservable. The node's body is written by S4 from the nodes the claim reads.

use std::collections::{BTreeMap, hash_map};

use filament_exact::{NodeKey, PathSegment, Rational, ValueType};
use fsl_forms::DeclaredClauseKind;

use super::temporal::{FormulaCondition, FormulaInput, stage_fault};
use super::{
    Binder, Binders, FUNCTION_BODY, FunctionInput, Lowering, ModelClause, NodeContent, NodeRef,
    SemanticNode, literal_operand_text,
};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag,
};
use crate::check::probabilistic::{
    Bound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim, CheckedProbabilisticModel,
    CheckedTemporalFormula, ClaimActivation, Event, FractionMeasure, ProbForm, ReachTarget, Reward,
    Statement, Weighting,
};
use crate::check::refusal::CheckRefusal;
use filament_semantic_value::location::ExpressionLocation as Location;

/// What lowering wrote for one reward: the node of the object type that
/// declares its operation, its clause function, and the operand that names the
/// function's content.
#[derive(Clone, Debug)]
pub(crate) struct LoweredReward {
    pub(crate) declaring: NodeKey,
    pub(crate) function: NodeKey,
    pub(crate) operand: PathSegment,
}

/// The semantic form of a probabilistic claim's node.
const FORM: &str = "probabilistic_claim";

impl Lowering<'_> {
    /// FSpec FR-640 "Emission": root the nodes a checked random parameter or
    /// workload refers to, so a package that emits it holds them: the model
    /// node of each declaring type and of each workload's model type, and the
    /// type node of each random parameter, which brings the unit of a
    /// quantity and the variant type, with its case nodes, of an enum.
    pub(crate) fn probabilistic_declarations(
        &mut self,
        model: &CheckedProbabilisticModel,
    ) -> Result<Vec<Option<LoweredReward>>, CheckRefusal> {
        for random in model.random_parameters() {
            let at = random.declaration_location();
            self.charge(1, &at)?;
            if let Some(declaring) = random.operation().declaring_key() {
                self.rooted_model_node(declaring, &at)?;
            }
            if let NodeRef::Admitted(key) = self.type_ref(random.value_type(), &at)? {
                self.record_generated(key, &at);
            }
        }
        for workload in model.workloads() {
            let at = workload.declaration_location();
            self.charge(1, &at)?;
            for (operation, _) in workload.weights() {
                if let Some(declaring) = operation.declaring_key() {
                    self.rooted_model_node(declaring, &at)?;
                }
            }
            if let Some(model) = workload.model_key() {
                self.rooted_model_node(model, &at)?;
            }
        }
        for delay in model.delays() {
            let at = delay.declaration_location();
            self.charge(1, &at)?;
            if let Some(declaring) = delay.operation().declaring_key() {
                self.rooted_model_node(declaring, &at)?;
            }
        }
        // Functions of one owner share a key path and take their ordinals in
        // the order they are lowered, so the rewards are lowered by
        // (operation, name), not by declaration order: writing them in
        // another order changes no key.
        let rewards = model.rewards();
        let mut order: Vec<usize> = (0..rewards.len()).collect();
        order.sort_by(|&left, &right| {
            (rewards[left].operation().key(), rewards[left].name())
                .cmp(&(rewards[right].operation().key(), rewards[right].name()))
        });
        let mut keys: Vec<Option<LoweredReward>> = vec![None; rewards.len()];
        for at in order {
            keys[at] = self.reward_function(&rewards[at])?;
        }
        Ok(keys)
    }

    /// The model node of `declaration`, given a `generated` occurrence at
    /// `at` when nothing else recorded one.
    fn rooted_model_node(
        &mut self,
        declaration: &filament_checked::key::DeclarationKey,
        at: &crate::check::Location,
    ) -> Result<(), CheckRefusal> {
        let key = self.model_node(declaration, at)?;
        self.record_generated(key, at);
        Ok(())
    }

    /// Give `key` a `generated` occurrence at `at` when nothing else recorded
    /// one, which places the nodes it brings with it at the declaration.
    fn record_generated(&mut self, key: NodeKey, at: &Location) {
        if !self.occurrences.has(key.clone()) {
            self.record(key, OccurrenceRole::Generated, at.clone());
        }
    }

    /// Key and add the node of each of `claims`, recording its one `claim`
    /// occurrence at the claim's declaration. The keys are in `claims`'
    /// order. `model` holds what the claims read and `rewards` the function
    /// node and content operand of each of its rewards, in its order.
    pub(crate) fn probabilistic_claims(
        &mut self,
        model: &CheckedProbabilisticModel,
        rewards: &[Option<LoweredReward>],
    ) -> Result<Vec<(NodeKey, Option<NodeKey>, Statement)>, CheckRefusal> {
        let claims = model.claims();
        let reward_keys: Vec<Option<NodeKey>> = model
            .rewards()
            .iter()
            .zip(rewards)
            .map(|(reward, lowered)| {
                lowered.as_ref().and_then(|lowered| {
                    reward.node_key_over(&self.package_name, &lowered.declaring, &lowered.operand)
                })
            })
            .collect();
        let reward_units: Vec<Option<NodeKey>> = rewards
            .iter()
            .map(|lowered| {
                lowered
                    .as_ref()
                    .and_then(|lowered| self.reward_unit(&lowered.function))
            })
            .collect();
        let mut paths: Vec<Vec<PathSegment>> = Vec::with_capacity(claims.len());
        let mut lowered: Vec<(Option<NodeKey>, Statement)> = Vec::with_capacity(claims.len());
        for claim in claims {
            let mut path = claim
                .reads(&self.package_name, model, &reward_keys)
                .operands();
            let (over, scope) = self.claim_scope(claim)?;
            let reward_unit = claim.reward_read().and_then(|name| {
                model
                    .rewards()
                    .iter()
                    .zip(&reward_units)
                    .find(|(reward, _)| {
                        reward.name() == name
                            && reward.operation().declaring_key().is_some_and(|declaring| {
                                declaring.package == claim.subject.package.identity
                            })
                    })
                    .and_then(|(_, unit)| unit.clone())
            });
            let statement =
                self.claim_statement(claim, over.as_ref(), scope, reward_unit.as_ref())?;
            statement.spell(&mut path);
            paths.push(path);
            lowered.push((over, statement));
        }
        let mut ranked: Vec<(usize, &CheckedProbabilisticClaim)> =
            claims.iter().enumerate().collect();
        // Within one name and path, by content; equal content is equal
        // claims, whose tie order is unobservable.
        ranked.sort_by(|(left_at, left), (right_at, right)| {
            (&left.name, &paths[*left_at])
                .cmp(&(&right.name, &paths[*right_at]))
                .then_with(|| left.content.cmp(&right.content))
        });
        let mut keys = Vec::with_capacity(claims.len());
        let mut ordinal = 0_u64;
        let mut previous: Option<(&str, &[PathSegment])> = None;
        for (index, claim) in ranked {
            let path = (claim.name.as_str(), paths[index].as_slice());
            if previous == Some(path) {
                ordinal += 1;
            } else {
                ordinal = 0;
            }
            previous = Some(path);
            let key = self.probabilistic_claim(claim, &paths[index], ordinal)?;
            let (over, statement) = lowered[index].clone();
            keys.push((index, (key, over, statement)));
        }
        keys.sort_by_key(|(index, _)| *index);
        Ok(keys.into_iter().map(|(_, key)| key).collect())
    }

    /// Add `claim`'s node at `ordinal` of the path its name and the nodes it
    /// reads (`operands`) make. The body is written by S4, which holds those
    /// nodes.
    fn probabilistic_claim(
        &mut self,
        claim: &CheckedProbabilisticClaim,
        operands: &[PathSegment],
        ordinal: u64,
    ) -> Result<NodeKey, CheckRefusal> {
        let at = claim.declaration_location();
        self.charge(1, &at)?;
        let mut path = Vec::with_capacity(operands.len() + 1);
        path.push(PathSegment::Text(claim.name.clone()));
        path.extend(operands.iter().cloned());
        let key =
            super::super::derived_node_key(&self.package_name, NodeTag::Claim, FORM, path, ordinal)
                .map_err(|_| stage_fault(&at, "probabilistic-claim-key-is-well-formed"))?;
        match self.graph.nodes.entry(key.clone()) {
            hash_map::Entry::Vacant(slot) => {
                slot.insert(SemanticNode {
                    key: key.clone(),
                    recursion: None,
                    nominal: None,
                    content: NodeContent {
                        node_tag: NodeTag::Claim,
                        semantic_form: FORM,
                        semantic_type: None,
                        declaration: None,
                        owner: None,
                        owner_model: None,
                        body: BodyTerm::aggregate(Vec::new()),
                    },
                });
            }
            hash_map::Entry::Occupied(_) => {
                return Err(stage_fault(&at, "probabilistic-claim-key-is-unheld"));
            }
        }
        self.record(key.clone(), OccurrenceRole::Claim, at);
        Ok(key)
    }
}

impl Lowering<'_> {
    /// FSpec FR-640 `reward`: the clause function of `reward`'s expression,
    /// owned by the operation member it is declared on (FR-094), with a
    /// parameter for each of the operation's parameters in declared order,
    /// the random ones included. A reward whose operation names no declaring
    /// object type lowers to nothing, and emission refuses it with its locus.
    ///
    /// The function's clause kind is `reward` (FSpec FR-322): a precondition
    /// consumer never reads it as a guard and no `operation_bodies` row is
    /// demanded for the operation. Rewards are lowered after every other
    /// clause function and rank by (operation key, reward name) (FR-640
    /// "Reward identity"). A reward on a source-declared operation is a
    /// `SourceOwner` declaration `[type, operation, "reward", name]`.
    fn reward_function(&mut self, reward: &Reward) -> Result<Option<LoweredReward>, CheckRefusal> {
        let at = reward.declaration_location();
        self.charge(1, &at)?;
        let operation = reward.operation();
        let Some(declaring) = operation.declaring_key() else {
            return Ok(None);
        };
        let declaring_node = self.model_node(declaring, &at)?;
        self.record_generated(declaring_node.clone(), &at);
        let source_declaration = if operation.model().is_empty() {
            let type_name =
                crate::model::intake::type_identity_segment(&declaring.package, &declaring.node)
                    .ok_or_else(|| stage_fault(&at, "reward-source-type-is-named"))?;
            Some(vec![
                type_name.to_owned(),
                operation.name().to_owned(),
                "reward".to_owned(),
                reward.name().to_owned(),
            ])
        } else {
            None
        };
        let clause = ModelClause {
            declaration: operation.key().clone(),
            kind: DeclaredClauseKind::Reward,
            source_declaration,
        };
        let location = reward.evaluation_location();
        let name = format!("{}::{}", operation.name(), reward.name());
        let input = FunctionInput {
            name: &name,
            kind: fsl_forms::DeclarationKind::Function,
            location: &location,
            parameters: reward.parameters(),
            parameter_spans: None,
            result: reward.value_type(),
            body: reward.expression(),
            body_slots: reward.slot_names(),
            measure: None,
            measure_slots: &[],
            population_targets: &[],
            result_population_target: None,
            clause: Some(&clause),
        };
        // No function-table slot belongs to a reward.
        let index = self.functions.len();
        let NodeRef::Admitted(function) = self.function(index, &input)? else {
            return Err(stage_fault(&at, "reward-function-is-keyed"));
        };
        let operand = self
            .function_body_operand(&function)
            .ok_or_else(|| stage_fault(&at, "reward-function-has-a-body"))?;
        Ok(Some(LoweredReward {
            declaring: declaring_node,
            function,
            operand,
        }))
    }

    /// The operand that names what the function node `function` computes:
    /// the key of the node its `body` binding references, which is keyed by
    /// that expression's content, or a literal body's text. The expression
    /// was lowered once, under the lowering's own charge, so this reads one
    /// binding and walks nothing.
    fn function_body_operand(&self, function: &NodeKey) -> Option<PathSegment> {
        let BodyTerm::Aggregate(aggregate) = self.graph.nodes.get(function)?.body() else {
            return None;
        };
        aggregate.members.iter().find_map(|member| {
            let MemberTerm::Binding(Binding { name, value, .. }) = member else {
                return None;
            };
            if name != FUNCTION_BODY {
                return None;
            }
            match value {
                BindingValue::Leaf(LeafTerm::Reference {
                    target: NodeRef::Admitted(key),
                }) => Some(PathSegment::Node(key.clone())),
                BindingValue::Leaf(LeafTerm::Literal { value, .. }) => {
                    Some(PathSegment::Text(literal_operand_text(value)))
                }
                _ => None,
            }
        })
    }
}

/// What a claim's statement is lowered against: the binders its predicates
/// see, the Boolean type node, the claim's location and the units its
/// quantities are in.
struct StatementCx<'c> {
    claim: &'c CheckedProbabilisticClaim,
    scope: &'c [Binder],
    boolean: &'c NodeKey,
    at: &'c Location,
    reward_unit: Option<&'c NodeKey>,
}

/// The type of a bound's value (FSpec FR-640 "Exact-value positions").
enum Threshold {
    Rational,
    Integer,
    Quantity(NodeKey),
}

impl Lowering<'_> {
    /// The builtin scalar type node `form`, located at the claim, which the
    /// claim's body reads but its S3 node does not yet reference.
    fn claim_scalar(&mut self, form: &'static str, at: &Location) -> Result<NodeKey, CheckRefusal> {
        let key = self.scalar(form, at)?;
        self.record_generated(key.clone(), at);
        Ok(key)
    }

    /// The `value`/`parameter` node of a claim's `over`, and the binders its
    /// predicates see: `over` at slot 0, then the trigger.
    fn claim_scope(
        &mut self,
        claim: &CheckedProbabilisticClaim,
    ) -> Result<(Option<NodeKey>, Vec<Binder>), CheckRefusal> {
        let mut scope = Vec::new();
        let over = match &claim.over {
            None => None,
            Some(over) => {
                let at = claim.over_location();
                let semantic_type = self.binder_type(&over.value_type, None, &at)?;
                let parameter = self.parameter(&over.name, 0, semantic_type.clone(), &at)?;
                self.record(semantic_type.clone(), OccurrenceRole::Type, at);
                scope.push(Binder {
                    slot: 0,
                    parameter: parameter.clone().into(),
                    semantic_type: semantic_type.into(),
                });
                Some(parameter)
            }
        };
        Ok((over, scope))
    }

    /// The unit node a quantity bound of the reward whose function is
    /// `function` is in, or `None` for an integer reward.
    fn reward_unit(&self, function: &NodeKey) -> Option<NodeKey> {
        let nodes = &self.graph.nodes;
        let value_type = nodes
            .get(function)?
            .content
            .semantic_type
            .as_ref()?
            .admitted()?
            .clone();
        let node = nodes.get(&value_type)?;
        match node.content.semantic_form {
            "unit" | "compound_unit" => Some(value_type),
            "carried_quantity" | "bounded_quantity" => {
                let BodyTerm::Aggregate(aggregate) = &node.content.body else {
                    return None;
                };
                aggregate.members.iter().find_map(|member| {
                    let MemberTerm::Binding(Binding { name, value, .. }) = member else {
                        return None;
                    };
                    match value {
                        BindingValue::Leaf(leaf) if name == "unit" => {
                            leaf.reference_target()?.admitted().cloned()
                        }
                        _ => None,
                    }
                })
            }
            _ => None,
        }
    }

    /// The claim's `form`, `over`, `activation` and `confidence` bindings as
    /// a statement, lowering the predicates, formula and trigger it reads.
    fn claim_statement(
        &mut self,
        claim: &CheckedProbabilisticClaim,
        over: Option<&NodeKey>,
        mut scope: Vec<Binder>,
        reward_unit: Option<&NodeKey>,
    ) -> Result<Statement, CheckRefusal> {
        let at = claim.declaration_location();
        let boolean = self.type_node(&ValueType::Boolean, &at)?;
        let trigger = match &claim.activation {
            ClaimActivation::Origin => None,
            ClaimActivation::Each {
                binder, value_type, ..
            } => {
                let trigger_at = claim.declaration_location();
                let semantic_type = self.binder_type(value_type, None, &trigger_at)?;
                let level = scope.len();
                let parameter =
                    self.parameter(binder, level, semantic_type.clone(), &trigger_at)?;
                self.record(semantic_type.clone(), OccurrenceRole::Type, trigger_at);
                scope.push(Binder {
                    slot: level,
                    parameter: parameter.clone().into(),
                    semantic_type: semantic_type.into(),
                });
                Some(parameter)
            }
        };
        let cx = StatementCx {
            claim,
            scope: &scope,
            boolean: &boolean,
            at: &at,
            reward_unit,
        };
        let form = self.form_statement(&cx)?;
        let over = over.map_or_else(
            || Statement::Aggregate(Vec::new()),
            |over| Statement::Ref(over.clone()),
        );
        let activation = match (&claim.activation, trigger) {
            (
                ClaimActivation::Each {
                    guard,
                    guard_slot_names,
                    ..
                },
                Some(trigger),
            ) => {
                let guard = match guard {
                    Some(guard) => {
                        let mut binders = Binders::new(guard_slot_names, &scope);
                        let term = self.expression(guard.root(), &mut binders)?;
                        self.statement_ref(term, &cx)?
                    }
                    None => self.true_literal(&cx)?,
                };
                Statement::Aggregate(vec![
                    ("kind", Statement::Text("each".into())),
                    ("trigger", Statement::Ref(trigger)),
                    ("guard", guard),
                ])
            }
            _ => Statement::Aggregate(vec![("kind", Statement::Text("origin".into()))]),
        };
        let confidence = match &claim.confidence {
            None => Statement::Aggregate(Vec::new()),
            Some(confidence) => {
                let rational = self.claim_scalar("rational", &at)?;
                Statement::Aggregate(vec![
                    (
                        "alpha",
                        Statement::rational(rational.clone(), &confidence.alpha),
                    ),
                    (
                        "beta",
                        Statement::rational(rational.clone(), &confidence.beta),
                    ),
                    (
                        "indifference",
                        Statement::rational(rational, &confidence.indifference),
                    ),
                ])
            }
        };
        Ok(Statement::Aggregate(vec![
            ("form", form),
            ("over", over),
            ("activation", activation),
            ("confidence", confidence),
        ]))
    }

    /// The `value`/`literal` node of the Boolean constant `true`.
    fn true_literal(&mut self, cx: &StatementCx<'_>) -> Result<Statement, CheckRefusal> {
        let key = self.insert(
            cx.at,
            NodeTag::Value,
            "literal",
            Some(cx.boolean.clone()),
            None,
            BodyTerm::literal(cx.boolean.clone(), LiteralValue::Boolean(true)),
        )?;
        self.record(key.clone(), OccurrenceRole::Expression, cx.at.clone());
        Ok(Statement::Ref(key))
    }

    /// The reference to the node of a lowered Boolean `term`: the node it is,
    /// or the `value`/`literal` node of a constant.
    fn statement_ref(
        &mut self,
        term: LeafTerm,
        cx: &StatementCx<'_>,
    ) -> Result<Statement, CheckRefusal> {
        if let Some(target) = term.reference_target() {
            return target
                .admitted()
                .cloned()
                .map(Statement::Ref)
                .ok_or_else(|| stage_fault(cx.at, "claim-predicate-is-keyed"));
        }
        let key = self.insert(
            cx.at,
            NodeTag::Value,
            "literal",
            Some(cx.boolean.clone()),
            None,
            BodyTerm::Leaf(term),
        )?;
        self.record(key.clone(), OccurrenceRole::Expression, cx.at.clone());
        Ok(Statement::Ref(key))
    }

    /// The reference to the node of `predicate`.
    fn predicate_statement(
        &mut self,
        predicate: &CheckedPredicate,
        cx: &StatementCx<'_>,
    ) -> Result<Statement, CheckRefusal> {
        let mut binders = Binders::new(predicate.slot_names(), cx.scope);
        let body = predicate
            .body()
            .ok_or_else(|| stage_fault(cx.at, "a-package-predicate-has-no-source-body-to-lower"))?;
        let term = self.expression(body, &mut binders)?;
        self.statement_ref(term, cx)
    }

    fn bound_statement(
        &mut self,
        bound: &Bound,
        threshold: &Threshold,
        cx: &StatementCx<'_>,
    ) -> Result<Statement, CheckRefusal> {
        let (direction, value) = match bound {
            Bound::AtLeast(value) => ("at_least", value),
            Bound::AtMost(value) => ("at_most", value),
        };
        let value = match threshold {
            Threshold::Rational => {
                Statement::rational(self.claim_scalar("rational", cx.at)?, value)
            }
            Threshold::Integer => Statement::Exact {
                kind: "integer",
                ty: self.claim_scalar("integer", cx.at)?,
                text: value.numerator().to_string(),
            },
            Threshold::Quantity(unit) => Statement::quantity(unit.clone(), value),
        };
        Ok(Statement::Aggregate(vec![
            ("direction", Statement::Text(direction.into())),
            ("value", value),
        ]))
    }

    /// A window of `ticks` steps, or `timed` in the model's time unit.
    fn window_statement(
        &mut self,
        ticks: u64,
        timed: Option<&Rational>,
        cx: &StatementCx<'_>,
    ) -> Result<Statement, CheckRefusal> {
        match (timed, cx.claim.time_unit.clone()) {
            (Some(timed), Some(unit)) => Ok(Statement::quantity(unit, timed)),
            // FSpec FR-640 rule 7.3: a window over a timed subject is a time
            // quantity, so a step count there has no spelling.
            (None, Some(_)) => Err(stage_fault(cx.at, "step-window-on-a-timed-subject")),
            _ => Ok(Statement::integer(
                self.claim_scalar("integer", cx.at)?,
                ticks,
            )),
        }
    }

    /// The `weighting` binding of a measure weighted by `weight` (FR-498):
    /// `none`, `reward` (the claim's `reward` binding names the reward) or
    /// `delay` (FR-186 delay weighting, which reads the subject's delays and
    /// no reward).
    fn weighting_statement(weight: Option<&Weighting>) -> Statement {
        let kind = match weight {
            None => "none",
            Some(Weighting::Reward(_)) => "reward",
            Some(Weighting::Delay) => "delay",
        };
        Statement::Aggregate(vec![("kind", Statement::Text(kind.into()))])
    }

    fn fraction_statement(
        &mut self,
        fraction: &FractionMeasure,
        cx: &StatementCx<'_>,
    ) -> Result<Statement, CheckRefusal> {
        let predicate = self.predicate_statement(&fraction.predicate, cx)?;
        let window = self.window_statement(fraction.window, fraction.timed_window.as_ref(), cx)?;
        Ok(Statement::Aggregate(vec![
            ("kind", Statement::Text("fraction".into())),
            ("predicate", predicate),
            ("window", window),
            (
                "weighting",
                Self::weighting_statement(fraction.weight.as_ref()),
            ),
        ]))
    }

    /// The statement of `measure` and the type of a bound on it.
    fn measure_statement(
        &mut self,
        measure: &BoundedMeasure,
        cx: &StatementCx<'_>,
    ) -> Result<(Statement, Threshold), CheckRefusal> {
        let time = || {
            cx.claim
                .time_unit
                .clone()
                .ok_or_else(|| stage_fault(cx.at, "elapsed-claim-has-a-time-unit"))
        };
        let threshold = match measure {
            BoundedMeasure::Accumulate { .. } => {
                cx.reward_unit.map_or(Threshold::Rational, |unit| {
                    Threshold::Quantity(unit.clone())
                })
            }
            BoundedMeasure::Steps { .. } => Threshold::Integer,
            BoundedMeasure::Elapsed { .. } => Threshold::Quantity(time()?),
            BoundedMeasure::Fraction(_) => Threshold::Rational,
        };
        let statement = match measure {
            BoundedMeasure::Accumulate {
                from,
                until,
                within,
                timed_within,
                ..
            }
            | BoundedMeasure::Steps {
                from,
                until,
                within,
                timed_within,
            } => {
                let kind = if matches!(measure, BoundedMeasure::Steps { .. }) {
                    "steps"
                } else {
                    "accumulate"
                };
                let from = self.predicate_statement(from, cx)?;
                let until = self.predicate_statement(until, cx)?;
                let within = self.window_statement(*within, timed_within.as_ref(), cx)?;
                Statement::Aggregate(vec![
                    ("kind", Statement::Text(kind.into())),
                    ("from", from),
                    ("until", until),
                    ("within", within),
                ])
            }
            BoundedMeasure::Elapsed {
                from,
                until,
                within,
                ..
            } => {
                let from = self.predicate_statement(from, cx)?;
                let until = self.predicate_statement(until, cx)?;
                Statement::Aggregate(vec![
                    ("kind", Statement::Text("elapsed".into())),
                    ("from", from),
                    ("until", until),
                    ("within", Statement::quantity(time()?, within)),
                ])
            }
            BoundedMeasure::Fraction(fraction) => self.fraction_statement(fraction, cx)?,
        };
        Ok((statement, threshold))
    }

    fn form_statement(&mut self, cx: &StatementCx<'_>) -> Result<Statement, CheckRefusal> {
        let kind = |name: &str| ("kind", Statement::Text(name.to_owned()));
        let reward = || {
            cx.reward_unit.map_or(Threshold::Rational, |unit| {
                Threshold::Quantity(unit.clone())
            })
        };
        Ok(match &cx.claim.form {
            ProbForm::Probability { bound, event } => {
                let event = match event {
                    Event::Formula { formula, .. } => {
                        let (root, nodes) = self.formula_statement(formula, cx)?;
                        Statement::Aggregate(vec![
                            kind("formula"),
                            ("formula", Statement::Formula { root, nodes }),
                        ])
                    }
                    Event::Compare { measure, bound } => {
                        let (measure, threshold) = self.measure_statement(measure, cx)?;
                        let bound = self.bound_statement(bound, &threshold, cx)?;
                        Statement::Aggregate(vec![
                            kind("compare"),
                            ("measure", measure),
                            ("bound", bound),
                        ])
                    }
                };
                let bound = self.bound_statement(bound, &Threshold::Rational, cx)?;
                Statement::Aggregate(vec![
                    kind("probability"),
                    ("event", event),
                    ("bound", bound),
                ])
            }
            ProbForm::Quantile {
                q,
                measure,
                threshold,
            } => {
                let level = Statement::rational(self.claim_scalar("rational", cx.at)?, q);
                let (measure, kind_of) = self.measure_statement(measure, cx)?;
                let bound = self.bound_statement(threshold, &kind_of, cx)?;
                Statement::Aggregate(vec![
                    kind("quantile"),
                    ("level", level),
                    ("measure", measure),
                    ("bound", bound),
                ])
            }
            ProbForm::MeanOfFraction { bound, fraction } => {
                let measure = self.fraction_statement(fraction, cx)?;
                let bound = self.bound_statement(bound, &Threshold::Rational, cx)?;
                Statement::Aggregate(vec![
                    kind("mean_of_fraction"),
                    ("measure", measure),
                    ("bound", bound),
                ])
            }
            ProbForm::LongRunFraction {
                bound,
                predicate,
                weight,
            } => {
                let predicate = self.predicate_statement(predicate, cx)?;
                let bound = self.bound_statement(bound, &Threshold::Rational, cx)?;
                Statement::Aggregate(vec![
                    kind("long_run_fraction"),
                    ("predicate", predicate),
                    ("weighting", Self::weighting_statement(weight.as_ref())),
                    ("bound", bound),
                ])
            }
            ProbForm::Reach { bound, target } => {
                let target = match target {
                    ReachTarget::Eventually(predicate) | ReachTarget::Always(predicate) => {
                        let name = if matches!(target, ReachTarget::Always(_)) {
                            "always"
                        } else {
                            "eventually"
                        };
                        let predicate = self.predicate_statement(predicate, cx)?;
                        Statement::Aggregate(vec![kind(name), ("predicate", predicate)])
                    }
                    ReachTarget::Until(left, right) => {
                        let left = self.predicate_statement(left, cx)?;
                        let right = self.predicate_statement(right, cx)?;
                        Statement::Aggregate(vec![kind("until"), ("left", left), ("right", right)])
                    }
                };
                let bound = self.bound_statement(bound, &Threshold::Rational, cx)?;
                Statement::Aggregate(vec![kind("reach"), ("target", target), ("bound", bound)])
            }
            ProbForm::ExpectedReward { bound, until, .. } => {
                let until = self.predicate_statement(until, cx)?;
                let bound = self.bound_statement(bound, &reward(), cx)?;
                Statement::Aggregate(vec![
                    kind("expected_reward"),
                    ("until", until),
                    ("bound", bound),
                ])
            }
            ProbForm::MeanTime { bound, until } => {
                let unit = cx
                    .claim
                    .time_unit
                    .clone()
                    .ok_or_else(|| stage_fault(cx.at, "elapsed-claim-has-a-time-unit"))?;
                let until = self.predicate_statement(until, cx)?;
                let bound = self.bound_statement(bound, &Threshold::Quantity(unit), cx)?;
                Statement::Aggregate(vec![
                    kind("expected_elapsed"),
                    ("until", until),
                    ("bound", bound),
                ])
            }
        })
    }

    /// The `temporal`/`formula` node of an event formula, its `holds` atoms
    /// lowered in the claim's binders.
    fn formula_statement(
        &mut self,
        formula: &CheckedTemporalFormula,
        cx: &StatementCx<'_>,
    ) -> Result<(NodeKey, Vec<NodeKey>), CheckRefusal> {
        let conditions: BTreeMap<usize, FormulaCondition<'_>> = formula
            .conditions()
            .iter()
            .map(|(index, predicate)| {
                let root = predicate.body().ok_or_else(|| {
                    stage_fault(cx.at, "a-package-predicate-has-no-source-body-to-lower")
                })?;
                Ok((
                    *index,
                    FormulaCondition {
                        root,
                        slot_names: predicate.slot_names(),
                    },
                ))
            })
            .collect::<Result<_, CheckRefusal>>()?;
        let aggregates = BTreeMap::new();
        let at = cx.at.clone();
        self.temporal_formula_nodes(
            &FormulaInput {
                formula: formula.formula(),
                node_site: &|_| at.clone(),
                conditions: &conditions,
                aggregates: &aggregates,
                root_site: cx.at,
                tick_intervals: false,
            },
            cx.boolean.clone(),
            cx.scope,
        )
    }
}
