// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker's bounded subtraction, diagonal guard and refined regions
//! (FR-418) against hand-computed vectors and against point-wise
//! definitions on seeded random zones. The engine's DBM in `fsl-analyze` is
//! not a dependency of this crate, so its `subtract_bounded` results are
//! reproduced here as hand-computed vectors (cases 1 to 3 below).

use ix_trace_rs::trace;

use super::reference;
use super::{Coverage, DiagonalPredicate, Integer, TargetShape, Zone, ZoneError};

fn int(value: i64) -> Integer {
    Integer::from(value)
}

/// The universe over `dim - 1` clocks cut by `(i, j, c, strict)` bounds.
fn zone(dim: usize, bounds: &[(usize, usize, i64, bool)]) -> Zone {
    let mut zone = Zone::universe(dim);
    for &(i, j, c, strict) in bounds {
        zone.constrain(i, j, &int(c), strict).expect("in range");
    }
    zone
}

fn bound(zone: &Zone, i: usize, j: usize) -> Option<(i64, bool)> {
    zone.bound(i, j)
        .expect("in range")
        .map(|(c, strict)| (i64::try_from(c.as_big()).expect("small"), strict))
}

fn ceilings(values: &[i64]) -> Vec<Integer> {
    values.iter().copied().map(int).collect()
}

struct Seeded(u64);

impl Seeded {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) % n
    }

    fn index(&mut self, n: usize) -> usize {
        usize::try_from(self.below(n as u64)).expect("small")
    }

    fn small(&mut self, max: i64) -> i64 {
        i64::try_from(self.below(u64::try_from(max).expect("positive") + 1)).expect("small")
    }

    /// A zone over `dim - 1` clocks cut by up to four random bounds.
    fn zone(&mut self, dim: usize, max: i64) -> Zone {
        let mut zone = Zone::universe(dim);
        for _ in 0..self.below(5) {
            let (i, j) = (self.index(dim), self.index(dim));
            let c = self.small(max) - self.small(max);
            zone.constrain(i, j, &int(c), self.below(2) == 0)
                .expect("in range");
        }
        zone
    }
}

/// Every grid point of `dim - 1` clocks over `[0, top]` at scale `s`.
fn grid(dim: usize, top: i64) -> Vec<Vec<i64>> {
    let mut points = vec![vec![0_i64]];
    for _ in 1..dim {
        points = points
            .into_iter()
            .flat_map(|p| {
                (0..=top).map(move |c| {
                    let mut q = p.clone();
                    q.push(c);
                    q
                })
            })
            .collect();
    }
    points
}

/// Case 1: `x in [3, 7]`, subtract 2 under `2 <= x <= 5`, gives `[1, 3]`.
/// Hand-computed, as the engine's `subtract_bounded` gives.
#[trace("TC-371")]
#[test]
fn subtract_bounded_shifts_the_guarded_part_of_one_clock() {
    let mut z = zone(2, &[(1, 0, 7, false), (0, 1, -3, false)]);
    z.subtract_bounded(1, &int(2), &int(5)).expect("valid");
    assert_eq!(bound(&z, 1, 0), Some((3, false)));
    assert_eq!(bound(&z, 0, 1), Some((-1, false)));
}

/// Case 2: strictness survives, and a clock below the amount has no
/// successor. `x in (3, 7)`, subtract 3 under `3 <= x <= 6` gives `(0, 3)`;
/// `x in [0, 1]` subtract 2 is empty.
#[trace("TC-371")]
#[test]
fn subtract_bounded_keeps_strictness_and_empties_below_the_amount() {
    let mut z = zone(2, &[(1, 0, 7, true), (0, 1, -3, true)]);
    z.subtract_bounded(1, &int(3), &int(6)).expect("valid");
    assert_eq!(bound(&z, 1, 0), Some((3, false)));
    assert_eq!(bound(&z, 0, 1), Some((0, true)));

    let mut low = zone(2, &[(1, 0, 1, false)]);
    low.subtract_bounded(1, &int(2), &int(4)).expect("valid");
    assert!(low.is_empty());
}

