// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed model-refinement rows from an admitted package.

use filament_contract_model::{
    CheckedNodeId, CheckedPackageV2, CheckedSourceRegion, CheckedTerm, RefinementKey,
    RefinementObject, RefinementPopulation, RefinementStep,
};

/// The unsupported fact that prevents a package refinement from executing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefinementViewCode {
    /// The selected node is not an admitted refinement declaration.
    WrongForm,
    /// A protocol subject or step needs a protocol refinement execution form.
    Protocol,
    /// A history row needs a package history-expression execution form.
    History,
    /// Fairness needs a package liveness execution form.
    Fairness,
    /// An external abstract owner needs its retained model-refinement form.
    ExternalOwner,
    /// A selected row has no unambiguous original source region.
    RowSource,
    /// A mapping expression or parameter is outside the supported row subset.
    Expression,
}

/// An explicit unsupported package refinement fact, retaining its stable class.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("package refinement cannot execute its {fact} form")]
pub struct RefinementViewRefusal {
    /// Stable refusal class.
    pub code: RefinementViewCode,
    /// The original unsupported fact, named independently of the diagnostic prose.
    pub fact: &'static str,
}

/// A mapping row's original unsupported context or projection refusal.
#[derive(Debug, thiserror::Error)]
pub enum RefinementRowRefusal {
    /// The admitted row needs an execution form outside the supported subset.
    #[error(transparent)]
    View(#[from] RefinementViewRefusal),
    /// Preserve the existing package projection's identity, equality or fault class.
    #[error("refinement row projection refused: {0:?}")]
    Projection(crate::BareSelectionFailure),
}

impl RefinementViewRefusal {
    pub(crate) const fn new(code: RefinementViewCode, fact: &'static str) -> Self {
        Self { code, fact }
    }
}

/// A refinement's original admitted node and borrowed, closed mapping tables.
#[derive(Clone, Copy)]
pub struct RefinementView<'p> {
    package: &'p CheckedPackageV2,
    node: &'p CheckedNodeId,
    name: &'p str,
    populations: &'p [RefinementPopulation],
    objects: &'p [RefinementObject],
    steps: &'p [RefinementStep],
    abstract_root: &'p CheckedNodeId,
    concrete_root: &'p CheckedNodeId,
}

impl<'p> RefinementView<'p> {
    /// The exact admitted owning package.
    pub fn package(self) -> &'p CheckedPackageV2 {
        self.package
    }
    /// The selected document-held refinement node.
    pub fn node(self) -> &'p CheckedNodeId {
        self.node
    }
    /// The identifier recorded by the original producer.
    pub fn name(self) -> &'p str {
        self.name
    }
    /// The original population rows, without rebuilding source checking data.
    pub fn populations(self) -> &'p [RefinementPopulation] {
        self.populations
    }
    /// The original object and field expression rows.
    pub fn objects(self) -> &'p [RefinementObject] {
        self.objects
    }
    /// The original step rows and application expressions.
    pub fn steps(self) -> &'p [RefinementStep] {
        self.steps
    }
    /// The exact admitted abstract model root.
    pub fn abstract_root(self) -> &'p CheckedNodeId {
        self.abstract_root
    }
    /// The exact admitted concrete model root.
    pub fn concrete_root(self) -> &'p CheckedNodeId {
        self.concrete_root
    }
    /// The original declaration region, when it is unambiguous.
    pub fn declaration_region(self) -> Option<&'p CheckedSourceRegion> {
        let mut regions = self
            .package
            .source_map()
            .iter()
            .filter(|entry| {
                entry.node_id == *self.node
                    && entry.role == filament_contract_model::CheckedOccurrenceRole::Declaration
                    && entry.ordinal == 0
            })
            .flat_map(|entry| &entry.regions);
        let first = regions.next()?;
        regions.next().is_none().then_some(first)
    }
    /// The source region of the actual authored step at `ordinal`.
    pub fn step_region(self, ordinal: usize) -> Option<&'p CheckedSourceRegion> {
        let ordinal = u64::try_from(ordinal).ok()?;
        self.package
            .refinement_step_rows()?
            .iter()
            .find(|row| row.node_id == *self.node && row.ordinal == ordinal)
            .map(|row| &row.region)
    }
}

pub(crate) fn refinement_view<'p>(
    package: &'p CheckedPackageV2,
    node: &CheckedNodeId,
) -> Result<RefinementView<'p>, RefinementViewRefusal> {
    use RefinementViewCode as Code;
    let node = &package
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .ok_or_else(|| RefinementViewRefusal::new(Code::WrongForm, "declaration"))?
        .node_id;
    let CheckedTerm::Refinement {
        name,
        abstract_key,
        populations,
        objects,
        steps,
        histories,
        assume,
        ensure,
        protocol,
        abstract_internal,
        class_catalog,
        ..
    } = package
        .term(node)
        .ok_or_else(|| RefinementViewRefusal::new(Code::WrongForm, "declaration"))?
    else {
        return Err(RefinementViewRefusal::new(Code::WrongForm, "declaration"));
    };
    if protocol.is_some()
        || abstract_internal
            .as_ref()
            .is_some_and(|rows| !rows.is_empty())
        || class_catalog.is_some()
    {
        return Err(RefinementViewRefusal::new(Code::Protocol, "protocol"));
    }
    if !histories.is_empty() {
        return Err(RefinementViewRefusal::new(Code::History, "history"));
    }
    if !assume.is_empty() || !ensure.is_empty() {
        return Err(RefinementViewRefusal::new(Code::Fairness, "fairness"));
    }
    let RefinementKey::Local(abstract_root) = abstract_key.as_ref() else {
        return Err(RefinementViewRefusal::new(
            Code::ExternalOwner,
            "external abstract owner",
        ));
    };
    let concrete_root = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .map(|candidate| &candidate.semantic_type)
        .ok_or_else(|| RefinementViewRefusal::new(Code::WrongForm, "concrete model root"))?;
    Ok(RefinementView {
        package,
        node,
        name,
        populations,
        objects,
        steps,
        abstract_root,
        concrete_root,
    })
}

