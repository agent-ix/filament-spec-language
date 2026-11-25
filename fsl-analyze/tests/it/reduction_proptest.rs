// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-623 (TC-1583): a reduction selected in an analyze item never changes
//! the verdict, and a selection the preservation table does not admit is a
//! typed refusal of that item alone. The selections are enumerated over every
//! combination rather than sampled.

use fsl_eval::simulation::TransitionSystem;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, clause, counter_request_with, label, request_members, Workspace,
};
use crate::counter::{document, symmetric_document};

/// States reachable from the initial snapshot, bounded to `limit` unique keys.
pub(crate) fn reachable_states(
    system: &fsl_eval::simulation::model::ModelSystem<'_>,
    limit: usize,
) -> Vec<fsl_eval::simulation::model::ModelSimState> {
    let mut seen = std::collections::BTreeSet::new();
    let mut states = Vec::new();
    let mut queue = system.initial();
    while let Some(state) = queue.pop() {
        let key = system
            .state_key(&state, fsl_foundation::IdentityLimits::default())
            .expect("state encodes")
            .0;
        if !seen.insert(key) {
            continue;
        }
        if states.len() == limit {
            break;
        }
        let expansion = system.successors(&state).expect("the state expands");
        queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
        states.push(state);
    }
    states
}

/// The Counter request over two interchangeable counters `a` and `b`.
fn two_counters(ws: &Workspace, body: &str, items: Value) -> std::path::PathBuf {
    let path = counter_request_with(ws, body, request_members(Some(items)));
    ws.write("model.json", &symmetric_document());
    ws.write(
        "start.json",
        &crate::analyze_cli_support::counter_snapshot(&[("a", 0), ("b", 0)]),
    );
    path
}

fn selections() -> Vec<Value> {
    let symmetry = json!([{"population": "counters", "classes": [["a", "b"]]}]);
    vec![
        json!({}),
        json!({"symmetry": symmetry}),
        json!({"partial_order": true}),
        json!({"symmetry": symmetry, "partial_order": true}),
    ]
}

/// FR-623-AC-1: every selection gives the unreduced verdict.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
fn every_selection_keeps_the_verdict() {
    for (name, formula, expected) in [
        ("Holds", "always holds(c.value <= 3)", "proved"),
        ("Fails", "always holds(c.value <= 2)", "refuted"),
    ] {
        let items: Vec<Value> = selections()
            .into_iter()
            .map(|reductions| json!({"claim": name, "reductions": reductions}))
            .collect();
        let ws = Workspace::new(&format!("reductions-{name}"));
        let run = analyze(&two_counters(
            &ws,
            &clause(name, "", formula),
            Value::Array(items),
        ));
        for index in 0..selections().len() {
            assert_eq!(
                label(&run.item(index)),
                expected,
                "{name} selection {index}"
            );
        }
    }
}

/// FR-623-AC-3: partial-order reduction on a bounded MLTL clause is refused
/// for that item only.
#[trace("TC-1583", "FR-623-AC-3")]
#[trace("TC-1080", "FSpec-FR-520-AC-4")]
#[test]
fn a_non_preserving_selection_refuses_its_item_only() {
    let body = "temporal Window using t0 over (c: M::Counter) clock \"model-steps\" on origin \
                { eventually[0, 3] holds(c.value = 3) }\n"
        .to_owned()
        + &clause("Holds", "", "always holds(c.value <= 3)");
    let ws = Workspace::new("reductions-mltl");
    let run = analyze(&two_counters(
        &ws,
        &body,
        json!([{"claim": "Window", "reductions": {"partial_order": true}},
            {"claim": "Holds"}]),
    ));
    let refused = run.item(0);
    assert_eq!(label(&refused), "inconclusive");
    assert_eq!(
        refused["record"]["cause"]["kind"],
        "reduction-not-preserving"
    );
    assert_eq!(refused["record"]["cause"]["reduction"], "partial-order");
    assert_eq!(label(&run.item(1)), "proved");
    assert_eq!(run.exit, 22);
}

