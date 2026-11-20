// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small independent counter systems, rendered through the source and library APIs.

use super::value::VALUE_HEADER;
use crate::world::{Model, Operation};
use proptest::prelude::{BoxedStrategy, Strategy};

/// Independent description of a finite product of saturating counters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmallModel {
    processes: u8,
    maximum: u8,
}

impl SmallModel {
    /// Number of independently scheduled counters (between one and three).
    pub fn processes(&self) -> u8 {
        self.processes
    }

    /// Inclusive upper bound for each counter (between one and three).
    pub fn maximum(&self) -> u8 {
        self.maximum
    }
    /// Independent product-state bound, including the initial state.
    pub fn state_bound(&self) -> usize {
        (usize::from(self.maximum) + 1).pow(u32::from(self.processes))
    }

    /// Independent initial valuation, in process order.
    pub fn initial(&self) -> Vec<u8> {
        vec![0; usize::from(self.processes)]
    }

    /// One enabled process advances, or stutters at its maximum.
    /// Invalid valuations have no successors.
    pub fn successors(&self, state: &[u8]) -> Vec<Vec<u8>> {
        if state.len() != usize::from(self.processes)
            || state.iter().any(|value| *value > self.maximum)
        {
            return Vec::new();
        }
        (0..state.len())
            .map(|index| {
                let mut next = state.to_vec();
                next[index] = next[index].saturating_add(1).min(self.maximum);
                next
            })
            .collect()
    }

    fn post(&self) -> String {
        format!("(pre(self.value) < {0} and self.value = pre(self.value) + 1) or (pre(self.value) = {0} and self.value = pre(self.value))", self.maximum)
    }

    /// Standalone native object/operation declarations, with an admitted precondition.
    pub fn source(&self) -> String {
        format!("{VALUE_HEADER}object Counter {{ value: Int[0, {}]; population counters; operation inc() modifies self.value; }}\npre Enabled using v on Counter::inc {{ self.value <= {} }}\npost Advances using v on Counter::inc {{ {} }}\n", self.maximum, self.maximum, self.post())
    }

    /// The same domain and transition relation as the benchmark library representation.
    /// The library representation additionally carries the closed initial population.
    pub fn library(&self) -> Model {
        Model {
            package: "bench/property-counters".into(), object: "Counter".into(), population: "counters".into(), symmetric: false,
            integers: vec![("Level".into(), 0, i64::from(self.maximum))],
            fields: vec![("value".into(), "ix://bench/property-counters/Level".into())],
            operations: vec![Operation { name: "inc".into(), parameters: Vec::new(), modifies: vec!["value".into()] }],
            profiles: String::new(),
            body: format!("pre Enabled using v on M::Counter::inc {{ self.value <= {} }}\npost Advances using v on M::Counter::inc {{ {} }}", self.maximum, self.post()),
            objects: (0..self.processes).map(|index| (format!("counter{index}"), vec![("value".into(), 0)])).collect(),
            unrelated: 0,
        }
    }
}

/// One to three processes, each with a one-to-three counter upper bound.
pub fn small_models() -> BoxedStrategy<SmallModel> {
    (1u8..=3, 1u8..=3)
        .prop_map(|(processes, maximum)| SmallModel { processes, maximum })
        .boxed()
}
