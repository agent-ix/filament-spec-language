// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-812 to FR-814: the lowering of invariant state clauses over
//! object populations to TLA+ and Lean 4 text.
//!
//! The mapper reads the checked package's node graph for each obligation's
//! expression and the selected domain packages for the object types, their
//! fields and operations. It lowers integer and Boolean fields, operation
//! parameters, integer arithmetic and comparison, Boolean connectives, `pre`
//! and the exact `narrow` conversion. Any other construct is a refusal that
//! names it, so the export stops as a mapper failure rather than writing a
//! text that states less than the clause says.

mod temporal;

use self::temporal::{ClauseLow, ClauseRead, FairLow, FairRead};
use super::map::{mangle, Graph};
use super::request::Target;
use fsl_replay::protocol_analysis::node_text;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// A construct this build does not lower, and why.
pub(super) struct Refusal {
    pub(super) construct: String,
    pub(super) detail: String,
    /// The obligation whose clause failed, when one did.
    pub(super) obligation: Option<String>,
}

type Lower<T> = std::result::Result<T, Refusal>;

fn refuse<T>(construct: &str, detail: impl Into<String>) -> Lower<T> {
    Err(Refusal {
        construct: construct.to_owned(),
        detail: detail.into(),
        obligation: None,
    })
}

/// The lowered text, its name map and the output regions of each obligation.
pub(super) struct Lowered {
    pub(super) text: String,
    pub(super) name_map: Vec<Value>,
    pub(super) regions: BTreeMap<String, Vec<(usize, usize)>>,
    /// The output regions written: one per definition.
    pub(super) definitions: usize,
}

// ---- The model -----------------------------------------------------------

#[derive(Clone)]
enum Kind {
    Int {
        lo: Option<String>,
        hi: Option<String>,
    },
    Bool,
}

struct Field {
    id: String,
    name: String,
    kind: Kind,
}

struct Param {
    name: String,
    kind: Kind,
}

struct Operation {
    name: String,
    params: Vec<Param>,
    every: BTreeSet<String>,
    receiver: BTreeSet<String>,
}

struct Object {
    id: String,
    name: String,
    population: String,
    fields: Vec<Field>,
    operations: Vec<Operation>,
    /// The node of the object type in the package graph.
    node: usize,
}

impl Object {
    fn variable(&self) -> String {
        mangle("v_", &self.name)
    }
    fn record(&self) -> String {
        mangle("T_", &self.name)
    }
    fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }
}

fn str_of<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn items<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// A raw object type of a domain package. Its body is read lazily: a type no
/// obligation names never refuses the export.
struct RawObject {
    id: String,
    name: String,
    body: Lower<(Vec<Field>, Vec<Operation>)>,
    populations: Vec<String>,
    population_note: Option<String>,
}

fn short_name(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

fn kind_of(type_ref: &str, values: &BTreeMap<String, Kind>) -> Lower<Kind> {
    match type_ref {
        "ix://filament/native/Integer" => Ok(Kind::Int { lo: None, hi: None }),
        "ix://filament/native/Boolean" => Ok(Kind::Bool),
        other => values.get(other).cloned().map_or_else(
            || {
                refuse(
                    "model/type",
                    format!("the type `{other}` is neither an integer nor a Boolean"),
                )
            },
            Ok,
        ),
    }
}

fn read_object(
    node: &Value,
    id: &str,
    values: &BTreeMap<String, Kind>,
) -> Lower<(Vec<Field>, Vec<Operation>)> {
    if !items(node, "supertypes").is_empty() {
        return refuse(
            "model/object_type",
            format!("`{id}` has supertypes, which this build does not lower"),
        );
    }
    let mut fields = Vec::new();
    for field in items(node, "fields") {
        let name = str_of(field, "name").unwrap_or_default();
        if str_of(field, "presence") != Some("required")
            || field["nullable"].as_bool() != Some(false)
        {
            return refuse(
                "model/field",
                format!("`{id}` field `{name}` is not a required, non-null field"),
            );
        }
        fields.push(Field {
            id: str_of(field, "identity").unwrap_or_default().to_owned(),
            name: name.to_owned(),
            kind: kind_of(str_of(field, "typeRef").unwrap_or_default(), values)?,
        });
    }
    let mut operations = Vec::new();
    for operation in items(node, "operations") {
        let name = str_of(operation, "name").unwrap_or_default();
        if !items(operation, "pre").is_empty() || !items(operation, "post").is_empty() {
            return refuse(
                "model/operation",
                format!("`{id}` operation `{name}` carries requires or ensures clauses"),
            );
        }
        let frame = &operation["frame"];
        if !items(frame, "creates").is_empty() || !items(frame, "deletes").is_empty() {
            return refuse(
                "model/operation",
                "an operation that creates or deletes objects",
            );
        }
        let set = |key: &str| {
            items(frame, key)
                .iter()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect::<BTreeSet<_>>()
        };
        let mut params = Vec::new();
        for param in items(operation, "params") {
            let kind = kind_of(str_of(param, "typeRef").unwrap_or_default(), values)?;
            if matches!(kind, Kind::Bool) {
                return refuse("model/operation", "a Boolean operation parameter");
            }
            params.push(Param {
                name: str_of(param, "name").unwrap_or_default().to_owned(),
                kind,
            });
        }
        operations.push(Operation {
            name: name.to_owned(),
            params,
            every: set("modifies"),
            receiver: set("modifies_receiver"),
        });
    }
    Ok((fields, operations))
}

fn read_models(models: &[Vec<u8>]) -> Lower<BTreeMap<String, RawObject>> {
    let mut documents = Vec::new();
    for bytes in models {
        let document: Value = serde_json::from_slice(bytes)
            .map_err(|_| refusal("model/package", "a model package is not JSON"))?;
        documents.push(document);
    }
    let mut values = BTreeMap::new();
    for document in &documents {
        for node in items(document, "types") {
            let (Some(id), Some("integer")) = (str_of(node, "identity"), str_of(node, "scalar"))
            else {
                continue;
            };
            let bound = |keyword: &str| {
                items(node, "constraints")
                    .iter()
                    .find(|constraint| str_of(constraint, "keyword") == Some(keyword))
                    .and_then(|constraint| constraint["operands"]["value"].as_str())
                    .map(str::to_owned)
            };
            values.insert(
                id.to_owned(),
                Kind::Int {
                    lo: bound("min"),
                    hi: bound("max"),
                },
            );
        }
    }
    let mut objects = BTreeMap::new();
    for document in &documents {
        for node in items(document, "types") {
            if node["kind"]["name"] != "object_type" {
                continue;
            }
            let id = str_of(node, "identity")
                .ok_or_else(|| refusal("model/object_type", "an object type has no identity"))?;
            objects.insert(
                id.to_owned(),
                RawObject {
                    id: id.to_owned(),
                    name: short_name(id).to_owned(),
                    body: read_object(node, id, &values),
                    populations: Vec::new(),
                    population_note: None,
                },
            );
        }
    }
    for document in &documents {
        for population in items(document, "populations") {
            let id = str_of(population, "identity").unwrap_or_default();
            let members = items(population, "members");
            for member in members.iter().filter_map(Value::as_str) {
                if let Some(object) = objects.get_mut(member) {
                    object.populations.push(id.to_owned());
                    if members.len() != 1 || object.populations.len() != 1 {
                        object.population_note =
                            Some(format!("`{id}` is not the one population of `{member}`"));
                    }
                }
            }
        }
    }
    Ok(objects)
}

fn refusal(construct: &str, detail: &str) -> Refusal {
    Refusal {
        construct: construct.to_owned(),
        detail: detail.to_owned(),
        obligation: None,
    }
}

// ---- Expressions ---------------------------------------------------------

#[derive(Clone, Copy, Eq, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Lt,
    Le,
    Gt,
    Ge,
    EqInt,
    NeInt,
    And,
    Or,
    Implies,
    EqBool,
    NeBool,
}