/// FR-623-AC-4: an unknown constraint name and an undeclared symmetry refuse.
#[trace("TC-1583", "FR-623-AC-4")]
#[test]
fn malformed_selections_are_refused_typed() {
    let ws = Workspace::new("reductions-malformed");
    let request = two_counters(
        &ws,
        &clause("Holds", "", "always holds(c.value <= 3)"),
        json!([{"claim": "Holds", "reductions": {"constraint": "NoSuchConstraint"}},
            {"claim": "Holds", "reductions": {"symmetry": "declared"}}]),
    );
    // The unit annotates no population `symmetric`, so `"declared"` has no class.
    ws.write("model.json", &document(false));
    let run = analyze(&request);
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["code"],
        "invalid_runtime_input"
    );
    assert_eq!(label(&run.item(1)), "declined");
    assert_eq!(run.item(1)["record"]["cause"]["kind"], "invalid-input");
}

/// QSL-2061: generated reduction subjects. Each is a small `reduction_world`
/// package with a symmetric population of two to four counters, a contract, a
/// property, and random starting values; the four selections must agree.
mod generated {
    use std::cell::RefCell;

    use fsl_proptest_policy as prop;
    use fsl_replay::{
        CounterexampleKind, ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement,
    };
    use fsl_route::reductions::{Reductions, SymmetryDeclaration};
    use ix_trace_rs::trace;
    use proptest::prelude::*;
    use proptest::test_runner::{TestCaseError, TestError};

    use crate::reduction_world::{counter, counters, value, Atom, Clause, Shape, World, INFINITE};

    const KEYS: [&str; 4] = ["a", "b", "c", "d"];

    #[derive(Clone, Copy, Debug)]
    enum Formula {
        AlwaysAtMost,
        AlwaysNot,
        Eventually,
        /// `fair weak` (whole) then `always eventually`.
        FairWhole,
        /// `fair weak each` then `always eventually`.
        FairEach,
    }

    /// One generated subject and property.
    #[derive(Clone, Debug)]
    pub(crate) struct Subject {
        size: usize,
        max: u64,
        cycle: bool,
        starts: Vec<u64>,
        formula: Formula,
        k: u64,
    }

    pub(crate) fn subjects(min_size: usize) -> BoxedStrategy<Subject> {
        subjects_for_sizes_and_bounds(min_size..=4, 1u64..=3)
    }

    fn subjects_for_sizes_and_bounds<S, B>(sizes: S, bounds: B) -> BoxedStrategy<Subject>
    where
        S: Strategy<Value = usize> + 'static,
        B: Strategy<Value = u64> + 'static,
    {
        (
            sizes,
            bounds,
            any::<bool>(),
            proptest::collection::vec(0u64..=3, 4),
            0usize..5,
            0u64..=3,
        )
            .prop_map(|(size, max, cycle, starts, formula, k)| {
                let formula = match formula {
                    0 => Formula::AlwaysAtMost,
                    1 => Formula::AlwaysNot,
                    2 => Formula::Eventually,
                    3 if cycle => Formula::FairWhole,
                    4 if cycle => Formula::FairEach,
                    _ => Formula::AlwaysAtMost,
                };
                Subject {
                    size,
                    max,
                    cycle,
                    starts: starts.into_iter().map(|v| v.min(max)).collect(),
                    formula,
                    k: k.min(max),
                }
            })
            .boxed()
    }

    /// P1/P2 still sample all 256 cases, populations of two to four, and
    /// counter bounds one through three. They bias toward the smallest bound
    /// so complete four-way searches meet QSL-2061's per-file runtime limit.
    fn verdict_subjects() -> BoxedStrategy<Subject> {
        let bounds = prop_oneof![64 => Just(1u64), 1 => Just(2u64), 1 => Just(3u64)];
        subjects_for_sizes_and_bounds(2usize..=4, bounds)
    }

    /// Every distinct ordering of `values`.
    fn arrangements(values: &[u64]) -> Vec<Vec<u64>> {
        if values.len() <= 1 {
            return vec![values.to_vec()];
        }
        let mut all = std::collections::BTreeSet::new();
        for i in 0..values.len() {
            let mut rest = values.to_vec();
            let head = rest.remove(i);
            for mut tail in arrangements(&rest) {
                tail.insert(0, head);
                all.insert(tail);
            }
        }
        all.into_iter().collect()
    }

