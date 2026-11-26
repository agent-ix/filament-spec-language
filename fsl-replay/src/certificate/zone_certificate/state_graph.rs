// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-245 over FSpec FR-712's state-graph forms.
//!
//! The forms speak of timed states, and a certificate node holds many, while
//! an edge says only that some valuation of its source can take it, into a
//! target that may be wider than where it lands. So neither node zones nor
//! edges decide a form here. After the coverage checks, the checker explores
//! the timed states itself, from the initial states, under the aM bounds, and
//! reads each form on the valuations:
//!
//! - `E(P)`, the valuations with a run to a `P` state, is its own least
//!   fixpoint of delay-and-step predecessors over the model states;
//! - `possible`: each initial valuation is in `E(P)`;
//! - `always possible`: the listed `ef_nodes` are the nodes whose whole zone
//!   is in `E(P)`, and every explored `Q` valuation is in `E(P)`;
//! - `unique path`: every `X` valuation reached is in `E(Y)`, and no two
//!   transition sequences, read backwards from `Y`, lead from one of them.
//!
//! Every zone held, every predecessor or successor computed, every piece a
//! zone subtraction makes and every pair of zones intersected is charged to
//! `max_certificate_edges`.

use std::collections::{BTreeMap, VecDeque};

use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{GuardedStep, ModelSimState, PlaceConstraint};
use fsl_eval::simulation::StateKey;
use fsl_semantics::check::{PredicateRole, StateGraphForm};
use ix_trace_rs::implements;

use super::{
    at_node, fault, intersect, malformed, reject, CertificateLocus, CertificateMember,
    CertificateRule, Checker, CheckerFault, Halt, Step, ValueType, ZoneCertificateRefusal,
};
use crate::certificate::zone::Zone;

/// A model state the checker's exploration reached.
struct Location {
    state: ModelSimState,
    invariants: Vec<PlaceConstraint>,
    elapses: bool,
}

/// One step of a location through one convex part of its guard.
struct Arc {
    from: usize,
    to: usize,
    step: GuardedStep,
    part: usize,
}

/// A zone of a location.
type Region = (usize, Zone);

/// The locations reached and the steps between them.
#[derive(Default)]
struct Space {
    locations: Vec<Location>,
    index: BTreeMap<StateKey, usize>,
    arcs: Vec<Arc>,
    /// The arcs leaving each location, once computed.
    leaving: Vec<Option<Vec<usize>>>,
}

