// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the definitions of the
//! source-owned units the package's quantities name, read from the package's
//! own unit and dimension nodes through Contract IR's
//! `read_unit_definition` and admitted by the same `admit_unit_graph` the
//! source route admits its units through, so that a bytes-only evaluation
//! resolves a quantity's unit exactly as the source route does.

use filament_contract_model::nominal_reads::{
    NominalDeclarationRow, NominalIdentity, NominalReadCode,
};
use filament_contract_model::{
    CheckedNodeOwner, CheckedPackageV2, CheckedRational, NativeJoinBudget, SelectedSourceUnitRow,
};
use filament_exact::{Integer, NodeKey, Rational};
use filament_semantic_value::quantity::UnitTable;
use fsl_semantics::value::{
    admit_unit_graph, DimensionPreimage, NodeOwner, OwnerSelection, OwnerSubject, UnitPreimage,
};

use crate::checked_v2::wire_graph;

/// Admitted nominal definitions retained for the complete runtime world.
#[derive(Clone, Debug, Default)]
pub(crate) struct UnitDefinitions {
    dimensions: Vec<(DimensionPreimage, NodeKey)>,
    units: Vec<(UnitPreimage, NodeKey)>,
    owners: Vec<NodeOwner>,
    admitted: UnitTable,
}

impl UnitDefinitions {
    /// Read the authentic closure for a source-owned base Time dimension
    /// and a direct non-affine unit. Derived units remain unsupported here.
    pub(crate) fn direct_time_nodes(
        &self,
        package: &CheckedPackageV2,
        unit: &NodeKey,
        dimension: &NodeKey,
    ) -> Option<(String, Vec<fsl_semantics::check::TimeNominalNode>)> {
        use fsl_semantics::check::{TimeNominalForm, TimeNominalNode, TimeNominalOrigin};
        use fsl_semantics::value::nominal_payload::{
            DimensionPayload, NominalPayload, UnitPayload,
        };
        let (dimension_definition, _) = self.dimensions.iter().find(|(_, key)| key == dimension)?;
        if !dimension_definition.is_base()
            || !dimension_definition.terms().is_empty()
            || dimension_definition.qualified_declaration() != ["Time"]
        {
            return None;
        }
        let (unit_definition, _) = self.units.iter().find(|(_, key)| key == unit)?;
        let payload = UnitPayload::of(unit_definition)?;
        if payload.dimension() != dimension
            || payload.target().is_some()
            || payload.offset().numerator() != &Integer::from(0_i64)
        {
            return None;
        }
        let [unit_name] = unit_definition.qualified_declaration() else {
            return None;
        };
        let graph = wire_graph(package).ok()?;
        let origin = |key: &NodeKey, owner: &NodeOwner, qualified: &[String]| {
            let NodeOwner::Source(owner) = owner else {
                return None;
            };
            let node = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| graph.key(&node.node_id).ok().as_ref() == Some(key))?;
            let mut regions = package
                .source_map()
                .iter()
                .filter(|entry| {
                    entry.node_id == node.node_id
                        && entry.role == filament_contract_model::CheckedOccurrenceRole::Declaration
                })
                .flat_map(|entry| &entry.regions);
            let region = regions.next()?;
            if regions.next().is_some() {
                return None;
            }
            let region = crate::checked_v2::source_region(region).ok()?;
            Some(TimeNominalOrigin::Source {
                owner: owner.clone(),
                qualified_declaration: qualified.to_vec(),
                name_span: fsl_foundation::Span {
                    start: usize::try_from(region.start()).ok()?,
                    end: usize::try_from(region.end()).ok()?,
                },
            })
        };
        let dimension_node = TimeNominalNode {
            key: dimension.clone(),
            form: TimeNominalForm::Dimension,
            origin: origin(
                dimension,
                dimension_definition.owner(),
                dimension_definition.qualified_declaration(),
            )?,
            dependencies: Vec::new(),
            nominal: Some(NominalPayload::Dimension(DimensionPayload::of(
                dimension_definition,
            ))),
        };
        let unit_node = TimeNominalNode {
            key: unit.clone(),
            form: TimeNominalForm::Unit,
            origin: origin(
                unit,
                unit_definition.owner(),
                unit_definition.qualified_declaration(),
            )?,
            dependencies: vec![dimension.clone()],
            nominal: Some(NominalPayload::Unit(payload)),
        };
        Some((unit_name.clone(), vec![dimension_node, unit_node]))
    }

    /// Read clock metadata from the table admitted with these definitions.
    pub(crate) fn time_unit_scale(&self, unit: &str) -> Option<Rational> {
        use filament_semantic_value::quantity::QuantityUnit;
        use filament_semantic_value::unit::NominalUnitForm;
        let named = |segments: &[String], name: &str| {
            segments.last().is_some_and(|last| last == name) || segments.join("::") == name
        };
        self.admitted.ids().find_map(|id| {
            let QuantityUnit::Declared(declared) = self.admitted.get(id)? else {
                return None;
            };
            let node = self.admitted.nominal_node(&declared.key())?;
            if !matches!(node.form, NominalUnitForm::Unit { .. })
                || !named(&node.declaration.qualified_declaration, unit)
                || declared.is_affine()
            {
                return None;
            }
            let dimension = self.admitted.nominal_node(&declared.dimension_node())?;
            named(&dimension.declaration.qualified_declaration, "Time")
                .then(|| declared.canonical().scale().clone())
        })
    }
}

