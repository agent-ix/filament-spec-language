// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FSpec FR-418 JSON of a task-product `ZoneCertificate` (AC-6 to AC-11).
//!
//! Only the members a task-product certificate carries are written and read:
//! `identity`, `scale`, `bounds`, `nodes`, `edges`, `initials`,
//! `admitted` and `task_product`. A certificate with another member is
//! refused, never written partially.
//! A no-release source certificate supplies its original timed-source context
//! to name every LU clock with the owner's canonical `clock_place` encoding.
//! Source guard parts, horizon fragments and delay-only edges are retained.
//!
//! The wire `admitted` is `{start, edges}`: the path of certificate edges the
//! witness follows. The typed witness also names its divergent target, which
//! the wire does not carry, so it is recovered canonically: when the last
//! edge ends at a node some earlier edge of the list leaves, the smallest
//! closed suffix is the cycle and the rest the path; otherwise the list is
//! the path to a quiescent node. A producer writes only witnesses this
//! reading returns unchanged. This canonical decode is the v1 reading until
//! FR-418 gives `admitted` an explicit `target` member.
//!
//! Receiver references: FR-181 spells an object reference with a node-id
//! `object_type` and a lowercase-hex `identity`, but the evaluator writes the
//! effective-id object `{owner_effective_type, original}` (FR-361-AC-13,
//! TC-2370) and an opaque text identity. The `task_product` shape check
//! accepts the writer's spelling until the specification reconciles the two.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use filament_exact::Integer;
use fsl_eval::simulation::model::{PlaceConstraint, TimedSystem};
use fsl_eval::simulation::StateKey;
use fsl_semantics::check::ClockOp;
use serde_json::{json, Map, Value};

use super::{CertEdge, CertNode, DivergenceWitness, DivergentTarget, ZoneCertificate};
use crate::certificate::task_product::{
    array, child, count, integer, member, object, rational, rational_wire, refuse,
    source_transition, source_transition_wire, TaskProductCertificate, TaskProductShapeReason,
    TaskProductShapeRefusal, TaskTransitionTemplate,
};
use crate::certificate::zone::{LuBounds, Zone};

/// Why a certificate has no wire, or a wire no certificate.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ZoneCertificateWireError {
    /// The certificate carries a member the task-product wire does not.
    #[error("the certificate carries a member outside the task-product form: {0}")]
    Unsupported(&'static str),
    /// A value has no wire spelling.
    #[error("a value has no wire spelling")]
    Unspelled,
    /// The wire is not a task-product `ZoneCertificate`.
    #[error("{}: {:?}", .0.pointer, .0.reason)]
    Shape(TaskProductShapeRefusal),
}

impl From<TaskProductShapeRefusal> for ZoneCertificateWireError {
    fn from(refusal: TaskProductShapeRefusal) -> Self {
        Self::Shape(refusal)
    }
}

fn zone_wire(zone: &Zone) -> Result<Value, ZoneCertificateWireError> {
    let dim = zone.dim();
    let mut rows = Vec::with_capacity(dim);
    for i in 0..dim {
        let mut row = Vec::with_capacity(dim);
        for j in 0..dim {
            row.push(
                match zone
                    .bound(i, j)
                    .map_err(|_| ZoneCertificateWireError::Unspelled)?
                {
                    Some((bound, strict)) => json!({"bound": bound.to_string(), "strict": strict}),
                    None => json!("infinity"),
                },
            );
        }
        rows.push(Value::Array(row));
    }
    Ok(Value::Array(rows))
}

fn read_zone(value: &Value, pointer: &str) -> Result<Zone, TaskProductShapeRefusal> {
    let rows = array(value, pointer)?;
    let dim = rows.len();
    let mut cells = Vec::with_capacity(dim * dim);
    for (i, row) in rows.iter().enumerate() {
        let at = child(pointer, i);
        let row = array(row, &at)?;
        if row.len() != dim {
            return refuse(&at, TaskProductShapeReason::NotCanonical);
        }
        for (j, entry) in row.iter().enumerate() {
            let at = child(&at, j);
            if entry.as_str() == Some("infinity") {
                cells.push(None);
                continue;
            }
            let map = object(entry, &at, &["bound", "strict"], &[])?;
            let bound = integer(member(map, "bound"), &child(&at, "bound"))?;
            let Some(strict) = member(map, "strict").as_bool() else {
                return refuse(&child(&at, "strict"), TaskProductShapeReason::WrongKind);
            };
            cells.push(Some((bound, strict)));
        }
    }
    Zone::from_matrix(dim, cells).or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))
}

