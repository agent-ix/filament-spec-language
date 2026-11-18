// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: the forms of a package the package route's model-check input does
//! not read yet.
//!
//! The package route builds a model check's input from a [`BarePackage`]'s
//! admitted nodes. A node of a form it does not read yet refuses the input,
//! so no input that leaves the node out is built.

use filament_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedNodeTag, CheckedSemanticNodeV2, CompositeTypeForm,
    ModelForm, ScalarTypeForm, TemporalForm,
};

use crate::BarePackage;

/// A form of a package the package route's model-check input does not read
/// yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnreadModelForm {
    /// A state clause, frame, transition, operation anchor or snapshot.
    State,
    /// A timed model, time source, clock place or timed clause.
    TimedModel,
    /// A `terminal` declaration.
    Terminal,
    /// A protocol declaration.
    Protocol,
    /// A hybrid model.
    Hybrid,
    /// A variant or union type, which a value's canonical form names.
    Variant,
    /// An enumeration, which a value's canonical form names.
    Enumeration,
    /// An `object` declaration of the unit's own source package (FR-569).
    SourceObject,
}

impl UnreadModelForm {
    /// The form's name in a refusal record.
    pub fn name(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::TimedModel => "timed-model",
            Self::Terminal => "terminal",
            Self::Protocol => "protocol",
            Self::Hybrid => "hybrid",
            Self::Variant => "variant",
            Self::Enumeration => "enumeration",
            Self::SourceObject => "source-object",
        }
    }
}

fn unread(node: &CheckedSemanticNodeV2) -> Option<UnreadModelForm> {
    let kind = CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))?;
    match kind {
        CheckedNodeKind::State(_) => Some(UnreadModelForm::State),
        CheckedNodeKind::TimedModel(_) => Some(UnreadModelForm::TimedModel),
        CheckedNodeKind::Temporal(TemporalForm::Terminal) => Some(UnreadModelForm::Terminal),
        CheckedNodeKind::Protocol(_) => Some(UnreadModelForm::Protocol),
        CheckedNodeKind::Hybrid(_) => Some(UnreadModelForm::Hybrid),
        CheckedNodeKind::CompositeType(CompositeTypeForm::Union)
        | CheckedNodeKind::Model(ModelForm::VariantType) => Some(UnreadModelForm::Variant),
        CheckedNodeKind::ScalarType(ScalarTypeForm::Enum) => Some(UnreadModelForm::Enumeration),
        _ => None,
    }
}

impl BarePackage<'_> {
    /// The first node, in the package's node order, of a form the package
    /// route's model-check input does not read yet; else the first source
    /// object of the supply's identity rows (FR-940).
    pub fn first_unread_model_form(&self) -> Option<(UnreadModelForm, &CheckedNodeId)> {
        let found = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .find_map(|node| unread(node).map(|form| (form, &node.node_id)));
        if found.is_some() {
            return found;
        }
        let rows = self.supply()?.identities;
        rows.source_objects()
            .first()
            .map(|row| (UnreadModelForm::SourceObject, &row.structural_node))
    }
}
