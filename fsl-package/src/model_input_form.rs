// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: the forms of a package the package route's model-check input does
//! not read yet.
//!
//! The package route builds a model check's input from a [`BarePackage`]'s
//! admitted nodes. A node of a form it does not read yet refuses the input,
//! so no input that leaves the node out is built.

use filament_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedNodeTag, CheckedSemanticNodeV2, CompositeTypeForm,
    ModelForm, ScalarTypeForm, StateForm,
};

use crate::BarePackage;

/// A form of a package the package route's model-check input does not read
/// yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnreadModelForm {
    /// A Boolean-field transition or a snapshot. A state clause, frame and
    /// operation anchor are read: the clause by its `claim_names` row and
    /// package form, the frame and anchor by theirs (FR-940).
    State,
    /// A timed model, time source, clock place or timed clause.
    TimedModel,
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
            Self::Hybrid => "hybrid",
            Self::Variant => "variant",
            Self::Enumeration => "enumeration",
            Self::SourceObject => "source-object",
        }
    }
}

fn unread(package: &BarePackage<'_>, node: &CheckedSemanticNodeV2) -> Option<UnreadModelForm> {
    let kind = CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))?;
    match kind {
        CheckedNodeKind::State(
            StateForm::StateClause | StateForm::Frame | StateForm::OperationAnchor,
        ) => None,
        CheckedNodeKind::State(_) => Some(UnreadModelForm::State),
        CheckedNodeKind::TimedModel(_) => Some(UnreadModelForm::TimedModel),
        // FR-940: protocol declarations are read through
        // `AnalysisView::protocol_execution_named` and the typed package
        // control/role surfaces in `protocol_view`.
        CheckedNodeKind::Protocol(_) => None,
        CheckedNodeKind::Hybrid(_) => Some(UnreadModelForm::Hybrid),
        CheckedNodeKind::Model(ModelForm::VariantType) => {
            // A payload-free enumeration is read from its complete shape
            // row (FR-531); a variant with payloads has none.
            let read = package
                .supply()
                .is_some_and(|supply| supply.identities.holds_model_enum(&node.node_id));
            (!read).then_some(UnreadModelForm::Variant)
        }
        CheckedNodeKind::CompositeType(CompositeTypeForm::Union) => Some(UnreadModelForm::Variant),
        CheckedNodeKind::ScalarType(ScalarTypeForm::Enum) => Some(UnreadModelForm::Enumeration),
        _ => None,
    }
}

impl BarePackage<'_> {
    /// The first node, in the package's node order, of a form the package
    /// route's model-check input does not read yet. A source object is read
    /// by no node and is not one.
    pub fn first_unread_node_form(&self) -> Option<(UnreadModelForm, &CheckedNodeId)> {
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .find_map(|node| unread(self, node).map(|form| (form, &node.node_id)))
    }

    /// The first node, in the package's node order, of a form the package
    /// route's model-check input does not read yet; else the first source
    /// object of the supply's identity rows (FR-940). The source model the
    /// rows describe holds no field, population or effect frame
    /// ([`BarePackage::source_model`]), so no input that steps it is built.
    pub fn first_unread_model_form(&self) -> Option<(UnreadModelForm, &CheckedNodeId)> {
        self.first_unread_observation_form(false)
    }

    /// Unread forms for a supplied timed trace, whose stamps replace timed-model stepping.
    pub fn first_unread_timed_trace_form(&self) -> Option<(UnreadModelForm, &CheckedNodeId)> {
        self.first_unread_observation_form(true)
    }

    fn first_unread_observation_form(
        &self,
        timed_trace: bool,
    ) -> Option<(UnreadModelForm, &CheckedNodeId)> {
        let found = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .find_map(|node| {
                unread(self, node)
                    .filter(|form| !(timed_trace && *form == UnreadModelForm::TimedModel))
                    .map(|form| (form, &node.node_id))
            });
        if found.is_some() {
            return found;
        }
        let rows = self.supply()?.identities;
        rows.source_objects()
            .first()
            .map(|row| (UnreadModelForm::SourceObject, &row.structural_node))
    }
}