#[derive(Clone)]
enum E {
    Int(String),
    Bool(bool),
    Param(String),
    Field {
        object: usize,
        key: Box<E>,
        field: usize,
        primed: bool,
    },
    Neg(Box<E>),
    Not(Box<E>),
    Bin(Op, Box<E>, Box<E>),
    Conj(Vec<E>),
}

/// A value and the definedness conditions of its operations, in evaluation
/// order (FSpec FR-812 "Arithmetic and undefinedness").
struct Low {
    value: E,
    defined: Vec<E>,
}

impl Low {
    fn clause(self) -> E {
        let mut all = self.defined;
        all.push(self.value);
        E::Conj(all)
    }
}

fn conj(mut items: Vec<E>) -> E {
    match items.len() {
        0 => E::Bool(true),
        1 => items.remove(0),
        _ => E::Conj(items),
    }
}

struct Lowerer<'a> {
    graph: &'a Graph<'a>,
    objects: &'a [Object],
}

impl Lowerer<'_> {
    fn object_of(&self, id: &str) -> Option<usize> {
        self.objects.iter().position(|object| object.id == id)
    }

    fn term(&mut self, term: &Value, primed: bool) -> Lower<Low> {
        match str_of(term, "term") {
            Some("reference") => {
                let position = self
                    .graph
                    .position_of(&term["target"])
                    .ok_or_else(|| refusal("checked package", "a reference names no node"))?;
                self.node(position, primed)
            }
            Some("literal") => Self::literal(term),
            other => refuse(
                "expression/term",
                format!("a `{}` term", other.unwrap_or("unknown")),
            ),
        }
    }

    fn literal(term: &Value) -> Lower<Low> {
        let value = &term["value"];
        let value = match str_of(term, "value_kind") {
            Some("integer") => value.as_str().map(|text| E::Int(text.to_owned())),
            Some("boolean") => value
                .as_bool()
                .or_else(|| value.as_str().map(|text| text == "true"))
                .map(E::Bool),
            _ => None,
        };
        value.map_or_else(
            || {
                refuse(
                    "value/literal",
                    "a literal that is neither an integer nor a Boolean",
                )
            },
            |value| {
                Ok(Low {
                    value,
                    defined: Vec::new(),
                })
            },
        )
    }

    fn node(&mut self, position: usize, primed: bool) -> Lower<Low> {
        let node = self.graph.node(position);
        let body = serde_json::to_value(&node.body)
            .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
        let construct = format!("{}/{}", node.node_tag, node.semantic_form);
        if &*node.node_tag == "value" && &*node.semantic_form == "parameter" {
            let name = items(&body, "members")
                .iter()
                .find(|member| str_of(member, "name") == Some("name"))
                .and_then(|member| member["value"]["value"].as_str())
                .ok_or_else(|| refusal(&construct, "a parameter has no name"))?;
            return Ok(Low {
                value: E::Param(mangle("p_", name)),
                defined: Vec::new(),
            });
        }
        match str_of(&body, "term") {
            Some("literal") => Self::literal(&body),
            Some("application") => {
                let identity = body["operation"]["identity"].as_str().unwrap_or_default();
                self.application(&construct, identity, &body, primed)
            }
            _ => refuse(&construct, "a node that is not an application or a literal"),
        }
    }

    fn argument(
        &mut self,
        construct: &str,
        body: &Value,
        index: usize,
        primed: bool,
    ) -> Lower<Low> {
        let term = items(body, "arguments")
            .get(index)
            .ok_or_else(|| refusal(construct, "an application is missing an argument"))?;
        self.term(term, primed)
    }

    fn application(
        &mut self,
        construct: &str,
        identity: &str,
        body: &Value,
        primed: bool,
    ) -> Lower<Low> {
        let op = match identity {
            "filament.op.integer.add" => Some(Op::Add),
            "filament.op.integer.sub" => Some(Op::Sub),
            "filament.op.integer.mul" => Some(Op::Mul),
            "filament.op.integer.lt" => Some(Op::Lt),
            "filament.op.integer.le" => Some(Op::Le),
            "filament.op.integer.gt" => Some(Op::Gt),
            "filament.op.integer.ge" => Some(Op::Ge),
            "filament.op.integer.eq" => Some(Op::EqInt),
            "filament.op.integer.ne" => Some(Op::NeInt),
            "filament.op.boolean.and" => Some(Op::And),
            "filament.op.boolean.or" => Some(Op::Or),
            "filament.op.boolean.implies" => Some(Op::Implies),
            "filament.op.boolean.eq" => Some(Op::EqBool),
            "filament.op.boolean.ne" => Some(Op::NeBool),
            _ => None,
        };
        if let Some(op) = op {
            let left = self.argument(construct, body, 0, primed)?;
            let right = self.argument(construct, body, 1, primed)?;
            let mut defined = left.defined;
            let value = E::Bin(op, Box::new(left.value.clone()), Box::new(right.value));
            // `and`, `or` and `implies` skip the right operand (FR-006), so
            // its definedness holds only where it is evaluated.
            let guard = match op {
                Op::And | Op::Implies => Some(left.value),
                Op::Or => Some(E::Not(Box::new(left.value))),
                _ => None,
            };
            match guard {
                Some(guard) if !right.defined.is_empty() => defined.push(E::Bin(
                    Op::Implies,
                    Box::new(guard),
                    Box::new(conj(right.defined)),
                )),
                _ => defined.extend(right.defined),
            }
            return Ok(Low { value, defined });
        }
        match identity {
            "filament.op.boolean.not" | "filament.op.integer.negate" => {
                let inner = self.argument(construct, body, 0, primed)?;
                let wrap = if identity.ends_with("not") {
                    E::Not
                } else {
                    E::Neg
                };
                Ok(Low {
                    value: wrap(Box::new(inner.value)),
                    defined: inner.defined,
                })
            }
            "filament.op.state.pre" => self.argument(construct, body, 0, false),
            "filament.op.model.deref" => self.argument(construct, body, 0, primed),
            "filament.op.record.project" => self.project(construct, body, primed),
            "filament.op.numeric.narrow" => self.narrow(construct, body, primed),
            other => refuse(construct, format!("the operation `{other}`")),
        }
    }

    fn project(&mut self, construct: &str, body: &Value, primed: bool) -> Lower<Low> {
        let member = &body["operation"]["member"];
        let declaration = &member["declaration"]["path"];
        let object = declaration
            .get(2)
            .and_then(Value::as_str)
            .and_then(|id| self.object_of(id))
            .ok_or_else(|| refusal(construct, "a field of a type outside the exported state"))?;
        let name = str_of(member, "name").unwrap_or_default();
        let field = self
            .objects
            .get(object)
            .and_then(|object| object.field_index(name))
            .ok_or_else(|| refusal(construct, "a member that is not a field"))?;
        let key = self.argument(construct, body, 0, primed)?;
        Ok(Low {
            value: E::Field {
                object,
                key: Box::new(key.value),
                field,
                primed,
            },
            defined: key.defined,
        })
    }

    fn narrow(&mut self, construct: &str, body: &Value, primed: bool) -> Lower<Low> {
        let target = self
            .graph
            .position_of(&body["operation"]["member"]["declaration"])
            .ok_or_else(|| refusal(construct, "a conversion to no known type"))?;
        let node = self.graph.node(target);
        let range = serde_json::to_value(&node.body)
            .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
        if &*node.semantic_form != "integer_range" {
            return refuse(
                construct,
                "a conversion to a type that is not an integer range",
            );
        }
        let bound = |name: &str| {
            items(&range, "members")
                .iter()
                .find(|member| str_of(member, "name") == Some(name))
                .and_then(|member| member["value"]["value"].as_str())
                .map(str::to_owned)
        };
        let (Some(lo), Some(hi)) = (bound("min"), bound("max")) else {
            return refuse(construct, "an integer range without both bounds");
        };
        let mut inner = self.argument(construct, body, 0, primed)?;
        inner.defined.push(E::Bin(
            Op::And,
            Box::new(E::Bin(
                Op::Le,
                Box::new(E::Int(lo)),
                Box::new(inner.value.clone()),
            )),
            Box::new(E::Bin(
                Op::Le,
                Box::new(inner.value.clone()),
                Box::new(E::Int(hi)),
            )),
        ));
        Ok(inner)
    }
}

