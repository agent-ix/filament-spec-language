// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-931-AC-11/12: canonical atom labels from checked expressions. Source
//! check nodes are normalized to this one arena before printing, so aliases
//! and source trivia cannot affect labels.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use filament_checked::node::{
    Arithmetic, CheckedLiteral, Connective, IntegerDivisionOperator, NodeKind, Visit,
};
use filament_exact::IeeeWidth;
use filament_exact::{CollectionKind, Integer, NodeKey, Value, ValueType};
use filament_semantic_value::declaration::EqualityOperator;
use fsl_semantics::check::{CheckedNode, NodeId};

#[derive(Clone)]
struct ExpressionNode {
    kind: NodeKind<usize, CheckedLiteral>,
    value_type: ValueType,
    type_argument: Option<String>,
}

#[derive(Default)]
pub(super) struct CanonicalRows {
    declarations: BTreeMap<NodeKey, String>,
    functions: BTreeMap<usize, String>,
    record_fields: BTreeMap<NodeKey, Vec<String>>,
    union_members: BTreeMap<NodeKey, Vec<String>>,
    type_arguments: BTreeMap<String, String>,
}

/// A checked expression arena with children remapped to its own indices.
pub(super) struct CheckedExpression {
    nodes: Vec<ExpressionNode>,
    root: usize,
}

impl CheckedExpression {
    /// Normalize a source condition from the checked tree, without reading
    /// the expression's source span.
    pub(super) fn from_source(root: CheckedNode<'_>) -> Result<Self, String> {
        let mut pending = vec![(root, false)];
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();
        while let Some((node, exiting)) = pending.pop() {
            let id = node.id();
            if exiting {
                postorder.push(node);
                continue;
            }
            if !visited.insert(id) {
                continue;
            }
            pending.push((node, true));
            let children = checked_children(node);
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
        }
        let indices = postorder
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id(), index))
            .collect::<HashMap<_, _>>();
        let nodes = postorder
            .iter()
            .map(|node| {
                let kind = node.kind().map_ref(
                    |child| indices.get(child).copied().unwrap_or(usize::MAX),
                    Clone::clone,
                );
                ExpressionNode {
                    kind,
                    value_type: node.value_type().clone(),
                    type_argument: node.type_argument().map(str::to_owned),
                }
            })
            .collect();
        let root = *indices
            .get(&root.id())
            .ok_or_else(|| "checked expression root is retained".to_owned())?;
        Ok(Self { nodes, root })
    }

    /// Normalize an FR-940 reader projection into the same arena used by the
    /// source route. Reader node indices and source `NodeId`s are independent
    /// of the printer, so both routes render the exact checked node forms
    /// through one implementation.
    pub(super) fn from_projection(
        projection: &fsl_package::ReaderProtocolProjection,
    ) -> Result<Self, String> {
        let root = projection.root();
        let mut pending = vec![(root, false)];
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();
        while let Some((index, exiting)) = pending.pop() {
            if exiting {
                postorder.push(index);
                continue;
            }
            if !visited.insert(index) {
                continue;
            }
            let children = projection
                .children(index)
                .ok_or_else(|| "reader projection children are retained".to_owned())?;
            pending.push((index, true));
            pending.extend(children.iter().rev().map(|child| (*child, false)));
        }
        let indices = postorder
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect::<HashMap<_, _>>();
        let nodes = postorder
            .iter()
            .map(|index| {
                let kind = projection
                    .kind(*index)
                    .ok_or_else(|| "reader projection node is retained".to_owned())?
                    .map_ref(
                        |child| indices.get(child).copied().unwrap_or(usize::MAX),
                        Clone::clone,
                    );
                let value_type = projection
                    .value_type(*index)
                    .ok_or_else(|| "reader projection type is retained".to_owned())?
                    .clone();
                Ok(ExpressionNode { kind, value_type })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let root = *indices
            .get(&projection.root())
            .ok_or_else(|| "reader projection root is retained".to_owned())?;
        Ok(Self { nodes, root })
    }

    /// Render one checked condition using the FR-003 expression precedence
    /// rules and declaration keys held by the checked tree.
    ///
    /// Every binder prints under the declared name its checked binding site
    /// carries (FR-931 "Binders"), and a local read resolves through the
    /// binding sites that enclose it, innermost first. The same node under
    /// two scopes is rendered once per scope.
    pub(super) fn render(
        &self,
        binders: &[String],
        rows: &CanonicalRows,
    ) -> Result<String, String> {
        let binders = binders
            .iter()
            .enumerate()
            .map(|(slot, name)| (slot, name.clone()))
            .collect::<Vec<_>>();
        self.render_with_slots(&binders, rows)
    }

    /// Render with the reader projection's explicit local-slot mapping.
    /// Package projections retain local slots by their admitted binder rows,
    /// which need not be dense in the clause's declaration order.
    pub(super) fn render_with_slots(
        &self,
        binders: &[(usize, String)],
        rows: &CanonicalRows,
    ) -> Result<String, String> {
        let mut scopes = Vec::<Scope>::new();
        let mut root_env: Env = None;
        for (slot, name) in binders {
            scopes.push(Scope {
                parent: root_env,
                slot: *slot,
                entry: Entry::Name(name.clone()),
            });
            root_env = Some(scopes.len() - 1);
        }
        let mut printer = Printer {
            expression: self,
            rows,
            scopes,
            memo: HashMap::new(),
            plans: HashMap::new(),
            reads: HashMap::new(),
        };
        let mut pending = vec![Step::Enter(self.root, root_env)];
        while let Some(step) = pending.pop() {
            match step {
                Step::Enter(index, env) => {
                    let key = (index, env);
                    if printer.memo.contains_key(&key) || printer.plans.contains_key(&key) {
                        continue;
                    }
                    let plan = printer.plan(index, env)?;
                    pending.push(Step::Exit(index, env));
                    pending.extend(
                        plan.iter()
                            .rev()
                            .map(|(child, child_env)| Step::Enter(*child, *child_env)),
                    );
                    printer.plans.insert(key, plan);
                }
                Step::Exit(index, env) => {
                    let key = (index, env);
                    if printer.memo.contains_key(&key) {
                        continue;
                    }
                    let printed = printer.print(index, env)?;
                    if let Some(reads) = printed.reads {
                        printer.reads.insert(key, reads);
                    }
                    printer.memo.insert(key, (printed.text, printed.precedence));
                }
            }
        }
        printer
            .memo
            .get(&(self.root, root_env))
            .map(|(text, _)| text.clone())
            .ok_or_else(|| "checked expression root is rendered".to_owned())
    }
}

impl CanonicalRows {
    fn aggregate_type_argument(&self, node: &ExpressionNode) -> String {
        node.type_argument
            .as_ref()
            .and_then(|name| self.type_arguments.get(name))
            .cloned()
            .unwrap_or_else(|| self.type_argument(&node.value_type))
    }

    fn type_argument(&self, value_type: &ValueType) -> String {
        let mut spellings = fsl_semantics::model::spelling::SourceSpellings::default();
        for (key, declaration) in &self.declarations {
            spellings.name_node(key.clone(), declaration.clone());
        }
        fsl_semantics::model::spelling::fsl_spelling(
            &fsl_semantics::model::domain_package::ValueTypeRef::Resolved(Box::new(
                value_type.clone(),
            )),
            &spellings,
            None,
        )
    }
}

/// One scope: the binding site's slot and what a read of it prints as.
struct Scope {
    parent: Env,
    slot: usize,
    entry: Entry,
}

enum Entry {
    /// A binder printed under its declared name.
    Name(String),
    /// A checker-made IEEE ordering operand binder: a read of it prints the
    /// expression it holds (FR-931 "Binders").
    Operand { value: Key },
}

/// The innermost enclosing scope, `None` outside every binding site.
type Env = Option<usize>;
/// A node rendered under one scope.
type Key = (usize, Env);

enum Step {
    Enter(usize, Env),
    Exit(usize, Env),
}

/// FR-397 "Operand binders": the declared name of the checker's own binder.
const CHECKER_OPERAND: &str = "#ieee_operand";

/// A node's text, its precedence, and the checker binders it still needs a
/// `let` for.
struct Printed {
    text: String,
    precedence: u8,
    reads: Option<BTreeSet<usize>>,
}

struct Printer<'a> {
    expression: &'a CheckedExpression,
    rows: &'a CanonicalRows,
    scopes: Vec<Scope>,
    memo: HashMap<Key, (String, u8)>,
    plans: HashMap<Key, Vec<Key>>,
    /// For a checker operand `let` chain or the comparison it wraps: the
    /// checker binders (scope indices) its reads still need a `let` for.
    reads: HashMap<Key, BTreeSet<usize>>,
}