/// Why the selected source-unit rows are no unit table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitDefinitionRefusal {
    /// IR's nominal read refused with its own code; `LimitReached` is the
    /// caller's native allowance running out.
    Read(NominalReadCode),
    /// The read definitions are not an admitted unit graph.
    Graph,
    /// A selected row names no node of the package.
    Unjoined,
}

/// A checked rational as `Rational`, only when its spelling is the reduced one:
/// a preimage carries canonical spellings, so an unreduced spelling is refused
/// rather than silently normalised.
fn rational(value: &CheckedRational) -> Option<Rational> {
    let parsed = Rational::new(
        value.numerator.parse::<Integer>().ok()?,
        value.denominator.parse::<Integer>().ok()?,
    )
    .ok()?;
    (parsed.numerator().to_string() == *value.numerator
        && parsed.denominator().to_string() == *value.denominator)
        .then_some(parsed)
}

/// The preimage owner of a source-owned node; only a source owns a unit or
/// dimension the bare route reads.
fn owner(package_name: &str, owner: Option<&CheckedNodeOwner>) -> Option<NodeOwner> {
    match owner? {
        CheckedNodeOwner::Source {
            authority,
            identity,
        } => Some(NodeOwner::Source(OwnerSubject::new(
            &**authority,
            &**identity,
            package_name,
        ))),
        CheckedNodeOwner::ModelRoot { .. } | CheckedNodeOwner::Model { .. } => None,
    }
}

