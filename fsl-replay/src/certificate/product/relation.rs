// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-163: HP-1 evidence and independent reachable-closure reconstruction.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use base64::Engine as _;
use filament_exact::{Cancel, Meter};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{
    check_model_bounds, ModelRelationDescriptor, ModelRelationTuple, ModelSimState, ModelSystem,
};
use fsl_eval::simulation::{ExpansionLimit, StateKey, TransitionSystem, BOUND_POSITIONS};
use fsl_eval::value::ModelRelationEvaluation;
use fsl_semantics::check::HyperForm;
use serde::{ser::SerializeStruct, Serialize, Serializer};

use super::{
    ModelRelationClosureRequest, ProductClosure, ProductClosureCertificate, ProductClosureCheck,
    ProductClosureRefusal,
};
use crate::{
    CertificateLocus, CertificateRejection, CertificateRule, IncompleteCause, ModelCheckLimit,
    ModelCheckLimits,
};

/// One complete applicable labelled edge, including its observed result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperTransitionEdge {
    /// The actual pre-state's canonical key.
    pub pre_state: StateKey,
    /// Operation, receiver and arguments.
    pub transition: ModelTransition,
    /// The actual post-state's canonical key.
    pub post_state: StateKey,
    /// A typed canonical result; absence means the operation has no result.
    pub result: Option<String>,
}

impl Serialize for HyperTransitionEdge {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let key = |key: &StateKey| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes())
        };
        let result: Option<serde_json::Value> = self
            .result
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(serde::ser::Error::custom)?;
        let mut object = serializer.serialize_struct("HyperTransitionEdge", 4)?;
        object.serialize_field("pre_state", &key(&self.pre_state))?;
        object.serialize_field(
            "transition",
            &crate::outcome::TimedTransitionWire::from(&self.transition),
        )?;
        object.serialize_field("post_state", &key(&self.post_state))?;
        object.serialize_field("result", &result)?;
        object.end()
    }
}

/// Complete applicable edges of one execution, in canonical edge order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HyperExecutionEdges {
    /// The declared execution variable.
    pub execution: String,
    /// Every applicable labelled edge, including distinct results.
    pub transitions: Vec<HyperTransitionEdge>,
}

/// The reachable closure of one alias, shared by its execution variables.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HyperSubjectClosure {
    /// The model alias in its declaring source unit.
    pub alias: String,
    /// Actual state keys, in ascending octet order without duplicates.
    #[serde(serialize_with = "serialize_states")]
    pub states: Vec<StateKey>,
    /// Execution entries in binding order restricted to this alias.
    pub executions: Vec<HyperExecutionEdges>,
}

fn serialize_states<S: Serializer>(states: &[StateKey], serializer: S) -> Result<S::Ok, S::Error> {
    let keys: Vec<_> = states
        .iter()
        .map(|key| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes()))
        .collect();
    keys.serialize(serializer)
}

/// One complete tuple of labelled edges in execution-binding order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HyperTransitionTuple {
    /// Exactly one edge for every execution variable.
    pub transitions: Vec<HyperTransitionEdge>,
}

/// The tuple whose evaluation would exceed this check's own allowance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperTupleStop {
    /// The one-based ordinal of the first unpaid tuple.
    pub count: u64,
    /// The complete tuple, which has not been evaluated.
    pub first_unpaid: HyperTransitionTuple,
}

struct Graph {
    initial: BTreeSet<StateKey>,
    states: BTreeMap<StateKey, ModelSimState>,
    edges: Vec<HyperTransitionEdge>,
    undefined: Option<StateKey>,
}

fn rejection(rule: CertificateRule, key: &StateKey) -> ProductClosureCheck {
    ProductClosureCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::HyperState(key.clone()),
    })
}

fn stopped(limit: ModelCheckLimit, value: u64) -> ProductClosureRefusal {
    ProductClosureRefusal::Stopped(IncompleteCause::LimitReached {
        limit,
        value,
        required: None,
    })
}

fn poll(cancel: &Cancel) -> Result<(), ProductClosureRefusal> {
    if cancel.poll() {
        if let Some(source) = cancel.cause() {
            return Err(ProductClosureRefusal::Stopped(IncompleteCause::Cancelled {
                source,
            }));
        }
    }
    Ok(())
}