/// Case 3: differences between other clocks stay, the subtracted clock's
/// differences move. `x in [2, 6]`, `y in [1, 5]`, `x - y = 1`; subtract 2
/// from `x` under `2 <= x <= 4` gives `x in [0, 2]`, `y in [1, 3]`,
/// `x - y = -1`, `y - x = 1`.
#[trace("TC-371")]
#[test]
fn subtract_bounded_moves_the_differences_of_the_subtracted_clock() {
    let mut z = zone(
        3,
        &[
            (1, 0, 6, false),
            (0, 1, -2, false),
            (2, 0, 5, false),
            (0, 2, -1, false),
            (1, 2, 1, false),
            (2, 1, -1, false),
        ],
    );
    z.subtract_bounded(1, &int(2), &int(4)).expect("valid");
    assert_eq!(bound(&z, 1, 0), Some((2, false)));
    assert_eq!(bound(&z, 0, 1), Some((0, false)));
    assert_eq!(bound(&z, 2, 0), Some((3, false)));
    assert_eq!(bound(&z, 0, 2), Some((-1, false)));
    assert_eq!(bound(&z, 1, 2), Some((-1, false)));
    assert_eq!(bound(&z, 2, 1), Some((1, false)));
}

#[trace("TC-371")]
#[test]
fn subtract_bounded_refuses_what_it_cannot_subtract_and_changes_nothing() {
    let original = zone(2, &[(1, 0, 7, false)]);
    for (clock, amount, ceiling, expected) in [
        (0, 1, 5, ZoneError::SubtractReferenceClock),
        (1, -1, 5, ZoneError::InvalidSubtractionRange),
        (1, 1, 0, ZoneError::InvalidSubtractionRange),
        (1, 6, 5, ZoneError::InvalidSubtractionRange),
        (2, 1, 5, ZoneError::ClockOutOfRange { clock: 2, dim: 2 }),
    ] {
        let mut z = original.clone();
        assert_eq!(
            z.subtract_bounded(clock, &int(amount), &int(ceiling)),
            Err(expected)
        );
        assert_eq!(z, original);
    }
}

/// The subtraction agrees point-wise with its definition on seeded zones:
/// `p` is a successor exactly when `p + amount` on that clock is in the zone
/// and within the guard. Grid scale 2, so strict bounds are exercised.
#[trace("TC-371")]
#[test]
fn subtract_bounded_agrees_with_its_definition_on_a_grid() {
    const S: i64 = 2;
    let mut rng = Seeded(0x5eed_0418);
    let mut nonempty = 0_u32;
    for _ in 0..300 {
        let dim = 2 + rng.index(2);
        let input = rng.zone(dim, 4);
        let clock = 1 + rng.index(dim - 1);
        let ceiling = 1 + rng.small(4);
        let amount = rng.small(ceiling);
        let mut result = input.clone();
        result
            .subtract_bounded(clock, &int(amount), &int(ceiling))
            .expect("valid");
        for p in grid(dim, 7 * S) {
            let mut before = p.clone();
            before[clock] += amount * S;
            let expected = reference::zone_contains(&input, &before, S)
                && amount * S <= before[clock]
                && before[clock] <= ceiling * S;
            assert_eq!(
                reference::zone_contains(&result, &p, S),
                expected,
                "subtract x{clock} by {amount} under {ceiling} of {input:?} at {p:?}"
            );
            nonempty += u32::from(expected);
        }
    }
    assert!(nonempty > 100, "the sampled cases reach nonempty results");
}

/// Bounds of any size subtract exactly (FR-418-AC-4).
#[trace("TC-371", "FR-418-AC-4")]
#[test]
fn subtract_bounded_is_exact_beyond_machine_integers() {
    let big = Integer::power_of_ten(40);
    let mut z = Zone::universe(2);
    z.constrain(1, 0, &big.add(&big), false).expect("in range");
    z.constrain(0, 1, &big.neg(), false).expect("in range");
    z.subtract_bounded(1, &big, &big.add(&big)).expect("valid");
    assert_eq!(z.bound(1, 0).expect("in range"), Some((big, false)));
    assert_eq!(
        z.bound(0, 1).expect("in range"),
        Some((Integer::from(0_i64), false))
    );
}

#[trace("TC-371")]
#[test]
fn constrain_diagonal_is_constrain_over_two_real_clocks() {
    let mut z = zone(3, &[(1, 0, 6, false), (2, 0, 4, false)]);
    z.constrain_diagonal(1, 2, &int(1), true).expect("diagonal");
    assert_eq!(bound(&z, 1, 2), Some((1, true)));
    // Closure tightens the clock's own bound: `x < y + 1 <= 5`.
    assert_eq!(bound(&z, 1, 0), Some((5, true)));
    let mut tight = zone(3, &[(2, 0, 2, false)]);
    tight
        .constrain_diagonal(1, 2, &int(1), false)
        .expect("diagonal");
    assert_eq!(bound(&tight, 1, 0), Some((3, false)));
    for (left, right, expected) in [
        (0, 1, ZoneError::NotDiagonal { left: 0, right: 1 }),
        (1, 0, ZoneError::NotDiagonal { left: 1, right: 0 }),
        (2, 2, ZoneError::NotDiagonal { left: 2, right: 2 }),
        (3, 1, ZoneError::ClockOutOfRange { clock: 3, dim: 3 }),
    ] {
        let mut z = zone(3, &[]);
        assert_eq!(
            z.constrain_diagonal(left, right, &int(0), false),
            Err(expected)
        );
    }
}

