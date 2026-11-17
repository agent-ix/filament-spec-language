// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-502, FR-820: the covered positions of one ledger binding, as one
//! append-only set shared by every ledger that holds it.
//!
//! A monitor carries one ledger forward and reports it after every step.
//! Each report keeps the covered set as it was at that step, so a set held
//! by value would make every step cost the whole prefix. [`CoveredList`] is a
//! persistent set: a clone is a view of the set at that moment, O(1) to
//! take, and an insertion copies only the path to the new position (expected
//! O(log n)) and leaves every earlier view unchanged. Positions are never
//! removed. The set lists its positions in identity-then-stamp order, the
//! order the ledger's wire form writes, and two sets are equal when they
//! hold the same positions.
//!
//! The set is a treap whose priorities are a hash of each position, so its
//! shape depends only on the positions it holds and not on the order they
//! arrived in.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use super::ledger::ObservedPosition;

type Link = Option<Arc<Node>>;

struct Node {
    position: ObservedPosition,
    priority: u64,
    left: Link,
    right: Link,
}

/// The covered positions of one ledger binding (FR-502): an append-only
/// set whose clones share structure (FR-820).
#[derive(Clone, Default)]
pub struct CoveredList {
    root: Link,
    len: usize,
}

fn priority_of(position: &ObservedPosition) -> u64 {
    let mut hasher = DefaultHasher::new();
    position.hash(&mut hasher);
    hasher.finish()
}

fn node(position: ObservedPosition, priority: u64, left: Link, right: Link) -> Arc<Node> {
    Arc::new(Node {
        position,
        priority,
        left,
        right,
    })
}

/// The positions of `link` below `key` and above it; `key` is not held.
fn split(link: Option<&Arc<Node>>, key: &ObservedPosition) -> (Link, Link) {
    let Some(held) = link else {
        return (None, None);
    };
    if held.position < *key {
        let (below, above) = split(held.right.as_ref(), key);
        let kept = node(
            held.position.clone(),
            held.priority,
            held.left.clone(),
            below,
        );
        (Some(kept), above)
    } else {
        let (below, above) = split(held.left.as_ref(), key);
        let kept = node(
            held.position.clone(),
            held.priority,
            above,
            held.right.clone(),
        );
        (below, Some(kept))
    }
}

/// `link` with `position`, which it does not hold, copying only the path to
/// it.
fn insert(link: Option<&Arc<Node>>, position: ObservedPosition, priority: u64) -> Arc<Node> {
    match link {
        None => node(position, priority, None, None),
        Some(held) if priority > held.priority => {
            let (left, right) = split(Some(held), &position);
            node(position, priority, left, right)
        }
        Some(held) if position < held.position => node(
            held.position.clone(),
            held.priority,
            Some(insert(held.left.as_ref(), position, priority)),
            held.right.clone(),
        ),
        Some(held) => node(
            held.position.clone(),
            held.priority,
            held.left.clone(),
            Some(insert(held.right.as_ref(), position, priority)),
        ),
    }
}

impl CoveredList {
    /// A set with no position.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many positions the set holds.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the set holds no position.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the set holds `position`.
    pub fn contains(&self, position: &ObservedPosition) -> bool {
        let mut link = self.root.as_ref();
        while let Some(held) = link {
            link = match position.cmp(&held.position) {
                std::cmp::Ordering::Less => held.left.as_ref(),
                std::cmp::Ordering::Greater => held.right.as_ref(),
                std::cmp::Ordering::Equal => return true,
            };
        }
        false
    }

    /// Add `position`; returns whether it was not held already. Every other
    /// view of the set is unchanged.
    pub fn insert(&mut self, position: ObservedPosition) -> bool {
        if self.contains(&position) {
            return false;
        }
        let priority = priority_of(&position);
        self.root = Some(insert(self.root.as_ref(), position, priority));
        self.len = self.len.saturating_add(1);
        true
    }

    /// Whether `self` and `other` are one version of one set: the same
    /// shared structure, so neither holds a position the other lacks.
    pub fn shares(&self, other: &Self) -> bool {
        self.len == other.len
            && match (&self.root, &other.root) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }

    /// The positions, in identity-then-stamp order.
    pub fn iter(&self) -> Iter<'_> {
        let mut iter = Iter { stack: Vec::new() };
        iter.descend(self.root.as_ref());
        iter
    }
}

/// The positions of a [`CoveredList`], in identity-then-stamp order.
pub struct Iter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iter<'a> {
    fn descend(&mut self, mut link: Option<&'a Arc<Node>>) {
        while let Some(held) = link {
            self.stack.push(held);
            link = held.left.as_ref();
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a ObservedPosition;

    fn next(&mut self) -> Option<Self::Item> {
        let held = self.stack.pop()?;
        self.descend(held.right.as_ref());
        Some(&held.position)
    }
}

impl<'a> IntoIterator for &'a CoveredList {
    type Item = &'a ObservedPosition;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq for CoveredList {
    fn eq(&self, other: &Self) -> bool {
        self.shares(other) || (self.len == other.len && self.iter().eq(other.iter()))
    }
}

impl Eq for CoveredList {}

impl std::fmt::Debug for CoveredList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl Extend<ObservedPosition> for CoveredList {
    fn extend<I: IntoIterator<Item = ObservedPosition>>(&mut self, positions: I) {
        for position in positions {
            self.insert(position);
        }
    }
}

impl FromIterator<ObservedPosition> for CoveredList {
    fn from_iter<I: IntoIterator<Item = ObservedPosition>>(positions: I) -> Self {
        let mut list = Self::new();
        list.extend(positions);
        list
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;

    use super::*;

    fn at(identity: &str, stamp: u64) -> ObservedPosition {
        ObservedPosition {
            identity: identity.to_owned(),
            stamp,
        }
    }

    #[trace("FR-502-AC-7", "FR-820-AC-1")]
    #[test]
    fn the_set_lists_its_positions_as_a_sorted_set_does_and_earlier_views_stay() {
        let mut list = CoveredList::new();
        let mut set = BTreeSet::new();
        let mut views = Vec::new();
        for n in 0..300u64 {
            let position = at(&format!("p{}", (n * 7919) % 211), n % 13);
            assert_eq!(list.insert(position.clone()), set.insert(position.clone()));
            assert!(list.contains(&position));
            assert!(!list.insert(position));
            views.push((list.clone(), set.clone()));
        }
        for (view, expected) in &views {
            assert_eq!(view.len(), expected.len());
            assert!(view.iter().eq(expected.iter()));
            assert_eq!(format!("{view:?}"), format!("{expected:?}"));
        }
        assert!(!list.contains(&at("absent", 0)));
        let rebuilt: CoveredList = set.iter().rev().cloned().collect();
        assert_eq!(rebuilt, list);
        assert!(!rebuilt.shares(&list));
        assert!(list.clone().shares(&list));
    }
}