#[derive(Default)]
struct Counts {
    states: usize,
    edges: usize,
}

impl Counts {
    fn state(&mut self, limits: &ModelCheckLimits) -> Result<(), ProductClosureRefusal> {
        self.states = self
            .states
            .checked_add(1)
            .ok_or(ProductClosureRefusal::Recompute("state-count"))?;
        if self.states > limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                u64::try_from(limits.limits.max_states)
                    .map_err(|_| ProductClosureRefusal::Input("max-states"))?,
            ));
        }
        Ok(())
    }

    fn edge(&mut self, limits: &ModelCheckLimits) -> Result<(), ProductClosureRefusal> {
        self.edges = self
            .edges
            .checked_add(1)
            .ok_or(ProductClosureRefusal::Recompute("transition-count"))?;
        if self.edges > limits.limits.max_transitions {
            return Err(stopped(
                ModelCheckLimit::MaxTransitions,
                u64::try_from(limits.limits.max_transitions)
                    .map_err(|_| ProductClosureRefusal::Input("max-transitions"))?,
            ));
        }
        Ok(())
    }
}

fn reconstruct(
    system: &ModelSystem<'_>,
    request: &ModelRelationClosureRequest<'_>,
    alias_index: usize,
    limits: &ModelCheckLimits,
    counts: &mut Counts,
    cancel: &Cancel,
) -> Result<Graph, ProductClosureRefusal> {
    let mut graph = Graph {
        initial: BTreeSet::new(),
        states: BTreeMap::new(),
        edges: Vec::new(),
        undefined: None,
    };
    let mut queue = VecDeque::new();
    let mut initials = BTreeMap::new();
    for state in system.initial() {
        let (_, key) = system
            .state_key(&state)
            .map_err(|_| ProductClosureRefusal::Recompute("state-key"))?;
        initials.entry(key).or_insert(state);
    }
    if initials.is_empty() {
        return Err(ProductClosureRefusal::Input("no-initial-state"));
    }
    for (key, state) in initials {
        poll(cancel)?;
        counts.state(limits)?;
        graph.initial.insert(key.clone());
        graph.states.insert(key.clone(), state);
        queue.push_back(key);
    }
    while let Some(key) = queue.pop_front() {
        poll(cancel)?;
        let state = graph
            .states
            .get(&key)
            .ok_or(ProductClosureRefusal::Recompute("queued-state"))?;
        let expansion = system.relation_successors(state, cancel).map_err(|stop| {
            if let Some(source) = cancel.cause() {
                return ProductClosureRefusal::Stopped(IncompleteCause::Cancelled { source });
            }
            match stop.limit {
                Some(ExpansionLimit::MaxCandidates) => stopped(
                    ModelCheckLimit::MaxCandidates,
                    request.subjects[alias_index]
                        .simulation
                        .expansion
                        .max_candidates,
                ),
                Some(ExpansionLimit::EvaluationMeter(counter)) => stopped(
                    ModelCheckLimit::EvaluationMeter { counter },
                    crate::meter_bound(&request.meter, counter),
                ),
                None if stop.is_internal_failure() => {
                    ProductClosureRefusal::Recompute("model-expansion")
                }
                None => ProductClosureRefusal::Stopped(IncompleteCause::ResourceExhausted),
            }
        })?;
        if !expansion.findings.is_empty() && graph.undefined.is_none() {
            graph.undefined = Some(key.clone());
        }
        let mut ordered = Vec::new();
        for (labelled, next) in expansion.successors {
            poll(cancel)?;
            let (_, target) = system
                .state_key(&next)
                .map_err(|_| ProductClosureRefusal::Recompute("successor-key"))?;
            let order = quire_canonical::to_vec(
                &labelled,
                quire_canonical::Limits::new(fsl_foundation::IdentityLimits::default().input_bytes),
            )
            .map_err(|_| ProductClosureRefusal::Recompute("transition-key"))?;
            ordered.push((order, target, labelled, next));
        }
        ordered.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, target, labelled, next) in ordered {
            counts.edge(limits)?;
            if !graph.states.contains_key(&target) {
                counts.state(limits)?;
                graph.states.insert(target.clone(), next);
                queue.push_back(target.clone());
            }
            graph.edges.push(HyperTransitionEdge {
                pre_state: key.clone(),
                transition: labelled.transition,
                post_state: target,
                result: labelled.result,
            });
        }
    }
    Ok(graph)
}

