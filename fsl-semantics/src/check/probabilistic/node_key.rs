// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-640: the structural key of each `probabilistic` node and the nodes
//! a probabilistic claim reads (FSL FR-498).
//!
//! A `probabilistic` node's path is `["probabilistic", form, operand...]`,
//! its operands the model type's node and the canonical spelling of what the
//! declaration holds, so a changed weight, support value or expression changes
//! its key. A claim's path holds the keys of the nodes it reads as operand
//! segments, so the claim's key changes with each of them. Lowering keys the
//! claim and S4 writes each node under the same keys, so both read this module.

use filament_exact::{NodeKey, PathSegment, Rational, Value};
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::claim::{BoundedMeasure, Event, ProbForm, Weighting};
use super::{
    CheckedProbabilisticClaim, CheckedProbabilisticModel, Delay, DelayDistribution, ProbScheduler,
    RandomParameter, Reward, Workload,
};
use crate::model::key::declaring_node_key;

/// The `probabilistic` node key of `form` with `operand`.
fn key(package: &str, form: &str, operand: Vec<PathSegment>) -> Option<NodeKey> {
    let mut path = vec![
        PathSegment::Text("probabilistic".into()),
        PathSegment::Text(form.into()),
    ];
    path.extend(operand);
    NodeKey::new(package, path, 0).ok()
}

/// An exact weight in its FR-205 canonical spelling, `n` or `n/d`.
fn spelling(value: &Rational) -> String {
    if value.is_integer() {
        value.numerator().to_string()
    } else {
        format!("{}/{}", value.numerator(), value.denominator())
    }
}

/// The canonical spelling of a support value, as its node's identity carries
/// it: integers and rationals in FR-205 form, a quantity as its magnitude, a
/// text as its JSON string encoding, a Boolean as `true` or `false` and an enum
/// member as its case name. `None` for a value no node can carry yet.
fn support_spelling(value: &Value) -> Option<String> {
    match value {
        Value::Integer(value) => Some(value.to_string()),
        Value::Rational(value) => Some(spelling(value)),
        Value::Text(value) => serde_json::to_string(value.retained()).ok(),
        Value::Quantity(value) => Some(spelling(value.magnitude())),
        Value::Boolean(value) => Some(value.to_string()),
        Value::Enum(member) => Some(member.variant().case().to_owned()),
        _ => None,
    }
}

