// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scoped certificate metadata. Legacy Zone scalar-loop work is excluded
//! from QSL-2726; this module does not invent a cost for those limb loops.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

pub(super) fn source_names(
    source: &TimedSystem<'_, '_>,
    s: &mut ReferenceScope<'_, '_, '_, '_>,
) -> Result<Vec<String>, ZoneCertificateReferenceError> {
    let mut names = s.storage(source.places().len())?;
    for place in 0..source.places().len() {
        names.push(s.clock_place_identity(source, place)?);
    }
    Ok(names)
}

fn lu(
    value: &Option<Integer>,
    s: &mut ReferenceScope<'_, '_, '_, '_>,
) -> Result<Value, ReferenceCodecError> {
    match value {
        None => s.text("infinity"),
        Some(value) => {
            let bound = s.integer(value)?;
            s.record([("bound", bound), ("strict", Value::Bool(false))])
        }
    }
}

impl<O> ZoneCertificate<O> {
    pub(super) fn write_scoped(
        &self,
        identity: Value,
        source: Option<&TimedSystem<'_, '_>>,
        s: &mut ReferenceScope<'_, '_, '_, '_>,
    ) -> Result<Value, ZoneCertificateReferenceError> {
        let unsupported = |field| {
            ZoneCertificateReferenceError::from(ZoneCertificateWireError::Unsupported(field))
        };
        if self.components.is_some() {
            return Err(unsupported("components"));
        }
        if self.non_zeno.is_some() {
            return Err(unsupported("non_zeno"));
        }
        if self.divergence.is_some() {
            return Err(unsupported("divergence"));
        }
        if self.state_graph.is_some() {
            return Err(unsupported("state_graph"));
        }
        if self.refinement.is_some() {
            return Err(unsupported("refinement"));
        }
        let names = source.map(|source| source_names(source, s)).transpose()?;
        let mut bounds = s.array(0)?;
        if !self.lu_bounds.lower.is_empty() || !self.lu_bounds.upper.is_empty() {
            let names = names.as_ref().ok_or_else(|| unsupported("bounds"))?;
            if self.lu_bounds.lower.len() != names.len()
                || self.lu_bounds.upper.len() != names.len()
            {
                return Err(unsupported("bounds"));
            }
            let mut seen: Vec<&StateKey> = s.storage(self.nodes.len())?;
            for node in &self.nodes {
                let mut duplicate = false;
                for previous in &seen {
                    s.work(previous.as_bytes().len())?;
                    s.work(node.state_key.as_bytes().len())?;
                    if previous.as_bytes() == node.state_key.as_bytes() {
                        duplicate = true;
                        break;
                    }
                }
                if duplicate {
                    continue;
                }
                seen.push(&node.state_key);
            }
            let count = seen
                .len()
                .checked_mul(names.len())
                .ok_or(fsl_eval::simulation::model::ModelKeyError::SizeOverflow)
                .map_err(ZoneCertificateWireError::from)?;
            bounds = s.array(count)?;
            for key in seen {
                for (index, name) in names.iter().enumerate() {
                    let row = s.at_index("/bounds", bounds.len(), |s| {
                        let state = s.state(key)?;
                        let clock = s.text(name)?;
                        let lower = lu(&self.lu_bounds.lower[index], s)?;
                        let upper = lu(&self.lu_bounds.upper[index], s)?;
                        s.record([
                            ("state", state),
                            ("clock", clock),
                            ("lower", lower),
                            ("upper", upper),
                        ])
                    })?;
                    bounds.push(row);
                }
            }
        }
        let mut nodes = s.array(self.nodes.len())?;
        for (index, node) in self.nodes.iter().enumerate() {
            if node.automaton_state != 0 || node.product.is_some() {
                return Err(unsupported("nodes"));
            }
            nodes.push(s.at_index("/nodes", index, |s| {
                let dim = node.zone.dim();
                dim.checked_mul(dim)
                    .ok_or_else(|| s.carrier_refusal("zone matrix dimension"))?;
                let mut rows = s.array(dim)?;
                for row in 0..dim {
                    let mut cells = s.array(dim)?;
                    for column in 0..dim {
                        // Matrix visitation is charged; existing Zone scalar
                        // extraction/arithmetic keeps its legacy work policy.
                        s.work(1)?;
                        cells.push(
                            match node
                                .zone
                                .bound(row, column)
                                .map_err(|_| s.carrier_refusal("zone bound"))?
                            {
                                Some((bound, strict)) => {
                                    let bound = s.integer(&bound)?;
                                    s.record([("bound", bound), ("strict", Value::Bool(strict))])?
                                }
                                None => s.text("infinity")?,
                            },
                        );
                    }
                    rows.push(Value::Array(cells));
                }
                let state = s.state(&node.state_key)?;
                let mut out = s.record([
                    ("automaton", Value::from(0)),
                    ("state", state),
                    ("zone", Value::Array(rows)),
                ])?;
                if node.locked {
                    s.put(&mut out, "locked", Value::Bool(true))?;
                }
                Ok::<_, ReferenceCodecError>(out)
            })?);
        }
        let mut edges = s.array(self.edges.len())?;
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.silent.is_some()
                || (source.is_none() && (edge.part.is_some() || edge.horizon.is_some()))
            {
                return Err(unsupported("edges"));
            }
            let transition = s.at_index("/edges", index, |s| {
                s.at_pointer("/transition", |s| {
                    match (&edge.transition, &edge.task_transition) {
                        (Some(transition), None) => Ok(s.encode_task_transition(transition)?),
                        (None, Some(template)) => Ok(s.encode_task_template(template)?),
                        (None, None) if source.is_some() && edge.part.is_none() => Ok(Value::Null),
                        _ => Err(unsupported("edges")),
                    }
                })
            })?;
            let mut out = s.record([
                ("node", Value::from(edge.from)),
                ("target", Value::from(edge.to)),
                ("transition", transition),
            ])?;
            if let Some(part) = edge.part {
                s.put(&mut out, "part", Value::from(part))?;
            }
            if let Some(horizon) = &edge.horizon {
                let mut constraints = s.array(horizon.len())?;
                for constraint in horizon {
                    let operator = s.text(op_text(constraint.op()))?;
                    let bound = s.rational(constraint.bound())?;
                    constraints.push(s.record([
                        ("place", Value::from(constraint.place())),
                        ("operator", operator),
                        ("bound", bound),
                    ])?);
                }
                s.put(&mut out, "horizon", Value::Array(constraints))?;
            }
            edges.push(out);
        }
        let mut initials = s.array(self.initials.len())?;
        for initial in &self.initials {
            s.work(1)?;
            initials.push(Value::from(*initial));
        }
        let scale = s.integer(&self.scale)?;
        let mut out = s.record([
            ("identity", identity),
            ("scale", scale),
            ("bounds", Value::Array(bounds)),
            ("nodes", Value::Array(nodes)),
            ("edges", Value::Array(edges)),
            ("initials", Value::Array(initials)),
        ])?;
        if let Some(witness) = &self.admitted {
            let cycle = match &witness.target {
                DivergentTarget::Cycle(cycle) => cycle.as_slice(),
                DivergentTarget::Quiescent(_) => &[],
                DivergentTarget::Terminal(_) => return Err(unsupported("admitted")),
            };
            let len = witness
                .path
                .len()
                .checked_add(cycle.len())
                .ok_or(fsl_eval::simulation::model::ModelKeyError::SizeOverflow)
                .map_err(ZoneCertificateWireError::from)?;
            let mut edges = s.array(len)?;
            for edge in witness.path.iter().chain(cycle) {
                s.work(1)?;
                edges.push(Value::from(*edge));
            }
            let admitted = s.record([
                ("start", Value::from(witness.start)),
                ("edges", Value::Array(edges)),
            ])?;
            s.put(&mut out, "admitted", admitted)?;
        }
        if let Some(member) = &self.task_product {
            let member = s.at_pointer("/task_product", |s| member.to_wire_with_references(s))?;
            s.put(&mut out, "task_product", member)?;
        }
        Ok(out)
    }
}