/// Hand count: two clocks with ceiling 1 have, per clock, the classes
/// `= 0`, `(0, 1)`, `= 1` and `> 1`; of the sixteen pairs only
/// `((0, 1), (0, 1))` splits, three ways by fraction order (`<`, `=`,
/// `>`), so 18 regions. The diagonal `x - y <= 0` splits only the
/// saturated pair `(> 1, > 1)`, so 19.
#[trace("TC-371")]
#[test]
fn the_universe_of_two_clocks_has_the_hand_counted_regions() {
    let all = Zone::universe(3);
    assert_eq!(
        all.region_normalize(&ceilings(&[1, 1]), &[])
            .expect("ok")
            .len(),
        18
    );
    let diagonal = DiagonalPredicate {
        left: 1,
        right: 2,
        value: int(0),
        strict: false,
    };
    assert_eq!(
        all.region_normalize(&ceilings(&[1, 1]), &[diagonal])
            .expect("ok")
            .len(),
        19
    );
}

/// Integer part, zero fraction and fraction order: `x in (0, 1)` is one
/// region for ceiling 2, `x in (0, 2]` is four (`(0,1)`, `1`, `(1,2)`, `2`),
/// a clock above its ceiling is one saturated region wherever it is, and
/// `x = 0.5, y = 0.25` lies in the one region `0 < y < x < 1`.
#[trace("TC-371")]
#[test]
fn regions_keep_integer_part_zero_fraction_order_and_saturate() {
    let ceil2 = ceilings(&[2]);
    let unit = zone(2, &[(0, 1, 0, true), (1, 0, 1, true)]);
    assert_eq!(
        unit.region_normalize(&ceil2, &[]).expect("ok"),
        vec![unit.clone()]
    );
    let two = zone(2, &[(0, 1, 0, true), (1, 0, 2, false)]);
    assert_eq!(two.region_normalize(&ceil2, &[]).expect("ok").len(), 4);
    let far = zone(2, &[(0, 1, -5, false), (1, 0, 9, false)]);
    let saturated = zone(2, &[(0, 1, -2, true)]);
    assert_eq!(
        far.region_normalize(&ceil2, &[]).expect("ok"),
        vec![saturated]
    );

    let mut ordered = Zone::universe(3);
    ordered.constrain(1, 0, &int(1), true).expect("in range");
    ordered.constrain(2, 1, &int(0), true).expect("in range");
    ordered.constrain(0, 2, &int(0), true).expect("in range");
    let regions = ordered
        .region_normalize(&ceilings(&[1, 1]), &[])
        .expect("ok");
    assert_eq!(regions, vec![ordered]);
}

/// A successor with a saturated clock is covered by a target that holds
/// the whole saturated class, and not by one that cuts it.
#[trace("TC-371")]
#[test]
fn saturation_decides_coverage_above_the_ceiling() {
    let ceil = ceilings(&[2]);
    let successor = zone(2, &[(1, 0, 5, false), (0, 1, -5, false)]);
    let whole = zone(2, &[(0, 1, -2, true)]);
    let cut = zone(2, &[(0, 1, -2, true), (1, 0, 7, false)]);
    assert!(successor.covered_by(&whole, &ceil, &[]).expect("ok"));
    assert!(!successor.covered_by(&cut, &ceil, &[]).expect("ok"));
    // The closure of `x in (0, 1)` is the region `(0, 1)`, not the point `x = 1`.
    let point = zone(2, &[(1, 0, 1, false), (0, 1, -1, false)]);
    let half = zone(2, &[(1, 0, 1, true), (0, 1, 0, true)]);
    assert!(!half.covered_by(&point, &ceilings(&[2]), &[]).expect("ok"));
}