impl Printer<'_> {
    fn bind(&mut self, env: Env, slot: usize, entry: Entry) -> Env {
        self.scopes.push(Scope {
            parent: env,
            slot,
            entry,
        });
        Some(self.scopes.len() - 1)
    }

    fn lookup(&self, env: Env, slot: usize) -> Option<usize> {
        let mut cursor = env;
        while let Some(index) = cursor {
            let scope = self.scopes.get(index)?;
            if scope.slot == slot {
                return Some(index);
            }
            cursor = scope.parent;
        }
        None
    }

    /// The nodes `index` renders from, each with the scope it is read under,
    /// in the order the binding sites' operands evaluate.
    fn plan(&mut self, index: usize, env: Env) -> Result<Vec<Key>, String> {
        let node = self
            .expression
            .nodes
            .get(index)
            .ok_or_else(|| "checked expression operands are retained".to_owned())?;
        // FR-931 "Binders": the checker's ordering composition prints as one
        // comparison, so only its two operands are rendered.
        if let Some((lesser, greater, _)) = self.composition(index, env) {
            return Ok(vec![(lesser, env), (greater, env)]);
        }
        Ok(match &node.kind {
            NodeKind::Let {
                slot,
                name,
                value,
                body,
            } => {
                let entry = if name == CHECKER_OPERAND {
                    Entry::Operand {
                        value: (*value, env),
                    }
                } else {
                    Entry::Name(name.clone())
                };
                let inner = self.bind(env, *slot, entry);
                vec![(*value, env), (*body, inner)]
            }
            NodeKind::Query {
                slot,
                name,
                source,
                body,
                ..
            } => {
                let inner = self.bind(env, *slot, Entry::Name(name.clone()));
                vec![(*source, env), (*body, inner)]
            }
            NodeKind::Fold {
                accumulator,
                binder,
                accumulator_name,
                binder_name,
                source,
                step,
                identity,
            } => {
                let inner = self.bind(env, *accumulator, Entry::Name(accumulator_name.clone()));
                let inner = self.bind(inner, *binder, Entry::Name(binder_name.clone()));
                let mut plan = vec![(*source, env)];
                plan.extend(identity.map(|identity| (identity, env)));
                plan.push((*step, inner));
                plan
            }
            NodeKind::Seq {
                slot,
                name,
                low,
                high,
                body,
            } => {
                let inner = self.bind(env, *slot, Entry::Name(name.clone()));
                vec![(*low, env), (*high, env), (*body, inner)]
            }
            NodeKind::Case {
                scrutinee, arms, ..
            } => {
                let mut plan = vec![(*scrutinee, env)];
                for arm in arms {
                    if arm.names.len() != arm.binders.len() {
                        return Err("checked case arm names every payload binder".to_owned());
                    }
                    let mut inner = env;
                    for (slot, name) in arm.binders.iter().zip(&arm.names) {
                        inner = self.bind(inner, *slot, Entry::Name(name.clone()));
                    }
                    plan.push((arm.body, inner));
                }
                plan
            }
            kind => {
                let mut children = Vec::new();
                let _ = kind.map_ref(
                    |child| {
                        children.push(*child);
                    },
                    Clone::clone,
                );
                children.into_iter().map(|child| (child, env)).collect()
            }
        })
    }

    fn text_of(&self, key: Key) -> Result<&(String, u8), String> {
        self.memo
            .get(&key)
            .ok_or_else(|| "checked expression operands precede their node".to_owned())
    }

    /// Text and precedence of a plain operand read under the node's scope.
    fn operand(&self, index: usize, env: Env) -> Result<(String, u8), String> {
        self.text_of((index, env)).cloned()
    }

    /// `C(l, g)` / `C=(l, g)` of FR-931 "Binders": the operands `l` and `g`
    /// and whether the composition is strict, when `index` is the checker's
    /// IEEE ordering composition over locals and literals.
    fn composition(&self, index: usize, env: Env) -> Option<(usize, usize, bool)> {
        let nodes = &self.expression.nodes;
        let kind = |index: usize| nodes.get(index).map(|node| &node.kind);
        let NodeKind::Connective(Connective::And, _, ordinary, relation) = kind(index)? else {
            return None;
        };
        let NodeKind::Connective(Connective::And, _, left_ordinary, right_ordinary) =
            kind(*ordinary)?
        else {
            return None;
        };
        let NodeKind::IeeeNumericEquality(l0, l1) = kind(*left_ordinary)? else {
            return None;
        };
        let NodeKind::IeeeNumericEquality(g0, g1) = kind(*right_ordinary)? else {
            return None;
        };
        let (total, equal, strict) = match kind(*relation)? {
            NodeKind::Connective(Connective::And, _, total, apart) => {
                let NodeKind::Not(equal) = kind(*apart)? else {
                    return None;
                };
                (*total, *equal, true)
            }
            NodeKind::Connective(Connective::Or, _, total, equal) => (*total, *equal, false),
            _ => return None,
        };
        let NodeKind::IeeeTotalOrder(_, l2, g2) = kind(total)? else {
            return None;
        };
        let NodeKind::IeeeNumericEquality(l3, g3) = kind(equal)? else {
            return None;
        };
        let is_operand =
            |index: usize| matches!(kind(index), Some(NodeKind::Local(_) | NodeKind::Literal(_)));
        if !is_operand(*l0) || !is_operand(*g0) {
            return None;
        }
        let operands = [(*l0, *l1, *l2, *l3), (*g0, *g1, *g2, *g3)];
        for (a, b, c, d) in operands {
            for other in [b, c, d] {
                if !self.same_operand(a, other, env) {
                    return None;
                }
            }
        }
        Some((*l0, *g0, strict))
    }

    /// Whether `a` and `b` are the same local read or the same literal.
    fn same_operand(&self, a: usize, b: usize, env: Env) -> bool {
        let nodes = &self.expression.nodes;
        match (nodes.get(a).map(|n| &n.kind), nodes.get(b).map(|n| &n.kind)) {
            (Some(NodeKind::Local(x)), Some(NodeKind::Local(y))) => {
                self.lookup(env, *x) == self.lookup(env, *y)
            }
            (Some(NodeKind::Literal(x)), Some(NodeKind::Literal(y))) => x == y,
            _ => false,
        }
    }

    /// The checker binder a local read reads, if it reads one.
    fn operand_binder(&self, index: usize, env: Env) -> Option<usize> {
        let NodeKind::Local(slot) = &self.expression.nodes.get(index)?.kind else {
            return None;
        };
        let scope = self.lookup(env, *slot)?;
        matches!(self.scopes[scope].entry, Entry::Operand { .. }).then_some(scope)
    }

    /// Whether the node at `index` is a population.
    fn is_population(&self, index: usize) -> bool {
        self.expression.nodes.get(index).is_some_and(|node| {
            matches!(
                &node.kind,
                NodeKind::AllInstances { .. }
                    | NodeKind::PopulationObjects { .. }
                    | NodeKind::NamedPopulation { .. }
            )
        })
    }

    /// Whether the node at `index` is the literal `true`, the checked form
    /// of a population `count` that writes no `where`.
    fn is_true(&self, index: usize) -> bool {
        self.expression.nodes.get(index).is_some_and(|node| {
            matches!(
                &node.kind,
                NodeKind::Literal(CheckedLiteral(Value::Boolean(true)))
            )
        })
    }

    /// The population and `where` condition texts of the filter at `index`,
    /// when it filters a population under the binder `slot` named `name` that
    /// the enclosing `only` or `sum` also binds, so the pair is one
    /// population form; any other filter source is a collection filter.
    fn population_filter(
        &self,
        index: usize,
        slot: usize,
        name: &str,
        env: Env,
    ) -> Option<(String, String)> {
        let NodeKind::Query {
            visit: Visit::Filter,
            slot: filter_slot,
            name: filter_name,
            source,
            ..
        } = &self.expression.nodes.get(index)?.kind
        else {
            return None;
        };
        if *filter_slot != slot || filter_name != name || !self.is_population(*source) {
            return None;
        }
        let plan = self.plans.get(&(index, env))?;
        let population = self.memo.get(plan.first()?)?.0.clone();
        let condition = self.memo.get(plan.get(1)?)?.0.clone();
        Some((population, condition))
    }

    /// The condition and summand texts of the guarded summand
    /// `if q then e else 0` planned under `key`, the form the checker gives
    /// an integer population sum's `where`.
    fn guarded_summand(&self, key: Key) -> Option<(String, String)> {
        let node = self.expression.nodes.get(key.0)?;
        if !matches!(node.value_type, ValueType::Integer | ValueType::Int(_)) {
            return None;
        }
        let NodeKind::If { otherwise, .. } = &node.kind else {
            return None;
        };
        let zero = matches!(
            &self.expression.nodes.get(*otherwise)?.kind,
            NodeKind::Literal(CheckedLiteral(Value::Integer(value))) if *value == Integer::zero()
        );
        let plan = self.plans.get(&key)?;
        let condition = self.memo.get(plan.first()?)?.0.clone();
        let summand = self.memo.get(plan.get(1)?)?.0.clone();
        zero.then_some((condition, summand))
    }

    fn print(&self, index: usize, env: Env) -> Result<Printed, String> {
        let rows = self.rows;
        let node = &self.expression.nodes[index];
        let plan =
            self.plans.get(&(index, env)).cloned().ok_or_else(|| {
                "checked expression node is planned before it is printed".to_owned()
            })?;
        let rendered = |child: usize| self.operand(child, env);
        let child = |child: usize| rendered(child).map(|(text, _)| text);
        let child_prec = |child: usize| rendered(child).map(|(_, precedence)| precedence);
        let planned = |position: usize| -> Result<(String, u8), String> {
            let key = plan
                .get(position)
                .copied()
                .ok_or_else(|| "checked binding site operands are planned".to_owned())?;
            self.text_of(key).cloned()
        };
        let mut reads = None;
        let printed = match &node.kind {
            NodeKind::Literal(literal) => (literal_text(&literal.0, &rows.declarations)?, 10),
            NodeKind::Coerce(operand, _) => (child(*operand)?, child_prec(*operand)?),
            NodeKind::Local(slot) => {
                let scope = self
                    .lookup(env, *slot)
                    .ok_or_else(|| "checked local has a binding site".to_owned())?;
                match &self.scopes[scope].entry {
                    Entry::Name(name) => (name.clone(), 10),
                    Entry::Operand { value } => self.text_of(*value)?.clone(),
                }
            }
            NodeKind::Let { name, .. } => {
                let (value, _) = planned(0)?;
                let (body, body_precedence) = planned(1)?;
                if name == CHECKER_OPERAND {
                    // FR-931 "Binders": the checker's operand `let` is part of
                    // the comparison it wraps and prints as that comparison.
                    let inner = plan[1];
                    let own = inner
                        .1
                        .ok_or_else(|| "checker operand binder has a scope".to_owned())?;
                    let mut needed = self.reads.get(&inner).cloned().ok_or_else(|| {
                        malformed("a checker operand binder outside an IEEE ordering")
                    })?;
                    if !needed.remove(&own) {
                        return Err(malformed("a checker operand binder no operand reads"));
                    }
                    reads = Some(needed);
                    (body, body_precedence)
                } else {
                    (format!("let {name} = {value} in {body}"), 0)
                }
            }
            NodeKind::Case {
                declaration, arms, ..
            } => {
                let (scrutinee, _) = planned(0)?;
                let members = rows
                    .union_members
                    .get(declaration)
                    .ok_or_else(|| "checked case has its union's declared members".to_owned())?;
                let mut printed_arms = String::new();
                if !arms.windows(2).all(|pair| pair[0].member < pair[1].member) {
                    return Err("checked case arms are in declared member order".to_owned());
                }
                for (position, arm) in arms.iter().enumerate() {
                    let member = members
                        .get(arm.member)
                        .ok_or_else(|| "checked case arm names a declared member".to_owned())?;
                    let (body, _) = planned(position + 1)?;
                    let binders = if arm.names.is_empty() {
                        String::new()
                    } else {
                        format!("({})", arm.names.join(", "))
                    };
                    printed_arms.push_str(&format!("{member}{binders}: {body}; "));
                }
                (format!("case {scrutinee} {{ {printed_arms}}}"), 0)
            }
            NodeKind::UnionValue {
                declaration,
                member,
                arguments,
            } => {
                let identity = rows
                    .declarations
                    .get(declaration)
                    .ok_or_else(|| "union value has a canonical declaration identity".to_owned())?;
                let members = rows
                    .union_members
                    .get(declaration)
                    .ok_or_else(|| "union value has its union's declared members".to_owned())?;
                let member = members
                    .get(*member)
                    .ok_or_else(|| "union value names a declared member".to_owned())?;
                let arguments = arguments
                    .iter()
                    .map(|argument| child(*argument))
                    .collect::<Result<Vec<_>, _>>()?;
                if arguments.is_empty() {
                    (format!("{identity}::{member}"), 10)
                } else {
                    (
                        format!("{identity}::{member}({})", arguments.join(", ")),
                        10,
                    )
                }
            }
            NodeKind::Query {
                visit,
                slot,
                name,
                source: source_index,
                body: body_index,
            } => {
                let keyword = match visit {
                    Visit::Map => "map",
                    Visit::Filter => "filter",
                    Visit::Forall => "forall",
                    Visit::Exists => "exists",
                    Visit::Count => "count",
                    Visit::Sum => "sum",
                    Visit::Only => "only",
                };
                let prefix = match visit {
                    Visit::Count | Visit::Sum | Visit::Only => {
                        format!("{keyword}<{}>", rows.aggregate_type_argument(node))
                    }
                    _ => keyword.to_owned(),
                };
                let filtered = matches!(visit, Visit::Only | Visit::Sum)
                    .then(|| self.population_filter(*source_index, *slot, name, env))
                    .flatten();
                let guarded = (matches!(visit, Visit::Sum) && self.is_population(*source_index))
                    .then(|| plan.get(1).and_then(|key| self.guarded_summand(*key)))
                    .flatten();
                let text = if let Some((population, condition)) = filtered {
                    let (body, _) = planned(1)?;
                    format!("{prefix}({name} in {population} where {condition}: {body})")
                } else if let Some((condition, summand)) = guarded {
                    let (source, _) = planned(0)?;
                    format!("{prefix}({name} in {source} where {condition}: {summand})")
                } else if matches!(visit, Visit::Count) && self.is_population(*source_index) {
                    let (source, _) = planned(0)?;
                    let (condition, _) = planned(1)?;
                    if self.is_true(*body_index) {
                        format!("{prefix}({name} in {source})")
                    } else {
                        format!("{prefix}({name} in {source} where {condition})")
                    }
                } else {
                    let (source, _) = planned(0)?;
                    let (body, _) = planned(1)?;
                    format!("{prefix}({name} in {source}: {body})")
                };
                (text, 10)
            }
            NodeKind::Fold {
                accumulator_name,
                binder_name,
                identity,
                ..
            } => {
                let (source, _) = planned(0)?;
                let (step, _) = planned(if identity.is_some() { 2 } else { 1 })?;
                let type_argument = rows.aggregate_type_argument(node);
                match identity {
                    Some(_) => {
                        let (identity, _) = planned(1)?;
                        (
                            format!(
                                "fold<{type_argument}>({accumulator_name}, {binder_name} in {source}: {step}, identity: {identity})"
                            ),
                            10,
                        )
                    }
                    None => (
                        format!(
                            "reduce<{type_argument}>({accumulator_name}, {binder_name} in {source}: {step})"
                        ),
                        10,
                    ),
                }
            }
            NodeKind::Seq { name, .. } => {
                let (low, _) = planned(0)?;
                let (high, _) = planned(1)?;
                let (body, _) = planned(2)?;
                (format!("seq({name} in {low} .. {high}: {body})"), 10)
            }
            // Checked field and attribute names are already alias-free; no selected
            // row is needed to spell them. A `let`, `if`, `case` or operator base is
            // parenthesised before the postfix `.` (FR-931 "Parenthesisation").
            NodeKind::Attribute {
                reference, field, ..
            } => (
                format!(
                    "{}.{}",
                    postfix_base(child(*reference)?, child_prec(*reference)?),
                    field.name
                ),
                9,
            ),
            NodeKind::Field { operand, field, .. } => (
                format!(
                    "{}.{}",
                    postfix_base(child(*operand)?, child_prec(*operand)?),
                    field.name()
                ),
                9,
            ),
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => (
                format!(
                    "if {} then {} else {}",
                    child(*condition)?,
                    child(*then)?,
                    child(*otherwise)?
                ),
                1,
            ),
            NodeKind::Contains(collection, value) => (
                format!("contains({}, {})", child(*collection)?, child(*value)?),
                10,
            ),
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let constructor = match collection_type.kind() {
                    CollectionKind::Sequence => "sequence",
                    CollectionKind::Set => "set",
                    CollectionKind::Bag => "bag",
                    CollectionKind::OrderedSet => "ordered_set",
                };
                let elements = elements
                    .iter()
                    .map(|element| child(*element))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                (format!("{constructor}[{elements}]"), 10)
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let function = rows.functions.get(function).ok_or_else(|| {
                    "checked call has a canonical declaration identity".to_owned()
                })?;
                let arguments = arguments
                    .iter()
                    .map(|argument| child(*argument))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                (format!("{function}({arguments})"), 10)
            }
            NodeKind::ImportedCall {
                callee, arguments, ..
            } => {
                let function = rows.declarations.get(callee).ok_or_else(|| {
                    "imported call has a canonical declaration identity".to_owned()
                })?;
                let arguments = arguments
                    .iter()
                    .map(|argument| child(*argument))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                (format!("{function}({arguments})"), 10)
            }
            NodeKind::Record { declaration, slots } => {
                let identity = rows.declarations.get(declaration).ok_or_else(|| {
                    "record constructor has a canonical declaration identity".to_owned()
                })?;
                let fields = rows.record_fields.get(declaration).ok_or_else(|| {
                    "record constructor has checked declaration field order".to_owned()
                })?;
                if fields.len() != slots.len() {
                    return Err("record constructor slots match its declaration".to_owned());
                }
                let members = fields
                    .iter()
                    .zip(slots)
                    .filter_map(|(field, slot)| match slot {
                        filament_checked::node::RecordSlot::Absent => None,
                        filament_checked::node::RecordSlot::Null => {
                            Some(Ok(format!("{field}: none")))
                        }
                        filament_checked::node::RecordSlot::Present(value) => {
                            Some(child(*value).map(|value| format!("{field}: {value}")))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                (format!("{identity} {{ {members} }}"), 10)
            }
            NodeKind::Order(operator, _, left, right) => binary(
                child(*left)?,
                child_prec(*left)?,
                ordering_text(*operator),
                child(*right)?,
                child_prec(*right)?,
                4,
                false,
            ),
            NodeKind::Equality(operator, _, left, right) => binary(
                child(*left)?,
                child_prec(*left)?,
                equality_text(*operator),
                child(*right)?,
                child_prec(*right)?,
                4,
                false,
            ),
            NodeKind::Arithmetic(operator, left, right) => {
                let (operator, precedence, right_associative) = match operator {
                    Arithmetic::Add => ("+", 6, false),
                    Arithmetic::Subtract => ("-", 6, false),
                    Arithmetic::Multiply => ("*", 7, false),
                };
                binary(
                    child(*left)?,
                    child_prec(*left)?,
                    operator,
                    child(*right)?,
                    child_prec(*right)?,
                    precedence,
                    right_associative,
                )
            }
            NodeKind::IntegerDivision {
                operator,
                left,
                right,
                ..
            } => binary(
                child(*left)?,
                child_prec(*left)?,
                match operator {
                    IntegerDivisionOperator::Div => "div",
                    IntegerDivisionOperator::Rem => "rem",
                    IntegerDivisionOperator::Mod => "mod",
                },
                child(*right)?,
                child_prec(*right)?,
                7,
                false,
            ),
            NodeKind::Connective(operator, _, left, right) => {
                if let Some((lesser, greater, strict)) = self.composition(index, env) {
                    return self.print_composition(env, lesser, greater, strict);
                }
                let (operator, precedence, right_associative) = match operator {
                    Connective::And => ("and", 3, false),
                    Connective::Or => ("or", 2, false),
                    Connective::Implies => ("implies", 1, true),
                };
                binary(
                    child(*left)?,
                    child_prec(*left)?,
                    operator,
                    child(*right)?,
                    child_prec(*right)?,
                    precedence,
                    right_associative,
                )
            }
            NodeKind::Not(operand) => unary("not ", child(*operand)?, child_prec(*operand)?, 8),
            NodeKind::Negate(operand) => unary("-", child(*operand)?, child_prec(*operand)?, 8),
            NodeKind::Size(operand) => (format!("size({})", child(*operand)?), 10),
            NodeKind::PopulationObjects { population } => (declaration_text(population), 10),
            NodeKind::NamedPopulation { population, .. } => (declaration_text(population), 10),
            NodeKind::AllInstances { population } => {
                let population = child(*population)?;
                let type_argument = match &node.value_type {
                    ValueType::Collection(collection) => match collection.element() {
                        ValueType::Reference(identity) => identity
                            .owner_chain()
                            .last()
                            .map(|key| declaration_uri(key.package(), key.node())),
                        _ => None,
                    },
                    _ => None,
                };
                let prefix = type_argument
                    .map(|argument| format!("allInstances<{argument}>"))
                    .unwrap_or_else(|| "allInstances".to_owned());
                (format!("{prefix}({population})"), 10)
            }
            _ => {
                return Err("checked expression has no canonical printer for this form".to_owned());
            }
        };
        Ok(Printed {
            text: printed.0,
            precedence: printed.1,
            reads,
        })
    }

    /// FR-931 "Binders": the checker's IEEE ordering composition prints as
    /// the one comparison it denotes. With no operand binder or one, the
    /// lesser operand is on the left; with two, the source orientation holds.
    fn print_composition(
        &self,
        env: Env,
        lesser: usize,
        greater: usize,
        strict: bool,
    ) -> Result<Printed, String> {
        let lesser_binder = self.operand_binder(lesser, env);
        let greater_binder = self.operand_binder(greater, env);
        let (left, left_precedence) = self.operand(lesser, env)?;
        let (right, right_precedence) = self.operand(greater, env)?;
        let needed = lesser_binder
            .into_iter()
            .chain(greater_binder)
            .collect::<BTreeSet<_>>();
        if needed.len()
            != usize::from(lesser_binder.is_some()) + usize::from(greater_binder.is_some())
        {
            return Err(malformed(
                "an IEEE ordering operand reading one binder twice",
            ));
        }
        // The outer `let` (the source's left operand) is the earlier scope.
        let greater_is_source_left = matches!(
            (lesser_binder, greater_binder),
            (Some(l), Some(g)) if g < l
        );
        let (operator, text) = if greater_is_source_left {
            (
                if strict { ">" } else { ">=" },
                (right, right_precedence, left, left_precedence),
            )
        } else {
            (
                if strict { "<" } else { "<=" },
                (left, left_precedence, right, right_precedence),
            )
        };
        let (text, precedence) = binary(text.0, text.1, operator, text.2, text.3, 4, false);
        Ok(Printed {
            text,
            precedence,
            reads: Some(needed),
        })
    }
}

/// `base` as the operand of a postfix `.`: grouped unless it binds at least as
/// tightly as a postfix form.
fn postfix_base(base: String, precedence: u8) -> String {
    if precedence < 9 {
        format!("({base})")
    } else {
        base
    }
}

fn malformed(what: &str) -> String {
    format!("checked expression is malformed: {what}")
}

/// Build labels solely from the retained checked source conditions and their
/// positional clause binders. Source slices are deliberately not involved.
pub(super) fn source_labels(
    valuation: &fsl_replay::spine::TemporalValuation,
) -> Result<BTreeMap<fsl_semantics::check::FormulaNodeId, String>, String> {
    let clause = &valuation.clause;
    let mut binders = vec![clause.over().0.clone()];
    if let Some((trigger, _)) = clause.trigger() {
        binders.push(trigger.to_owned());
    }
    binders.extend(clause.captures().map(|(name, _)| name.to_owned()));
    let rows = CanonicalRows {
        declarations: valuation.canonical_declarations.clone(),
        functions: valuation.canonical_functions.clone(),
        record_fields: valuation.canonical_record_fields.clone(),
        union_members: valuation.canonical_union_members.clone(),
        type_arguments: valuation.canonical_type_arguments.clone(),
    };
    valuation
        .checked_conditions
        .iter()
        .map(|(atom, body)| {
            CheckedExpression::from_source(body.root())
                .and_then(|expression| expression.render(&binders, &rows))
                .map(|label| (*atom, format!("holds({label})")))
        })
        .collect()
}

/// Build the canonical atom text from an FR-940 package clause-term view.
/// Binder names and slot identities come only from the admitted package
/// projection and clause form; the caller supplies the package-derived type
/// rows used by the shared printer.
pub(super) fn package_label(
    term: &fsl_package::BareClauseTerm,
    over: &str,
    trigger: Option<&str>,
    captures: &[String],
    rows: &CanonicalRows,
) -> Result<String, String> {
    use fsl_package::ClauseBinder;

    let mut names = Vec::with_capacity(1 + usize::from(trigger.is_some()) + captures.len());
    names.push(over.to_owned());
    if let Some(trigger) = trigger {
        names.push(trigger.to_owned());
    }
    names.extend(captures.iter().cloned());
    let slots = term
        .binders()
        .iter()
        .map(|(slot, binder)| {
            let ordinal = match binder {
                ClauseBinder::Over => 0,
                ClauseBinder::Trigger => {
                    if trigger.is_none() {
                        return Err("reader projection trigger binder is retained".to_owned());
                    }
                    1
                }
                ClauseBinder::Capture(index) => 1 + usize::from(trigger.is_some()) + index,
            };
            let name = names
                .get(ordinal)
                .ok_or_else(|| "reader projection clause binder is retained".to_owned())?;
            Ok((*slot, name.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    CheckedExpression::from_projection(term.projection())?
        .render_with_slots(&slots, rows)
        .map(|label| format!("holds({label})"))
}

fn checked_children(node: CheckedNode<'_>) -> Vec<CheckedNode<'_>> {
    let mut ids = Vec::<NodeId>::new();
    let _ = node.kind().map_ref(
        |child| {
            let index = ids.len();
            ids.push(*child);
            index
        },
        Clone::clone,
    );
    ids.into_iter().map(|id| node.at(id)).collect()
}

fn binary(
    left: String,
    left_precedence: u8,
    operator: &str,
    right: String,
    right_precedence: u8,
    precedence: u8,
    right_associative: bool,
) -> (String, u8) {
    let left =
        if left_precedence < precedence || (right_associative && left_precedence == precedence) {
            format!("({left})")
        } else {
            left
        };
    let right = if right_precedence < precedence
        || (!right_associative && right_precedence == precedence)
    {
        format!("({right})")
    } else {
        right
    };
    (format!("{left} {operator} {right}"), precedence)
}

fn unary(operator: &str, operand: String, precedence: u8, own_precedence: u8) -> (String, u8) {
    if precedence < own_precedence {
        (
            format!("{} ({operand})", operator.trim_end()),
            own_precedence,
        )
    } else {
        (format!("{operator}{operand}"), own_precedence)
    }
}

fn ordering_text(operator: filament_exact::OrderingOperator) -> &'static str {
    use filament_exact::OrderingOperator as Operator;
    match operator {
        Operator::Less => "<",
        Operator::LessOrEqual => "<=",
        Operator::Greater => ">",
        Operator::GreaterOrEqual => ">=",
    }
}

fn equality_text(operator: EqualityOperator) -> &'static str {
    match operator {
        EqualityOperator::Equal => "=",
        EqualityOperator::NotEqual => "!=",
    }
}

fn declaration_text(declaration: &filament_checked::key::DeclarationKey) -> String {
    declaration_uri(&declaration.package, &declaration.node)
}

fn declaration_uri(package: &str, node: &str) -> String {
    format!("ix://{package}/{node}")
}

fn literal_text(value: &Value, declarations: &BTreeMap<NodeKey, String>) -> Result<String, String> {
    match value {
        Value::Boolean(value) => Ok(value.to_string()),
        Value::Float(value) => Ok(match value.width() {
            IeeeWidth::Binary32 => format!("float32(bits: 0x{:08x})", value.bits()),
            IeeeWidth::Binary64 => format!("float64(bits: 0x{:016x})", value.bits()),
        }),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Rational(value) => Ok(format!(
            "rational({}, {})",
            value.numerator(),
            value.denominator()
        )),
        Value::Decimal(value) => Ok(format!(
            "decimal({}, {})",
            value.representation().coefficient(),
            value.representation().scale()
        )),
        Value::Text(value) => serde_json::to_string(value.retained()).map_err(|e| e.to_string()),
        Value::String(value) => serde_json::to_string(value).map_err(|e| e.to_string()),
        Value::Enum(member) => {
            let owner = member.variant().owner();
            let identity = declarations
                .get(owner)
                .cloned()
                .ok_or_else(|| "enum atom declaration key is available".to_owned())?;
            Ok(format!("{identity}::{}", member.variant().case()))
        }
        _ => Err("checked literal has no canonical expression spelling".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_checked::node::ConnectiveOrigin;
    use filament_exact::{IeeeValue, Integer, IntegerInterval};
    use ix_trace_rs::trace;

    fn ordinary(nodes: &mut Vec<ExpressionNode>, kind: NodeKind<usize, CheckedLiteral>) -> usize {
        nodes.push(ExpressionNode {
            kind,
            value_type: ValueType::Boolean,
            type_argument: None,
        });
        nodes.len() - 1
    }

    fn typed(
        nodes: &mut Vec<ExpressionNode>,
        kind: NodeKind<usize, CheckedLiteral>,
        value_type: ValueType,
    ) -> usize {
        nodes.push(ExpressionNode {
            kind,
            value_type,
            type_argument: None,
        });
        nodes.len() - 1
    }

    fn float(nodes: &mut Vec<ExpressionNode>, bits: u64) -> usize {
        ordinary(
            nodes,
            NodeKind::Literal(CheckedLiteral(Value::Float(IeeeValue::binary64(bits)))),
        )
    }

    /// The checker's FR-397 composition of `lesser < greater` (strict) or
    /// `lesser <= greater` over two operand nodes.
    fn composition(
        nodes: &mut Vec<ExpressionNode>,
        lesser: usize,
        greater: usize,
        strict: bool,
    ) -> usize {
        let and = |nodes: &mut Vec<ExpressionNode>, left, right| {
            ordinary(
                nodes,
                NodeKind::Connective(Connective::And, ConnectiveOrigin::Synthesized, left, right),
            )
        };
        let lesser_ordinary = ordinary(nodes, NodeKind::IeeeNumericEquality(lesser, lesser));
        let greater_ordinary = ordinary(nodes, NodeKind::IeeeNumericEquality(greater, greater));
        let both = and(nodes, lesser_ordinary, greater_ordinary);
        let total = ordinary(
            nodes,
            NodeKind::IeeeTotalOrder(filament_checked::node::OrderedKind::Float, lesser, greater),
        );
        let equal = ordinary(nodes, NodeKind::IeeeNumericEquality(lesser, greater));
        let relation = if strict {
            let apart = ordinary(nodes, NodeKind::Not(equal));
            and(nodes, total, apart)
        } else {
            ordinary(
                nodes,
                NodeKind::Connective(Connective::Or, ConnectiveOrigin::Synthesized, total, equal),
            )
        };
        and(nodes, both, relation)
    }

    fn operand_let(
        nodes: &mut Vec<ExpressionNode>,
        slot: usize,
        value: usize,
        body: usize,
    ) -> usize {
        ordinary(
            nodes,
            NodeKind::Let {
                slot,
                name: CHECKER_OPERAND.to_owned(),
                value,
                body,
            },
        )
    }

    fn render(nodes: Vec<ExpressionNode>, root: usize) -> Result<String, String> {
        CheckedExpression { nodes, root }.render(&[], &CanonicalRows::default())
    }

    const A: u64 = 0x3ff0_0000_0000_0000;
    const A_TEXT: &str = "float64(bits: 0x3ff0000000000000)";
    const Z_TEXT: &str = "float64(bits: 0x0000000000000000)";

    /// FR-931-AC-14: with one bound operand the composition prints
    /// lesser-left, the checker `let` and its name never print.
    #[test]
    #[trace("TC-3039", "FR-931-AC-14")]
    fn one_operand_binder_prints_the_lesser_operand_left() {
        let mut nodes = Vec::new();
        let a = float(&mut nodes, A);
        let negated = ordinary(&mut nodes, NodeKind::Negate(a));
        let zero = float(&mut nodes, 0);
        let bound = ordinary(&mut nodes, NodeKind::Local(0));
        // `-A < Z`: the left operand is bound, the right is a literal.
        let body = composition(&mut nodes, bound, zero, true);
        let root = operand_let(&mut nodes, 0, negated, body);
        assert_eq!(
            render(nodes, root).unwrap(),
            format!("-{A_TEXT} < {Z_TEXT}")
        );

        // `Z >= -A` is the same tree, non-strict, with the operands read the other way.
        let mut nodes = Vec::new();
        let a = float(&mut nodes, A);
        let negated = ordinary(&mut nodes, NodeKind::Negate(a));
        let zero = float(&mut nodes, 0);
        let bound = ordinary(&mut nodes, NodeKind::Local(0));
        let body = composition(&mut nodes, bound, zero, false);
        let root = operand_let(&mut nodes, 0, negated, body);
        assert_eq!(
            render(nodes, root).unwrap(),
            format!("-{A_TEXT} <= {Z_TEXT}")
        );
    }

    /// FR-931-AC-14: with two bound operands the source orientation holds,
    /// so `-A > -Z` and `-Z < -A` print as two different texts.
    #[test]
    #[trace("TC-3039", "FR-931-AC-14")]
    fn two_operand_binders_keep_the_source_orientation() {
        let two = |greater_first: bool| {
            let mut nodes = Vec::new();
            let a = float(&mut nodes, A);
            let left = ordinary(&mut nodes, NodeKind::Negate(a));
            let z = float(&mut nodes, 0);
            let right = ordinary(&mut nodes, NodeKind::Negate(z));
            let outer = ordinary(&mut nodes, NodeKind::Local(0));
            let inner = ordinary(&mut nodes, NodeKind::Local(1));
            // `-A > -Z` composes (lesser, greater) = (inner, outer);
            // `-A < -Z` composes (outer, inner).
            let body = if greater_first {
                composition(&mut nodes, inner, outer, true)
            } else {
                composition(&mut nodes, outer, inner, true)
            };
            let inner_let = operand_let(&mut nodes, 1, right, body);
            let root = operand_let(&mut nodes, 0, left, inner_let);
            render(nodes, root).unwrap()
        };
        assert_eq!(two(true), format!("-{A_TEXT} > -{Z_TEXT}"));
        assert_eq!(two(false), format!("-{A_TEXT} < -{Z_TEXT}"));
    }

    /// FR-931-AC-14: a `#ieee_operand` let that wraps no composition, or
    /// whose binder nothing reads, is malformed and prints no text.
    #[test]
    #[trace("TC-3039", "FR-931-AC-14")]
    fn a_stray_checker_operand_let_is_malformed() {
        let mut nodes = Vec::new();
        let value = float(&mut nodes, A);
        let body = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
        );
        let root = operand_let(&mut nodes, 0, value, body);
        assert!(render(nodes, root).unwrap_err().contains("malformed"));
    }

    /// FR-931 "Parenthesisation": a `case` or `let` under a field read is
    /// A two-member union `U` (`A`, `B`) with its declaration rows.
    fn union_rows() -> (NodeKey, CanonicalRows) {
        let key =
            NodeKey::new("p", vec![filament_exact::PathSegment::Text("U".into())], 0).unwrap();
        let mut rows = CanonicalRows::default();
        rows.declarations.insert(key.clone(), "ix://p/U".to_owned());
        rows.union_members
            .insert(key.clone(), vec!["A".to_owned(), "B".to_owned()]);
        (key, rows)
    }

    /// A `case` over `U::B` whose arms name members `first` and `second`.
    fn case_over(
        nodes: &mut Vec<ExpressionNode>,
        key: &NodeKey,
        first: usize,
        second: usize,
    ) -> usize {
        let scrutinee = ordinary(
            nodes,
            NodeKind::UnionValue {
                declaration: key.clone(),
                member: 1,
                arguments: vec![],
            },
        );
        let truth = ordinary(
            nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
        );
        let arm = |member| filament_checked::node::CaseArm {
            member,
            binders: vec![],
            names: vec![],
            body: truth,
        };
        ordinary(
            nodes,
            NodeKind::Case {
                scrutinee,
                declaration: key.clone(),
                arms: vec![arm(first), arm(second)],
            },
        )
    }

    /// FR-931 "Parenthesisation": a `case`, `let` or `if` under a field read
    /// is parenthesised so the text parses back to the same tree.
    #[test]
    #[trace("TC-3039", "FR-931-AC-13")]
    fn a_case_or_let_under_a_field_read_is_parenthesised() {
        let (key, rows) = union_rows();
        let field_of = |build: &dyn Fn(&mut Vec<ExpressionNode>) -> usize| {
            let mut nodes = Vec::new();
            let operand = build(&mut nodes);
            let field = filament_checked::node::RecordField::new(key.clone(), "f");
            let root = ordinary(
                &mut nodes,
                NodeKind::Field {
                    operand,
                    field,
                    optional: false,
                },
            );
            CheckedExpression { nodes, root }
                .render(&[], &rows)
                .unwrap()
        };
        assert_eq!(
            field_of(&|nodes| case_over(nodes, &key, 0, 1)),
            "(case ix://p/U::B { A: true; B: true; }).f"
        );
        assert_eq!(
            field_of(&|nodes| {
                let value = float(nodes, A);
                let body = ordinary(nodes, NodeKind::Local(0));
                ordinary(
                    nodes,
                    NodeKind::Let {
                        slot: 0,
                        name: "v".to_owned(),
                        value,
                        body,
                    },
                )
            }),
            format!("(let v = {A_TEXT} in v).f")
        );
        assert_eq!(
            field_of(&|nodes| {
                let condition = ordinary(
                    nodes,
                    NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
                );
                let then = float(nodes, A);
                let otherwise = float(nodes, 0);
                ordinary(
                    nodes,
                    NodeKind::If {
                        condition,
                        then,
                        otherwise,
                    },
                )
            }),
            format!("(if true then {A_TEXT} else {Z_TEXT}).f")
        );
    }

    /// FR-931-AC-13: arms out of declared member order are a typed fault,
    /// not a reordered print.
    #[test]
    #[trace("TC-3039", "FR-931-AC-13")]
    fn case_arms_out_of_declared_order_are_an_internal_fault() {
        let (key, rows) = union_rows();
        let mut nodes = Vec::new();
        let root = case_over(&mut nodes, &key, 1, 0);
        let fault = CheckedExpression { nodes, root }
            .render(&[], &rows)
            .unwrap_err();
        assert!(fault.contains("declared member order"), "{fault}");
    }

    /// FR-931-AC-14: an author's `let ieee_operand_0 = e in ...` is an
    /// ordinary `let` printed with its declared name.
    #[test]
    #[trace("TC-3039", "FR-931-AC-14")]
    fn an_authors_let_prints_under_its_declared_name() {
        let mut nodes = Vec::new();
        let value = float(&mut nodes, A);
        let read = ordinary(&mut nodes, NodeKind::Local(0));
        let root = ordinary(
            &mut nodes,
            NodeKind::Let {
                slot: 0,
                name: "ieee_operand_0".to_owned(),
                value,
                body: read,
            },
        );
        assert_eq!(
            render(nodes, root).unwrap(),
            format!("let ieee_operand_0 = {A_TEXT} in ieee_operand_0")
        );
    }

    /// FR-931-AC-16: query forms print the result type retained by the
    /// checked node, including a structural bounded integer.
    #[test]
    #[trace("TC-2601", "FR-931-AC-16")]
    fn count_sum_and_only_print_the_checked_result_type() {
        let bounded = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(4_i64)).unwrap(),
        );
        for (visit, integer_body, result_type, expected) in [
            (
                Visit::Count,
                false,
                bounded.clone(),
                "count<Int[0, 4]>(x in xs: true)",
            ),
            (
                Visit::Sum,
                true,
                ValueType::Integer,
                "sum<Integer>(x in xs: 0)",
            ),
            (
                Visit::Only,
                true,
                bounded.clone(),
                "only<Int[0, 4]>(x in xs: 0)",
            ),
        ] {
            let mut nodes = Vec::new();
            let source = ordinary(&mut nodes, NodeKind::Local(0));
            let body = if integer_body {
                typed(
                    &mut nodes,
                    NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::zero()))),
                    ValueType::Integer,
                )
            } else {
                typed(
                    &mut nodes,
                    NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
                    ValueType::Boolean,
                )
            };
            let root = typed(
                &mut nodes,
                NodeKind::Query {
                    visit,
                    slot: 1,
                    name: "x".to_owned(),
                    source,
                    body,
                },
                result_type,
            );
            let expression = CheckedExpression { nodes, root };
            assert_eq!(
                expression
                    .render(&["xs".to_owned()], &CanonicalRows::default())
                    .unwrap(),
                expected
            );
        }
    }

    /// FR-931-AC-16: a named scalar aggregate argument keeps its declaration
    /// identity while an otherwise identical structural type stays structural.
    #[test]
    #[trace("TC-2601", "FR-931-AC-16")]
    fn aggregate_type_alias_keeps_its_source_identity() {
        let bounded = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(4_i64)).unwrap(),
        );
        let mut nodes = Vec::new();
        let source = ordinary(&mut nodes, NodeKind::Local(0));
        let body = typed(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
            ValueType::Boolean,
        );
        let root = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Count,
                slot: 1,
                name: "x".to_owned(),
                source,
                body,
            },
            bounded,
        );
        nodes[root].type_argument = Some("Small".to_owned());
        let expression = CheckedExpression { nodes, root };
        let rows = CanonicalRows {
            type_arguments: BTreeMap::from([("Small".to_owned(), "ix://I/Small".to_owned())]),
            ..CanonicalRows::default()
        };
        assert_eq!(
            expression.render(&["xs".to_owned()], &rows).unwrap(),
            "count<ix://I/Small>(x in xs: true)"
        );
    }

    /// FR-931-AC-16: fold and reduce take their printed type from the checked
    /// result node and distinguish themselves by the retained identity.
    #[test]
    #[trace("TC-2601", "FR-931-AC-16")]
    fn fold_and_reduce_print_the_checked_result_type() {
        for (identity, expected) in [
            (Some(0), "fold<Integer>(acc, x in xs: acc, identity: 0)"),
            (None, "reduce<Integer>(acc, x in xs: acc)"),
        ] {
            let mut nodes = Vec::new();
            let source = ordinary(&mut nodes, NodeKind::Local(0));
            let identity_node = identity.map(|_| {
                typed(
                    &mut nodes,
                    NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::zero()))),
                    ValueType::Integer,
                )
            });
            let step = ordinary(&mut nodes, NodeKind::Local(1));
            let root = typed(
                &mut nodes,
                NodeKind::Fold {
                    accumulator: 1,
                    binder: 2,
                    accumulator_name: "acc".to_owned(),
                    binder_name: "x".to_owned(),
                    source,
                    step,
                    identity: identity_node,
                },
                ValueType::Integer,
            );
            let expression = CheckedExpression { nodes, root };
            assert_eq!(
                expression
                    .render(&["xs".to_owned()], &CanonicalRows::default())
                    .unwrap(),
                expected
            );
        }
    }

    /// FR-931-AC-17: population `where` clauses print in their source form;
    /// population counts omit the expression colon.
    #[test]
    #[trace("TC-2601", "FR-931-AC-17")]
    fn population_query_forms_print_where_without_a_filter_call() {
        let population = filament_checked::key::DeclarationKey {
            package: "p".to_owned(),
            node: "P".to_owned(),
        };
        let mut nodes = Vec::new();
        let population_node = ordinary(
            &mut nodes,
            NodeKind::NamedPopulation {
                population: population.clone(),
                members: Vec::new(),
            },
        );
        let condition = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
        );
        let filtered_population = ordinary(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Filter,
                slot: 1,
                name: "x".to_owned(),
                source: population_node,
                body: condition,
            },
        );
        let result = typed(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::zero()))),
            ValueType::Integer,
        );
        let only = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Only,
                slot: 1,
                name: "x".to_owned(),
                source: filtered_population,
                body: result,
            },
            ValueType::Integer,
        );
        assert_eq!(
            CheckedExpression {
                nodes: nodes.clone(),
                root: only,
            }
            .render(&[], &CanonicalRows::default())
            .unwrap(),
            "only<Integer>(x in ix://p/P where true: 0)"
        );

        let count = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Count,
                slot: 1,
                name: "x".to_owned(),
                source: population_node,
                body: condition,
            },
            ValueType::Integer,
        );
        assert_eq!(
            CheckedExpression { nodes, root: count }
                .render(&[], &CanonicalRows::default())
                .unwrap(),
            "count<Integer>(x in ix://p/P)"
        );
    }

    /// FR-931-AC-17: an integer population sum whose `where` the checker
    /// folded into an `if q then e else 0` summand prints its `where` form.
    #[test]
    #[trace("TC-2601", "FR-931-AC-17")]
    fn a_folded_population_sum_prints_its_where() {
        let population = filament_checked::key::DeclarationKey {
            package: "p".to_owned(),
            node: "P".to_owned(),
        };
        let mut nodes = Vec::new();
        let source = ordinary(
            &mut nodes,
            NodeKind::NamedPopulation {
                population,
                members: Vec::new(),
            },
        );
        let condition = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
        );
        let summand = typed(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::from(3u64)))),
            ValueType::Integer,
        );
        let zero = typed(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::zero()))),
            ValueType::Integer,
        );
        let guarded = typed(
            &mut nodes,
            NodeKind::If {
                condition,
                then: summand,
                otherwise: zero,
            },
            ValueType::Integer,
        );
        let sum = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Sum,
                slot: 1,
                name: "x".to_owned(),
                source,
                body: guarded,
            },
            ValueType::Integer,
        );
        assert_eq!(
            CheckedExpression { nodes, root: sum }
                .render(&[], &CanonicalRows::default())
                .unwrap(),
            "sum<Integer>(x in ix://p/P where true: 3)"
        );
    }

    /// FR-931-AC-17: a population `count` writes its `where` only when its
    /// checked form holds one.
    #[test]
    #[trace("TC-2601", "FR-931-AC-17")]
    fn a_population_count_prints_where_only_with_a_condition() {
        let population = filament_checked::key::DeclarationKey {
            package: "p".to_owned(),
            node: "P".to_owned(),
        };
        let mut nodes = Vec::new();
        let source = ordinary(
            &mut nodes,
            NodeKind::NamedPopulation {
                population,
                members: Vec::new(),
            },
        );
        let condition = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(false))),
        );
        let count = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Count,
                slot: 1,
                name: "x".to_owned(),
                source,
                body: condition,
            },
            ValueType::Integer,
        );
        assert_eq!(
            CheckedExpression { nodes, root: count }
                .render(&[], &CanonicalRows::default())
                .unwrap(),
            "count<Integer>(x in ix://p/P where false)"
        );
    }

    /// FR-931-AC-17: a collection `filter` that is the source of a `count`
    /// keeps its own body, never read as a population `where`.
    #[test]
    #[trace("TC-2601", "FR-931-AC-17")]
    fn a_collection_filter_source_keeps_its_body() {
        let mut nodes = Vec::new();
        let collection = ordinary(&mut nodes, NodeKind::Local(0));
        let kept = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
        );
        let filtered = ordinary(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Filter,
                slot: 1,
                name: "y".to_owned(),
                source: collection,
                body: kept,
            },
        );
        let body = ordinary(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(false))),
        );
        let count = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Count,
                slot: 1,
                name: "x".to_owned(),
                source: filtered,
                body,
            },
            ValueType::Integer,
        );
        assert_eq!(
            CheckedExpression { nodes, root: count }
                .render(&["xs".to_owned()], &CanonicalRows::default())
                .unwrap(),
            "count<Integer>(x in filter(y in xs: true): false)"
        );
    }

    /// FR-931-AC-16: a named type argument with no owner of its own (an
    /// imported or qualified name) prints its retained authored name, never
    /// the structural spelling.
    #[test]
    #[trace("TC-2601", "FR-931-AC-16")]
    fn an_imported_alias_prints_its_authored_name() {
        let bounded = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(4_i64)).unwrap(),
        );
        let mut nodes = Vec::new();
        let source = ordinary(&mut nodes, NodeKind::Local(0));
        let body = typed(
            &mut nodes,
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
            ValueType::Boolean,
        );
        let root = typed(
            &mut nodes,
            NodeKind::Query {
                visit: Visit::Count,
                slot: 1,
                name: "x".to_owned(),
                source,
                body,
            },
            bounded,
        );
        nodes[root].type_argument = Some("Lib::Small".to_owned());
        let rows = CanonicalRows {
            type_arguments: BTreeMap::from([("Lib::Small".to_owned(), "Lib::Small".to_owned())]),
            ..CanonicalRows::default()
        };
        assert_eq!(
            CheckedExpression { nodes, root }
                .render(&["xs".to_owned()], &rows)
                .unwrap(),
            "count<Lib::Small>(x in xs: true)"
        );
    }
}
