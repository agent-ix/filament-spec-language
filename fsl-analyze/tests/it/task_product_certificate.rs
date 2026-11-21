// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (S4 of QSL-2240): the checker of a fixed-work task-product
//! certificate recomputes the member, the task automaton and the closed
//! refined-region quotient itself, and rejects each tampered certificate at
//! the member or node the vectors ZC-18..ZC-30 name.
//!
//! The certificates here are built from the checker's own quotient
//! (`explore_task_product`), so a mistake the checker's DBM and the
//! producer's would share is invisible to them: the independent producer
//! engine of S5 removes that common mode. What these tests do establish is
//! that the checker rejects every way a certificate can differ from the
//! recomputation.

use std::collections::{BTreeMap, VecDeque};

use filament_exact::{Cancel, Integer};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_eval::simulation::model::{
    TaskAutomaton, TaskProductSystem, TimedSystem, DEFAULT_MAX_CLOCKS,
};
use fsl_replay::{
    check_zone_certificate, explore_task_product, recomputed_member, CertEdge, CertNode,
    CertificateCheckLimits, CertificateItem, CertificateLocus, CertificateMember,
    CertificateRule, CertifiedItem, DivergenceWitness, DivergentTarget, QuotientIdentity,
    QuotientStop, TaskProductQuotient, Zone, ZoneCertificate, ZoneCertificateRefusal,
    ZoneCertificateRequest, ZoneCertificateVerdict,
};
use ix_trace_rs::trace;

use crate::task_automaton_crosscheck::world_for;
use crate::timed_world::World;

type Certificate = ZoneCertificate<OccurrenceKey>;

fn claim_of<'a>(world: &'a World, name: &str) -> &'a fsl_semantics::check::CheckedScheduleClaim {
    world
        .package
        .graph()
        .task_sets()
        .claims()
        .iter()
        .find(|claim| claim.name == name)
        .expect("the claim")
}

/// The entrywise upper bound of two canonical zones: a convex zone holding
/// both.
fn hull(left: &Zone, right: &Zone) -> Zone {
    let dim = left.dim();
    let mut cells = Vec::new();
    for i in 0..dim {
        for j in 0..dim {
            let a = left.bound(i, j).expect("bound");
            let b = right.bound(i, j).expect("bound");
            cells.push(match (a, b) {
                (Some(a), Some(b)) => Some(if (&a.0, !a.1) >= (&b.0, !b.1) { a } else { b }),
                _ => None,
            });
        }
    }
    Zone::from_matrix(dim, cells).expect("matrix")
}

struct Built {
    certificate: Certificate,
    quotient: TaskProductQuotient,
    /// The certificate edge of each distinct (node, identity, node) triple.
    edges: BTreeMap<(usize, usize, usize), usize>,
}

fn build(world: &World, claim_name: &str) -> Built {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim_name).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let cancel = Cancel::new();
    let mut digits = 0_u32;
    let (scale, quotient) = loop {
        let scale = Integer::from(10_u64.pow(digits));
        match explore_task_product(&automaton, &scale, 100_000_000, &cancel) {
            Ok(quotient) => break (scale, quotient),
            Err(QuotientStop::Scale) => digits += 1,
            Err(other) => panic!("the quotient does not build: {other:?}"),
        }
    };
    // One node per product key: the hull of its regions, in discovery order.
    let mut node_of_key = BTreeMap::new();
    let mut nodes: Vec<CertNode> = Vec::new();
    let mut node_of_vertex = Vec::new();
    for vertex in &quotient.vertices {
        let node = *node_of_key.entry(vertex.key.clone()).or_insert_with(|| {
            nodes.push(CertNode {
                state_key: vertex.key.clone(),
                automaton_state: 0,
                zone: vertex.zone.clone(),
                locked: false,
                product: None,
            });
            nodes.len() - 1
        });
        nodes[node].zone = hull(&nodes[node].zone, &vertex.zone);
        node_of_vertex.push(node);
    }
    let mut edges = BTreeMap::new();
    let mut list = Vec::new();
    for step in &quotient.discrete {
        let triple = (node_of_vertex[step.from], step.identity, node_of_vertex[step.to]);
        edges.entry(triple).or_insert_with(|| {
            let mut edge = CertEdge {
                from: triple.0,
                transition: None,
                task_transition: None,
                part: None,
                horizon: None,
                to: triple.2,
                silent: None,
            };
            match &quotient.identities[step.identity] {
                QuotientIdentity::Source(transition) => edge.transition = Some(transition.clone()),
                QuotientIdentity::Template(template) => {
                    edge.task_transition = Some(template.clone());
                }
            }
            list.push(edge);
            list.len() - 1
        });
    }
    // Certificate edges by source node, in discovery order.
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by_key(|&index| (list[index].from, index));
    let mut renumber = vec![0; list.len()];
    for (new, &old) in order.iter().enumerate() {
        renumber[old] = new;
    }
    let sorted: Vec<CertEdge> = order.iter().map(|&old| list[old].clone()).collect();
    for index in edges.values_mut() {
        *index = renumber[*index];
    }
    let initials = quotient
        .initials
        .iter()
        .map(|initial| node_of_key[&initial.key])
        .collect();
    let member = recomputed_member(&automaton, &scale, &quotient).expect("a finite product");
    let occurrence = claim_of(world, claim_name)
        .occurrence()
        .expect("the claim's occurrence");
    let mut certificate = ZoneCertificate {
        obligation: occurrence,
        scale,
        lu_bounds: fsl_replay::LuBounds {
            lower: Vec::new(),
            upper: Vec::new(),
        },
        nodes,
        edges: sorted,
        initials,
        components: None,
        non_zeno: None,
        divergence: None,
        admitted: None,
        state_graph: None,
        refinement: None,
        task_product: Some(member),
    };
    let built = Built {
        certificate: certificate.clone(),
        quotient,
        edges,
    };
    certificate.admitted = Some(admitted(&built, &node_of_vertex));
    Built {
        certificate,
        ..built
    }
}