/// A diagonal predicate whose truth differs inside what would otherwise be
/// one region keeps the two truths apart: the closure of `x = y = 3`
/// (ceilings 1) is the saturated square without the predicate and its
/// `x - y <= 0` half with it.
#[trace("TC-371")]
#[test]
fn diagonal_truth_decides_coverage() {
    let ceil = ceilings(&[1, 1]);
    let predicate = DiagonalPredicate {
        left: 1,
        right: 2,
        value: int(0),
        strict: false,
    };
    let point = zone(
        3,
        &[
            (1, 0, 3, false),
            (0, 1, -3, false),
            (2, 0, 3, false),
            (0, 2, -3, false),
        ],
    );
    let below = zone(3, &[(0, 1, -1, true), (0, 2, -1, true), (1, 2, 0, false)]);
    let wide = zone(3, &[(0, 1, -1, true), (0, 2, -1, true), (1, 2, 4, false)]);
    let with = [predicate.clone()];
    assert!(point.covered_by(&below, &ceil, &with).expect("ok"));
    assert!(point.covered_by(&wide, &ceil, &with).expect("ok"));
    // Without the predicate the closure is the whole saturated square, so
    // the half is not enough.
    assert!(!point.covered_by(&below, &ceil, &[]).expect("ok"));
}

/// Normalization is idempotent and its regions partition the zone's
/// points, and `region_covers` is inclusion for a target that is a union
/// of whole regions, on seeded zones.
#[trace("TC-371")]
#[test]
fn normalize_is_idempotent_and_covers_is_inclusion_after_normalization() {
    const S: i64 = 4;
    let mut rng = Seeded(0x5eed_4180);
    let mut aligned = 0_u32;
    for round in 0..120 {
        let dim = if round % 8 == 0 { 4 } else { 3 };
        let ceil = ceilings(&vec![2; dim - 1]);
        let diagonals = if round % 2 == 0 {
            vec![DiagonalPredicate {
                left: 1,
                right: 2,
                value: int(1),
                strict: true,
            }]
        } else {
            Vec::new()
        };
        let source = rng.zone(dim, 3);
        let regions = source.region_normalize(&ceil, &diagonals).expect("ok");
        assert_eq!(regions.is_empty(), source.is_empty());
        for region in &regions {
            assert!(!region.is_empty());
            assert!(
                region.region_normalize(&ceil, &diagonals).expect("ok") == vec![region.clone()],
                "a region is its own normal form: {region:?}"
            );
        }
        let again: Vec<Zone> = {
            let mut all: Vec<Zone> = regions
                .iter()
                .flat_map(|r| r.region_normalize(&ceil, &diagonals).expect("ok"))
                .collect();
            all.sort();
            all.dedup();
            all
        };
        assert_eq!(again, regions);
        if dim == 3 {
            for p in grid(dim, 5 * S) {
                let inside = reference::zone_contains(&source, &p, S);
                let holding = regions
                    .iter()
                    .filter(|r| reference::zone_contains(r, &p, S))
                    .count();
                assert!(holding <= 1, "regions are disjoint at {p:?}");
                assert!(
                    !inside || holding == 1,
                    "a point of the zone is in a region"
                );
            }
        }

        let target = rng.zone(dim, 3);
        let covered = source.covered_by(&target, &ceil, &diagonals).expect("ok");
        assert!(!covered || target.includes(&source).expect("same dim"));
        if target.covered_by(&target, &ceil, &diagonals).expect("ok") {
            aligned += 1;
            assert_eq!(
                covered,
                target.includes(&source).expect("same dim"),
                "inclusion for the union of regions {target:?} of {source:?}"
            );
        }
        assert!(regions
            .iter()
            .all(|r| r.covered_by(&r.clone(), &ceil, &diagonals).expect("ok")));
    }
    assert!(aligned > 20, "enough sampled targets are unions of regions");
}

#[trace("TC-371")]
#[test]
fn region_operations_refuse_malformed_arguments() {
    let z = Zone::universe(3);
    assert_eq!(
        z.region_normalize(&ceilings(&[1]), &[]),
        Err(ZoneError::ExtrapolationLength { max: 1, clocks: 2 })
    );
    assert_eq!(
        z.region_normalize(&ceilings(&[1, -1]), &[]),
        Err(ZoneError::NegativeCeiling { clock: 2 })
    );
    let bad = DiagonalPredicate {
        left: 1,
        right: 1,
        value: int(0),
        strict: false,
    };
    assert_eq!(
        z.region_normalize(&ceilings(&[1, 1]), &[bad]),
        Err(ZoneError::NotDiagonal { left: 1, right: 1 })
    );
    assert_eq!(
        z.covered_by(&Zone::universe(2), &ceilings(&[1, 1]), &[]),
        Err(ZoneError::DimensionMismatch { left: 3, right: 2 })
    );
}