fn name(declaration: Option<&filament_contract_model::CheckedDeclaration>) -> Option<Vec<String>> {
    Some(
        declaration?
            .qualified_name
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
}

/// The unit table of the source-owned units `rows` select, charged to
/// `budget`. A selected row that IR already joined to the package's nodes is
/// read; the read and the graph's admission are IR's and the source route's,
/// not a second implementation.
pub(crate) fn read_unit_table(
    package: &CheckedPackageV2,
    rows: &[SelectedSourceUnitRow],
    budget: &mut NativeJoinBudget,
) -> Result<UnitDefinitions, UnitDefinitionRefusal> {
    use UnitDefinitionRefusal::{Graph, Read, Unjoined};
    if rows.is_empty() {
        return Ok(UnitDefinitions::default());
    }
    let wire = wire_graph(package).map_err(|_| Graph)?;
    let key = |id: &filament_contract_model::CheckedNodeId| -> Result<NodeKey, _> {
        wire.key(id).map_err(|_| Graph)
    };
    let mut declared = Vec::new();
    for row in rows {
        let node = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.node_id == row.structural_node)
            .ok_or(Unjoined)?;
        let (Some(owner), Some(declaration)) = (node.owner.as_ref(), node.declaration.as_ref())
        else {
            return Err(Unjoined);
        };
        declared.push(NominalDeclarationRow {
            node: &row.structural_node,
            owner,
            declaration,
            identity: NominalIdentity::Unit(&row.unit_id),
        });
    }
    let requested: Vec<_> = rows.iter().map(|row| row.structural_node.clone()).collect();
    let read = package
        .read_unit_definition(&requested, &declared, budget)
        .map_err(|refusal| Read(refusal.code))?;
    let mut dimensions = Vec::new();
    for dimension in &read.dimensions {
        let mut terms = Vec::new();
        for term in dimension.terms {
            terms.push((
                key(&term.dimension_node_id)?,
                term.exponent.parse::<Integer>().map_err(|_| Graph)?,
            ));
        }
        terms.sort_by(|left, right| left.0.cmp(&right.0));
        // A dimension no selected source owns is not in the owner selection.
        let owner = owner(package.name(), dimension.node.owner.as_ref()).ok_or(Graph)?;
        let qualified = name(dimension.node.declaration.as_ref()).ok_or(Graph)?;
        let preimage = if dimension.base {
            DimensionPreimage::new(owner, qualified, terms)
        } else {
            DimensionPreimage::new_derived(owner, qualified, terms)
        }
        .map_err(|_| Graph)?;
        dimensions.push((preimage, key(&dimension.node.node_id)?));
    }
    let mut units = Vec::new();
    for unit in &read.units {
        let owner = owner(package.name(), unit.node.owner.as_ref()).ok_or(Graph)?;
        let target = unit.target.map(|target| key(&target.node_id)).transpose()?;
        let preimage = UnitPreimage::new(
            owner,
            name(unit.node.declaration.as_ref()).ok_or(Graph)?,
            key(&unit.dimension.node_id)?,
            target,
            &rational(unit.scale).ok_or(Graph)?,
            &rational(unit.offset).ok_or(Graph)?,
        )
        .map_err(|_| Graph)?;
        // The row's identity is a claim of the producer: it must be the key
        // the table holds the unit by, or a quantity naming it would not
        // resolve.
        let unit_key = key(&unit.node.node_id)?;
        if *unit.identity != filament_exact::UnitId::Declared(unit_key.clone()) {
            return Err(Graph);
        }
        units.push((preimage, unit_key));
    }
    // The owners the package selected: its locked raw sources.
    let owners: Vec<_> = package
        .lock()
        .sources
        .iter()
        .map(|source| {
            NodeOwner::Source(OwnerSubject::new(
                &*source.authority,
                &*source.identity,
                package.name(),
            ))
        })
        .collect();
    let admitted = admit(
        dimensions.clone(),
        units.clone(),
        &OwnerSelection::new(owners.clone()),
    )?;
    Ok(UnitDefinitions {
        dimensions,
        units,
        owners,
        admitted,
    })
}

/// Rebuild the world's graph from its admitted package definitions, then apply
/// the same equivalence classes and runtime projection as the source route.
pub(crate) fn world_units(
    package: &crate::bare::BarePackage<'_>,
) -> Result<UnitTable, UnitDefinitionRefusal> {
    use filament_contract_model::{CheckedDimensionOperand, CheckedNodeKind, CheckedTerm};
    use fsl_semantics::check::EquivalenceClasses;
    use UnitDefinitionRefusal::Graph;

    let mut dimensions = std::collections::BTreeMap::new();
    let mut units = std::collections::BTreeMap::new();
    let mut owners = Vec::new();
    let mut pairs = Vec::new();
    for view in std::iter::once(package).chain(package.dependency_views()) {
        for (definition, key) in &view.units.dimensions {
            dimensions.insert(key.clone(), definition.clone());
        }
        for (definition, key) in &view.units.units {
            units.insert(key.clone(), definition.clone());
        }
        owners.extend(view.units.owners.iter().cloned());
        let admitted = view.package();
        for node in &admitted.resolved_graph().nodes {
            let Some(CheckedTerm::NominalBody(body)) = admitted.term(&node.node_id) else {
                continue;
            };
            let Some(([first, second], ratio)) = body.dimension_equivalence_details() else {
                continue;
            };
            let resolve = |operand: &CheckedDimensionOperand| {
                let (owner, id) = match operand {
                    CheckedDimensionOperand::Local(id) => (admitted, id),
                    CheckedDimensionOperand::Package(reference) => (
                        admitted
                            .direct_dependencies()
                            .find(|dependency| dependency.name() == reference.package.as_ref())
                            .ok_or(Graph)?,
                        reference.node.node_id(),
                    ),
                };
                // Resolve in the exact owning package, never the importer's
                // node table or a guessed transitive edge.
                if owner.term(id).is_none()
                    || !matches!(
                        owner.node_kind(id),
                        Some(CheckedNodeKind::ScalarType(
                            filament_contract_model::ScalarTypeForm::Dimension
                        ))
                    )
                {
                    return Err(Graph);
                }
                wire_graph(owner)
                    .map_err(|_| Graph)?
                    .key(id)
                    .map_err(|_| Graph)
            };
            pairs.push((
                resolve(first)?,
                resolve(second)?,
                rational(ratio).ok_or(Graph)?,
            ));
        }
    }
    let graph = admit_unit_graph(
        dimensions
            .into_iter()
            .map(|(key, definition)| (definition, key)),
        units.into_iter().map(|(key, definition)| (definition, key)),
        &OwnerSelection::new(owners),
    )
    .map_err(|_| Graph)?;
    let classes = EquivalenceClasses::from_pairs(&pairs);
    let graph = graph
        .with_dimension_equivalences(&classes.memberships())
        .map_err(|_| Graph)?;
    Ok(UnitTable::declared(&graph))
}