// ---- Temporal clause reads -------------------------------------------------

/// The declared name of the `value`/`parameter` node at `position`.
fn parameter_name(graph: &Graph<'_>, position: usize) -> Option<String> {
    let body = serde_json::to_value(&graph.node(position).body).ok()?;
    items(&body, "members")
        .iter()
        .find(|member| str_of(member, "name") == Some("name"))
        .and_then(|member| member["value"]["value"].as_str())
        .map(str::to_owned)
}

/// The object type the `over` binder at `position` ranges over: its
/// identity and its node.
fn context_of(graph: &Graph<'_>, position: usize) -> Lower<(String, usize)> {
    let fail = |detail: &str| refusal("temporal/temporal_clause", detail);
    let node = graph.node(position);
    let ty = graph
        .position(&node.semantic_type)
        .ok_or_else(|| fail("the `over` binder has no type node"))?;
    let body = serde_json::to_value(&graph.node(ty).body)
        .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
    let target = body["members"]
        .get(0)
        .map(|member| &member["target"])
        .ok_or_else(|| fail("the `over` binder is not typed by an object type"))?;
    let id = target["path"]
        .get(2)
        .and_then(Value::as_str)
        .ok_or_else(|| fail("the `over` binder is not typed by an object type"))?;
    let object = graph
        .position_of(target)
        .ok_or_else(|| fail("the `over` binder's object type is no package node"))?;
    Ok((id.to_owned(), object))
}

/// The fairness constraints of the clause at `position`, in written order.
fn fairness_of(graph: &Graph<'_>, position: usize) -> Lower<Vec<FairRead>> {
    let body = serde_json::to_value(&graph.node(position).body)
        .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
    let mut reads = Vec::new();
    for member in items(&body["arguments"][4], "members") {
        let node = graph
            .position_of(&member["target"])
            .ok_or_else(|| refusal("temporal/fairness", "a fairness names no package node"))?;
        let body = serde_json::to_value(&graph.node(node).body)
            .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
        let fairness = &body["operation"]["member"];
        let target = &fairness["target"];
        if str_of(target, "kind") != Some("operation") {
            return refuse(
                "temporal/fairness",
                "a fairness over a target other than a model operation",
            );
        }
        reads.push(FairRead {
            strong: str_of(fairness, "fairness_kind") == Some("strong"),
            each: str_of(fairness, "granularity") == Some("each"),
            declaration: target["declaration"]["path"]
                .get(2)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            operation: str_of(target, "name").unwrap_or_default().to_owned(),
        });
    }
    Ok(reads)
}

// ---- Rendering -----------------------------------------------------------

/// One operation's lowered clauses.
struct OpLow {
    pre: Vec<E>,
    post: Vec<E>,
    node: usize,
}

/// What the renderers read.
struct Input<'a> {
    objects: &'a [Object],
    operations: &'a [Vec<OpLow>],
    snapshot: Option<&'a Value>,
    show: &'a Show<'a>,
    first: usize,
    clauses: &'a [(String, usize, usize, E)],
    temporals: &'a [ClauseLow],
}

struct Show<'a> {
    target: Target,
    objects: &'a [Object],
}

fn int_text(value: &str) -> String {
    if value.starts_with('-') {
        format!("({value})")
    } else {
        value.to_owned()
    }
}