pub(super) fn check(
    request: &ModelRelationClosureRequest<'_>,
    relation: &ModelRelationDescriptor,
    certificate: &ProductClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<ProductClosureCheck, ProductClosureRefusal> {
    let ProductClosure::Relation(subjects) = &certificate.closure else {
        return Err(ProductClosureRefusal::Input("relation-closure"));
    };
    if certificate.form != HyperForm::StepRelation
        || !certificate.instance.is_empty()
        || certificate.reduction.is_some()
        || certificate.components.is_some()
        || subjects.len() != relation.aliases().len()
        || request.subjects.len() != subjects.len()
    {
        return Err(ProductClosureRefusal::Input("relation-certificate"));
    }
    for (subject, alias) in subjects.iter().zip(relation.aliases()) {
        if &subject.alias != alias {
            return Err(ProductClosureRefusal::Input("relation-alias-order"));
        }
        let executions: Vec<_> = relation
            .executions()
            .iter()
            .zip(relation.execution_aliases())
            .filter(|(_, owner)| *owner == alias)
            .map(|((name, _), _)| name.as_str())
            .collect();
        if !subject
            .executions
            .iter()
            .map(|execution| execution.execution.as_str())
            .eq(executions)
        {
            return Err(ProductClosureRefusal::Input("relation-execution-order"));
        }
    }
    for subject in subjects {
        for pair in subject.states.windows(2) {
            if pair[0] >= pair[1] {
                return Ok(rejection(CertificateRule::Malformed, &pair[1]));
            }
        }
    }
    let mut systems = Vec::new();
    for (subject, alias) in request.subjects.iter().zip(relation.aliases()) {
        poll(cancel)?;
        let expected = relation
            .model_package_identity(alias)
            .ok_or(ProductClosureRefusal::Input("model-alias"))?;
        let system = ModelSystem::for_model_identity(subject.simulation.clone(), expected)
            .map_err(|_| ProductClosureRefusal::Input("model-subject"))?;
        if system.model() != Some(expected) {
            return Err(ProductClosureRefusal::Input("model-alias"));
        }
        check_model_bounds(&system, BOUND_POSITIONS)
            .map_err(|_| ProductClosureRefusal::Input("model-bounds"))?;
        systems.push(system);
    }
    let mut counts = Counts::default();
    let graphs: Vec<_> = systems
        .iter()
        .enumerate()
        .map(|(index, system)| reconstruct(system, request, index, limits, &mut counts, cancel))
        .collect::<Result<_, _>>()?;
    for (subject, graph) in subjects.iter().zip(&graphs) {
        if let Some(key) = graph
            .initial
            .iter()
            .find(|key| subject.states.binary_search(key).is_err())
        {
            return Ok(rejection(CertificateRule::InitialMissing, key));
        }
    }
    for (subject, graph) in subjects.iter().zip(&graphs) {
        let supplied: BTreeSet<_> = subject.states.iter().cloned().collect();
        let reached: BTreeSet<_> = graph.states.keys().cloned().collect();
        if let Some(key) = supplied.symmetric_difference(&reached).next() {
            return Ok(rejection(
                if supplied.contains(key) {
                    CertificateRule::Malformed
                } else {
                    CertificateRule::SuccessorMissing
                },
                key,
            ));
        }
    }
    for graph in &graphs {
        if let Some(key) = &graph.undefined {
            return Ok(rejection(CertificateRule::UndefinedMember, key));
        }
    }
    let mut choices = Vec::new();
    let mut owners = Vec::new();
    for ((execution, operation), alias) in relation
        .executions()
        .iter()
        .zip(relation.execution_aliases())
    {
        let owner = relation
            .aliases()
            .iter()
            .position(|known| known == alias)
            .ok_or(ProductClosureRefusal::Input("execution-alias"))?;
        let operation = fsl_semantics::check::operation_text(operation);
        let edges: Vec<_> = graphs[owner]
            .edges
            .iter()
            .filter(|edge| edge.transition.operation() == operation)
            .cloned()
            .collect();
        let supplied = subjects[owner]
            .executions
            .iter()
            .find(|entry| &entry.execution == execution)
            .ok_or(ProductClosureRefusal::Input("execution"))?;
        if supplied.transitions != edges {
            return Err(ProductClosureRefusal::Input("execution-edges"));
        }
        choices.push(edges);
        owners.push(owner);
    }
    if choices.iter().any(Vec::is_empty) {
        return Ok(ProductClosureCheck::Accepted);
    }
    let mut positions = vec![0; choices.len()];
    let mut evaluated = 0_u64;
    loop {
        poll(cancel)?;
        let transitions: Vec<_> = choices
            .iter()
            .zip(&positions)
            .map(|(edges, position)| edges[*position].clone())
            .collect();
        let tuple = HyperTransitionTuple { transitions };
        let ordinal = evaluated
            .checked_add(1)
            .ok_or(ProductClosureRefusal::Recompute("tuple-count"))?;
        if evaluated >= limits.max_relation_tuples {
            return Ok(ProductClosureCheck::Stopped {
                cause: IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxRelationTuples,
                    value: limits.max_relation_tuples,
                    required: None,
                },
                tuple: Some(HyperTupleStop {
                    count: ordinal,
                    first_unpaid: tuple,
                }),
            });
        }
        let observe = |post: bool| -> Result<Vec<_>, ProductClosureRefusal> {
            tuple
                .transitions
                .iter()
                .zip(&owners)
                .map(|(edge, owner)| {
                    let key = if post {
                        &edge.post_state
                    } else {
                        &edge.pre_state
                    };
                    let state = graphs[*owner]
                        .states
                        .get(key)
                        .ok_or(ProductClosureRefusal::Recompute("tuple-state"))?;
                    systems[*owner]
                        .observe(state.state())
                        .map_err(|_| ProductClosureRefusal::Recompute("tuple-observation"))
                })
                .collect()
        };
        let pre = observe(false)?;
        let post = observe(true)?;
        let pre_env: Vec<_> = pre
            .iter()
            .map(|observation| observation.environment())
            .collect();
        let post_env: Vec<_> = post
            .iter()
            .map(|observation| observation.environment())
            .collect();
        let labels: Vec<_> = tuple
            .transitions
            .iter()
            .map(|edge| &edge.transition)
            .collect();
        let results: Vec<_> = tuple
            .transitions
            .iter()
            .map(|edge| edge.result.as_deref())
            .collect();
        let evaluation = request
            .evaluator
            .evaluate_model_relation(
                relation,
                ModelRelationTuple {
                    transitions: &labels,
                    results: &results,
                    pre: &pre_env,
                    post: &post_env,
                },
                &mut Meter::new(request.meter).with_cancel(cancel.clone()),
            )
            .map_err(|_| ProductClosureRefusal::Recompute("relation-body"))?;
        evaluated = ordinal;
        let rule = match evaluation {
            ModelRelationEvaluation::Holds => None,
            ModelRelationEvaluation::Refuted => Some(CertificateRule::ViolatingMember),
            ModelRelationEvaluation::Undefined(_) => Some(CertificateRule::UndefinedMember),
            ModelRelationEvaluation::Undetermined(_) => {
                poll(cancel)?;
                Some(CertificateRule::UndefinedMember)
            }
        };
        if let Some(rule) = rule {
            return Ok(ProductClosureCheck::Rejected(CertificateRejection {
                rule,
                at: CertificateLocus::HyperTransitionTuple(tuple),
            }));
        }
        let mut advanced = false;
        for (position, edges) in positions.iter_mut().zip(&choices).rev() {
            *position += 1;
            if *position < edges.len() {
                advanced = true;
                break;
            }
            *position = 0;
        }
        if !advanced {
            return Ok(ProductClosureCheck::Accepted);
        }
    }
}