/// The unit table of the dimension and unit nodes, admitted as the source
/// route admits its units.
fn admit(
    dimensions: Vec<(DimensionPreimage, NodeKey)>,
    units: Vec<(UnitPreimage, NodeKey)>,
    owners: &OwnerSelection,
) -> Result<UnitTable, UnitDefinitionRefusal> {
    let graph =
        admit_unit_graph(dimensions, units, owners).map_err(|_| UnitDefinitionRefusal::Graph)?;
    Ok(UnitTable::declared(&graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn source_owner(identity: &str) -> NodeOwner {
        NodeOwner::Source(OwnerSubject::new("a", identity, "pkg"))
    }

    /// R's `Money` and `U`, owned by `owner`, with their keys.
    fn money(owner: &NodeOwner) -> (DimensionPreimage, NodeKey, UnitPreimage, NodeKey) {
        let dimension = DimensionPreimage::new(owner.clone(), vec!["Money".to_owned()], Vec::new())
            .expect("a dimension");
        let dimension_key = dimension.key().expect("a key");
        let unit = UnitPreimage::new(
            owner.clone(),
            vec!["U".to_owned()],
            dimension_key.clone(),
            None,
            &Rational::from_integer(Integer::from(1_i64)),
            &Rational::from_integer(Integer::from(0_i64)),
        )
        .expect("a unit");
        let unit_key = unit.key().expect("a key");
        (dimension, dimension_key, unit, unit_key)
    }

    /// FR-533-AC-19: the unit table admits the units of a selected owner and
    /// refuses a unit whose owner the package did not select.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn a_unit_of_an_unselected_owner_is_no_unit_graph() {
        let owned = source_owner("u");
        let (dimension, dimension_key, unit, unit_key) = money(&owned);
        let parts = || {
            (
                vec![(dimension.clone(), dimension_key.clone())],
                vec![(unit.clone(), unit_key.clone())],
            )
        };
        let (dimensions, units) = parts();
        assert!(admit(dimensions, units, &OwnerSelection::new([owned.clone()])).is_ok());
        let (dimensions, units) = parts();
        assert_eq!(
            admit(
                dimensions,
                units,
                &OwnerSelection::new([source_owner("other")])
            )
            .unwrap_err(),
            UnitDefinitionRefusal::Graph
        );
    }

    /// FR-533-AC-19: only a source owns a unit or dimension the bare route
    /// reads; a node of a model owner has no owner in the selection, so a
    /// dimension (or unit) it owns is refused as no unit graph.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn a_node_no_source_owns_has_no_selectable_owner() {
        let source = CheckedNodeOwner::Source {
            authority: "a".into(),
            identity: "u".into(),
        };
        assert_eq!(owner("pkg", Some(&source)), Some(self::source_owner("u")));
        for other in [
            CheckedNodeOwner::ModelRoot {
                identity: "m".into(),
            },
            CheckedNodeOwner::Model {
                identity: "m".into(),
                node: "n".into(),
            },
        ] {
            assert_eq!(owner("pkg", Some(&other)), None);
        }
        assert_eq!(owner("pkg", None), None);
    }

    /// FR-533-AC-19: a conversion spelled unreduced is refused, not normalised.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn an_unreduced_conversion_spelling_is_refused() {
        let spelled = |numerator: &str, denominator: &str| CheckedRational {
            numerator: numerator.into(),
            denominator: denominator.into(),
        };
        assert!(rational(&spelled("1", "2")).is_some());
        assert!(rational(&spelled("2", "4")).is_none());
        assert!(rational(&spelled("0", "1")).is_some());
        assert!(rational(&spelled("0", "3")).is_none());
    }

    fn admitted_units(
        name: &str,
        declarations: &str,
    ) -> (crate::Emission, std::sync::Arc<CheckedPackageV2>) {
        use filament_contract_model::CheckedPackageEvidence;
        use filament_semantic_value::checking::{CheckMode, CheckingLimits};
        use fsl_semantics::check::PackageDeclarations;
        use fsl_semantics::library::LibraryName;
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declarations}"
        );
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("fixture", name),
            "unit.native",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("the source parses");
        let declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            name.to_owned(),
            fsl_forms::build_unit(&parsed).expect("the unit forms"),
            Vec::new(),
            Vec::new(),
        )
        .expect("the unit assembles");
        let checked = crate::CheckedPackage::link(
            declarations
                .check(
                    CheckMode::Linked,
                    CheckingLimits::default(),
                    &fsl_semantics::check::test_support::ZeroRewardEvaluator,
                )
                .expect("the unit checks"),
        );
        let emission = crate::emit_checked(&checked, crate::EmitLimits::fixture())
            .expect("the declarations emit");
        let wire: serde_json::Value =
            serde_json::from_slice(emission.package().bytes()).expect("the emitted wire");
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"]
            .as_array()
            .expect("the selected features")
        {
            evidence.support_feature(feature.as_str().expect("a feature identity"));
        }
        let crate::checked_v2::Read::Verified { admitted, .. } = crate::checked_v2::read_v2(
            emission.package().bytes(),
            LibraryName::new(emission.package().name()).expect("the package name"),
            crate::V2ReadLimits::fixture(),
            &evidence,
        ) else {
            panic!("the actual emitted declarations must admit");
        };
        (emission, admitted)
    }

    #[trace("FR-944-AC-10", "FR-944-AC-13", "FR-533-AC-19")]
    #[test]
    fn admitted_time_unit_scales_keep_exact_factors_and_reject_other_dimensions() {
        let (emission, package) = admitted_units(
            "clock-units",
            "dimension Time; unit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\n\
             dimension Money; unit U : Money = rational(1, 1);",
        );
        let mut budget = crate::V2ReadLimits::fixture().native_join_budget();
        let identities = emission.identity_rows(&package, &mut budget).unwrap();
        let equality = emission.equality_witness(&package, &mut budget).unwrap();
        let bare = crate::BarePackage::new(&package)
            .with_supply(
                crate::BareSupply {
                    identities: &identities,
                    equality: Some(&equality),
                },
                &mut budget,
            )
            .expect("the actual admitted units join");
        assert_eq!(
            bare.time_unit_scale("s"),
            Some(Rational::from_integer(Integer::one()))
        );
        assert_eq!(
            bare.time_unit_scale("ms"),
            Some(Rational::new(Integer::one(), Integer::from(1000_i64)).unwrap())
        );
        assert!(bare.declares_unit("U"));
        assert_eq!(bare.time_unit_scale("U"), None);
        assert_eq!(bare.time_unit_scale("missing"), None);
    }

    /// FR-533-AC-23: an equivalence's package operand reads the exact admitted
    /// direct dependency, and the resulting world converts into its actual unit.
    /// Source-library quantity exports remain owned by their producer contract.
    #[trace("FR-533-AC-23", "TC-1333", "FSpec-FR-142-AC-26")]
    #[test]
    fn a_package_equivalence_resolves_its_admitted_direct_dependency() {
        use filament_contract_model::{
            encode_checked_package_v2, CheckedDeclaredNodeId, CheckedDependencySelection,
            CheckedDimensionOperand, CheckedNodeId, CheckedNodeKind, CheckedNodePathSegment,
            CheckedOccurrence, CheckedOccurrenceRole, CheckedPackageEvidence,
            CheckedPackageNodeRef, CheckedPackageSidecarsRef,
            CheckedPackageSourceCorrespondenceRef, CheckedTerm, NominalBody, ScalarTypeForm,
        };
        use filament_exact::{Meter, Outcome, Quantity, ScalarLimits, UnitId};
        use filament_semantic_value::quantity::{convert_quantity, QuantityTarget};
        use fsl_semantics::library::LibraryName;
        let (dependency_emission, dependency) = admitted_units(
            "time-library",
            "dimension Time; unit s : Time = rational(1, 1);",
        );
        let (root_emission, root) = admitted_units(
            "importing-unit",
            "dimension Beat; unit beat : Beat = rational(1, 1);",
        );
        let dimension = |package: &CheckedPackageV2| {
            package
                .resolved_graph()
                .nodes
                .iter()
                .position(|node| {
                    matches!(
                        package.node_kind(&node.node_id),
                        Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Dimension))
                    )
                })
                .expect("the declared dimension")
        };
        let local_index = dimension(&root);
        let foreign_index = dimension(&dependency);
        let local = root.resolved_graph().nodes[local_index].node_id.clone();
        let foreign = dependency.resolved_graph().nodes[foreign_index]
            .node_id
            .clone();
        assert!(
            matches!(&dependency.resolved_graph().nodes[foreign_index].owner,
            Some(CheckedNodeOwner::Source { authority, identity }) if &**authority == "fixture" && &**identity == "time-library")
        );
        let local_key = wire_graph(&root)
            .expect("the root keys")
            .key(&local)
            .expect("the local dimension key");
        let foreign_key = wire_graph(&dependency)
            .expect("the dependency keys")
            .key(&foreign)
            .expect("the exact dependency dimension key");
        let quarter = Rational::new(Integer::from(1_i64), Integer::from(4_i64)).expect("a quarter");
        let foreign_operand = CheckedDimensionOperand::Package(CheckedPackageNodeRef {
            package: dependency.name().into(),
            node: CheckedDeclaredNodeId::try_new(foreign.clone())
                .expect("the actual declared dimension"),
        });
        let local_operand = CheckedDimensionOperand::Local(local.clone());
        let (operands, dimension_ids, ratio) = if local_key < foreign_key {
            (
                [local_operand, foreign_operand],
                [local.clone(), foreign],
                quarter,
            )
        } else {
            (
                [foreign_operand, local_operand],
                [foreign, local.clone()],
                Rational::from_integer(Integer::from(4_i64)),
            )
        };
        // As in IR's mixed dimension-operand fixture, the producer-selected
        // anchor path names the two genuine declared dimensions. The body
        // carries the typed Package reference authenticated during admission.
        let mut path = vec![
            CheckedNodePathSegment::Name("scalar_type".into()),
            CheckedNodePathSegment::Name("dimension_equivalence".into()),
        ];
        for dimension in &dimension_ids {
            path.push(CheckedNodePathSegment::Name(dimension.package.clone()));
            path.extend(dimension.path.iter().cloned());
            path.push(CheckedNodePathSegment::Name(
                dimension.ordinal.to_string().into(),
            ));
        }
        let id = CheckedNodeId {
            package: root.name().into(),
            path,
            ordinal: 0,
        };
        let mut graph = root.graph().clone();
        let mut equivalence = graph.nodes[local_index].clone();
        equivalence.node_id = id.clone();
        equivalence.semantic_type = id.clone();
        equivalence.semantic_form = "dimension_equivalence".into();
        equivalence.dependencies = vec![local.clone()];
        equivalence.declaration = None;
        equivalence.occurrences = vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Anchor,
            ordinal: 0,
        }];
        equivalence.body = CheckedTerm::NominalBody(Box::new(
            NominalBody::try_dimension_equivalence(
                operands,
                CheckedRational {
                    numerator: ratio.numerator().to_string().into(),
                    denominator: ratio.denominator().to_string().into(),
                },
            )
            .expect("the admitted dimension pair"),
        ));
        graph.nodes.push(equivalence);
        graph.operand_rows.push(Vec::new());
        let mut source_map = root.source_map().to_vec();
        let mut occurrence = source_map
            .iter()
            .find(|entry| entry.node_id == local)
            .expect("the local source occurrence")
            .clone();
        occurrence.node_id = id;
        occurrence.role = CheckedOccurrenceRole::Anchor;
        occurrence.ordinal = 0;
        source_map.push(occurrence);
        let mut lock = root.lock().clone();
        lock.dependency_selections.push(CheckedDependencySelection {
            identity: dependency.name().into(),
        });
        let bytes = encode_checked_package_v2(
            root.name(),
            &lock,
            &graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: None,
            },
            CheckedPackageSidecarsRef {
                checked_term_data: root.checked_term_data(),
                temporal_bindings: root.temporal_bindings(),
                temporal_clause_names: root.temporal_clause_names(),
                claim_names: root.claim_names(),
                schedulability_claims: None,
                operation_effects: None,
                operation_bodies: None,
            },
            root.capability_report(),
            root.diagnostics(),
            crate::EmitLimits::fixture().canonical(),
        )
        .expect("the real IR writer encodes the selected package");
        let mut evidence = CheckedPackageEvidence::new();
        for feature in &lock.required_features {
            evidence.support_feature(&**feature);
        }
        evidence
            .insert_dependency_package(dependency.name(), (*dependency).clone())
            .expect("the actual admitted dependency evidence");
        let read = crate::checked_v2::read_v2(
            &bytes,
            LibraryName::new(root.name()).expect("the importing identity"),
            crate::V2ReadLimits::fixture(),
            &evidence,
        );
        let crate::checked_v2::Read::Verified { admitted, .. } = read else {
            panic!("the selected dependency package must admit: {read:?}");
        };
        assert_eq!(admitted.direct_dependencies().len(), 1);
        let last = admitted
            .resolved_graph()
            .nodes
            .last()
            .expect("the equivalence");
        let Some(CheckedTerm::NominalBody(body)) = admitted.term(&last.node_id) else {
            panic!("the admitted nominal body");
        };
        let (operands, _) = body
            .dimension_equivalence_details()
            .expect("the actual shared accessor");
        assert_eq!(
            operands
                .iter()
                .filter(|operand| matches!(operand, CheckedDimensionOperand::Package(_)))
                .count(),
            1
        );
        let mut budget = crate::V2ReadLimits::fixture().native_join_budget();
        let root_equality = root_emission
            .equality_witness(&admitted, &mut budget)
            .expect("the original root's unit witnesses");
        let identities = dependency_emission
            .identity_rows(&dependency, &mut budget)
            .expect("dependency identities");
        let equality = dependency_emission
            .equality_witness(&dependency, &mut budget)
            .expect("dependency unit witnesses");
        let artifacts = [crate::DependencyArtifact {
            identity: dependency.name().to_owned(),
            bytes: dependency_emission.package().bytes().into(),
            identity_bytes: identities
                .to_bytes(&dependency)
                .expect("identity bytes")
                .into(),
            equality_bytes: equality
                .to_bytes(&dependency)
                .expect("equality bytes")
                .into(),
            package: dependency,
            identities,
            equality,
        }];
        let mut bare = crate::BarePackage::new(&admitted)
            .with_dependency_artifacts(&artifacts, &mut budget)
            .expect("the actual dependency artifacts join");
        bare.units = read_unit_table(&admitted, root_equality.source_units(), &mut budget)
            .expect("the admitted local unit definitions");
        let table = world_units(&bare).expect("the Package operand reconstructs the runtime graph");
        let declared_unit = |package: &CheckedPackageV2| {
            let node = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| {
                    matches!(
                        package.node_kind(&node.node_id),
                        Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Unit))
                    )
                })
                .expect("the actual declared unit");
            UnitId::declared(
                wire_graph(package)
                    .expect("the keys")
                    .key(&node.node_id)
                    .expect("the unit key"),
            )
        };
        let source = Quantity::new(
            Rational::from_integer(Integer::one()),
            declared_unit(&admitted),
        );
        let target = declared_unit(&artifacts[0].package);
        let mut meter = Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
        let Outcome::Completed(converted) = convert_quantity(
            table.resolve(&source).expect("the local source unit"),
            table
                .get(target.clone())
                .expect("the actual dependency unit"),
            &QuantityTarget::Exact,
            &mut meter,
        )
        .expect("the class conversion is eligible") else {
            panic!("the exact conversion completes");
        };
        let (quantity, _) = converted
            .identified_quantity()
            .expect("the exact target quantity");
        assert_eq!(
            quantity.magnitude(),
            &Rational::new(Integer::from(1_i64), Integer::from(4_i64)).expect("a quarter")
        );
        assert_eq!(quantity.unit(), &target);
    }
}