/// `region_covers` over an arbitrary target with no work limit, as a bool.
trait CoveredBy {
    fn covered_by(
        &self,
        target: &Zone,
        ceilings: &[Integer],
        diagonals: &[DiagonalPredicate],
    ) -> Result<bool, ZoneError>;
}

impl CoveredBy for Zone {
    fn covered_by(
        &self,
        target: &Zone,
        ceilings: &[Integer],
        diagonals: &[DiagonalPredicate],
    ) -> Result<bool, ZoneError> {
        let outcome = self.region_covers(
            target,
            ceilings,
            diagonals,
            TargetShape::Arbitrary,
            &mut || true,
        )?;
        assert_ne!(outcome, Coverage::Exhausted, "no limit was set");
        Ok(outcome == Coverage::Covered)
    }
}

/// `x in (1, 2)`, `y in (0, 1)`, ceilings 2: the fraction order splits the
/// box in three, `frac x < frac y` (`x - y < 1`), `=` (`x - y = 1`) and `>`
/// (`x - y > 1`). Hand-computed bounds of each region.
#[trace("TC-371")]
#[test]
fn fraction_order_splits_boxes_with_different_integer_parts() {
    let boxed = zone(
        3,
        &[
            (0, 1, -1, true),
            (1, 0, 2, true),
            (0, 2, 0, true),
            (2, 0, 1, true),
        ],
    );
    let regions = boxed.region_normalize(&ceilings(&[2, 2]), &[]).expect("ok");
    assert_eq!(regions.len(), 3);
    let by_x_minus_y: Vec<_> = regions
        .iter()
        .map(|r| (bound(r, 1, 2), bound(r, 2, 1)))
        .collect();
    let want = [
        (Some((1, true)), Some((0, true))),
        (Some((1, false)), Some((-1, false))),
        (Some((2, true)), Some((-1, true))),
    ];
    for expected in want {
        assert!(
            by_x_minus_y.contains(&expected),
            "{expected:?} in {by_x_minus_y:?}"
        );
    }
}

/// The region of a scaled point by the definition: per clock the class
/// (saturated above the ceiling, else integer part and zero fraction), the
/// order of the fractional parts of the unsaturated clocks, and each
/// predicate's truth.
fn region_key(
    point: &[i64],
    s: i64,
    ceilings: &[i64],
    diagonals: &[(usize, usize, i64, bool)],
) -> Vec<i64> {
    let mut key = Vec::new();
    let clocks = point.len() - 1;
    for k in 1..=clocks {
        let p = point[k];
        if p > ceilings[k - 1] * s {
            key.push(-1);
        } else {
            key.push(p / s);
            key.push(i64::from(p % s == 0));
        }
    }
    for a in 1..=clocks {
        for b in a + 1..=clocks {
            if point[a] <= ceilings[a - 1] * s && point[b] <= ceilings[b - 1] * s {
                key.push((point[a] % s - point[b] % s).signum());
            }
        }
    }
    for &(l, r, c, strict) in diagonals {
        let diff = point[l] - point[r];
        key.push(i64::from(if strict { diff < c * s } else { diff <= c * s }));
    }
    key
}

/// Point by point: with a grid of quarters, two points are in one region
/// exactly when their definition keys agree. A partition that cut fraction
/// order the other way, or merged integer parts, fails here.
#[trace("TC-371")]
#[test]
fn regions_are_the_definitions_classes_on_a_grid_of_quarters() {
    const S: i64 = 4;
    for (ceil, diag) in [
        (vec![2_i64, 2], vec![]),
        (vec![2, 1], vec![(1_usize, 2_usize, 1_i64, true)]),
        (vec![2, 1, 2], vec![(1, 3, 0, false)]),
    ] {
        let dim = ceil.len() + 1;
        let predicates: Vec<DiagonalPredicate> = diag
            .iter()
            .map(|&(left, right, c, strict)| DiagonalPredicate {
                left,
                right,
                value: int(c),
                strict,
            })
            .collect();
        let regions = Zone::universe(dim)
            .region_normalize(&ceilings(&ceil), &predicates)
            .expect("ok");
        let mut owner_of_key = std::collections::HashMap::new();
        let mut key_of_owner = std::collections::HashMap::new();
        for p in grid(dim, 4 * S) {
            let owners: Vec<usize> = (0..regions.len())
                .filter(|&i| reference::zone_contains(&regions[i], &p, S))
                .collect();
            assert_eq!(owners.len(), 1, "one region holds {p:?}");
            let key = region_key(&p, S, &ceil, &diag);
            assert_eq!(
                *owner_of_key.entry(key.clone()).or_insert(owners[0]),
                owners[0],
                "{p:?}"
            );
            assert_eq!(
                *key_of_owner.entry(owners[0]).or_insert(key.clone()),
                key,
                "{p:?}"
            );
        }
    }
}