impl RandomParameter {
    /// The key of the parameter's `random_parameter` node in `package`, or
    /// `None` when its declaring type has no `model` node or a support value
    /// is not one a node carries.
    #[must_use]
    pub fn node_key(&self, package: &str) -> Option<NodeKey> {
        let operation = self.operation();
        let declaring = declaring_node_key(operation.declaring_key()?, package)?;
        let content = self
            .support()
            .iter()
            .map(|support| {
                Some(format!(
                    "{}*{}",
                    support_spelling(&support.value.0)?,
                    spelling(&support.weight)
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        key(
            package,
            "random_parameter",
            vec![
                PathSegment::Node(declaring),
                PathSegment::Text(operation.name().into()),
                PathSegment::Text(self.parameter().into()),
                PathSegment::Text(content.join(",")),
            ],
        )
    }
}

impl Workload {
    /// The key of the workload's `workload` node in `package`, or `None` when
    /// no model type declares its operations. Its weights are spelled in
    /// ascending (declaring node, operation name) order, so the order they are
    /// written in changes nothing.
    #[must_use]
    pub fn node_key(&self, package: &str) -> Option<NodeKey> {
        let model = declaring_node_key(self.model_key()?, package)?;
        let mut weights = Vec::with_capacity(self.weights().len());
        for (operation, weight) in self.weights() {
            let declaring = declaring_node_key(operation.declaring_key()?, package)?;
            weights.push((declaring, operation.name(), weight));
        }
        weights.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));
        let content: Vec<String> = weights
            .iter()
            .map(|(_, operation, weight)| format!("{operation}*{}", spelling(weight)))
            .collect();
        key(
            package,
            "workload",
            vec![
                PathSegment::Node(model),
                PathSegment::Text(self.name().into()),
                PathSegment::Text(content.join(",")),
            ],
        )
    }
}

impl Delay {
    /// The key of the delay's `delay_distribution` node in `package`, or
    /// `None` when its declaring type has no `model` node.
    #[must_use]
    pub fn node_key(&self, package: &str) -> Option<NodeKey> {
        let operation = self.operation();
        let declaring = declaring_node_key(operation.declaring_key()?, package)?;
        let (family, content) = match self.distribution() {
            DelayDistribution::Uniform => ("uniform", String::new()),
            DelayDistribution::UniformRange { a, b } => (
                "uniform_interval",
                format!("{},{}", spelling(a), spelling(b)),
            ),
            DelayDistribution::Exponential { rate } => ("exponential", spelling(rate)),
            DelayDistribution::Discrete(rows) => (
                "discrete",
                rows.iter()
                    .map(|(delay, weight)| format!("{}*{}", spelling(delay), spelling(weight)))
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        };
        key(
            package,
            "delay_distribution",
            vec![
                PathSegment::Node(declaring),
                PathSegment::Text(operation.name().into()),
                PathSegment::Text(family.into()),
                PathSegment::Text(content),
            ],
        )
    }
}

impl Reward {
    /// The key of the reward's `reward` node in `package` once its function
    /// is lowered, or `None` before that or when its declaring type has no
    /// `model` node.
    #[must_use]
    pub fn node_key(&self, package: &str) -> Option<NodeKey> {
        self.node_key_over(package, self.declaring_node()?, self.expression_operand()?)
    }

    /// [`Self::node_key`] with the `expression` operand its lowering produced,
    /// for the lowering that has not recorded it on the reward yet.
    pub(crate) fn node_key_over(
        &self,
        package: &str,
        declaring: &NodeKey,
        expression: &PathSegment,
    ) -> Option<NodeKey> {
        let operation = self.operation();
        key(
            package,
            "reward",
            vec![
                PathSegment::Node(declaring.clone()),
                PathSegment::Text(operation.name().into()),
                PathSegment::Text(self.name().into()),
                expression.clone(),
            ],
        )
    }
}

/// One fairness constraint a claim `under every scheduler` states.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FairnessRead {
    /// The `model` node that declares the constrained operation.
    pub declaring: NodeKey,
    /// The constrained operation's name.
    pub operation: String,
    /// `weak` or `strong`.
    pub kind: &'static str,
    /// `whole` or `each`.
    pub granularity: &'static str,
}

/// The scheduler a claim reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchedulerRead {
    /// `under W`: `W`'s node.
    Workload(NodeKey),
    /// `under every scheduler`: its fairness set, ascending, with no repeat.
    Every(Vec<FairnessRead>),
}

/// What a claim reads, which unresolved declarations a node cannot carry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimReadFault {
    /// The workload the claim is stated under has no node of the subject's
    /// model.
    Workload(String),
    /// The reward the claim's form reads has no node of the subject's model.
    Reward(String),
    /// The subject's `model_root` or a fairness operation has no key.
    Subject,
}

/// The nodes a probabilistic claim reads (FSpec FR-640 "Probabilistic claim
/// node"), by their keys, each list ascending.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimReads {
    /// The subject's `model_root` node.
    pub subject: Option<NodeKey>,
    /// The scheduler, `None` when it could not be resolved.
    pub scheduler: Option<SchedulerRead>,
    /// Every `reward` node of the one reward the form reads.
    pub rewards: Vec<NodeKey>,
    /// Every `random_parameter` node of the subject's model.
    pub distributions: Vec<NodeKey>,
    /// Every `delay_distribution` node of the subject's model.
    pub delays: Vec<NodeKey>,
    /// The first thing the claim reads that no node carries.
    pub fault: Option<ClaimReadFault>,
}