    impl Subject {
        fn contract(&self) -> String {
            let max = self.max;
            if self.cycle {
                format!(
                    "post Cycle using v on M::Counter::bump {{ \
                     (pre(self.v) < {max} and self.v - pre(self.v) = 1) or \
                     (pre(self.v) = {max} and self.v = 0) }}\n"
                )
            } else {
                format!(
                    "pre Below using v on M::Counter::bump {{ self.v < {max} }}\n\
                     post Up using v on M::Counter::bump {{ self.v - pre(self.v) = 1 }}\n"
                )
            }
        }

        fn formula(&self) -> String {
            let k = self.k;
            match self.formula {
                Formula::AlwaysAtMost => format!("always holds(x.v <= {k})"),
                Formula::AlwaysNot => format!("always holds(x.v != {k})"),
                Formula::Eventually => format!("eventually holds(x.v = {k})"),
                Formula::FairWhole => {
                    format!("fair weak M::Counter::bump; always eventually holds(x.v = {k})")
                }
                Formula::FairEach => {
                    format!("fair weak each M::Counter::bump; always eventually holds(x.v = {k})")
                }
            }
        }

        /// The one atom of the formula; `skew` reads it of the last object alone,
        /// which is no symmetric property.
        fn atoms(&self, skew: bool) -> Vec<Atom> {
            let k = i64::try_from(self.k).expect("small");
            let last = self.keys()[self.size - 1];
            let test: fn(i64, i64) -> bool = match self.formula {
                Formula::AlwaysAtMost => |v, k| v <= k,
                Formula::AlwaysNot => |v, k| v != k,
                _ => |v, k| v == k,
            };
            vec![Box::new(move |state, object| {
                Some((skew && object != last) || test(value(state, object), k))
            })]
        }

        pub(crate) fn keys(&self) -> &[&'static str] {
            &KEYS[..self.size]
        }

        pub(crate) fn world(&self) -> World {
            let formula = self.formula();
            // The initial set is closed under the population's symmetry: one
            // initial state per distinct arrangement of the start values.
            let starts: Vec<_> = arrangements(&self.starts[..self.size])
                .into_iter()
                .map(|values| {
                    self.keys()
                        .iter()
                        .zip(values)
                        .map(|(key, v)| (*key, counter(i64::try_from(v).expect("small"))))
                        .collect::<Vec<_>>()
                })
                .collect();
            World::new(
                Shape {
                    symmetric: true,
                    max: self.max,
                    link: false,
                },
                &self.contract(),
                &[Clause {
                    profile: INFINITE,
                    formula: &formula,
                }],
                &starts,
                self.keys(),
            )
        }

        fn declaration(&self) -> Vec<SymmetryDeclaration> {
            vec![SymmetryDeclaration {
                population: counters(),
                classes: vec![self.keys().iter().map(|k| (*k).to_owned()).collect()],
            }]
        }