impl<O> Checker<'_, '_, O> {
    /// FSpec FR-712's form of the state-graph claim, decided on the timed
    /// states.
    #[implements(
        "FR-852-AC-32",
        "FSpec-FR-712-AC-1",
        "FSpec-FR-712-AC-2",
        "FSpec-FR-712-AC-3",
        "FSpec-FR-712-AC-10"
    )]
    pub(super) fn state_graph(&self) -> Step<(), O> {
        let Some(state_graph) = &self.state_graph else {
            return malformed(CertificateMember::Nodes);
        };
        let claim = state_graph.claim;
        match (claim.over(), state_graph.over) {
            (None, None) => {}
            (Some(parameter), Some(reference)) => {
                let instances = self
                    .timed
                    .system()
                    .instances(&ValueType::Reference(parameter.object.clone()))
                    .map_err(|_| Halt::Refusal(ZoneCertificateRefusal::Undecided))?;
                let bound = instances.iter().any(|instance| {
                    self.timed
                        .system()
                        .written_reference(parameter.object.clone(), instance)
                        .is_ok_and(|candidate| candidate == *reference)
                });
                if !bound {
                    return Err(Halt::Refusal(ZoneCertificateRefusal::Undecided));
                }
            }
            _ => return Err(Halt::Refusal(ZoneCertificateRefusal::Undecided)),
        }
        let (target, from) = match claim.form() {
            StateGraphForm::Possible { target } => (target, None),
            StateGraphForm::AlwaysPossible { from, target } => (target, Some(from)),
            StateGraphForm::UniquePath { from, to } => (to, Some(from)),
        };
        let target_values =
            self.predicate_values(target, PredicateRole::Target, state_graph.over)?;
        let from_values = from
            .map(|from| self.predicate_values(from, PredicateRole::From, state_graph.over))
            .transpose()?;

        let mut space = Space::default();
        let mut points = Vec::new();
        let mut seeds = Vec::new();
        for start in self.timed.initial() {
            self.poll()?;
            let location = self.locate(&mut space, start.discrete())?;
            let mut point = Zone::zero(self.dim());
            self.constrain(&mut point, &space.locations[location].invariants)?;
            if point.is_empty() {
                continue;
            }
            let mut zone = point.clone();
            self.settle(&space.locations[location].state, &mut zone)?;
            points.push((location, point));
            seeds.push((location, zone));
        }
        let reached = self.explore(&mut space, seeds)?;

        // The certificate node that names each location. A step whose clock
        // guard never holds can name a location nothing reaches, which no
        // node names and no form reads; the coverage checks gave every
        // reached one a node.
        let mut names = vec![None; space.locations.len()];
        let mut at_nodes = vec![None; self.certificate.nodes.len()];
        for (node, state) in self.states.iter().enumerate() {
            let Some(state) = state else {
                continue;
            };
            if let Some(&location) = space.index.get(&self.state_key(state)?) {
                at_nodes[node] = Some(location);
                names[location].get_or_insert(node);
            }
        }
        if reached
            .iter()
            .any(|(location, _)| names[*location].is_none())
        {
            return Err(fault(CheckerFault::UnreachedNode));
        }
        let per_location = |values: &[bool]| {
            names
                .iter()
                .map(|node| node.is_some_and(|node| values[node]))
                .collect::<Vec<_>>()
        };
        let name = |location: usize| at_node(names[location].unwrap_or_default());
        let target_values = per_location(&target_values);
        let eventually = self.eventually(&space, &target_values)?;

        match claim.form() {
            StateGraphForm::Possible { .. } => {
                for (location, point) in &points {
                    if !self.outside(point, &eventually[*location])?.is_empty() {
                        return reject(
                            CertificateRule::InitialMissing,
                            CertificateLocus::CertificateMember(CertificateMember::Initials),
                        );
                    }
                }
                Ok(())
            }
            StateGraphForm::AlwaysPossible { .. } => {
                let Some(part) = &self.certificate.state_graph else {
                    return malformed(CertificateMember::Nodes);
                };
                let mut expected = Vec::new();
                for (node, location) in at_nodes.iter().enumerate() {
                    let Some(location) = location else {
                        continue;
                    };
                    self.poll()?;
                    let zone = &self.certificate.nodes[node].zone;
                    if self.outside(zone, &eventually[*location])?.is_empty() {
                        expected.push(node);
                    }
                }
                if part.ef_nodes != expected {
                    return reject(CertificateRule::RankBroken, at_node(0));
                }
                let from =
                    per_location(&from_values.ok_or_else(|| fault(CheckerFault::Automaton))?);
                for (location, zone) in &reached {
                    if from[*location] && !self.outside(zone, &eventually[*location])?.is_empty() {
                        return reject(CertificateRule::SuccessorMissing, name(*location));
                    }
                }
                Ok(())
            }
            StateGraphForm::UniquePath { .. } => {
                let from =
                    per_location(&from_values.ok_or_else(|| fault(CheckerFault::Automaton))?);
                let sources: Vec<Region> = reached
                    .into_iter()
                    .filter(|(location, _)| from[*location])
                    .collect();
                for (location, zone) in &sources {
                    if !self.outside(zone, &eventually[*location])?.is_empty() {
                        return reject(CertificateRule::CountBroken, name(*location));
                    }
                }
                match self.second_sequence(&mut space, &sources, &target_values)? {
                    Some(location) => reject(CertificateRule::CountBroken, name(location)),
                    None => Ok(()),
                }
            }
        }
    }

    /// The index of `state`, adding it when new.
    fn locate(&self, space: &mut Space, state: &ModelSimState) -> Step<usize, O> {
        let key = self.state_key(state)?;
        if let Some(&location) = space.index.get(&key) {
            return Ok(location);
        }
        let location = space.locations.len();
        space.index.insert(key, location);
        space.locations.push(Location {
            state: state.clone(),
            invariants: self.invariants(state)?,
            elapses: self.elapses(state)?,
        });
        space.leaving.push(None);
        Ok(location)
    }

    /// The arcs leaving `location`, computing them and their targets once.
    fn departures(&self, space: &mut Space, location: usize) -> Step<Vec<usize>, O> {
        if let Some(arcs) = &space.leaving[location] {
            return Ok(arcs.clone());
        }
        let steps = self
            .timed
            .guarded_steps(&space.locations[location].state)
            .map_err(|_| Halt::Refusal(ZoneCertificateRefusal::Undecided))?;
        let mut arcs = Vec::new();
        for step in steps {
            let to = self.locate(space, step.post())?;
            for part in 0..step.parts().len() {
                arcs.push(space.arcs.len());
                space.arcs.push(Arc {
                    from: location,
                    to,
                    step: step.clone(),
                    part,
                });
            }
        }
        space.leaving[location] = Some(arcs.clone());
        Ok(arcs)
    }

    /// The valuations `arc` reaches from `zone`, closed under the delays its
    /// target allows.
    fn image(&self, space: &Space, arc: usize, zone: &Zone) -> Step<Zone, O> {
        self.charge(1)?;
        let arc = &space.arcs[arc];
        let target = &space.locations[arc.to];
        let mut next = zone.clone();
        self.constrain(&mut next, &arc.step.parts()[arc.part])?;
        if !next.is_empty() {
            self.apply_clock_updates(&mut next, &arc.step)?;
            self.constrain(&mut next, &target.invariants)?;
        }
        if !next.is_empty() {
            self.settle(&target.state, &mut next)?;
        }
        Ok(next)
    }

    /// The valuations from which a delay and then `arc` reach `landing`.
    fn origin(&self, space: &Space, arc: usize, landing: &Zone) -> Step<Zone, O> {
        self.charge(1)?;
        let arc = &space.arcs[arc];
        let source = &space.locations[arc.from];
        let mut landing = landing.clone();
        self.constrain(&mut landing, &space.locations[arc.to].invariants)?;
        let mut zone = self.preimage_clock_updates(&landing, &arc.step)?;
        self.constrain(&mut zone, &arc.step.parts()[arc.part])?;
        self.constrain(&mut zone, &source.invariants)?;
        if source.elapses && !zone.is_empty() {
            zone.down();
            self.constrain(&mut zone, &source.invariants)?;
        }
        Ok(zone)
    }

    /// The zones reachable from `seeds`, breadth-first, each held unless the
    /// aM bounds cover it by one already held at its location. Every
    /// reachable valuation is bisimilar to one of these, and each of these
    /// is reachable.
    fn explore(&self, space: &mut Space, seeds: Vec<Region>) -> Step<Vec<Region>, O> {
        let cover = self.cover_for_zone(self.dim())?;
        let mut reached: Vec<Region> = Vec::new();
        let mut held: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut pending: VecDeque<Region> = seeds.into();
        let mut next = 0;
        loop {
            while let Some((location, zone)) = pending.pop_front() {
                if zone.is_empty() {
                    continue;
                }
                self.charge(1)?;
                let at = held.entry(location).or_default();
                let mut covered = false;
                for &other in at.iter() {
                    if zone.alu_covered_by(&reached[other].1, &cover)? {
                        covered = true;
                        break;
                    }
                }
                if !covered {
                    at.push(reached.len());
                    reached.push((location, zone));
                }
            }
            let Some((location, zone)) = reached.get(next).cloned() else {
                return Ok(reached);
            };
            next += 1;
            for arc in self.departures(space, location)? {
                let image = self.image(space, arc, &zone)?;
                pending.push_back((space.arcs[arc].to, image));
            }
        }
    }

    /// `E(P)` per location: from the `P` states, add each arc's origin of
    /// each zone until no arc has a zone left to read. It ends because an
    /// origin of a union of regions is one.
    fn eventually(&self, space: &Space, target: &[bool]) -> Step<Vec<Vec<Zone>>, O> {
        let mut found: Vec<Vec<Zone>> = vec![Vec::new(); space.locations.len()];
        for (location, _) in target.iter().enumerate().filter(|(_, holds)| **holds) {
            let mut zone = Zone::universe(self.dim());
            self.constrain(&mut zone, &space.locations[location].invariants)?;
            if !zone.is_empty() {
                found[location].push(zone);
            }
        }
        let mut read = vec![0_usize; space.arcs.len()];
        loop {
            let mut grew = false;
            for (arc, read) in read.iter_mut().enumerate() {
                let (from, to) = (space.arcs[arc].from, space.arcs[arc].to);
                while let Some(landing) = found[to].get(*read).cloned() {
                    *read += 1;
                    let origin = self.origin(space, arc, &landing)?;
                    if origin.is_empty() {
                        continue;
                    }
                    for piece in self.outside(&origin, &found[from])? {
                        self.charge(1)?;
                        found[from].push(piece);
                        grew = true;
                    }
                }
            }
            if !grew {
                return Ok(found);
            }
        }
    }

    /// A location of `sources` with a valuation from which two transition
    /// sequences lead to a `target` state.
    ///
    /// Each sequence is held as the valuations it leads from, shortest first.
    /// Only a sequence that meets a state reachable from `sources` is
    /// extended: while none share a source valuation they are disjoint on
    /// those states, so they are finitely many.
    fn second_sequence(
        &self,
        space: &mut Space,
        sources: &[Region],
        target: &[bool],
    ) -> Step<Option<usize>, O> {
        let onward = self.explore(space, sources.to_vec())?;
        let mut first = Vec::new();
        for (location, _) in target.iter().enumerate().filter(|(_, holds)| **holds) {
            let mut zone = Zone::universe(self.dim());
            self.constrain(&mut zone, &space.locations[location].invariants)?;
            first.push((location, zone));
        }
        let mut sequences: VecDeque<Vec<Region>> = VecDeque::from([first]);
        // The source valuations some sequence already leads from.
        let mut taken: Vec<Region> = Vec::new();
        while let Some(sequence) = sequences.pop_front() {
            self.poll()?;
            let mine = self.shared(&sequence, sources)?;
            if let Some((location, _)) = self.shared(&mine, &taken)?.first() {
                return Ok(Some(*location));
            }
            self.charge(mine.len() as u64)?;
            taken.extend(mine);
            let mut longer: BTreeMap<&ModelTransition, Vec<Region>> = BTreeMap::new();
            for (arc, edge) in space.arcs.iter().enumerate() {
                for (_, zone) in sequence.iter().filter(|(location, _)| *location == edge.to) {
                    let origin = self.origin(space, arc, zone)?;
                    if origin.is_empty() {
                        continue;
                    }
                    let regions = longer.entry(edge.step.transition()).or_default();
                    let held: Vec<Zone> = regions
                        .iter()
                        .filter(|(location, _)| *location == edge.from)
                        .map(|(_, zone)| zone.clone())
                        .collect();
                    for piece in self.outside(&origin, &held)? {
                        self.charge(1)?;
                        regions.push((edge.from, piece));
                    }
                }
            }
            for regions in longer.into_values() {
                if !self.shared(&regions, &onward)?.is_empty() {
                    sequences.push_back(regions);
                }
            }
        }
        Ok(None)
    }

    /// [`outside`], charged to `max_certificate_edges`.
    #[implements("FR-852-AC-33")]
    fn outside(&self, zone: &Zone, held: &[Zone]) -> Step<Vec<Zone>, O> {
        outside(zone, held, &|count| self.charge(count))
    }

    /// The nonempty intersections of `left` with `right`, location by
    /// location, each pair charged.
    #[implements("FR-852-AC-33")]
    fn shared(&self, left: &[Region], right: &[Region]) -> Step<Vec<Region>, O> {
        let mut both = Vec::new();
        for (location, zone) in left {
            for (_, other) in right.iter().filter(|(other, _)| other == location) {
                self.charge(1)?;
                let mut zone = zone.clone();
                intersect(&mut zone, other)?;
                if !zone.is_empty() {
                    both.push((*location, zone));
                }
            }
        }
        Ok(both)
    }
}