impl ClaimReads {
    /// The operand segments of the claim's path after its name, in the body's
    /// member order: `"subject", {root}, "scheduler", S, "reward", {r}...,
    /// "distributions", {p}..., "delays", {d}...`.
    #[must_use]
    pub fn operands(&self) -> Vec<PathSegment> {
        let text = |value: &str| PathSegment::Text(value.to_owned());
        let mut path = vec![text("subject")];
        path.extend(self.subject.iter().cloned().map(PathSegment::Node));
        path.push(text("scheduler"));
        match &self.scheduler {
            Some(SchedulerRead::Workload(node)) => path.push(PathSegment::Node(node.clone())),
            Some(SchedulerRead::Every(fairness)) => {
                path.push(text("every"));
                for member in fairness {
                    path.push(text(member.kind));
                    path.push(text(member.granularity));
                    path.push(PathSegment::Node(member.declaring.clone()));
                    path.push(text(&member.operation));
                }
            }
            None => {}
        }
        path.push(text("reward"));
        path.extend(self.rewards.iter().cloned().map(PathSegment::Node));
        path.push(text("distributions"));
        path.extend(self.distributions.iter().cloned().map(PathSegment::Node));
        path.push(text("delays"));
        path.extend(self.delays.iter().cloned().map(PathSegment::Node));
        path
    }
}

impl CheckedProbabilisticClaim {
    /// The name of the one reward the claim's form reads (FSpec FR-640):
    /// `accumulate R`, a fraction or long-run fraction `weighted by R` and
    /// `expected accumulate R`; `None` for every other form.
    #[must_use]
    pub fn reward_read(&self) -> Option<&str> {
        fn measure(measure: &BoundedMeasure) -> Option<&str> {
            match measure {
                BoundedMeasure::Accumulate { reward, .. } => Some(reward.as_str()),
                BoundedMeasure::Fraction(fraction) => {
                    fraction.weight.as_ref().and_then(Weighting::reward_name)
                }
                BoundedMeasure::Steps { .. } | BoundedMeasure::Elapsed { .. } => None,
            }
        }
        match &self.form {
            ProbForm::Probability {
                event: Event::Compare { measure: read, .. },
                ..
            }
            | ProbForm::Quantile { measure: read, .. } => measure(read),
            ProbForm::MeanOfFraction { fraction, .. } => {
                fraction.weight.as_ref().and_then(Weighting::reward_name)
            }
            ProbForm::LongRunFraction { weight, .. } => {
                weight.as_ref().and_then(Weighting::reward_name)
            }
            ProbForm::ExpectedReward { reward, .. } => Some(reward.as_str()),
            ProbForm::Probability {
                event: Event::Formula { .. },
                ..
            }
            | ProbForm::Reach { .. }
            | ProbForm::MeanTime { .. } => None,
        }
    }

    /// The nodes the claim reads, keyed in `package`. `rewards` holds the key
    /// of each reward of `model`, in `model`'s order, `None` for one with no
    /// node.
    #[must_use]
    pub fn reads(
        &self,
        package: &str,
        model: &CheckedProbabilisticModel,
        rewards: &[Option<NodeKey>],
    ) -> ClaimReads {
        let identity = self.subject.package.identity.as_str();
        let belongs = |declaring: Option<&filament_checked::key::DeclarationKey>| {
            declaring.is_some_and(|declaring| declaring.package == identity)
        };
        let mut fault = None;
        let subject = super::super::model_root_key(&self.subject.package).ok();
        if subject.is_none() {
            fault = Some(ClaimReadFault::Subject);
        }
        let scheduler = match &self.scheduler {
            ProbScheduler::Workload(name) => {
                let node = model
                    .workload(name)
                    .filter(|workload| belongs(workload.model_key()))
                    .and_then(|workload| workload.node_key(package));
                if node.is_none() {
                    fault.get_or_insert(ClaimReadFault::Workload(name.clone()));
                }
                node.map(SchedulerRead::Workload)
            }
            ProbScheduler::Every => {
                let mut set = Vec::with_capacity(self.fairness.len());
                for (constraint, declaring) in self.fairness.iter().zip(&self.fairness_declaring) {
                    let Some(declaring) = declaring
                        .as_ref()
                        .and_then(|declaring| declaring_node_key(declaring, package))
                    else {
                        fault.get_or_insert(ClaimReadFault::Subject);
                        continue;
                    };
                    set.push(FairnessRead {
                        declaring,
                        operation: constraint.operation().name().to_owned(),
                        kind: match constraint.kind() {
                            FairnessKind::Weak => "weak",
                            FairnessKind::Strong => "strong",
                        },
                        granularity: match constraint.granularity() {
                            FairnessGranularity::Whole => "whole",
                            FairnessGranularity::Each => "each",
                        },
                    });
                }
                set.sort();
                set.dedup();
                Some(SchedulerRead::Every(set))
            }
        };
        let mut read_rewards = Vec::new();
        if let Some(name) = self.reward_read() {
            for (reward, node) in model.rewards().iter().zip(rewards) {
                if reward.name() == name && belongs(reward.operation().declaring_key()) {
                    read_rewards.extend(node.clone());
                }
            }
            if read_rewards.is_empty() {
                fault.get_or_insert(ClaimReadFault::Reward(name.to_owned()));
            }
            read_rewards.sort();
        }
        let mut distributions: Vec<NodeKey> = model
            .random_parameters()
            .iter()
            .filter(|random| belongs(random.operation().declaring_key()))
            .filter_map(|random| random.node_key(package))
            .collect();
        distributions.sort();
        let mut delays: Vec<NodeKey> = model
            .delays()
            .iter()
            .filter(|delay| belongs(delay.operation().declaring_key()))
            .filter_map(|delay| delay.node_key(package))
            .collect();
        delays.sort();
        ClaimReads {
            subject,
            scheduler,
            rewards: read_rewards,
            distributions,
            delays,
            fault,
        }
    }
}