impl Show<'_> {
    fn exprs(&self, items: &[E]) -> Lower<Vec<String>> {
        items.iter().map(|item| self.expr(item)).collect()
    }

    fn expr(&self, e: &E) -> Lower<String> {
        let tla = self.target == Target::Tla;
        Ok(match e {
            E::Int(value) => int_text(value),
            E::Bool(true) => if tla { "TRUE" } else { "True" }.to_owned(),
            E::Bool(false) => if tla { "FALSE" } else { "False" }.to_owned(),
            E::Param(name) => name.clone(),
            E::Field {
                object,
                key,
                field,
                primed,
            } => {
                let object = self
                    .objects
                    .get(*object)
                    .ok_or_else(|| refusal("checked package", "a read of an unexported type"))?;
                let field = object
                    .fields
                    .get(*field)
                    .ok_or_else(|| refusal("checked package", "a read of an unknown field"))?;
                let key = self.expr(key)?;
                let name = mangle("f_", &field.name);
                if tla {
                    let tick = if *primed { "'" } else { "" };
                    format!("{}{tick}[{key}].{name}", object.variable())
                } else {
                    let state = if *primed { "s'" } else { "s" };
                    let read = format!(
                        "({} {key} {state}.{}).{name}",
                        mangle("F_get_", &object.name),
                        object.variable()
                    );
                    match field.kind {
                        Kind::Bool => format!("({read} = true)"),
                        Kind::Int { .. } => read,
                    }
                }
            }
            E::Neg(inner) => format!("(-{})", self.expr(inner)?),
            E::Not(inner) => {
                if tla {
                    format!("~{}", self.expr(inner)?)
                } else {
                    format!("Not {}", self.expr(inner)?)
                }
            }
            E::Bin(op, left, right) => {
                let (left, right) = (self.expr(left)?, self.expr(right)?);
                match (op, tla) {
                    (Op::NeBool, true) => format!("~({left} <=> {right})"),
                    (Op::NeInt, false) => format!("(Ne {left} {right})"),
                    (Op::NeBool, false) => format!("Not ({left} <-> {right})"),
                    _ => {
                        let symbol = match (op, tla) {
                            (Op::Add, _) => "+",
                            (Op::Sub, _) => "-",
                            (Op::Mul, _) => "*",
                            (Op::Lt, _) => "<",
                            (Op::Le, _) => "<=",
                            (Op::Gt, _) => ">",
                            (Op::Ge, _) => ">=",
                            (Op::EqInt, _) => "=",
                            (Op::And, _) => "/\\",
                            (Op::Or, _) => "\\/",
                            (Op::NeInt, _) => "/=",
                            (Op::Implies, true) => "=>",
                            (Op::Implies, false) => "->",
                            (Op::EqBool, true) => "<=>",
                            (Op::EqBool, false) => "<->",
                            (Op::NeBool, _) => "<=>",
                        };
                        format!("({left} {symbol} {right})")
                    }
                }
            }
            E::Conj(items) => format!("({})", self.exprs(items)?.join(" /\\ ")),
        })
    }
}

/// The scalar values of `text`, as FSpec FR-812 writes a text.
fn scalars(text: &str, target: Target) -> String {
    let values: Vec<String> = text.chars().map(|c| u32::from(c).to_string()).collect();
    match target {
        Target::Tla => format!("<<{}>>", values.join(", ")),
        Target::Lean => format!("[{}]", values.join(", ")),
    }
}

// ---- The module ----------------------------------------------------------

struct Definition {
    node: usize,
    name: Option<String>,
    body: String,
    obligation: Option<String>,
}

fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c == ' ' || !c.is_ascii() || c.is_ascii_control() {
            out.push_str(&format!("\\u{{{:X}}}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

fn node_id_text(graph: &Graph<'_>, position: usize) -> Lower<String> {
    node_text(&graph.node(position).node_id)
        .map_err(|_| refusal("checked package", "a node id does not spell"))
}

fn source_region(graph: &Graph<'_>, position: usize) -> Option<(String, u64, u64)> {
    let id = &graph.node(position).node_id;
    graph
        .package
        .source_map()
        .iter()
        .filter(|entry| &entry.node_id == id)
        .find_map(|entry| entry.regions.first())
        .map(|region| {
            (
                format!("{}/{}", region.source.authority, region.source.identity),
                region.start,
                region.end,
            )
        })
}

fn comment(graph: &Graph<'_>, position: usize, target: Target) -> Lower<String> {
    let marker = if target == Target::Tla { "\\*" } else { "--" };
    let region = source_region(graph, position).map_or_else(
        || "none".to_owned(),
        |(source, start, end)| format!("{}:{start}-{end}", escape(&source)),
    );
    Ok(format!(
        "{marker} fsl: {} {region}",
        escape(&node_id_text(graph, position)?)
    ))
}

/// The clause nodes of the package: kind, context type, operation, node.
struct StateClause {
    position: usize,
    kind: String,
    context: String,
    context_node: usize,
    /// The operation anchor node of an operation clause.
    anchor: Option<usize>,
    operation: Option<String>,
    expression: Value,
}

fn state_clauses(graph: &Graph<'_>) -> Lower<Vec<StateClause>> {
    let nodes = &graph.package.resolved_graph().nodes;
    let mut clauses = Vec::new();
    for position in 0..nodes.len() {
        let node = graph.node(position);
        if &*node.node_tag != "state" || &*node.semantic_form != "state_clause" {
            continue;
        }
        let body = serde_json::to_value(&node.body)
            .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
        let kind = body["operation"]["member"]["clause"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let arguments = items(&body, "arguments");
        let (Some(context), Some(expression)) = (arguments.get(1), arguments.get(2)) else {
            return refuse("state/state_clause", "a clause without a context or a body");
        };
        let target = &context["target"];
        let anchor = graph.position_of(target);
        let (context_id, context_node, operation, anchor) = if target["path"][0] == "state" {
            let anchor = anchor.ok_or_else(|| refusal("state/state_clause", "no anchor node"))?;
            let anchor_body = serde_json::to_value(&graph.node(anchor).body)
                .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
            let member = |name: &str| {
                items(&anchor_body, "members")
                    .iter()
                    .find(|member| str_of(member, "name") == Some(name))
                    .map(|member| member["value"].clone())
            };
            let context =
                member("context").ok_or_else(|| refusal("state/operation_anchor", "no context"))?;
            let operation =
                member("operation").and_then(|v| v["value"].as_str().map(str::to_owned));
            let context_node = graph
                .position_of(&context["target"])
                .ok_or_else(|| refusal("state/operation_anchor", "no context node"))?;
            (
                context["target"]["path"][2]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                context_node,
                operation,
                Some(anchor),
            )
        } else {
            (
                target["path"][2].as_str().unwrap_or_default().to_owned(),
                anchor.ok_or_else(|| refusal("state/state_clause", "no context node"))?,
                None,
                None,
            )
        };
        clauses.push(StateClause {
            position,
            kind,
            context: context_id,
            context_node,
            anchor,
            operation,
            expression: expression.clone(),
        });
    }
    Ok(clauses)
}

fn range_conditions(path: &str, kind: &Kind, target: Target) -> Vec<String> {
    let tla = target == Target::Tla;
    match kind {
        Kind::Bool => {
            if tla {
                vec![format!("{path} \\in BOOLEAN")]
            } else {
                Vec::new()
            }
        }
        Kind::Int { lo, hi } => {
            let mut out = Vec::new();
            if tla {
                out.push(format!("{path} \\in Int"));
            }
            if let Some(lo) = lo {
                out.push(format!("{path} >= {}", int_text(lo)));
            }
            if let Some(hi) = hi {
                out.push(format!("{path} <= {}", int_text(hi)));
            }
            out
        }
    }
}

/// The parameter's domain in TLA+: an explicit range, `Int` or `BOOLEAN`.
fn tla_domain(kind: &Kind) -> String {
    match kind {
        Kind::Bool => "BOOLEAN".to_owned(),
        Kind::Int {
            lo: Some(lo),
            hi: Some(hi),
        } => format!("{}..{}", int_text(lo), int_text(hi)),
        Kind::Int { .. } => "Int".to_owned(),
    }
}

/// Lower the invariants named in `wanted` (name and clause node) of the
/// checked package, with the request's `initial` snapshot.
///
/// # Errors
///
/// A [`Refusal`] naming the first construct this build does not lower.
pub(super) fn lower(
    graph: &Graph<'_>,
    models: &[Vec<u8>],
    snapshot: Option<&Value>,
    target: Target,
    module: &str,
    wanted: &[(String, usize)],
) -> Lower<Lowered> {
    let clauses = state_clauses(graph)?;
    let mut raw = read_models(models)?;
    let mut contexts: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut reads: Vec<ClauseRead> = Vec::new();
    for (name, position) in wanted {
        if &*graph.node(*position).node_tag == "temporal" {
            let read = temporal::read(graph, name, *position).map_err(|mut refusal| {
                refusal.obligation = Some(name.clone());
                refusal
            })?;
            contexts
                .entry(read.context.clone())
                .or_insert_with(|| (read.context_node, name.clone()));
            reads.push(read);
            continue;
        }
        let clause = clauses
            .iter()
            .find(|clause| clause.position == *position)
            .ok_or_else(|| refusal("state/state_clause", "an obligation that is no clause"))?;
        if clause.kind != "invariant" {
            return refuse("state/state_clause", "a clause that is not an invariant");
        }
        contexts
            .entry(clause.context.clone())
            .or_insert_with(|| (clause.context_node, name.clone()));
    }
    let mut objects = Vec::new();
    for (id, (node, owner)) in &contexts {
        let attribute = |mut refusal: Refusal| {
            refusal.obligation = Some(owner.clone());
            refusal
        };
        let object = raw.remove(id).ok_or_else(|| {
            attribute(refusal(
                "model/object_type",
                &format!("`{id}` is in no selected model"),
            ))
        })?;
        let (fields, operations) = object.body.map_err(attribute)?;
        if let Some(note) = object.population_note {
            return Err(attribute(refusal("model/population", &note)));
        }
        let population = object.populations.into_iter().next().ok_or_else(|| {
            attribute(refusal(
                "model/population",
                &format!("`{id}` is in no population"),
            ))
        })?;
        objects.push(Object {
            id: object.id,
            name: object.name,
            population,
            fields,
            operations,
            node: *node,
        });
    }
    let names: BTreeSet<&str> = objects.iter().map(|object| object.name.as_str()).collect();
    if names.len() != objects.len() {
        return refuse("model/object_type", "two exported types share one name");
    }

    let mut lowerer = Lowerer {
        graph,
        objects: &objects,
    };
    // Each operation's clauses, lowered once: preconditions in the pre-state,
    // postconditions with bare reads in the post-state.
    let mut operations: Vec<Vec<OpLow>> = Vec::new();
    for object in &objects {
        let owner = contexts.get(&object.id).map(|(_, owner)| owner.clone());
        let attribute = |mut refusal: Refusal| {
            refusal.obligation = owner.clone();
            refusal
        };
        let mut per = Vec::new();
        for operation in &object.operations {
            let mut lowered = OpLow {
                pre: Vec::new(),
                post: Vec::new(),
                node: object.node,
            };
            for clause in clauses.iter().filter(|clause| {
                clause.context == object.id && clause.operation.as_deref() == Some(&operation.name)
            }) {
                if let Some(anchor) = clause.anchor {
                    lowered.node = anchor;
                }
                let low = lowerer
                    .term(&clause.expression, clause.kind == "postcondition")
                    .map_err(attribute)?;
                match clause.kind.as_str() {
                    "precondition" => lowered.pre.push(low.clause()),
                    "postcondition" => lowered.post.push(low.clause()),
                    other => {
                        return Err(attribute(refusal(
                            "state/state_clause",
                            &format!("a `{other}` operation clause"),
                        )))
                    }
                }
            }
            per.push(lowered);
        }
        operations.push(per);
    }
    let mut lowered_clauses = Vec::new();
    for (name, position) in wanted {
        if reads.iter().any(|read| read.position == *position) {
            continue;
        }
        let clause = clauses
            .iter()
            .find(|clause| clause.position == *position)
            .ok_or_else(|| refusal("state/state_clause", "an obligation that is no clause"))?;
        let attribute = |mut refusal: Refusal| {
            refusal.obligation = Some(name.clone());
            refusal
        };
        let low = lowerer.term(&clause.expression, false).map_err(attribute)?;
        let object = objects
            .iter()
            .position(|object| object.id == clause.context)
            .ok_or_else(|| refusal("model/object_type", "an unexported clause context"))?;
        lowered_clauses.push((name.clone(), *position, object, low.clause()));
    }
    let mut temporals = Vec::new();
    for read in reads {
        let attribute = |mut refusal: Refusal| {
            refusal.obligation = Some(read.name.clone());
            refusal
        };
        let object = objects
            .iter()
            .position(|object| object.id == read.context)
            .ok_or_else(|| refusal("model/object_type", "an unexported clause context"))
            .map_err(attribute)?;
        let (mut cur, mut next) = (Vec::new(), Vec::new());
        for &leaf in &read.atoms {
            cur.push(lowerer.node(leaf, false).map_err(attribute)?.clause());
            next.push(lowerer.node(leaf, true).map_err(attribute)?.clause());
        }
        let mut fairness = Vec::new();
        for fair in &read.fairness {
            let found = objects
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.id == fair.declaration)
                .and_then(|(index, candidate)| {
                    candidate
                        .operations
                        .iter()
                        .position(|operation| operation.name == fair.operation)
                        .map(|operation| (index, operation))
                });
            let (object, operation) = found
                .ok_or_else(|| {
                    refusal(
                        "temporal/fairness",
                        "a fairness over an operation outside the exported state",
                    )
                })
                .map_err(attribute)?;
            fairness.push(FairLow {
                strong: fair.strong,
                each: fair.each,
                object,
                operation,
            });
        }
        temporals.push(ClauseLow {
            name: read.name,
            position: read.position,
            object,
            binder: read.binder,
            tf: read.tf,
            cur,
            next,
            fairness,
        });
    }

    let show = Show {
        target,
        objects: &objects,
    };
    let mut definitions = Vec::new();
    let first = objects.first().map_or(0, |object| object.node);
    let input = Input {
        objects: &objects,
        operations: &operations,
        snapshot,
        show: &show,
        first,
        clauses: &lowered_clauses,
        temporals: &temporals,
    };
    let extra = match target {
        Target::Tla => tla(&input, &mut definitions)?,
        Target::Lean => {
            lean(&input, &mut definitions)?;
            Vec::new()
        }
    };
    assemble(graph, target, module, &objects, &extra, definitions)
}

// ---- Snapshot ------------------------------------------------------------

enum Lit {
    Int(String),
    Bool(bool),
}

impl Lit {
    fn text(&self, target: Target) -> String {
        match (self, target) {
            (Self::Int(value), _) => int_text(value),
            (Self::Bool(true), Target::Tla) => "TRUE".to_owned(),
            (Self::Bool(false), Target::Tla) => "FALSE".to_owned(),
            (Self::Bool(value), Target::Lean) => value.to_string(),
        }
    }
}

/// The objects of the snapshot's population of `object`, sorted by key
/// scalars, with each field's value.
fn snapshot_rows(object: &Object, snapshot: &Value) -> Lower<Vec<(String, Vec<Lit>)>> {
    let mut rows = Vec::new();
    for population in items(snapshot, "populations") {
        if str_of(population, "population") != Some(object.population.as_str()) {
            continue;
        }
        for row in items(population, "objects") {
            let key = str_of(row, "key").unwrap_or_default();
            let mut values = Vec::new();
            for field in &object.fields {
                let value = &row["fields"][field.name.as_str()];
                let lit = match &field.kind {
                    Kind::Int { .. } => value["integer"].as_str().map(|n| Lit::Int(n.to_owned())),
                    Kind::Bool => value["boolean"].as_bool().map(Lit::Bool),
                };
                values.push(lit.ok_or_else(|| {
                    refusal(
                        "state/snapshot",
                        &format!("`{key}` has no readable value for `{}`", field.name),
                    )
                })?);
            }
            rows.push((key.to_owned(), values));
        }
    }
    rows.sort_by_key(|(key, _)| key.chars().map(u32::from).collect::<Vec<_>>());
    Ok(rows)
}

fn record_literal(object: &Object, values: &[Lit], target: Target) -> String {
    let pairs: Vec<String> = object
        .fields
        .iter()
        .zip(values)
        .map(|(field, value)| {
            let name = mangle("f_", &field.name);
            let value = value.text(target);
            match target {
                Target::Tla => format!("{name} |-> {value}"),
                Target::Lean => format!("{name} := {value}"),
            }
        })
        .collect();
    match target {
        Target::Tla => format!("[{}]", pairs.join(", ")),
        Target::Lean => format!("{{ {} }}", pairs.join(", ")),
    }
}

fn bullets(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("  /\\ {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn define(node: usize, name: Option<String>, body: String, obligation: Option<&str>) -> Definition {
    Definition {
        node,
        name,
        body,
        obligation: obligation.map(str::to_owned),
    }
}

/// The signature parameters of `operation`: the receiver, then its own.
fn parameters(operation: &Operation) -> Vec<(String, Option<&Kind>)> {
    std::iter::once(("p_self".to_owned(), None))
        .chain(
            operation
                .params
                .iter()
                .map(|param| (mangle("p_", &param.name), Some(&param.kind))),
        )
        .collect()
}

/// Which fields an operation leaves to the frame: `(field, receiver only)`.
fn frame_fields<'a>(object: &'a Object, operation: &Operation) -> Vec<(&'a Field, bool)> {
    object
        .fields
        .iter()
        .filter(|field| !operation.every.contains(&field.id))
        .map(|field| (field, operation.receiver.contains(&field.id)))
        .collect()
}

// ---- TLA+ ----------------------------------------------------------------

fn tla(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<Vec<String>> {
    let target = Target::Tla;
    let objects = input.objects;
    let variables: Vec<String> = objects.iter().map(Object::variable).collect();
    out.push(define(
        input.first,
        None,
        format!("vars == <<{}>>", variables.join(", ")),
        None,
    ));
    let mut type_ok = Vec::new();
    for object in objects {
        let v = object.variable();
        type_ok.push(format!("DOMAIN {v} \\subseteq Seq(Nat)"));
        let fields: Vec<String> = object
            .fields
            .iter()
            .flat_map(|field| {
                range_conditions(
                    &format!("{v}[k].{}", mangle("f_", &field.name)),
                    &field.kind,
                    target,
                )
            })
            .collect();
        if !fields.is_empty() {
            type_ok.push(format!("\\A k \\in DOMAIN {v} : {}", fields.join(" /\\ ")));
        }
    }
    out.push(define(
        input.first,
        None,
        format!("TypeOK ==\n{}", bullets(&type_ok)),
        None,
    ));
    let init = match input.snapshot {
        None => "Init == TypeOK".to_owned(),
        Some(snapshot) => {
            let mut parts = Vec::new();
            for object in objects {
                let rows = snapshot_rows(object, snapshot)?;
                let keys: Vec<String> = rows.iter().map(|(key, _)| scalars(key, target)).collect();
                let cases: Vec<String> = rows
                    .iter()
                    .zip(&keys)
                    .map(|((_, values), key)| {
                        format!("k = {key} -> {}", record_literal(object, values, target))
                    })
                    .collect();
                let function = if rows.is_empty() {
                    format!("{} = [k \\in {{}} |-> 0]", object.variable())
                } else {
                    format!(
                        "{} = [k \\in {{{}}} |-> CASE {}]",
                        object.variable(),
                        keys.join(", "),
                        cases.join(" [] ")
                    )
                };
                parts.push(function);
            }
            format!("Init ==\n{}", bullets(&parts))
        }
    };
    out.push(define(input.first, None, init, None));
    let mut disjuncts = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let v = object.variable();
        for (operation, lowered) in object.operations.iter().zip(&input.operations[index]) {
            let name = mangle("op_", &operation.name);
            let params = parameters(operation);
            let names: Vec<&str> = params.iter().map(|(name, _)| name.as_str()).collect();
            let mut lines = vec![format!("p_self \\in DOMAIN {v}")];
            for (param, kind) in params.iter().skip(1) {
                if let Some(kind) = kind {
                    lines.extend(range_conditions(param, kind, target));
                }
            }
            lines.extend(input.show.exprs(&lowered.pre)?);
            lines.extend(input.show.exprs(&lowered.post)?);
            lines.push(format!("DOMAIN {v}' = DOMAIN {v}"));
            for (field, receiver) in frame_fields(object, operation) {
                let f = mangle("f_", &field.name);
                let guard = if receiver { "k /= p_self => " } else { "" };
                lines.push(format!(
                    "\\A k \\in DOMAIN {v} : {guard}{v}'[k].{f} = {v}[k].{f}"
                ));
            }
            for other in objects.iter().filter(|other| other.id != object.id) {
                lines.push(format!("UNCHANGED {}", other.variable()));
            }
            lines.push("TypeOK'".to_owned());
            out.push(define(
                lowered.node,
                Some(name.clone()),
                format!("{name}({}) ==\n{}", names.join(", "), bullets(&lines)),
                None,
            ));
            let binders: Vec<String> = params
                .iter()
                .map(|(param, kind)| match kind {
                    None => format!("{param} \\in DOMAIN {v}"),
                    Some(kind) => format!("{param} \\in {}", tla_domain(kind)),
                })
                .collect();
            disjuncts.push(format!(
                "  \\/ \\E {} : {name}({})",
                binders.join(", "),
                names.join(", ")
            ));
        }
    }
    let next = if disjuncts.is_empty() {
        "Next == FALSE".to_owned()
    } else {
        format!("Next ==\n{}", disjuncts.join("\n"))
    };
    out.push(define(input.first, None, next, None));
    out.push(define(
        input.first,
        None,
        "Spec == Init /\\ [][Next]_vars /\\ WF_vars(Next)".to_owned(),
        None,
    ));
    for (name, position, object, formula) in input.clauses {
        let Some(object) = objects.get(*object) else {
            continue;
        };
        let clause = mangle("c_", name);
        out.push(define(
            *position,
            Some(clause.clone()),
            format!(
                "{clause} == \\A p_self \\in DOMAIN {} : {}",
                object.variable(),
                input.show.expr(formula)?
            ),
            Some(name),
        ));
        let theorem = mangle("Th_", name);
        out.push(define(
            *position,
            Some(theorem.clone()),
            format!("THEOREM {theorem} == Spec => []{clause}"),
            Some(name),
        ));
    }
    temporal::tla(input, out)
}

// ---- Lean 4 --------------------------------------------------------------

fn lean(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
    let target = Target::Lean;
    let objects = input.objects;
    out.push(define(
        input.first,
        None,
        "def F_key_lt (a : List Nat) (b : List Nat) : Prop :=\n  match a, b with\n  | [], [] => False\n  | [], _ :: _ => True\n  | _ :: _, [] => False\n  | x :: xs, y :: ys => x < y \\/ (x = y /\\ F_key_lt xs ys)"
            .to_owned(),
        None,
    ));
    for object in objects {
        let record = object.record();
        let fields: Vec<String> = object
            .fields
            .iter()
            .map(|field| {
                let ty = match field.kind {
                    Kind::Int { .. } => "Int",
                    Kind::Bool => "Bool",
                };
                format!("  {} : {ty}", mangle("f_", &field.name))
            })
            .collect();
        out.push(define(
            object.node,
            Some(record.clone()),
            format!("structure {record} where\n{}", fields.join("\n")),
            None,
        ));
        let pair = format!("List (Prod (List Nat) {record})");
        let defaults: Vec<String> = object
            .fields
            .iter()
            .map(|field| {
                let value = match field.kind {
                    Kind::Int { .. } => "0",
                    Kind::Bool => "false",
                };
                format!("{} := {value}", mangle("f_", &field.name))
            })
            .collect();
        let default = mangle("F_default_", &object.name);
        let get = mangle("F_get_", &object.name);
        let has = mangle("F_has_", &object.name);
        let sorted = mangle("F_sorted_", &object.name);
        let ok = mangle("F_ok_", &object.name);
        out.push(define(
            object.node,
            None,
            format!("def {default} : {record} := {{ {} }}", defaults.join(", ")),
            None,
        ));
        out.push(define(
            object.node,
            None,
            format!(
                "def {get} (k : List Nat) (l : {pair}) : {record} :=\n  match l with\n  | [] => {default}\n  | e :: rest => if e.1 = k then e.2 else {get} k rest"
            ),
            None,
        ));
        out.push(define(
            object.node,
            None,
            format!(
                "def {has} (k : List Nat) (l : {pair}) : Prop :=\n  match l with\n  | [] => False\n  | e :: rest => e.1 = k \\/ {has} k rest"
            ),
            None,
        ));
        out.push(define(
            object.node,
            None,
            format!(
                "def {sorted} (l : {pair}) : Prop :=\n  match l with\n  | [] => True\n  | e :: rest =>\n    match rest with\n    | [] => True\n    | f :: more => F_key_lt e.1 f.1 /\\ {sorted} rest"
            ),
            None,
        ));
        let conditions: Vec<String> = object
            .fields
            .iter()
            .flat_map(|field| {
                range_conditions(
                    &format!("e.2.{}", mangle("f_", &field.name)),
                    &field.kind,
                    target,
                )
            })
            .collect();
        let each = if conditions.is_empty() {
            "True".to_owned()
        } else {
            conditions.join(" /\\ ")
        };
        out.push(define(
            object.node,
            None,
            format!(
                "def {ok} (l : {pair}) : Prop :=\n  match l with\n  | [] => True\n  | e :: rest => ({each}) /\\ {ok} rest"
            ),
            None,
        ));
    }
    let state_fields: Vec<String> = objects
        .iter()
        .map(|object| {
            format!(
                "  {} : List (Prod (List Nat) {})",
                object.variable(),
                object.record()
            )
        })
        .collect();
    out.push(define(
        input.first,
        None,
        format!("structure State where\n{}", state_fields.join("\n")),
        None,
    ));
    let well_typed: Vec<String> = objects
        .iter()
        .flat_map(|object| {
            let v = object.variable();
            [
                format!("{} s.{v}", mangle("F_sorted_", &object.name)),
                format!("{} s.{v}", mangle("F_ok_", &object.name)),
            ]
        })
        .collect();
    out.push(define(
        input.first,
        None,
        format!(
            "def WF (s : State) : Prop :=\n  {}",
            if well_typed.is_empty() {
                "True".to_owned()
            } else {
                well_typed.join(" /\\\n  ")
            }
        ),
        None,
    ));
    let init = match input.snapshot {
        None => "def Init (s : State) : Prop := WF s".to_owned(),
        Some(snapshot) => {
            let mut parts = Vec::new();
            for object in objects {
                let rows = snapshot_rows(object, snapshot)?;
                let entries: Vec<String> = rows
                    .iter()
                    .map(|(key, values)| {
                        format!(
                            "({}, {})",
                            scalars(key, target),
                            record_literal(object, values, target)
                        )
                    })
                    .collect();
                parts.push(format!(
                    "s.{} = [{}]",
                    object.variable(),
                    entries.join(", ")
                ));
            }
            let body = if parts.is_empty() {
                "True".to_owned()
            } else {
                parts.join(" /\\\n  ")
            };
            format!("def Init (s : State) : Prop :=\n  {body}")
        }
    };
    out.push(define(input.first, None, init, None));
    let mut steps = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let v = object.variable();
        let has = mangle("F_has_", &object.name);
        let get = mangle("F_get_", &object.name);
        for (operation, lowered) in object.operations.iter().zip(&input.operations[index]) {
            let name = mangle("op_", &operation.name);
            let params = parameters(operation);
            let typed: Vec<String> = params
                .iter()
                .map(|(param, kind)| {
                    let ty = match kind {
                        None => "List Nat",
                        Some(Kind::Bool) => "Bool",
                        Some(Kind::Int { .. }) => "Int",
                    };
                    format!("({param} : {ty})")
                })
                .collect();
            let mut lines = vec![format!("{has} p_self s.{v}")];
            for (param, kind) in params.iter().skip(1) {
                if let Some(kind) = kind {
                    lines.extend(range_conditions(param, kind, target));
                }
            }
            lines.extend(input.show.exprs(&lowered.pre)?);
            lines.extend(input.show.exprs(&lowered.post)?);
            lines.push(format!(
                "List.map (fun e => e.1) s'.{v} = List.map (fun e => e.1) s.{v}"
            ));
            for (field, receiver) in frame_fields(object, operation) {
                let f = mangle("f_", &field.name);
                let guard = if receiver { "Not (k = p_self) -> " } else { "" };
                lines.push(format!(
                    "forall k, {has} k s.{v} -> {guard}({get} k s'.{v}).{f} = ({get} k s.{v}).{f}"
                ));
            }
            for other in objects.iter().filter(|other| other.id != object.id) {
                let w = other.variable();
                lines.push(format!("s'.{w} = s.{w}"));
            }
            lines.push("WF s'".to_owned());
            out.push(define(
                lowered.node,
                Some(name.clone()),
                format!(
                    "def {name} (s : State) (s' : State) {} : Prop :=\n  {}",
                    typed.join(" "),
                    lines
                        .iter()
                        .map(|line| format!("({line})"))
                        .collect::<Vec<_>>()
                        .join(" /\\\n  ")
                ),
                None,
            ));
            let binders: Vec<String> = params
                .iter()
                .map(|(param, _)| format!("exists {param},"))
                .collect();
            let names: Vec<&str> = params.iter().map(|(param, _)| param.as_str()).collect();
            steps.push(format!(
                "({} {name} s s' {})",
                binders.join(" "),
                names.join(" ")
            ));
        }
    }
    let step = if steps.is_empty() {
        "False".to_owned()
    } else {
        steps.join(" \\/\n  ")
    };
    out.push(define(
        input.first,
        None,
        format!("def Step (s : State) (s' : State) : Prop :=\n  {step}"),
        None,
    ));
    out.push(define(
        input.first,
        None,
        "structure Behaviour where\n  st : Nat -> State\n  init : Init (st 0)\n  next : forall n, Step (st n) (st (n + 1)) \\/ (Not (exists t, Step (st n) t) /\\ st (n + 1) = st n)"
            .to_owned(),
        None,
    ));
    for (name, position, object, formula) in input.clauses {
        let Some(object) = objects.get(*object) else {
            continue;
        };
        let clause = mangle("c_", name);
        let holds = mangle("holds_", name);
        let ob = mangle("Ob_", name);
        out.push(define(
            *position,
            Some(clause.clone()),
            format!(
                "def {clause} (s : State) : Prop :=\n  forall p_self, {} p_self s.{} -> {}",
                mangle("F_has_", &object.name),
                object.variable(),
                input.show.expr(formula)?
            ),
            Some(name),
        ));
        out.push(define(
            *position,
            Some(holds.clone()),
            format!(
                "def {holds} (b : Behaviour) (i : Nat) : Prop :=\n  forall j, i <= j -> {clause} (b.st j)"
            ),
            Some(name),
        ));
        out.push(define(
            *position,
            Some(ob.clone()),
            format!("def {ob} : Prop :=\n  forall b : Behaviour, {holds} b 0"),
            Some(name),
        ));
    }
    temporal::lean(input, out)
}

// ---- Assembly ------------------------------------------------------------

fn assemble(
    graph: &Graph<'_>,
    target: Target,
    module: &str,
    objects: &[Object],
    extra: &[String],
    definitions: Vec<Definition>,
) -> Lower<Lowered> {
    let first = objects.first().map_or(0, |object| object.node);
    let mut text = match target {
        Target::Tla => {
            let variables: Vec<String> = objects
                .iter()
                .map(Object::variable)
                .chain(extra.iter().cloned())
                .collect();
            format!(
                "---- MODULE {module} ----\nEXTENDS Naturals, Integers, Sequences, FiniteSets, TLC\n\n{}\nVARIABLES {}\n",
                comment(graph, first, target)?,
                variables.join(", ")
            )
        }
        Target::Lean => format!("namespace {module}\n"),
    };
    let mut name_map = Vec::new();
    for object in objects {
        let region = source_region(graph, object.node)
            .map_or(Value::Null, |(_, start, end)| json!([start, end]));
        name_map.push(json!({
            "name": object.variable(),
            "node": node_id_text(graph, object.node)?,
            "region": region,
        }));
    }
    let count = definitions.len();
    let mut regions: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for definition in definitions {
        text.push('\n');
        text.push_str(&comment(graph, definition.node, target)?);
        text.push('\n');
        let start = text.len();
        text.push_str(&definition.body);
        let end = text.len();
        text.push('\n');
        if let Some(name) = definition.obligation {
            regions.entry(name).or_default().push((start, end));
        }
        if let Some(name) = definition.name {
            let region = source_region(graph, definition.node)
                .map_or(Value::Null, |(_, start, end)| json!([start, end]));
            name_map.push(json!({
                "name": name,
                "node": node_id_text(graph, definition.node)?,
                "region": region,
            }));
        }
    }
    match target {
        Target::Tla => text.push_str("\n====\n"),
        Target::Lean => text.push_str(&format!("\nend {module}\n")),
    }
    Ok(Lowered {
        text,
        name_map,
        regions,
        definitions: count,
    })
}