impl crate::BarePackage<'_> {
    /// Project the original row expression over its authentic `self` binder,
    /// borrowing the already constructed model world for representation checks.
    ///
    /// # Errors
    /// Refuses ambiguous row attribution, operation-parameter rows and unsupported
    /// projections. No world, checked source expression or binder is manufactured.
    pub fn project_refinement_row(
        &self,
        term: &CheckedTerm,
        region: &CheckedSourceRegion,
        world: &crate::BareWorld,
    ) -> Result<crate::ReaderProtocolProjection, RefinementRowRefusal> {
        self.project_refinement_row_with_parameter(term, None, region, world, false)
    }

    /// Project one argument expression of an admitted refinement step,
    /// allowing its checked source-operation parameter to bind.
    pub fn project_refinement_step_argument(
        &self,
        term: &CheckedTerm,
        receiver: &CheckedTerm,
        region: &CheckedSourceRegion,
        world: &crate::BareWorld,
    ) -> Result<crate::ReaderProtocolProjection, RefinementRowRefusal> {
        self.project_refinement_row_with_parameter(term, Some(receiver), region, world, true)
    }

    fn project_refinement_row_with_parameter(
        &self,
        term: &CheckedTerm,
        receiver: Option<&CheckedTerm>,
        region: &CheckedSourceRegion,
        world: &crate::BareWorld,
        allow_operation_parameter: bool,
    ) -> Result<crate::ReaderProtocolProjection, RefinementRowRefusal> {
        use RefinementViewCode as Code;
        let unsupported = |code, fact| RefinementViewRefusal::new(code, fact);
        let CheckedTerm::Reference { target } = term else {
            return Err(unsupported(Code::Expression, "inline mapping expression").into());
        };
        let package = self.package();
        let root = self
            .node_key(target)
            .ok_or_else(|| unsupported(Code::Expression, "mapping expression identity"))?;
        let sources: Vec<_> = package
            .source_map()
            .iter()
            .filter(|entry| entry.node_id == *target)
            .flat_map(|entry| &entry.regions)
            .filter(|source| {
                source.source == region.source
                    && source.start >= region.start
                    && source.end <= region.end
            })
            .collect();
        let site = match sources.as_slice() {
            [] => return Err(unsupported(Code::RowSource, "mapping row source").into()),
            [site] => *site,
            [first, rest @ ..]
                if rest.iter().all(|site| {
                    site.source == first.source
                        && site.start == first.start
                        && site.end == first.end
                }) =>
            {
                *first
            }
            sites if allow_operation_parameter => *sites
                .iter()
                .min_by_key(|site| site.end.saturating_sub(site.start))
                .expect("a nonempty source region list"),
            _ => {
                return Err(unsupported(Code::RowSource, "ambiguous mapping row source").into());
            }
        };
        let mut binders = Vec::new();
        let mut seen_binders = std::collections::BTreeSet::new();
        let mut unsupported_parameter = false;
        let lookup = |id: &CheckedNodeId| {
            package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| node.node_id == *id)
        };
        if let Some(receiver) = receiver {
            crate::introduced_binders::reach_term(&lookup, receiver, None, &mut |node| {
                if crate::reader_protocol_projection::parameter_name(&node.body) == Some("self")
                    && seen_binders.insert(node.node_id.clone())
                {
                    binders.push(node.node_id.clone());
                }
            });
        }
        crate::introduced_binders::reach_term(&lookup, term, None, &mut |node| {
            let is_self =
                crate::reader_protocol_projection::parameter_name(&node.body) == Some("self");
            let is_allowed_parameter = allow_operation_parameter
                && crate::reader_protocol_projection::parameter_name(&node.body)
                    .is_some_and(|name| !name.is_empty() && name != "self");
            if !is_self && !is_allowed_parameter {
                unsupported_parameter = true;
                return;
            }
            if seen_binders.insert(node.node_id.clone()) {
                binders.push(node.node_id.clone());
            }
        });
        if unsupported_parameter || binders.len() > usize::from(receiver.is_some()) + 1 {
            return Err(unsupported(Code::Expression, "mapping operation parameters").into());
        }
        let binders = binders
            .iter()
            .map(|node| {
                self.node_key(node)
                    .ok_or_else(|| unsupported(Code::Expression, "mapping binder identity"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (_, identities) = self.witness().map_err(RefinementRowRefusal::Projection)?;
        let walker = crate::reader_protocol_projection::BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(|fault| {
            RefinementRowRefusal::Projection(
                crate::protocol_selection::SlotSelectionFailure::Fault(fault).into(),
            )
        })?;
        let projection = walker
            .project_term(
                &root,
                &binders,
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                    name: format!("refinement row {}..{}", site.start, site.end),
                },
            )
            .map_err(crate::bare_term::walk_failure)
            .map_err(RefinementRowRefusal::Projection)?;
        if projection.has_unrepresented_types(world.types()) {
            return Err(unsupported(Code::Expression, "mapping type representation").into());
        }
        Ok(projection)
    }
}
