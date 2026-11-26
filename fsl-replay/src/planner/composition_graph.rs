// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044: bounded cycle rows with the source composition's edge ordering.

use filament_contract_model::{
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinLocus, NativeJoinLocusSource,
};
use fsl_foundation::source::provenance::SourceRegion;
use fsl_semantics::check::{CompositionGraphRefusal, composition_components};

/// An authenticated edge between declared guarantees, in rely-then-depends order.
pub(super) struct CompositionEdge<'a> {
    /// The guarantee carrying the premise.
    pub from: usize,
    /// The declared peer guarantee.
    pub to: usize,
    /// The actual authored premise region.
    pub source: &'a SourceRegion,
}

/// One cycle's declared guarantee positions and original edge provenance.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct CompositionCycleRows {
    /// Distinct tails of internal edges, in first internal-edge occurrence order.
    pub guarantees: Vec<usize>,
    /// The first internal edge at the minimum authored source start.
    pub source_edge: usize,
}

/// A cycle projection refusal, without allocating a diagnostic payload.
#[derive(Debug, thiserror::Error)]
pub(super) enum CompositionCycleRefusal {
    /// The shared SCC or native ledger's original refusal, including allocations.
    #[error(transparent)]
    Graph(#[from] CompositionGraphRefusal),
    /// An input edge names a position outside the declared guarantee rows.
    #[error("composition edge {edge} ({from}, {to}) exceeds {guarantee_count} guarantees")]
    InvalidEdge {
        /// Position in the actual input edge channel.
        edge: usize,
        /// Written tail position.
        from: usize,
        /// Written peer position.
        to: usize,
        /// Number of declared guarantee rows.
        guarantee_count: usize,
    },
    /// A retained cycle provenance position names no input edge.
    #[error("composition source edge {edge} exceeds {len} input edges")]
    SourceEdgeIndex {
        /// Retained edge position.
        edge: usize,
        /// Number of supplied edges.
        len: usize,
    },
}

/// Project source-equivalent cycle rows using the caller's cumulative ledger.
///
/// The caller supplies only declared-guarantee edges in authored rely-then-depends
/// order. The existing iterative Tarjan core supplies components. Internal-edge
/// tails retain authored order; minimum source starts select original edge rows.
/// Stable insertion retains Tarjan completion order when cycle starts tie.
/// Every inspected row, equality/order comparison, reserved buffer and moved
/// output entry is paid before its operation. No region or guarantee is cloned.
pub(super) fn composition_cycle_rows(
    guarantee_count: usize,
    edges: &[CompositionEdge<'_>],
    budget: &mut NativeJoinBudget,
) -> Result<Vec<CompositionCycleRows>, CompositionCycleRefusal> {
    let mut provision = CycleProvision { budget };
    // Admission also preserves an earlier caller denial for an empty graph.
    provision.work(1, None)?;
    for (edge_index, edge) in edges.iter().enumerate() {
        provision.work(1, Some(edge_index))?;
        provision.work(1, Some(edge_index))?;
        if edge.from >= guarantee_count {
            return Err(invalid_edge(edge_index, edge, guarantee_count));
        }
        provision.work(1, Some(edge_index))?;
        if edge.to >= guarantee_count {
            return Err(invalid_edge(edge_index, edge, guarantee_count));
        }
    }

    let mut successors = Vec::new();
    provision.room(&mut successors, guarantee_count, None)?;
    for _ in 0..guarantee_count {
        provision.work(1, None)?;
        successors.push(Vec::new());
    }
    for (edge_index, edge) in edges.iter().enumerate() {
        provision.work(1, Some(edge_index))?;
        let Some(row) = successors.get_mut(edge.from) else {
            return Err(invalid_edge(edge_index, edge, guarantee_count));
        };
        provision.room(row, 1, Some(edge_index))?;
        provision.work(1, Some(edge_index))?;
        row.push(edge.to);
    }
    let components = composition_components(&successors, provision.budget)?;
    let mut cycles: Vec<CompositionCycleRows> = Vec::new();
    for component in &components {
        provision.work(1, None)?;
        let mut guarantees = Vec::new();
        let mut source: Option<(usize, &SourceRegion)> = None;
        for (edge_index, edge) in edges.iter().enumerate() {
            provision.work(1, Some(edge_index))?;
            if !provision.contains(component, edge.from, edge_index)?
                || !provision.contains(component, edge.to, edge_index)?
            {
                continue;
            }
            if !provision.contains(&guarantees, edge.from, edge_index)? {
                provision.room(&mut guarantees, 1, Some(edge_index))?;
                provision.work(1, Some(edge_index))?;
                guarantees.push(edge.from);
            }
            match source {
                None => source = Some((edge_index, edge.source)),
                Some((_, first)) => {
                    provision.work(1, Some(edge_index))?;
                    if edge.source.start() < first.start() {
                        source = Some((edge_index, edge.source));
                    }
                }
            }
        }
        // A singleton without a self-edge has no internal edge or cycle.
        let Some((source_edge, source)) = source else {
            continue;
        };
        let mut destination = cycles.len();
        for previous in cycles.iter().rev() {
            provision.work(1, None)?;
            let previous_source = edges.get(previous.source_edge).ok_or(
                CompositionCycleRefusal::SourceEdgeIndex {
                    edge: previous.source_edge,
                    len: edges.len(),
                },
            )?;
            if previous_source.source.start() <= source.start() {
                break;
            }
            destination -= 1;
        }
        provision.room(&mut cycles, 1, None)?;
        // Vec::insert shifts exactly this suffix and writes one new entry.
        provision.work((cycles.len() - destination) as u128 + 1, None)?;
        cycles.insert(
            destination,
            CompositionCycleRows {
                guarantees,
                source_edge,
            },
        );
    }
    Ok(cycles)
}

fn invalid_edge(
    index: usize,
    edge: &CompositionEdge<'_>,
    guarantee_count: usize,
) -> CompositionCycleRefusal {
    CompositionCycleRefusal::InvalidEdge {
        edge: index,
        from: edge.from,
        to: edge.to,
        guarantee_count,
    }
}

struct CycleProvision<'a> {
    budget: &'a mut NativeJoinBudget,
}

impl CycleProvision<'_> {
    fn charge(
        &mut self,
        kind: NativeJoinChargeKind,
        amount: u128,
        edge: Option<usize>,
    ) -> Result<(), CompositionCycleRefusal> {
        let locus = NativeJoinLocus {
            source: edge.map_or(
                NativeJoinLocusSource::Package,
                NativeJoinLocusSource::AdapterRow,
            ),
            form: "composition_cycle_rows",
        };
        self.budget
            .accounted_charge(kind, amount, locus)
            .map_err(|denied| {
                CompositionGraphRefusal::Budget {
                    denied,
                    locus: self.budget.first_unpaid_locus(),
                    configured_bound: match denied.kind {
                        NativeJoinChargeKind::Work => self.budget.work_limit(),
                        NativeJoinChargeKind::Storage => self.budget.storage_limit(),
                    },
                }
                .into()
            })
    }

