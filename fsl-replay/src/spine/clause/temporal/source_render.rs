// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283: names from all actual atom occurrences, with canonical shared labels.

use fsl_foundation::source::provenance::SourceRegion;
use fsl_foundation::{Source, Span};
use fsl_semantics::check::{FormulaNode, FormulaNodeId, TemporalClauseNode};

use super::RunRefusal;

fn fault(invariant: &'static str) -> Box<RunRefusal> {
    Box::new(RunRefusal::Fault(
        filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
    ))
}

pub(super) fn atom_name(
    package: &fsl_package::CheckedPackage,
    sources: &[Source],
    node: &TemporalClauseNode,
    atom: FormulaNodeId,
) -> Result<super::RenderedClauseAtom, Box<RunRefusal>> {
    if !matches!(
        node.clause().formula().nodes().get(atom.index()),
        Some(FormulaNode::Holds { .. } | FormulaNode::Aggregate { .. })
    ) {
        return Err(fault("named-atom-is-a-holds-or-aggregate-node"));
    }
    let graph = package.graph();
    let declaration = graph
        .occurrence(node.identity(), node.claim())
        .and_then(|location| graph.region(location))
        .ok_or_else(|| fault("clause-declaration-has-a-region"))?;
    let declaration = SourceRegion::new(
        declaration.source().clone(),
        u64::try_from(node.clause().span().start).map_err(|_| fault("clause-span-fits"))?,
        u64::try_from(node.clause().span().end).map_err(|_| fault("clause-span-fits"))?,
    )
    .map_err(|_| fault("clause-declaration-has-a-region"))?;
    let key = node
        .formula_node_key(atom)
        .ok_or_else(|| fault("named-atom-has-a-producer-key"))?;
    let candidates = graph
        .occurrences()
        .filter(|(identity, origin, _)| identity == key && origin.role().as_str() == "expression")
        .filter_map(|(_, _, location)| graph.region(location))
        .filter(|region| {
            region.source() == declaration.source()
                && region.start() >= declaration.start()
                && region.end() <= declaration.end()
        })
        .collect::<Vec<_>>();
    let read = |region: &SourceRegion| {
        let missing = || {
            Box::new(RunRefusal::SourceNotSupplied {
                reference: fsl_foundation::SourceIdentity::new(
                    region.source().authority(),
                    region.source().identity(),
                ),
                clause: Some(node.clause().name().to_owned()),
            })
        };
        let source = sources
            .iter()
            .find(|source| source.reference() == region.source())
            .ok_or_else(missing)?;
        let span = Span {
            start: usize::try_from(region.start()).map_err(|_| missing())?,
            end: usize::try_from(region.end()).map_err(|_| missing())?,
        };
        source.slice(span).map(str::to_owned).ok_or_else(missing)
    };
    let mut common = None;
    let mut differing = false;
    for region in &candidates {
        let text = read(region)?;
        differing |= common.as_ref().is_some_and(|held| held != &text);
        common.get_or_insert(text);
    }
    let text = common.ok_or_else(|| fault("named-atom-has-an-expression-region"))?;
    if candidates.len() > 1 {
        read(&declaration)?;
    }
    if !differing {
        let region = if candidates.len() == 1 {
            candidates[0].clone()
        } else {
            declaration
        };
        return Ok(super::RenderedClauseAtom { name: text, region });
    }
    let valuation = super::super::TemporalValuation::from_node(node, graph, None, None);
    if matches!(
        node.clause().formula().nodes().get(atom.index()),
        Some(FormulaNode::Holds { .. })
    ) {
        let name = crate::canonical_atom::source_labels(&valuation)
            .ok()
            .and_then(|mut labels| labels.remove(&atom))
            .ok_or_else(|| fault("checked-holds-has-a-canonical-label"))?;
        return Ok(super::RenderedClauseAtom {
            name,
            region: declaration,
        });
    }
    let name = aggregate_label(node, atom, &valuation)
        .map_err(|_| fault("checked-aggregate-has-a-canonical-label"))?;
    Ok(super::RenderedClauseAtom {
        name,
        region: declaration,
    })
}

fn aggregate_label(
    node: &TemporalClauseNode,
    atom: FormulaNodeId,
    valuation: &super::super::TemporalValuation,
) -> Result<String, String> {
    use fsl_semantics::check::{AggregateKind, AggregateWindow};
    let missing = || "checked aggregate has its typed components".to_owned();
    let clause = node.clause();
    let aggregate = clause.aggregates().get(&atom.index()).ok_or_else(missing)?;
    let parts = node.aggregate_conditions(atom).ok_or_else(missing)?;
    let mut binders = vec![clause.over().0.clone()];
    if let Some((trigger, _)) = clause.trigger() {
        binders.push(trigger.to_owned());
    }
    binders.extend(clause.captures().map(|(name, _)| name.to_owned()));
    let rows = crate::canonical_atom::CanonicalRows::from_valuation(valuation);
    let render = |part: fsl_semantics::check::CheckedNode<'_>| {
        crate::canonical_atom::CheckedExpression::from_source(part)?.render(&binders, &rows)
    };
    let rational = |value: &filament_exact::Rational| {
        format!("rational({}, {})", value.numerator(), value.denominator())
    };
    let mut text = match &aggregate.kind {
        AggregateKind::Count => "count".to_owned(),
        AggregateKind::Sum | AggregateKind::Min | AggregateKind::Max => {
            let kind = match aggregate.kind {
                AggregateKind::Sum => "sum",
                AggregateKind::Min => "min",
                _ => "max",
            };
            format!("{kind} of {}", render(parts.value.ok_or_else(missing)?.0)?)
        }
        AggregateKind::Fraction => format!(
            "fraction holds({})",
            render(parts.predicate.ok_or_else(missing)?.0)?
        ),
        AggregateKind::Quantile(level) => format!(
            "quantile {} of {}",
            rational(level),
            render(parts.value.ok_or_else(missing)?.0)?
        ),
    };
    if let Some((filter, _)) = parts.filter {
        text.push_str(&format!(" where holds({})", render(filter)?));
    }
    let (lower, upper) = match &aggregate.window {
        AggregateWindow::Positions { lower, upper } => (lower.to_string(), upper.to_string()),
        AggregateWindow::Duration(window) => {
            let unit = crate::canonical_atom::unit_identity(window.unit())?;
            (
                format!("{} {unit}", rational(&window.lower)),
                format!("{} {unit}", rational(&window.upper)),
            )
        }
    };
    let comparison = match aggregate.comparison {
        fsl_forms::AggregateComparison::Less => "<",
        fsl_forms::AggregateComparison::LessEqual => "<=",
        fsl_forms::AggregateComparison::Equal => "=",
        fsl_forms::AggregateComparison::GreaterEqual => ">=",
        fsl_forms::AggregateComparison::Greater => ">",
    };
    text.push_str(&format!(
        " over past[{lower}, {upper}] min_count {} {comparison} {}",
        aggregate.min_count,
        render(parts.threshold.0)?
    ));
    Ok(text)
}