fn state_wire(key: &StateKey) -> Value {
    json!({"key": URL_SAFE_NO_PAD.encode(key.as_bytes())})
}

fn read_state(value: &Value, pointer: &str) -> Result<StateKey, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["key"], &[])?;
    let at = child(pointer, "key");
    let Some(text) = member(map, "key").as_str() else {
        return refuse(&at, TaskProductShapeReason::WrongKind);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .or_else(|_| refuse(&at, TaskProductShapeReason::NotCanonical))?;
    Ok(StateKey::from_canonical_bytes(bytes))
}

fn source_names(source: &TimedSystem<'_, '_>) -> Result<Vec<String>, ZoneCertificateWireError> {
    (0..source.places().len())
        .map(|place| {
            source
                .clock_place_identity(place)
                .ok_or(ZoneCertificateWireError::Unspelled)
        })
        .collect()
}

fn lu_wire(value: &Option<Integer>) -> Value {
    match value {
        Some(value) => json!({"bound":value.to_string(),"strict":false}),
        None => json!("infinity"),
    }
}

fn read_lu(value: &Value, at: &str) -> Result<Option<Integer>, TaskProductShapeRefusal> {
    if value.as_str() == Some("infinity") {
        return Ok(None);
    }
    let row = object(value, at, &["bound", "strict"], &[])?;
    if member(row, "strict") != &Value::Bool(false) {
        return refuse(&child(at, "strict"), TaskProductShapeReason::NotCanonical);
    }
    integer(member(row, "bound"), &child(at, "bound")).map(Some)
}

fn op_text(op: ClockOp) -> &'static str {
    match op {
        ClockOp::Less => "<",
        ClockOp::LessOrEqual => "<=",
        ClockOp::Equal => "=",
        ClockOp::GreaterOrEqual => ">=",
        ClockOp::Greater => ">",
    }
}

#[ix_trace_rs::string_edge]
fn read_constraint(value: &Value, at: &str) -> Result<PlaceConstraint, TaskProductShapeRefusal> {
    let row = object(value, at, &["place", "operator", "bound"], &[])?;
    let op = match member(row, "operator").as_str() {
        Some("<") => ClockOp::Less,
        Some("<=") => ClockOp::LessOrEqual,
        Some("=") => ClockOp::Equal,
        Some(">=") => ClockOp::GreaterOrEqual,
        Some(">") => ClockOp::Greater,
        _ => {
            return refuse(
                &child(at, "operator"),
                TaskProductShapeReason::UnknownVariant,
            )
        }
    };
    let place =
        usize::try_from(count(member(row, "place"), &child(at, "place"))?).map_err(|_| {
            TaskProductShapeRefusal {
                pointer: child(at, "place"),
                reason: TaskProductShapeReason::NotCanonical,
            }
        })?;
    Ok(PlaceConstraint::new(
        place,
        op,
        rational(member(row, "bound"), &child(at, "bound"))?,
    ))
}

impl<O> ZoneCertificate<O> {
    /// The certificate's wire value, `identity` the structural identity of
    /// the obligation it proves (FR-412).
    ///
    /// # Errors
    ///
    /// [`ZoneCertificateWireError`] when a member lies outside the
    /// task-product form or a value has no spelling.
    pub fn to_wire(&self, identity: Value) -> Result<Value, ZoneCertificateWireError> {
        self.write_wire(identity, None)
    }

    /// Writes source LU bounds with the original clock-place identities and
    /// preserves source guard fragments. No clock identity is inferred from bytes.
    ///
    /// # Errors
    /// Returns a shape or spelling refusal when the source context does not fit.
    pub fn to_wire_with_source(
        &self,
        identity: Value,
        source: &TimedSystem<'_, '_>,
    ) -> Result<Value, ZoneCertificateWireError> {
        self.write_wire(identity, Some(&source_names(source)?))
    }