        /// Unreduced, symmetry, partial-order, both (FR-623's four selections).
        fn selections(&self) -> [(&'static str, Reductions); 4] {
            [
                ("unreduced", Reductions::default()),
                (
                    "symmetry",
                    Reductions {
                        symmetry: self.declaration(),
                        ..Reductions::default()
                    },
                ),
                (
                    "partial-order",
                    Reductions {
                        partial_order: true,
                        ..Reductions::default()
                    },
                ),
                (
                    "both",
                    Reductions {
                        symmetry: self.declaration(),
                        partial_order: true,
                        ..Reductions::default()
                    },
                ),
            ]
        }

        pub(crate) fn describe(&self, skew: bool) -> String {
            format!(
                "population {:?}, starts {:?}, max {}, contract:\n{}clause: {}{}",
                self.keys(),
                &self.starts[..self.size],
                self.max,
                self.contract(),
                self.formula(),
                if skew {
                    " (atom read of the last object alone)"
                } else {
                    ""
                }
            )
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum Verdict {
        Holds,
        Violated,
    }

    type Outcome = Result<ModelCheckOutcome, fsl_analyze::model_check::ModelCheckRefusal>;

    fn verdict(outcome: &Outcome) -> Option<Verdict> {
        match outcome {
            Ok(ModelCheckOutcome::Holds { .. } | ModelCheckOutcome::HoldsReduced { .. }) => {
                Some(Verdict::Holds)
            }
            Ok(ModelCheckOutcome::Violated { .. }) => Some(Verdict::Violated),
            _ => None,
        }
    }

    fn run(subject: &Subject, world: &World, skew: bool, selection: Reductions) -> Outcome {
        world.run(
            Some(0),
            subject.atoms(skew),
            selection,
            None,
            ModelCheckLimits::default(),
        )
    }

    /// P1: all four selections settle the unreduced verdict.
    pub(crate) fn verdicts_agree(subject: &Subject, skew: bool) -> Result<Verdict, String> {
        let world = subject.world();
        let outcomes = std::thread::scope(|scope| {
            subject
                .selections()
                .into_iter()
                .map(|(label, selection)| {
                    let world = &world;
                    scope.spawn(move || {
                        let outcome = run(subject, world, skew, selection);
                        let found = verdict(&outcome).ok_or_else(|| {
                            format!(
                                "{label} did not settle a verdict: {outcome:?}\n{}",
                                subject.describe(skew)
                            )
                        });
                        (label, found)
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("a reduction search does not panic"))
                .collect::<Vec<_>>()
        });
        let mut expected = None;
        for (label, found) in outcomes {
            let found = found?;
            match expected {
                None => expected = Some(found),
                Some(unreduced) if unreduced != found => {
                    return Err(format!(
                        "unreduced {unreduced:?} but {label} {found:?}\n{}",
                        subject.describe(skew)
                    ))
                }
                Some(_) => {}
            }
        }
        Ok(expected.expect("four runs"))
    }

    /// P2: every counterexample of a reduced run replays on the unreduced model.
    fn counterexamples_replay(subject: &Subject, skew: bool) -> Result<bool, String> {
        let world = subject.world();
        let mut witnesses = Vec::new();
        // P2 concerns counterexamples produced by reduced runs; the unreduced
        // run is not another candidate. The same witness can be returned by
        // symmetry, partial-order, and combined search, so replay each distinct
        // witness once rather than repeating identical source reconstruction.
        let candidates = std::thread::scope(|scope| {
            subject
                .selections()
                .into_iter()
                .skip(1)
                .map(|(label, selection)| {
                    let world = &world;
                    scope.spawn(move || {
                        let candidate = match run(subject, world, skew, selection) {
                            Ok(ModelCheckOutcome::Violated { counterexample, .. }) => {
                                Some(counterexample.as_ref().clone())
                            }
                            _ => None,
                        };
                        (label, candidate)
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("a reduction search does not panic"))
                .collect::<Vec<_>>()
        });
        let candidates: Vec<_> = candidates
            .into_iter()
            .filter_map(|(label, candidate)| candidate.map(|witness| (label, witness)))
            .filter(|(_, witness)| {
                !matches!(witness.kind, CounterexampleKind::UndefinedEvaluation(_))
            })
            .filter(|(_, witness)| {
                if witnesses.contains(witness) {
                    false
                } else {
                    witnesses.push(witness.clone());
                    true
                }
            })
            .collect();
        let replayed = !candidates.is_empty();
        let results = std::thread::scope(|scope| {
            candidates
                .into_iter()
                .map(|(label, counterexample)| {
                    let world = &world;
                    scope.spawn(move || {
                        match world.replay_clause(0, subject.atoms(skew), counterexample) {
                            ModelTraceSettlement::Reproduced { .. } => Ok(()),
                            other => Err(format!(
                                "{label} counterexample does not replay: {other:?}\n{}",
                                subject.describe(skew)
                            )),
                        }
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("a replay does not panic"))
                .collect::<Vec<_>>()
        });
        for result in results {
            result?;
        }
        Ok(replayed)
    }

    /// QSL-2061 P1 (FR-623-AC-1): the four selections give one verdict.
    #[trace("TC-1583", "FR-623-AC-1")]
    #[test]
    fn reductions_keep_the_verdict() {
        let seen = RefCell::new([0u32; 2]);
        let populations = RefCell::new([0u32; 3]);
        let bounds = RefCell::new([0u32; 3]);
        prop::run(
            "reductions_keep_the_verdict",
            prop::config(256),
            &verdict_subjects(),
            |subject| {
                let found = verdicts_agree(&subject, false).map_err(TestCaseError::fail)?;
                seen.borrow_mut()[usize::from(found == Verdict::Violated)] += 1;
                populations.borrow_mut()[subject.size - 2] += 1;
                bounds.borrow_mut()[subject.max as usize - 1] += 1;
                Ok(())
            },
        )
        .unwrap();
        let seen = seen.into_inner();
        assert!(seen[0] >= 20 && seen[1] >= 20, "holds/violated: {seen:?}");
        assert!(
            populations.into_inner().into_iter().all(|count| count > 0),
            "each population size from two to four must be sampled"
        );
        assert!(
            bounds.into_inner().into_iter().all(|count| count > 0),
            "each counter bound from one through three must be sampled"
        );
    }

    /// QSL-2061 P2 (FR-623-AC-1): a reduced counterexample replays unreduced.
    #[trace("TC-1583", "FR-623-AC-1")]
    #[test]
    fn reduced_counterexamples_replay_unreduced() {
        let replays = RefCell::new(0u32);
        let populations = RefCell::new([0u32; 3]);
        let bounds = RefCell::new([0u32; 3]);
        prop::run(
            "reduced_counterexamples_replay_unreduced",
            prop::config(256),
            &verdict_subjects(),
            |subject| {
                populations.borrow_mut()[subject.size - 2] += 1;
                bounds.borrow_mut()[subject.max as usize - 1] += 1;
                if counterexamples_replay(&subject, false).map_err(TestCaseError::fail)? {
                    *replays.borrow_mut() += 1;
                }
                Ok(())
            },
        )
        .unwrap();
        assert!(*replays.borrow() >= 20, "only {} replays", replays.borrow());
        assert!(
            populations.into_inner().into_iter().all(|count| count > 0),
            "each population size from two to four must be sampled"
        );
        assert!(
            bounds.into_inner().into_iter().all(|count| count > 0),
            "each counter bound from one through three must be sampled"
        );
    }

    /// Negative (QSL-2061): a symmetry declared over a property that is not
    /// symmetric (the atom reads the last object alone) merges states that are not
    /// equivalent, and P1 fails with a two-member population.
    #[test]
    fn bad_quotient_is_caught() {
        let mut config = prop::config(256);
        config.failure_persistence = None;
        config.max_shrink_iters = 2_000;
        let outcome = prop::run("bad_quotient_is_caught", config, &subjects(2), |subject| {
            verdicts_agree(&subject, true)
                .map(drop)
                .map_err(TestCaseError::fail)
        });
        let Err(TestError::Fail(reason, minimal)) = outcome else {
            panic!("a non-symmetric property under a symmetry class must disagree");
        };
        assert_eq!(minimal.size, 2, "{reason}");
    }

    /// Negative (QSL-2061): a one-member population has no symmetric class,
    /// and naming it gives the same product state count as naming nothing.
    #[trace("TC-1583", "FR-623-AC-1")]
    #[test]
    fn a_population_with_no_symmetric_class_has_the_same_state_count() {
        prop::run(
            "a_population_with_no_symmetric_class_has_the_same_state_count",
            prop::config(256),
            &(1u64..=3, 0u64..=3, any::<bool>()),
            |(max, start, cycle)| {
                let subject = Subject {
                    size: 1,
                    max,
                    cycle,
                    starts: vec![start.min(max); 4],
                    formula: Formula::AlwaysAtMost,
                    k: max,
                };
                let world = subject.world();
                let states = |selection: Reductions| match run(&subject, &world, false, selection) {
                    Ok(
                        ModelCheckOutcome::Holds { stats, .. }
                        | ModelCheckOutcome::HoldsReduced { stats, .. },
                    ) => Ok(stats.product_states),
                    other => Err(TestCaseError::fail(format!("{other:?}"))),
                };
                let plain = states(Reductions::default())?;
                let named = states(Reductions {
                    symmetry: subject.declaration(),
                    ..Reductions::default()
                })?;
                prop_assert_eq!(plain, named);
                Ok(())
            },
        )
        .unwrap();
    }

    // -------------------------------------------------------- canonical form

    use fsl_eval::simulation::model::ModelSimState;
    use fsl_eval::simulation::{Permutation, SymmetryClass, SymmetryGroup};

    fn group(subject: &Subject) -> SymmetryGroup {
        SymmetryGroup {
            classes: vec![SymmetryClass {
                population: counters(),
                keys: subject.keys().iter().map(|k| (*k).to_owned()).collect(),
            }],
        }
    }

    /// The permutation sending key `i` to key `images[i]`.
    fn permutation(subject: &Subject, images: &[usize]) -> Permutation {
        let keys = subject.keys();
        Permutation::of(
            &counters(),
            keys.iter()
                .zip(images)
                .map(|(from, to)| ((*from).to_owned(), keys[*to].to_owned())),
        )
    }

    /// A shuffle of `0..4` restricted to `0..n`, as a permutation of `n` indices.
    fn images(shuffle: &[usize], n: usize) -> Vec<usize> {
        shuffle.iter().copied().filter(|i| *i < n).collect()
    }

    /// P3 and P4: for a reachable state `s` and a permutation `p`,
    /// `canon(p(s)) = canon(s)`, `canon(canon(s)) = canon(s)`, and `canon(s)`
    /// is a permutation image of `s`.
    pub(crate) fn canonical_laws(
        subject: &Subject,
        shuffle: &[usize],
        pick: usize,
    ) -> Result<(), String> {
        let world = subject.world();
        let system = world.system();
        let group = group(subject);
        let states = super::reachable_states(&system, 64);
        let state = &states[pick % states.len()];
        let key = |state: &ModelSimState| {
            system
                .state_key(state, fsl_foundation::IdentityLimits::default())
                .expect("encodes")
                .0
        };
        let canon =
            |state: &ModelSimState| system.canonicalise(state, &group).expect("canonicalises").0;
        let n = subject.keys().len();
        let permuted = system
            .permute(state, &permutation(subject, &images(shuffle, n)))
            .map_err(|stop| format!("permute: {stop:?}"))?;
        let once = canon(state);
        let canonical = key(&once);
        if key(&canon(&permuted)) != canonical {
            return Err(format!(
                "canon(p(s)) != canon(s) for p = {shuffle:?}\n{}",
                subject.describe(false)
            ));
        }
        if key(&canon(&once)) != canonical {
            return Err(format!(
                "canon is not idempotent\n{}",
                subject.describe(false)
            ));
        }
        // The representative lies in the orbit of `s`: some permutation of
        // the keys maps `s` onto it.
        let mut indices: Vec<usize> = (0..n).collect();
        loop {
            let image = system
                .permute(state, &permutation(subject, &indices))
                .map_err(|stop| format!("permute: {stop:?}"))?;
            if key(&image) == canonical {
                return Ok(());
            }
            // Next permutation in lexicographic order.
            let Some(i) = (1..n).rev().find(|i| indices[i - 1] < indices[*i]) else {
                return Err(format!(
                    "canon(s) is not a permutation image of s\n{}",
                    subject.describe(false)
                ));
            };
            let j = (i..n)
                .rev()
                .find(|j| indices[*j] > indices[i - 1])
                .expect("pivot");
            indices.swap(i - 1, j);
            indices[i..].reverse();
        }
    }

    fn shuffles() -> BoxedStrategy<Vec<usize>> {
        Just((0..4usize).collect::<Vec<_>>()).prop_shuffle().boxed()
    }

    /// QSL-2061 P3 and P4 (FR-152): the sort canonicaliser is permutation
    /// invariant, idempotent, and returns a member of the state's orbit.
    #[trace("TC-569", "FR-152-AC-2")]
    #[test]
    fn canonicalise_is_permutation_invariant() {
        prop::run(
            "canonicalise_is_permutation_invariant",
            prop::config(256),
            &(subjects(2), shuffles(), 0usize..64),
            |(subject, shuffle, pick)| {
                canonical_laws(&subject, &shuffle, pick).map_err(TestCaseError::fail)
            },
        )
        .unwrap();
    }

    /// QSL-2061 P4 (FR-152): `canon(canon(s)) = canon(s)` over every reachable state.
    #[trace("TC-569", "FR-152-AC-4")]
    #[test]
    fn canonicalise_is_idempotent() {
        prop::run(
            "canonicalise_is_idempotent",
            prop::config(256),
            &subjects(2),
            |subject| {
                let world = subject.world();
                let system = world.system();
                let group = group(&subject);
                let key = |s: &ModelSimState| {
                    system
                        .state_key(s, fsl_foundation::IdentityLimits::default())
                        .expect("encodes")
                        .0
                };
                for state in super::reachable_states(&system, 64) {
                    let once = system
                        .canonicalise(&state, &group)
                        .expect("canonicalises")
                        .0;
                    let twice = system.canonicalise(&once, &group).expect("canonicalises").0;
                    prop_assert_eq!(key(&once), key(&twice), "{}", subject.describe(false));
                }
                Ok(())
            },
        )
        .unwrap();
    }
}
