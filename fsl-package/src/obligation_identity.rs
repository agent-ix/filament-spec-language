// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-09: structural identity of one checked obligation.

use alloc::{boxed::Box, string::String};
use core::fmt;

use filament_exact::NodeKey;

/// The identity of the domain package that owns an obligation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelIdentity(String);

impl ModelIdentity {
    /// The identity text of the selected domain package.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// The identity text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&fsl_semantics::model::domain_package::DomainPackageRef> for ModelIdentity {
    fn from(selection: &fsl_semantics::model::domain_package::DomainPackageRef) -> Self {
        Self(selection.identity.clone())
    }
}

/// The structural identity of one obligation: its node, optional owning
/// domain package, and occurrence ordinal.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObligationIdentity(Box<ObligationParts>);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ObligationParts {
    node: NodeKey,
    model: Option<ModelIdentity>,
    ordinal: u32,
}

impl ObligationIdentity {
    /// The identity of a checked claim occurrence.
    pub fn from_claim_occurrence(
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
    ) -> Self {
        let ordinal = u32::try_from(claim.origin().ordinal()).unwrap_or(u32::MAX);
        Self::new(claim.node().clone(), None, ordinal)
    }

    /// The obligation of `node` owned by `model` at occurrence `ordinal`.
    pub fn new(node: NodeKey, model: Option<ModelIdentity>, ordinal: u32) -> Self {
        Self(Box::new(ObligationParts {
            node,
            model,
            ordinal,
        }))
    }

    /// The claim or obligation node.
    pub fn node(&self) -> &NodeKey {
        &self.0.node
    }

    /// The owning model, if any.
    pub fn model(&self) -> Option<&ModelIdentity> {
        self.0.model.as_ref()
    }

    /// The occurrence ordinal.
    pub fn ordinal(&self) -> u32 {
        self.0.ordinal
    }
}

impl fmt::Display for ObligationIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.node)?;
        if let Some(model) = &self.0.model {
            write!(f, "@{}", model.as_str())?;
        }
        write!(f, "#{}", self.0.ordinal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{Origin, PathSegment, Role};
    use fsl_foundation::source::provenance::OccurrenceKey;

    fn node(ordinal: u64) -> NodeKey {
        NodeKey::new("pkg", vec![PathSegment::Text("claim".to_owned())], ordinal).unwrap()
    }

    #[test]
    fn identity_equality_is_componentwise() {
        let a = ObligationIdentity::new(node(7), None, 0);
        assert_eq!(a, ObligationIdentity::new(node(7), None, 0));
        assert_ne!(a, ObligationIdentity::new(node(8), None, 0));
        assert_ne!(a, ObligationIdentity::new(node(7), None, 1));
        assert_ne!(
            a,
            ObligationIdentity::new(node(7), Some(ModelIdentity::new("m")), 0)
        );
    }

    #[test]
    fn claim_occurrence_mints_the_obligation_identity() {
        let claim = OccurrenceKey::new(node(3), Origin::new(Role::new("claim"), 17));
        assert_eq!(
            ObligationIdentity::from_claim_occurrence(&claim),
            ObligationIdentity::new(node(3), None, 17)
        );
    }
}