/// The TD-6 witness: a path to a vertex where time idles forever, or to a
/// closed walk with a positive delay, over the quotient.
fn admitted(built: &Built, node_of_vertex: &[usize]) -> DivergenceWitness {
    let quotient = &built.quotient;
    let count = quotient.vertices.len();
    // (to, certificate edge or None for a delay)
    let mut out: Vec<Vec<(usize, Option<usize>)>> = vec![Vec::new(); count];
    for step in &quotient.discrete {
        let triple = (
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        );
        out[step.from].push((step.to, Some(built.edges[&triple])));
    }
    for &(from, to) in &quotient.time {
        out[from].push((to, None));
    }
    let initials: Vec<usize> = quotient
        .initials
        .iter()
        .flat_map(|initial| initial.vertices.iter().copied())
        .collect();
    let reach = |from: &[usize]| {
        let mut parent: Vec<Option<(usize, Option<usize>)>> = vec![None; count];
        let mut seen = vec![false; count];
        let mut queue = VecDeque::new();
        for &vertex in from {
            seen[vertex] = true;
            queue.push_back(vertex);
        }
        while let Some(vertex) = queue.pop_front() {
            for &(to, edge) in &out[vertex] {
                if !seen[to] {
                    seen[to] = true;
                    parent[to] = Some((vertex, edge));
                    queue.push_back(to);
                }
            }
        }
        (seen, parent)
    };
    let path_to = |parent: &[Option<(usize, Option<usize>)>], mut vertex: usize| {
        let mut path = Vec::new();
        while let Some((before, edge)) = parent[vertex] {
            path.extend(edge);
            vertex = before;
        }
        path.reverse();
        (vertex, path)
    };
    let (seen, parent) = reach(&initials);
    let start_node = |vertex: usize| {
        let (root, _) = path_to(&parent, vertex);
        node_of_vertex[root]
    };
    // A vertex where time idles forever.
    if let Some(idle) = (0..count).find(|&vertex| seen[vertex] && quotient.time.contains(&(vertex, vertex)))
    {
        let (_, path) = path_to(&parent, idle);
        return DivergenceWitness {
            start: start_node(idle),
            path,
            target: DivergentTarget::Quiescent(node_of_vertex[idle]),
        };
    }
    // A closed walk through a delay: `to` back to `from` over any edges.
    for &(from, to) in &quotient.time {
        if !seen[from] {
            continue;
        }
        let (back, back_parent) = reach(&[to]);
        if !back[from] {
            continue;
        }
        let (_, cycle) = path_to(&back_parent, from);
        let (_, path) = path_to(&parent, to);
        if cycle.is_empty() {
            continue;
        }
        return DivergenceWitness {
            start: start_node(to),
            path,
            target: DivergentTarget::Cycle(cycle),
        };
    }
    panic!(
        "the product admits no time-divergent behaviour: {} vertices, {} discrete, {} time, {} initials",
        count,
        quotient.discrete.len(),
        quotient.time.len(),
        quotient.initials.len()
    );
}

fn verdict(
    world: &World,
    claim_name: &str,
    certificate: &Certificate,
    limits: CertificateCheckLimits,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<OccurrenceKey>> {
    check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::Schedulability {
                claim: claim_of(world, claim_name),
            },
        },
        &certificate.obligation,
        |obligation: &OccurrenceKey| CertificateItem::Schedulability(obligation),
        certificate,
        limits,
        &Cancel::new(),
    )
}

fn rejection(
    world: &World,
    claim_name: &str,
    certificate: &Certificate,
) -> (CertificateRule, CertificateLocus) {
    match verdict(world, claim_name, certificate, CertificateCheckLimits::default()) {
        Ok(ZoneCertificateVerdict::Rejected(rejection)) => (rejection.rule, rejection.at),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

fn member_malformed(world: &World, claim: &str, certificate: &Certificate) {
    assert_eq!(
        rejection(world, claim, certificate),
        (
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::TaskProduct)
        )
    );
}

fn node_at(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: index as u64,
    }
}

const SOLO: &str = "taskset S {\n\
     task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
     }\n\
     schedulable SFp on S under fixed_priority;\n\
     schedulable SEdf on S under edf;\n";

fn solo() -> World {
    world_for("S", &["a"], SOLO)
}

/// Two tasks declared `b` then `a` with equal deadlines and no priorities.
fn tied() -> World {
    world_for(
        "E",
        &["b", "a"],
        "taskset E {\n\
         task b periodic 6 ms deadline 6 ms bcet 1 ms wcet 1 ms lo;\n\
         task a periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms lo;\n\
         }\n\
         schedulable EEdf on E under edf;\n",
    )
}

/// FR-418-AC-13: the checker accepts the periodic task certificate it
/// recomputes, closed modulo the rank/slot of completed jobs, at the default
/// work limit; each node is a union of refined regions the checker
/// established itself.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn the_recomputed_periodic_certificate_is_accepted() {
    for claim in ["SFp", "SEdf"] {
        let world = solo();
        let built = build(&world, claim);
        assert_eq!(
            verdict(&world, claim, &built.certificate, CertificateCheckLimits::default()),
            Ok(ZoneCertificateVerdict::Accepted),
            "{claim}"
        );
    }
}