/// What a claim states in its `form`, `over`, `activation` and `confidence`
/// bindings (FSpec FR-640 "Probabilistic claim node"), as lowering holds it:
/// the nodes it references are keyed, each exact value is its canonical text.
/// The claim's path spells it ([`Statement::spell`]) and S4 writes the same
/// tree as the body's terms, so the path and the body cannot disagree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    /// A text literal.
    Text(String),
    /// A reference to a node.
    Ref(NodeKey),
    /// A reference to the root `temporal`/`formula` node of an event formula,
    /// with the key of every formula node by its `FormulaNodeId` index, so a
    /// cause can name a nested node (FR-498 member path). It spells and
    /// writes as a reference to `root`.
    Formula {
        /// The node the claim's `formula` binding references.
        root: NodeKey,
        /// The key of each formula node, by node index.
        nodes: Vec<NodeKey>,
    },
    /// An exact value: its `value_kind`, the node it is typed at (the unit
    /// node of a quantity) and its canonical text.
    Exact {
        /// The literal's `value_kind`.
        kind: &'static str,
        /// The type node of the literal.
        ty: NodeKey,
        /// The canonical value text.
        text: String,
    },
    /// An aggregate of named bindings; the empty aggregate states nothing.
    Aggregate(Vec<(&'static str, Statement)>),
}

impl Statement {
    /// The exact rational `value`.
    #[must_use]
    pub fn rational(ty: NodeKey, value: &Rational) -> Self {
        Self::Exact {
            kind: "rational",
            ty,
            text: spelling(value),
        }
    }

    /// The exact integer `value`.
    #[must_use]
    pub fn integer(ty: NodeKey, value: u64) -> Self {
        Self::Exact {
            kind: "integer",
            ty,
            text: value.to_string(),
        }
    }

    /// The exact quantity `value` of the unit node `unit`.
    #[must_use]
    pub fn quantity(unit: NodeKey, value: &Rational) -> Self {
        Self::Exact {
            kind: "quantity",
            ty: unit,
            text: spelling(value),
        }
    }

    /// The path segments that spell this statement: an aggregate as each
    /// binding name then its value, a text as itself, a reference as an
    /// operand, an exact value as its kind, the unit operand of a quantity
    /// and its canonical text. A literal's type is no segment of an integer
    /// or a rational, since its position fixes it.
    pub fn spell(&self, path: &mut Vec<PathSegment>) {
        match self {
            Self::Text(text) => path.push(PathSegment::Text(text.clone())),
            Self::Ref(key) | Self::Formula { root: key, .. } => {
                path.push(PathSegment::Node(key.clone()));
            }
            Self::Exact { kind, ty, text } => {
                path.push(PathSegment::Text((*kind).into()));
                if *kind == "quantity" {
                    path.push(PathSegment::Node(ty.clone()));
                }
                path.push(PathSegment::Text(text.clone()));
            }
            Self::Aggregate(members) => {
                for (name, value) in members {
                    path.push(PathSegment::Text((*name).into()));
                    value.spell(path);
                }
            }
        }
    }
}
