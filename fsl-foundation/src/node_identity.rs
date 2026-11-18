// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL-specific borrowed structural identity projections layered over
//! `filament_exact::node_wire`.

use alloc::vec::Vec;
use filament_exact::{InvalidNodeKey, NodeKey, NodePathIter, PathSegmentView};

/// Borrowed events and admitted traversal storage for the single structural
/// node-identity projection. The sink chooses how to retain or emit events;
/// this trait itself supplies no canonical writer, byte limit or allocation.
/// Every sink refusal propagates through its associated error unchanged.
pub trait NodeIdentitySink {
    /// The sink's typed refusal.
    type Error;
    /// Admit one traversal event.
    fn visit(&mut self) -> Result<(), Self::Error>;
    /// Begin a structural object.
    fn begin_object(&mut self) -> Result<(), Self::Error>;
    /// Name a structural member.
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error>;
    /// Emit the admitted ordinal as a JSON number.
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error>;
    /// Emit exact borrowed text.
    fn text(&mut self, text: &str) -> Result<(), Self::Error>;
    /// Begin a path.
    fn begin_array(&mut self) -> Result<(), Self::Error>;
    /// End a path.
    fn end_array(&mut self) -> Result<(), Self::Error>;
    /// End a structural object.
    fn end_object(&mut self) -> Result<(), Self::Error>;
    /// Admit and retain a traversal frame before its stack grows.
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error>;
}

struct CanonicalNode<'w, 's, S: quire_canonical::Sink + ?Sized>(
    &'w mut quire_canonical::Writer<'s, S>,
);
impl<S: quire_canonical::Sink + ?Sized> NodeIdentitySink for CanonicalNode<'_, '_, S> {
    type Error = quire_canonical::Error;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.0.begin_object()
    }
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        self.0.name(name)
    }
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error> {
        self.0.integer(i128::from(ordinal))
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0.string(text)
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        self.0.begin_array()
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        self.0.end_array()
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.0.end_object()
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.0.reserve_list(stack, 1)?;
        stack.push(path);
        Ok(())
    }
}

fn project_node_head<S: NodeIdentitySink>(
    package: &str,
    ordinal: u64,
    sink: &mut S,
) -> Result<(), S::Error> {
    sink.visit()?;
    sink.begin_object()?;
    sink.name("ordinal")?;
    sink.ordinal(ordinal)?;
    sink.name("package")?;
    sink.text(package)?;
    sink.name("path")?;
    sink.begin_array()
}

/// A validated structural node identity with a fixed borrowed text path.
/// It uses the same canonical projection as [`encode_node_key`] without
/// constructing an owned node identity, text or JSON tree.
#[derive(Clone, Copy, Debug)]
pub struct TextNodeIdentity<'a, const N: usize> {
    package: &'a str,
    path: [&'a str; N],
    ordinal: u64,
}

impl<'a, const N: usize> TextNodeIdentity<'a, N> {
    /// Validate the same package, nonempty path and ordinal constraints as
    /// [`NodeKey::new`], without allocating. Path text is opaque.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey::EmptyPackage`], [`InvalidNodeKey::EmptyPath`] or
    /// [`InvalidNodeKey::OrdinalOutOfRange`] for invalid structural input.
    pub fn new(package: &'a str, path: [&'a str; N], ordinal: u64) -> Result<Self, InvalidNodeKey> {
        if package.is_empty() {
            return Err(InvalidNodeKey::EmptyPackage);
        }
        if N == 0 {
            return Err(InvalidNodeKey::EmptyPath);
        }
        if ordinal > filament_exact::MAX_NODE_ORDINAL {
            return Err(InvalidNodeKey::OrdinalOutOfRange);
        }
        Ok(Self {
            package,
            path,
            ordinal,
        })
    }

    /// Emit this borrowed identity through the shared structural projection.
    ///
    /// # Errors
    /// The sink's admission or encoding refusal.
    pub fn project<S: NodeIdentitySink>(&self, sink: &mut S) -> Result<(), S::Error> {
        project_node_head(self.package, self.ordinal, sink)?;
        for segment in self.path {
            sink.visit()?;
            sink.text(segment)?;
        }
        sink.end_array()?;
        sink.end_object()
    }

    /// The owning package text.
    pub fn package(&self) -> &'a str {
        self.package
    }

    /// The direct opaque path segments.
    pub fn path(&self) -> &[&'a str; N] {
        &self.path
    }
}

impl<const N: usize> quire_canonical::Encode for TextNodeIdentity<'_, N> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        self.project(&mut CanonicalNode(writer))
    }
}

/// Emit a structural node identity directly into a canonical writer.
///
/// The writer admits its own storage and this traversal's stack before growth.
/// No intermediate identity text or JSON tree is retained.
///
/// # Errors
///
/// The writer's byte, reservation, allocation or scalar refusal.
pub fn encode_node_key<S: quire_canonical::Sink + ?Sized>(
    key: &NodeKey,
    writer: &mut quire_canonical::Writer<'_, S>,
) -> Result<(), quire_canonical::Error> {
    project_node_key(key, &mut CanonicalNode(writer))
}

/// Emit a node identity through the same structural recipe as the canonical
/// writer, admitting traversal and storage through the supplied sink.
///
/// # Errors
/// The sink's admission or encoding refusal.
pub fn project_node_key<S: NodeIdentitySink>(key: &NodeKey, sink: &mut S) -> Result<(), S::Error> {
    let mut stack = Vec::new();
    let root = key.view();
    project_node_head(root.package(), root.ordinal(), sink)?;
    sink.push_path(&mut stack, root.path())?;
    while let Some(path) = stack.last_mut() {
        sink.visit()?;
        match path.next() {
            Some(PathSegmentView::Text(text)) => sink.text(text)?,
            Some(PathSegmentView::Node(view)) => {
                project_node_head(view.package(), view.ordinal(), sink)?;
                sink.push_path(&mut stack, view.path())?;
            }
            None => {
                sink.end_array()?;
                sink.end_object()?;
                stack.pop();
            }
        }
    }
    Ok(())
}