    fn work(&mut self, amount: u128, edge: Option<usize>) -> Result<(), CompositionCycleRefusal> {
        self.charge(NativeJoinChargeKind::Work, amount, edge)
    }

    fn room<T>(
        &mut self,
        list: &mut Vec<T>,
        additional: usize,
        edge: Option<usize>,
    ) -> Result<(), CompositionCycleRefusal> {
        let needed = list.len() as u128 + additional as u128;
        if needed > list.capacity() as u128 {
            let target = needed.max(list.capacity().saturating_mul(2) as u128);
            self.charge(NativeJoinChargeKind::Storage, 1 + target, edge)?;
            list.try_reserve_exact(usize::try_from(target).unwrap_or(usize::MAX) - list.len())
                .map_err(CompositionGraphRefusal::Allocation)?;
        }
        Ok(())
    }

    fn contains(
        &mut self,
        members: &[usize],
        wanted: usize,
        edge: usize,
    ) -> Result<bool, CompositionCycleRefusal> {
        for member in members {
            self.work(1, Some(edge))?;
            if *member == wanted {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
#[cfg(test)]
mod cycle_rows_tests {
    use super::*;
    use filament_contract_model::NativeJoinDeniedCharge;
    use fsl_foundation::source::provenance::RawSourceRef;

    fn ledger(work: u64, storage: u64) -> NativeJoinBudget {
        let mut ledger = NativeJoinBudget::new(work, storage);
        ledger.resume();
        ledger
    }

    fn region(start: u64, end: u64) -> SourceRegion {
        SourceRegion::new(
            RawSourceRef::new("fs", "composition.fsl").unwrap(),
            start,
            end,
        )
        .unwrap()
    }

    fn edge(from: usize, to: usize, source: &SourceRegion) -> CompositionEdge<'_> {
        CompositionEdge { from, to, source }
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_keep_authored_tails_and_first_minimum_source_with_stable_cycle_ties() {
        let regions = [
            region(50, 54),
            region(10, 14),
            region(0, 4),
            region(10, 100),
            region(10, 11),
            region(10, 15),
            region(5, 9),
            region(1, 5),
        ];
        let edges = [
            edge(1, 0, &regions[0]),
            edge(0, 1, &regions[1]),
            edge(1, 2, &regions[2]),
            edge(3, 2, &regions[3]),
            edge(2, 3, &regions[4]),
            edge(3, 2, &regions[5]),
            edge(4, 4, &regions[6]),
            edge(5, 6, &regions[7]),
        ];
        // The 2/3 component completes before 0/1 through the cross-edge.
        // Cross-edge start 0 is not internal. Equal starts retain SCC order;
        // equal minimum edge starts retain the first authored edge, not its end.
        assert_eq!(
            composition_cycle_rows(8, &edges, &mut ledger(10_000, 10_000)).unwrap(),
            vec![
                CompositionCycleRows {
                    guarantees: vec![4],
                    source_edge: 6
                },
                CompositionCycleRows {
                    guarantees: vec![3, 2],
                    source_edge: 3
                },
                CompositionCycleRows {
                    guarantees: vec![1, 0],
                    source_edge: 1
                },
            ]
        );
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_sort_by_actual_source_start_and_filter_empty_or_acyclic_components() {
        let regions = [region(30, 31), region(20, 21), region(10, 11)];
        let edges = [
            edge(0, 0, &regions[0]),
            edge(1, 1, &regions[1]),
            edge(2, 2, &regions[2]),
        ];
        assert_eq!(
            composition_cycle_rows(3, &edges, &mut ledger(10_000, 10_000)).unwrap(),
            vec![
                CompositionCycleRows {
                    guarantees: vec![2],
                    source_edge: 2
                },
                CompositionCycleRows {
                    guarantees: vec![1],
                    source_edge: 1
                },
                CompositionCycleRows {
                    guarantees: vec![0],
                    source_edge: 0
                },
            ]
        );
        assert_eq!(
            composition_cycle_rows(0, &[], &mut ledger(10_000, 10_000)).unwrap(),
            vec![]
        );
        assert_eq!(
            composition_cycle_rows(3, &[], &mut ledger(10_000, 10_000)).unwrap(),
            vec![]
        );
        let chain = [edge(0, 1, &regions[0]), edge(1, 2, &regions[1])];
        assert_eq!(
            composition_cycle_rows(4, &chain, &mut ledger(10_000, 10_000)).unwrap(),
            vec![]
        );
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_refuse_invalid_input_indices_before_allocating_adjacency() {
        let source = region(0, 1);
        for (count, from, to) in [(0, 0, 0), (2, usize::MAX, 1), (2, 0, usize::MAX)] {
            let edges = [edge(from, to, &source)];
            let mut budget = ledger(10_000, 10_000);
            assert!(matches!(composition_cycle_rows(count, &edges, &mut budget),
                Err(CompositionCycleRefusal::InvalidEdge {
                    edge: 0, from: actual_from, to: actual_to, guarantee_count,
                }) if actual_from == from && actual_to == to && guarantee_count == count));
            assert_eq!(budget.consumed().storage, 0);
            assert_eq!(budget.first_unpaid(), None);
        }
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_exact_short_and_zero_limits_keep_original_native_denial_metadata() {
        let regions = [region(30, 31), region(20, 21), region(10, 11)];
        let edges = [
            edge(0, 0, &regions[0]),
            edge(1, 1, &regions[1]),
            edge(2, 2, &regions[2]),
        ];
        let mut measured = ledger(10_000, 10_000);
        let expected = composition_cycle_rows(3, &edges, &mut measured).unwrap();
        let paid = measured.consumed();
        let mut exact = ledger(paid.work, paid.storage);
        assert_eq!(
            composition_cycle_rows(3, &edges, &mut exact).unwrap(),
            expected
        );
        assert_eq!(exact.consumed(), paid);
        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            for bound in [
                0,
                match kind {
                    NativeJoinChargeKind::Work => paid.work - 1,
                    NativeJoinChargeKind::Storage => paid.storage - 1,
                },
            ] {
                let mut short = ledger(
                    if kind == NativeJoinChargeKind::Work {
                        bound
                    } else {
                        paid.work
                    },
                    if kind == NativeJoinChargeKind::Storage {
                        bound
                    } else {
                        paid.storage
                    },
                );
                let refusal = composition_cycle_rows(3, &edges, &mut short).unwrap_err();
                let CompositionCycleRefusal::Graph(CompositionGraphRefusal::Budget {
                    denied,
                    locus,
                    configured_bound,
                }) = refusal
                else {
                    panic!("the native owner retains its budget refusal");
                };
                assert_eq!(denied.kind, kind);
                assert_eq!(Some(denied), short.first_unpaid());
                assert_eq!(configured_bound, bound);
                assert_eq!(locus, short.first_unpaid_locus());
                assert!(denied.amount > 0);
                if kind == NativeJoinChargeKind::Work && bound == 0 {
                    assert_eq!(short.consumed().storage, 0);
                    assert_eq!(denied.amount, 1);
                }
            }
        }
        let mut reused = ledger(paid.work * 2, paid.storage * 2 - 1);
        composition_cycle_rows(3, &edges, &mut reused).unwrap();
        assert!(matches!(
            composition_cycle_rows(3, &edges, &mut reused),
            Err(CompositionCycleRefusal::Graph(
                CompositionGraphRefusal::Budget {
                    denied: NativeJoinDeniedCharge {
                        kind: NativeJoinChargeKind::Storage,
                        ..
                    },
                    ..
                }
            ))
        ));
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_preserve_an_earlier_denial_even_when_input_is_empty_or_malformed() {
        let source = region(0, 1);
        let malformed = [edge(usize::MAX, 0, &source)];
        for edges in [&[][..], &malformed[..]] {
            let mut budget = ledger(7, 3);
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::ReachedRow(37),
                form: "composition_selection",
            };
            let original = budget
                .accounted_charge(NativeJoinChargeKind::Storage, 4, locus)
                .unwrap_err();
            let before = budget.consumed();
            assert!(matches!(composition_cycle_rows(0, edges, &mut budget),
                Err(CompositionCycleRefusal::Graph(CompositionGraphRefusal::Budget {
                    denied, locus: Some(actual), configured_bound: 3,
                })) if denied == original && actual == locus));
            assert_eq!(budget.consumed(), before);
        }
    }

    /// Trace: FR-1044-AC-6
    #[test]
    fn cycle_rows_oversized_count_retains_a_distinct_paid_allocation_refusal() {
        // This capacity cannot fit Vec headers in an address space. Its finite
        // logical allowance pays exactly the requested reservation, before the
        // standard library refuses its physical capacity without allocating.
        let count = usize::MAX / core::mem::size_of::<Vec<usize>>() + 1;
        let storage = u64::try_from(count).unwrap().checked_add(1).unwrap();
        let mut budget = ledger(1, storage);
        assert!(matches!(
            composition_cycle_rows(count, &[], &mut budget),
            Err(CompositionCycleRefusal::Graph(
                CompositionGraphRefusal::Allocation(_)
            ))
        ));
        assert_eq!(budget.consumed().work, 1);
        assert_eq!(budget.consumed().storage, storage);
        assert_eq!(budget.first_unpaid(), None);
    }
}
