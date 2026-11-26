// SPDX-License-Identifier: AGPL-3.0-or-later
//! The lowered expression tree and the text emitter.
//!
//! A lowered expression is as deep as its source, so no walk of it recurses
//! (ADR-030 D-1). Nodes live in one arena, each after its children, and name
//! their children by id: dropping and cloning never follow a child. Equal
//! trees are stored once, so comparing two ids compares the trees.

use super::{refusal, Lower, Op, Refusal};
use filament_walk::{walk, Arena, Children, Id, Walk};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::ControlFlow;

pub(super) type ExprId = Id<E>;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum E {
    Int(String),
    Bool(bool),
    Float64Bits(String),
    Enum {
        owner: String,
        case: String,
    },
    Param(String),
    Field {
        object: usize,
        key: ExprId,
        field: usize,
        primed: bool,
    },
    Neg(ExprId),
    FloatNeg(ExprId),
    Not(ExprId),
    Bin(Op, ExprId, ExprId),
    Conj(Vec<ExprId>),
}

impl E {
    fn children(&self) -> Vec<ExprId> {
        match self {
            Self::Int(_)
            | Self::Bool(_)
            | Self::Float64Bits(_)
            | Self::Enum { .. }
            | Self::Param(_) => Vec::new(),
            Self::Field { key: inner, .. }
            | Self::Neg(inner)
            | Self::FloatNeg(inner)
            | Self::Not(inner) => vec![*inner],
            Self::Bin(_, left, right) => vec![*left, *right],
            Self::Conj(items) => items.clone(),
        }
    }
}

/// Every lowered expression of one export.
#[derive(Default)]
pub(super) struct Exprs {
    nodes: Arena<E>,
    ids: BTreeMap<E, ExprId>,
    /// Whether each node reads binary64, by its arena position.
    float64: Vec<bool>,
}

impl Exprs {
    /// The id of `node`, whose children are already ids of this arena.
    pub(super) fn add(&mut self, node: E) -> ExprId {
        if let Some(&id) = self.ids.get(&node) {
            return id;
        }
        let float64 = match &node {
            E::Float64Bits(_) | E::FloatNeg(_) | E::Bin(Op::EqFloat, _, _) => true,
            _ => node
                .children()
                .into_iter()
                .any(|child| self.uses_float64(child)),
        };
        let id = self.nodes.push(node.clone());
        self.float64.push(float64);
        self.ids.insert(node, id);
        id
    }

    pub(super) fn get(&self, id: ExprId) -> Lower<&E> {
        self.nodes
            .get(id)
            .ok_or_else(|| refusal("checked package", "a lowered expression is missing"))
    }

    pub(super) fn uses_float64(&self, id: ExprId) -> bool {
        self.float64.get(id.index()).copied().unwrap_or(false)
    }

    /// `root` with the parameter `from` renamed to `to`.
    pub(super) fn rename_param(&mut self, root: ExprId, from: &str, to: &str) -> ExprId {
        let mut rename = Rename {
            exprs: self,
            from,
            to,
            done: BTreeMap::new(),
        };
        match walk(&mut rename, root) {
            ControlFlow::Continue(()) => rename.renamed(root),
            ControlFlow::Break(never) => match never {},
        }
    }
}

struct Rename<'a> {
    exprs: &'a mut Exprs,
    from: &'a str,
    to: &'a str,
    done: BTreeMap<ExprId, ExprId>,
}

impl Rename<'_> {
    fn renamed(&self, id: ExprId) -> ExprId {
        self.done.get(&id).copied().unwrap_or(id)
    }
}

impl Walk for Rename<'_> {
    type Node = ExprId;
    type Frame = ExprId;
    type Stop = Infallible;

    fn enter(
        &mut self,
        id: ExprId,
        children: &mut Children<'_, ExprId>,
    ) -> ControlFlow<Infallible, ExprId> {
        if !self.done.contains_key(&id) {
            if let Ok(node) = self.exprs.get(id) {
                children.extend(node.children());
            }
        }
        ControlFlow::Continue(id)
    }

    fn exit(&mut self, id: ExprId) -> ControlFlow<Infallible> {
        if self.done.contains_key(&id) {
            return ControlFlow::Continue(());
        }
        let Ok(node) = self.exprs.get(id) else {
            return ControlFlow::Continue(());
        };
        let mut node = node.clone();
        match &mut node {
            E::Param(name) if name == self.from => self.to.clone_into(name),
            E::Field { key: inner, .. } | E::Neg(inner) | E::FloatNeg(inner) | E::Not(inner) => {
                *inner = self.renamed(*inner);
            }
            E::Bin(_, left, right) => {
                *left = self.renamed(*left);
                *right = self.renamed(*right);
            }
            E::Conj(items) => {
                for item in items {
                    *item = self.renamed(*item);
                }
            }
            E::Int(_) | E::Bool(_) | E::Float64Bits(_) | E::Enum { .. } | E::Param(_) => {}
        }
        let renamed = self.exprs.add(node);
        self.done.insert(id, renamed);
        ControlFlow::Continue(())
    }
}

/// One piece of a text: literal text, or a node whose text goes here.
pub(super) enum Piece<N> {
    Text(Cow<'static, str>),
    Node(N),
}

impl<N> Piece<N> {
    pub(super) fn text(text: impl Into<Cow<'static, str>>) -> Self {
        Self::Text(text.into())
    }
}

/// The text of `root`. `expand` appends the pieces of one node in written
/// order, and nodes are expanded in the order their text appears.
pub(super) fn render<N>(
    root: N,
    expand: impl FnMut(N, &mut Vec<Piece<N>>) -> Lower<()>,
) -> Lower<String> {
    let mut render = Render {
        text: String::new(),
        expand,
        node: PhantomData,
    };
    match walk(&mut render, Piece::Node(root)) {
        ControlFlow::Continue(()) => Ok(render.text),
        ControlFlow::Break(refusal) => Err(refusal),
    }
}

struct Render<N, F> {
    text: String,
    expand: F,
    node: PhantomData<fn(N)>,
}

impl<N, F: FnMut(N, &mut Vec<Piece<N>>) -> Lower<()>> Walk for Render<N, F> {
    type Node = Piece<N>;
    type Frame = ();
    type Stop = Refusal;

    fn enter(
        &mut self,
        piece: Piece<N>,
        children: &mut Children<'_, Piece<N>>,
    ) -> ControlFlow<Refusal, ()> {
        match piece {
            Piece::Text(text) => self.text.push_str(&text),
            Piece::Node(node) => {
                let mut pieces = Vec::new();
                if let Err(refusal) = (self.expand)(node, &mut pieces) {
                    return ControlFlow::Break(refusal);
                }
                children.extend(pieces);
            }
        }
        ControlFlow::Continue(())
    }

    fn exit(&mut self, (): ()) -> ControlFlow<Refusal> {
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Exprs, E};
    use ix_trace_rs::trace;

    /// A dense predicate quantifies its key, also under a float negation.
    #[test]
    #[trace("FSpec-FR-813-AC-18")]
    fn a_parameter_under_a_float_negation_is_renamed() {
        let mut exprs = Exprs::default();
        let receiver = exprs.add(E::Param("r_self".to_owned()));
        let negated = exprs.add(E::FloatNeg(receiver));
        let renamed = exprs.rename_param(negated, "r_self", "k");
        let key = exprs.add(E::Param("k".to_owned()));
        let expected = exprs.add(E::FloatNeg(key));
        assert!(renamed == expected, "the operand keeps its old name");
        assert!(exprs.rename_param(negated, "other", "k") == negated);
    }
}