    fn write_wire(
        &self,
        identity: Value,
        source: Option<&[String]>,
    ) -> Result<Value, ZoneCertificateWireError> {
        use ZoneCertificateWireError::Unsupported;
        let mut bounds = Vec::new();
        if !self.lu_bounds.lower.is_empty() || !self.lu_bounds.upper.is_empty() {
            let names = source.ok_or(Unsupported("bounds"))?;
            if names.len() != self.lu_bounds.lower.len()
                || names.len() != self.lu_bounds.upper.len()
            {
                return Err(Unsupported("bounds"));
            }
            let mut seen = std::collections::BTreeSet::new();
            for node in &self.nodes {
                if !seen.insert(node.state_key.as_bytes()) {
                    continue;
                }
                for (index, name) in names.iter().enumerate() {
                    bounds.push(json!({"state":state_wire(&node.state_key),"clock":name,"lower":lu_wire(&self.lu_bounds.lower[index]),"upper":lu_wire(&self.lu_bounds.upper[index])}));
                }
            }
        }
        if self.components.is_some() {
            return Err(Unsupported("components"));
        }
        if self.non_zeno.is_some() {
            return Err(Unsupported("non_zeno"));
        }
        if self.divergence.is_some() {
            return Err(Unsupported("divergence"));
        }
        if self.state_graph.is_some() {
            return Err(Unsupported("state_graph"));
        }
        if self.refinement.is_some() {
            return Err(Unsupported("refinement"));
        }
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if node.automaton_state != 0 || node.product.is_some() {
                return Err(Unsupported("nodes"));
            }
            let mut map = Map::new();
            map.insert("automaton".to_owned(), json!(0));
            if node.locked {
                map.insert("locked".to_owned(), json!(true));
            }
            map.insert("state".to_owned(), state_wire(&node.state_key));
            map.insert("zone".to_owned(), zone_wire(&node.zone)?);
            nodes.push(Value::Object(map));
        }
        let mut edges = Vec::with_capacity(self.edges.len());
        for edge in &self.edges {
            if edge.silent.is_some()
                || (source.is_none() && (edge.part.is_some() || edge.horizon.is_some()))
            {
                return Err(Unsupported("edges"));
            }
            let transition = match (&edge.transition, &edge.task_transition) {
                (Some(source), None) => source_transition_wire(source)
                    .map_err(|_| ZoneCertificateWireError::Unspelled)?,
                (None, Some(template)) => template
                    .to_wire()
                    .map_err(|_| ZoneCertificateWireError::Unspelled)?,
                (None, None) if source.is_some() && edge.part.is_none() => Value::Null,
                _ => return Err(Unsupported("edges")),
            };
            let mut row = json!({"node": edge.from, "target": edge.to, "transition": transition});
            if let Some(part) = edge.part {
                row["part"] = json!(part);
            }
            if let Some(horizon) = &edge.horizon {
                row["horizon"]=Value::Array(horizon.iter().map(|constraint|json!({"place":constraint.place(),"operator":op_text(constraint.op()),"bound":rational_wire(constraint.bound())})).collect());
            }
            edges.push(row);
        }
        let mut map = Map::new();
        if let Some(witness) = &self.admitted {
            let mut path = witness.path.clone();
            match &witness.target {
                DivergentTarget::Quiescent(_) => {}
                DivergentTarget::Cycle(cycle) => path.extend(cycle),
                DivergentTarget::Terminal(_) => return Err(Unsupported("admitted")),
            }
            map.insert(
                "admitted".to_owned(),
                json!({"edges": path, "start": witness.start}),
            );
        }
        map.insert("bounds".to_owned(), Value::Array(bounds));
        map.insert("edges".to_owned(), Value::Array(edges));
        map.insert("identity".to_owned(), identity);
        map.insert("initials".to_owned(), json!(self.initials));
        map.insert("nodes".to_owned(), Value::Array(nodes));
        map.insert("scale".to_owned(), json!(self.scale.to_string()));
        if let Some(member) = &self.task_product {
            map.insert(
                "task_product".to_owned(),
                member
                    .to_wire()
                    .map_err(|_| ZoneCertificateWireError::Unspelled)?,
            );
        }
        Ok(Value::Object(map))
    }

    /// Reads a task-product certificate proving `obligation`; the wire's
    /// `identity` is returned for the caller to compare with the item's.
    ///
    /// # Errors
    ///
    /// [`ZoneCertificateWireError`] naming the first offence.
    pub fn from_wire(
        value: &Value,
        obligation: O,
    ) -> Result<(Self, Value), ZoneCertificateWireError> {
        Self::read_wire(value, obligation, None)
    }

    /// Reads source bounds against the original source's complete clock layout.
    /// Every state/clock row must be present in canonical order; differing rows
    /// cannot silently collapse to one global bound.
    ///
    /// # Errors
    /// Returns the first malformed row or a source-context mismatch.
    pub fn from_wire_with_source(
        value: &Value,
        obligation: O,
        source: &TimedSystem<'_, '_>,
    ) -> Result<(Self, Value), ZoneCertificateWireError> {
        Self::read_wire(value, obligation, Some(&source_names(source)?))
    }

    fn read_wire(
        value: &Value,
        obligation: O,
        source: Option<&[String]>,
    ) -> Result<(Self, Value), ZoneCertificateWireError> {
        let map = object(
            value,
            "",
            &["identity", "scale", "bounds", "nodes", "edges", "initials"],
            &["admitted", "task_product"],
        )?;
        let scale = integer(member(map, "scale"), "/scale")?;
        let bounds = array(member(map, "bounds"), "/bounds")?;
        let mut nodes = Vec::new();
        for (index, node) in array(member(map, "nodes"), "/nodes")?.iter().enumerate() {
            let at = child("/nodes", index);
            let node = object(node, &at, &["state", "automaton", "zone"], &["locked"])?;
            if count(member(node, "automaton"), &child(&at, "automaton"))? != 0 {
                return Err(ZoneCertificateWireError::Unsupported("nodes"));
            }
            let locked = match node.get("locked") {
                None => false,
                Some(Value::Bool(true)) => true,
                Some(_) => {
                    return refuse(&child(&at, "locked"), TaskProductShapeReason::NotCanonical)
                        .map_err(Into::into)
                }
            };
            nodes.push(CertNode {
                state_key: read_state(member(node, "state"), &child(&at, "state"))?,
                automaton_state: 0,
                zone: read_zone(member(node, "zone"), &child(&at, "zone"))?,
                locked,
                product: None,
            });
        }
        let mut lu_bounds = LuBounds {
            lower: Vec::new(),
            upper: Vec::new(),
        };
        if !bounds.is_empty() {
            let names = source.ok_or(ZoneCertificateWireError::Unsupported("bounds"))?;
            let mut seen = std::collections::BTreeSet::new();
            let mut cursor = 0;
            for node in &nodes {
                if !seen.insert(node.state_key.as_bytes()) {
                    continue;
                }
                for (clock, name) in names.iter().enumerate() {
                    let at = child("/bounds", cursor);
                    let Some(value) = bounds.get(cursor) else {
                        return refuse(&at, TaskProductShapeReason::NotCanonical)
                            .map_err(Into::into);
                    };
                    let row = object(value, &at, &["state", "clock", "lower", "upper"], &[])?;
                    if read_state(member(row, "state"), &child(&at, "state"))? != node.state_key
                        || member(row, "clock").as_str() != Some(name)
                    {
                        return refuse(&at, TaskProductShapeReason::NotCanonical)
                            .map_err(Into::into);
                    }
                    let lower = read_lu(member(row, "lower"), &child(&at, "lower"))?;
                    let upper = read_lu(member(row, "upper"), &child(&at, "upper"))?;
                    if lu_bounds.lower.len() < names.len() {
                        lu_bounds.lower.push(lower);
                        lu_bounds.upper.push(upper);
                    } else if lu_bounds.lower[clock] != lower || lu_bounds.upper[clock] != upper {
                        return refuse(&at, TaskProductShapeReason::NotCanonical)
                            .map_err(Into::into);
                    }
                    cursor += 1;
                }
            }
            if cursor != bounds.len() {
                return refuse("/bounds", TaskProductShapeReason::NotCanonical).map_err(Into::into);
            }
        }
        let mut edges = Vec::new();
        for (index, edge) in array(member(map, "edges"), "/edges")?.iter().enumerate() {
            let at = child("/edges", index);
            let optional = if source.is_some() {
                &["part", "horizon"][..]
            } else {
                &[][..]
            };
            let edge = object(edge, &at, &["node", "transition", "target"], optional)?;
            let transition = member(edge, "transition");
            let template =
                transition.get("type").and_then(Value::as_str) == Some("task-transition-template");
            let at_transition = child(&at, "transition");
            let (source_step, task) = if template {
                (
                    None,
                    Some(TaskTransitionTemplate::read(transition, &at_transition)?),
                )
            } else if transition.is_null() && source.is_some() {
                (None, None)
            } else {
                (Some(source_transition(transition, &at_transition)?), None)
            };
            if source_step.is_none() && task.is_none() && edge.contains_key("part") {
                return refuse(&child(&at, "part"), TaskProductShapeReason::NotCanonical)
                    .map_err(Into::into);
            }
            edges.push(CertEdge {
                from: usize::try_from(count(member(edge, "node"), &child(&at, "node"))?)
                    .unwrap_or(usize::MAX),
                transition: source_step,
                task_transition: task,
                part: edge
                    .get("part")
                    .map(|value| {
                        count(value, &child(&at, "part"))
                            .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
                    })
                    .transpose()?,
                horizon: edge
                    .get("horizon")
                    .map(|value| {
                        array(value, &child(&at, "horizon"))?
                            .iter()
                            .enumerate()
                            .map(|(index, value)| {
                                read_constraint(value, &child(&child(&at, "horizon"), index))
                            })
                            .collect()
                    })
                    .transpose()?,
                to: usize::try_from(count(member(edge, "target"), &child(&at, "target"))?)
                    .unwrap_or(usize::MAX),
                silent: None,
            });
        }
        let initials = array(member(map, "initials"), "/initials")?
            .iter()
            .enumerate()
            .map(|(index, node)| {
                count(node, &child("/initials", index))
                    .map(|node| usize::try_from(node).unwrap_or(usize::MAX))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let admitted = match map.get("admitted") {
            None => None,
            Some(witness) => Some(read_witness(witness, &edges)?),
        };
        let task_product = match map.get("task_product") {
            None => None,
            Some(member) => Some(TaskProductCertificate::from_wire(member)?),
        };
        Ok((
            Self {
                obligation,
                scale,
                lu_bounds,
                nodes,
                edges,
                initials,
                components: None,
                non_zeno: None,
                divergence: None,
                admitted,
                state_graph: None,
                refinement: None,
                task_product,
            },
            member(map, "identity").clone(),
        ))
    }
}

fn read_witness(
    value: &Value,
    edges: &[CertEdge],
) -> Result<DivergenceWitness, TaskProductShapeRefusal> {
    let map = object(value, "/admitted", &["start", "edges"], &[])?;
    let start =
        usize::try_from(count(member(map, "start"), "/admitted/start")?).unwrap_or(usize::MAX);
    let path: Vec<usize> = array(member(map, "edges"), "/admitted/edges")?
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            count(edge, &child("/admitted/edges", index))
                .map(|edge| usize::try_from(edge).unwrap_or(usize::MAX))
        })
        .collect::<Result<_, _>>()?;
    // An index past the edges is left for the checker's own shape rule.
    let Some(&last) = path.last() else {
        return Ok(DivergenceWitness {
            start,
            path,
            target: DivergentTarget::Quiescent(start),
        });
    };
    let (Some(end), true) = (
        edges.get(last).map(|edge| edge.to),
        path.iter().all(|&e| e < edges.len()),
    ) else {
        return Ok(DivergenceWitness {
            start,
            path,
            target: DivergentTarget::Quiescent(start),
        });
    };
    match (0..path.len()).rev().find(|&k| edges[path[k]].from == end) {
        Some(k) => Ok(DivergenceWitness {
            start,
            target: DivergentTarget::Cycle(path[k..].to_vec()),
            path: path[..k].to_vec(),
        }),
        None => Ok(DivergenceWitness {
            start,
            path,
            target: DivergentTarget::Quiescent(end),
        }),
    }
}