/// `region_covers` stops at the first region the target does not include.
#[trace("TC-371")]
#[test]
fn region_covers_stops_at_the_first_uncovered_region() {
    let ceil = ceilings(&[3, 3]);
    let all = Zone::universe(3);
    let mut full = 0_u64;
    all.region_covers(&all, &ceil, &[], TargetShape::Arbitrary, &mut || {
        full += 1;
        true
    })
    .expect("ok");
    // The first region the walk reaches has `x = 0`: a target cut to
    // `x > 0` misses it at once.
    let target = zone(3, &[(0, 1, 0, true)]);
    let mut early = 0_u64;
    let outcome = all
        .region_covers(&target, &ceil, &[], TargetShape::Arbitrary, &mut || {
            early += 1;
            true
        })
        .expect("ok");
    assert_eq!(outcome, Coverage::NotCovered);
    assert!(early * 20 < full, "stopped after {early} of {full} steps");
}

/// For a target stated to be a union of whole regions no region is walked:
/// one unit of work answers by inclusion.
#[trace("TC-371")]
#[test]
fn region_covers_uses_inclusion_for_a_union_of_regions() {
    let ceil = ceilings(&[2, 2]);
    let successor = zone(3, &[(1, 0, 1, false), (0, 1, -1, false), (2, 0, 1, true)]);
    let target = zone(3, &[(1, 0, 2, false), (2, 0, 2, false)]);
    let mut work = 0_u64;
    let outcome = successor
        .region_covers(
            &target,
            &ceil,
            &[],
            TargetShape::UnionOfRegions,
            &mut || {
                work += 1;
                true
            },
        )
        .expect("ok");
    assert_eq!((outcome, work), (Coverage::Covered, 1));
    let narrow = zone(3, &[(1, 0, 0, false)]);
    assert_eq!(
        successor
            .region_covers(
                &narrow,
                &ceil,
                &[],
                TargetShape::UnionOfRegions,
                &mut || true
            )
            .expect("ok"),
        Coverage::NotCovered
    );
}

/// A spent budget is `Exhausted`, never `Covered`, on both paths, and the
/// same test with enough work is covered.
#[trace("TC-371")]
#[test]
fn region_covers_reports_an_exhausted_budget_never_covered() {
    let ceil = ceilings(&[2, 2]);
    let all = Zone::universe(3);
    for shape in [TargetShape::Arbitrary, TargetShape::UnionOfRegions] {
        for budget in [0_u64, 1, 5] {
            let mut left = budget;
            let outcome = all
                .region_covers(&all, &ceil, &[], shape, &mut || {
                    let spend = left > 0;
                    left = left.saturating_sub(1);
                    spend
                })
                .expect("ok");
            let enough = shape == TargetShape::UnionOfRegions && budget > 0;
            assert_eq!(
                outcome,
                if enough {
                    Coverage::Covered
                } else {
                    Coverage::Exhausted
                },
                "{shape:?} with budget {budget}"
            );
        }
        assert_eq!(
            all.region_covers(&all, &ceil, &[], shape, &mut || true)
                .expect("ok"),
            Coverage::Covered
        );
    }
}

/// A zone that admits a negative clock is refused, not cut at 0.
#[trace("TC-371")]
#[test]
fn a_zone_admitting_a_negative_clock_is_refused() {
    let negative = Zone::from_matrix(
        2,
        [
            Some((int(0), false)),
            Some((int(1), false)),
            Some((int(5), false)),
            Some((int(0), false)),
        ],
    )
    .expect("a matrix");
    assert!(!negative.is_empty());
    assert_eq!(
        negative.region_normalize(&ceilings(&[2]), &[]),
        Err(ZoneError::NegativeClockValue { clock: 1 })
    );
    assert_eq!(
        negative.covered_by(&Zone::universe(2), &ceilings(&[2]), &[]),
        Err(ZoneError::NegativeClockValue { clock: 1 })
    );
}
