// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checker locations' child-index paths, interned as one tree. A checker path
//! is as long as its position is deep, so a copy per position grows with the
//! square of the nesting depth; the tree holds each shared step once.

use std::collections::HashMap;

use filament_semantic_value::location::ExpressionLocation as Location;

/// One step of the tree: the path `parent` extended by the child index `index`.
pub(super) struct Step {
    pub(super) parent: usize,
    pub(super) index: usize,
    /// Steps from the empty path to here, this one included.
    pub(super) depth: usize,
}

/// Interned paths, equal for equal index sequences whatever their origin and
/// however their links were built.
#[derive(Default)]
pub(super) struct Paths {
    addresses: HashMap<usize, usize>,
    // Keep memoized link addresses alive, including independently built paths.
    retained: Vec<Location>,
    children: HashMap<(usize, usize), usize>,
    // Entry zero is the empty path; every step follows its parent.
    steps: Vec<Step>,
}

impl Paths {
    /// The id of `location`'s path. Costs the links no earlier call has seen.
    pub(super) fn intern(&mut self, location: &Location) -> usize {
        if self.steps.is_empty() {
            self.steps.push(Step {
                parent: 0,
                index: 0,
                depth: 0,
            });
        }
        if let Some(address) = location.chain_address() {
            if let Some(known) = self.addresses.get(&address) {
                return *known;
            }
            self.retained.push(location.clone());
        }
        let mut pending = Vec::new();
        let mut parent = 0;
        for (address, index) in location.ancestry() {
            if let Some(known) = self.addresses.get(&address) {
                parent = *known;
                break;
            }
            pending.push((address, index));
        }
        for (address, index) in pending.into_iter().rev() {
            let child = *self.children.entry((parent, index)).or_insert_with(|| {
                let child = self.steps.len();
                let depth = self.steps[parent].depth + 1;
                self.steps.push(Step {
                    parent,
                    index,
                    depth,
                });
                child
            });
            self.addresses.insert(address, child);
            parent = child;
        }
        parent
    }

    /// The last step of the path `id`.
    pub(super) fn step(&self, id: usize) -> Option<&Step> {
        self.steps.get(id)
    }

    /// Each interned path's place, by id, in the lexicographic order of the
    /// index sequences (a path before its extensions): the source order of the
    /// positions.
    pub(super) fn lexicographic_ranks(&self) -> Vec<usize> {
        let mut edges: Vec<(usize, usize, usize)> = self
            .steps
            .iter()
            .enumerate()
            .skip(1)
            .map(|(child, step)| (step.parent, step.index, child))
            .collect();
        edges.sort_unstable();
        let mut ranks = vec![0; self.steps.len()];
        let mut pending: Vec<usize> = (!self.steps.is_empty()).then_some(0).into_iter().collect();
        let mut next = 0;
        while let Some(at) = pending.pop() {
            ranks[at] = next;
            next += 1;
            let first = edges.partition_point(|edge| edge.0 < at);
            let end = edges.partition_point(|edge| edge.0 <= at);
            pending.extend(edges[first..end].iter().rev().map(|edge| edge.2));
        }
        ranks
    }
}

#[cfg(test)]
mod tests {
    use filament_semantic_value::location::ExpressionOwner;

    use super::*;

    fn at(path: &[usize]) -> Location {
        Location::at(ExpressionOwner::Expression, path)
    }

    /// Ranks order paths as their index sequences order: a prefix before its
    /// extensions and siblings by index, whatever order they were interned in.
    /// Equal paths built apart are one path.
    #[test]
    fn ranks_follow_the_index_sequences_and_equal_paths_share_an_id() {
        let mut paths = Paths::default();
        let ids: Vec<usize> = [
            &[1, 0][..],
            &[0, 2],
            &[],
            &[1],
            &[0],
            &[0, 2, 7],
            &[0, 10],
            &[1, 0],
        ]
        .into_iter()
        .map(|path| paths.intern(&at(path)))
        .collect();
        assert_eq!(ids[0], ids[7]);
        let ranks = paths.lexicographic_ranks();
        let ranked: Vec<usize> = ids.iter().map(|&id| ranks[id]).collect();
        // [] [0] [0,2] [0,2,7] [0,10] [1] [1,0]
        assert_eq!(ranked, [6, 2, 0, 5, 1, 3, 4, 6]);
    }

    /// A path shares its parent's steps: a chain interned at every position
    /// holds one step per level and the empty path.
    #[test]
    fn a_chain_interned_at_every_position_holds_one_step_per_level() {
        let mut paths = Paths::default();
        let mut location = at(&[]);
        for _ in 0..1_000 {
            location = location.child(1);
            paths.intern(&location);
        }
        assert_eq!(paths.lexicographic_ranks().len(), 1_001);
    }
}