/// The parts of `zone` that no zone of `held` holds.
///
/// `charge` is called with zero before each pair of a held zone and a piece
/// is read, to poll, and with the number of pieces each split makes; its
/// error stops the subtraction.
#[implements("FR-852-AC-33")]
fn outside<O>(
    zone: &Zone,
    held: &[Zone],
    charge: &impl Fn(u64) -> Step<(), O>,
) -> Step<Vec<Zone>, O> {
    let mut rest = Vec::new();
    if !zone.is_empty() {
        rest.push(zone.clone());
    }
    for other in held {
        if rest.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for piece in rest {
            charge(0)?;
            let mut meet = piece.clone();
            intersect(&mut meet, other)?;
            if meet.is_empty() {
                next.push(piece);
                continue;
            }
            let made = piece.minus(other)?;
            charge(u64::try_from(made.len()).unwrap_or(u64::MAX))?;
            next.extend(made);
        }
        rest = next;
    }
    Ok(rest)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use filament_exact::Integer;
    use ix_trace_rs::trace;

    use super::{outside, Halt, Step};
    use crate::certificate::zone::Zone;
    use crate::certificate::zone_certificate::ZoneCertificateRefusal;

    /// `low <= x <= low + 1` over one clock.
    fn unit(low: i64) -> Zone {
        let mut zone = Zone::universe(2);
        zone.constrain(1, 0, &Integer::from(low + 1), false)
            .expect("an upper bound");
        zone.constrain(0, 1, &Integer::from(-low), false)
            .expect("a lower bound");
        zone
    }

    /// A budget of `limit` pieces, stopping as the checker does.
    fn budget(limit: u64) -> impl Fn(u64) -> Step<(), ()> {
        let spent = Cell::new(0_u64);
        move |count| {
            spent.set(spent.get() + count);
            if spent.get() > limit {
                return Err(Halt::Refusal(ZoneCertificateRefusal::Undecided));
            }
            Ok(())
        }
    }

    #[trace("FR-852-AC-33")]
    #[test]
    fn a_wide_union_is_subtracted_within_its_budget() {
        // 20,000 held zones, none meeting the point: nothing is split.
        let held: Vec<Zone> = (0..20_000).map(|index| unit(2 * index + 1)).collect();
        let point = Zone::zero(2);
        let rest = outside::<()>(&point, &held, &budget(0));
        assert!(matches!(rest, Ok(parts) if parts == vec![point.clone()]));
        // 12,000 held zones that each split the rest stop at the budget.
        let held: Vec<Zone> = (0..12_000).map(|index| unit(2 * index + 1)).collect();
        let rest = outside(&Zone::universe(2), &held, &budget(200));
        assert!(matches!(rest, Err(Halt::Refusal(_))));
    }
}
