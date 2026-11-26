// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-812 to FR-814: lower state and temporal clauses over model
//! populations to TLA+ and Lean 4 text.
//!
//! The mapper reads the checked package's node graph for each obligation's
//! expression and the selected domain packages for the object types, their
//! fields and operations. It lowers bounded integer, Boolean and enum fields,
//! operation parameters, integer arithmetic and comparison, Boolean
//! connectives, `pre`, `narrow`, and the supported temporal operators. Any
//! other construct is a refusal that names it, so the export never writes a
//! text that states less than the clause says.

mod dense;
mod temporal;
mod tree;

use self::temporal::{ClauseLow, ClauseRead, Extra, FairLow, FairRead};
use self::tree::{render, ExprId, Exprs, Piece, E};
use super::map::{mangle, Graph};
use super::request::Target;
use filament_walk::{walk, Children, Walk};
use fsl_replay::protocol_analysis::node_text;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::rc::Rc;

/// A construct this build does not lower, and why.
pub(super) struct Refusal {
    pub(super) construct: String,
    pub(super) detail: String,
    /// The package node the construct is at, when the refusal knows it.
    pub(super) node: Option<usize>,
    /// The obligation whose clause failed, when one did.
    pub(super) obligation: Option<String>,
}

type Lower<T> = std::result::Result<T, Refusal>;

fn refuse<T>(construct: &str, detail: impl Into<String>) -> Lower<T> {
    Err(Refusal {
        construct: construct.to_owned(),
        detail: detail.into(),
        node: None,
        obligation: None,
    })
}

/// The lowered text, its name map and the output regions of each obligation.
pub(super) struct Lowered {
    pub(super) text: String,
    pub(super) name_map: Vec<Value>,
    pub(super) regions: BTreeMap<String, Vec<(usize, usize)>>,
    /// The condition of each obligation whose record is `conditional`.
    pub(super) conditions: BTreeMap<String, Vec<Value>>,
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
    Float64,
    Enum {
        id: String,
        name: String,
        cases: Vec<String>,
    },
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
    id: String,
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
    clocks: Vec<String>,
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
    fn field(&self, field: &Field) -> String {
        mangle("fld_", &field.name)
    }
    fn operation(&self, operation: &Operation) -> String {
        format!(
            "{}_{}",
            mangle("op_", &self.name),
            mangle("", &operation.name)
        )
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
    body: Lower<ObjectParts>,
    populations: Vec<String>,
    population_note: Option<String>,
}

struct ObjectParts {
    fields: Vec<Field>,
    clocks: Vec<String>,
    operations: Vec<Operation>,
}

fn short_name(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

fn enum_type_name(name: &str) -> String {
    mangle("E_", name)
}

fn enum_member_name(owner: &str, member: &str) -> String {
    format!("{}_{}", mangle("e_", owner), mangle("", member))
}

fn kind_of(type_ref: &str, values: &BTreeMap<String, Kind>) -> Lower<Kind> {
    match type_ref {
        "ix://filament/native/Integer" => Ok(Kind::Int { lo: None, hi: None }),
        "ix://filament/native/Boolean" => Ok(Kind::Bool),
        "ix://filament/native/Float64" => Ok(Kind::Float64),
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

fn read_object(node: &Value, id: &str, values: &BTreeMap<String, Kind>) -> Lower<ObjectParts> {
    if !items(node, "supertypes").is_empty() {
        return refuse(
            "model/object_type",
            format!("`{id}` has supertypes, which this build does not lower"),
        );
    }
    let mut fields = Vec::new();
    let mut clocks = Vec::new();
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
        let type_ref = str_of(field, "typeRef").unwrap_or_default();
        if type_ref == "ix://filament/native/Clock" {
            clocks.push(name.to_owned());
            continue;
        }
        fields.push(Field {
            id: str_of(field, "identity").unwrap_or_default().to_owned(),
            name: name.to_owned(),
            kind: kind_of(type_ref, values)?,
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
            id: str_of(operation, "identity").unwrap_or_default().to_owned(),
            name: name.to_owned(),
            params,
            every: set("modifies"),
            receiver: set("modifies_receiver"),
        });
    }
    Ok(ObjectParts {
        fields,
        clocks,
        operations,
    })
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
            let Some(id) = str_of(node, "identity") else {
                continue;
            };
            if str_of(node, "scalar") == Some("integer") {
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
            } else if node["kind"]["name"] == "variant_type"
                && node.get("variants").and_then(Value::as_array).is_some()
            {
                let mut cases: Vec<String> = items(node, "variants")
                    .iter()
                    .filter_map(|variant| str_of(variant, "name").map(str::to_owned))
                    .collect();
                cases.sort();
                values.insert(
                    id.to_owned(),
                    Kind::Enum {
                        id: id.to_owned(),
                        // Generated enum names must retain the full source
                        // identity: two declarations may share a terminal
                        // name while denoting different types.
                        name: id.to_owned(),
                        cases,
                    },
                );
            }
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

/// A typed refusal at the package node `node`.
fn defer_at<T>(node: usize, construct: &str, detail: impl Into<String>) -> Lower<T> {
    Err(Refusal {
        construct: construct.to_owned(),
        detail: detail.into(),
        node: Some(node),
        obligation: None,
    })
}

fn refusal(construct: &str, detail: &str) -> Refusal {
    Refusal {
        construct: construct.to_owned(),
        detail: detail.to_owned(),
        node: None,
        obligation: None,
    }
}

// ---- Expressions ---------------------------------------------------------

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
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
    EqEnum,
    NeEnum,
    And,
    Or,
    Implies,
    EqBool,
    NeBool,
    EqFloat,
}

/// A value and the definedness conditions of its operations, in evaluation
/// order (FSpec FR-812 "Arithmetic and undefinedness").
struct Low {
    value: ExprId,
    defined: Vec<ExprId>,
}

impl Low {
    fn clause(self, exprs: &mut Exprs) -> ExprId {
        let mut all = self.defined;
        all.push(self.value);
        exprs.add(E::Conj(all))
    }
}

fn conj(exprs: &mut Exprs, mut items: Vec<ExprId>) -> ExprId {
    match items.len() {
        0 => exprs.add(E::Bool(true)),
        1 => items.remove(0),
        _ => exprs.add(E::Conj(items)),
    }
}

struct Lowerer<'a> {
    graph: &'a Graph<'a>,
    objects: &'a [Object],
    exprs: Exprs,
}

/// One expression still to lower. `primed` reads the post-state.
enum Visit {
    Term {
        term: Value,
        primed: bool,
    },
    Node {
        position: usize,
        primed: bool,
    },
    /// The argument at `index` of the application `body`.
    Argument {
        construct: Rc<str>,
        body: Rc<Value>,
        index: usize,
        primed: bool,
    },
}

/// What an entered expression does with the values of its operands.
enum Combine {
    /// Nothing: a leaf, or a node whose value is its one operand's.
    Pass,
    Binary(Op),
    Not,
    Neg,
    FloatNeg,
    Field {
        object: usize,
        field: usize,
        primed: bool,
    },
    Narrow {
        lo: String,
        hi: String,
    },
}

/// The walk that lowers one expression: `lows` holds the values of the
/// operands not yet combined.
struct Lowering<'l, 'a> {
    lowerer: &'l mut Lowerer<'a>,
    lows: Vec<Low>,
}

impl Walk for Lowering<'_, '_> {
    type Node = Visit;
    type Frame = Combine;
    type Stop = Refusal;

    fn enter(
        &mut self,
        visit: Visit,
        children: &mut Children<'_, Visit>,
    ) -> ControlFlow<Refusal, Combine> {
        match self.enter_visit(visit, children) {
            Ok(combine) => ControlFlow::Continue(combine),
            Err(refusal) => ControlFlow::Break(refusal),
        }
    }

    fn exit(&mut self, combine: Combine) -> ControlFlow<Refusal> {
        match self.combine(combine) {
            Ok(()) => ControlFlow::Continue(()),
            Err(refusal) => ControlFlow::Break(refusal),
        }
    }
}

impl Lowering<'_, '_> {
    fn leaf(&mut self, value: E) -> Combine {
        let value = self.lowerer.exprs.add(value);
        self.lows.push(Low {
            value,
            defined: Vec::new(),
        });
        Combine::Pass
    }

    fn enter_visit(&mut self, visit: Visit, children: &mut Children<'_, Visit>) -> Lower<Combine> {
        match visit {
            Visit::Term { term, primed } => self.term(&term, primed, children),
            Visit::Argument {
                construct,
                body,
                index,
                primed,
            } => {
                let term = items(&body, "arguments")
                    .get(index)
                    .ok_or_else(|| refusal(&construct, "an application is missing an argument"))?;
                self.term(term, primed, children)
            }
            Visit::Node { position, primed } => self.node(position, primed, children),
        }
    }

    fn term(
        &mut self,
        term: &Value,
        primed: bool,
        children: &mut Children<'_, Visit>,
    ) -> Lower<Combine> {
        match str_of(term, "term") {
            Some("reference") => {
                let position = self
                    .lowerer
                    .graph
                    .position_of(&term["target"])
                    .ok_or_else(|| refusal("checked package", "a reference names no node"))?;
                children.push(Visit::Node { position, primed });
                Ok(Combine::Pass)
            }
            Some("literal") => {
                let value = self.lowerer.literal(term)?;
                Ok(self.leaf(value))
            }
            other => refuse(
                "expression/term",
                format!("a `{}` term", other.unwrap_or("unknown")),
            ),
        }
    }

    fn node(
        &mut self,
        position: usize,
        primed: bool,
        children: &mut Children<'_, Visit>,
    ) -> Lower<Combine> {
        let node = self.lowerer.graph.node(position);
        let body = serde_json::to_value(&node.body)
            .map_err(|_| refusal("checked package", "a node body does not serialize"))?;
        let construct = format!("{}/{}", node.node_tag, node.semantic_form);
        if &*node.node_tag == "value" && &*node.semantic_form == "parameter" {
            let name = items(&body, "members")
                .iter()
                .find(|member| str_of(member, "name") == Some("name"))
                .and_then(|member| member["value"]["value"].as_str())
                .ok_or_else(|| refusal(&construct, "a parameter has no name"))?;
            // The receiver is the operation's object-typed `self` parameter; a
            // declared parameter spelled `self` is an ordinary `p_` parameter
            // (FSpec FR-814).
            let receiver = name == "self" && self.lowerer.is_object_typed(position);
            return Ok(self.leaf(E::Param(if receiver {
                "r_self".to_owned()
            } else {
                mangle("p_", name)
            })));
        }
        match str_of(&body, "term") {
            Some("literal") => {
                let value = self.lowerer.literal(&body)?;
                Ok(self.leaf(value))
            }
            Some("application") => self.application(construct, body, primed, children),
            _ => refuse(&construct, "a node that is not an application or a literal"),
        }
    }

    /// Check the application `body` and name its operands, left first.
    fn application(
        &mut self,
        construct: String,
        body: Value,
        primed: bool,
        children: &mut Children<'_, Visit>,
    ) -> Lower<Combine> {
        let identity = body["operation"]["identity"].as_str().unwrap_or_default();
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
            "filament.op.enum.eq" => Some(Op::EqEnum),
            "filament.op.enum.ne" => Some(Op::NeEnum),
            "filament.op.boolean.and" => Some(Op::And),
            "filament.op.boolean.or" => Some(Op::Or),
            "filament.op.boolean.implies" => Some(Op::Implies),
            "filament.op.boolean.eq" => Some(Op::EqBool),
            "filament.op.boolean.ne" => Some(Op::NeBool),
            "filament.op.ieee.numeric_equal" => Some(Op::EqFloat),
            _ => None,
        };
        let (combine, operands, primed) = if let Some(op) = op {
            (Combine::Binary(op), 2, primed)
        } else {
            match identity {
                "filament.op.boolean.not" => (Combine::Not, 1, primed),
                "filament.op.integer.negate" => (Combine::Neg, 1, primed),
                "filament.op.ieee.float64.negate" => (Combine::FloatNeg, 1, primed),
                "filament.op.state.pre" => (Combine::Pass, 1, false),
                "filament.op.model.deref" => (Combine::Pass, 1, primed),
                "filament.op.record.project" => {
                    (self.lowerer.project(&construct, &body, primed)?, 1, primed)
                }
                "filament.op.numeric.narrow" => {
                    (self.lowerer.narrow(&construct, &body)?, 1, primed)
                }
                other => return refuse(&construct, format!("the operation `{other}`")),
            }
        };
        let (construct, body) = (Rc::<str>::from(construct), Rc::new(body));
        for index in 0..operands {
            children.push(Visit::Argument {
                construct: Rc::clone(&construct),
                body: Rc::clone(&body),
                index,
                primed,
            });
        }
        Ok(combine)
    }

    fn operand(&mut self) -> Lower<Low> {
        self.lows
            .pop()
            .ok_or_else(|| refusal("checked package", "an application lost an operand"))
    }

    fn combine(&mut self, combine: Combine) -> Lower<()> {
        let low = match combine {
            Combine::Pass => return Ok(()),
            Combine::Binary(op) => {
                let right = self.operand()?;
                let left = self.operand()?;
                let exprs = &mut self.lowerer.exprs;
                let mut defined = left.defined;
                let value = exprs.add(E::Bin(op, left.value, right.value));
                // `and`, `or` and `implies` skip the right operand (FR-006), so
                // its definedness holds only where it is evaluated.
                let guard = match op {
                    Op::And | Op::Implies => Some(left.value),
                    Op::Or => Some(exprs.add(E::Not(left.value))),
                    _ => None,
                };
                match guard {
                    Some(guard) if !right.defined.is_empty() => {
                        let held = conj(exprs, right.defined);
                        defined.push(exprs.add(E::Bin(Op::Implies, guard, held)));
                    }
                    _ => defined.extend(right.defined),
                }
                Low { value, defined }
            }
            Combine::Not => self.wrap(E::Not)?,
            Combine::Neg => self.wrap(E::Neg)?,
            Combine::FloatNeg => self.wrap(E::FloatNeg)?,
            Combine::Field {
                object,
                field,
                primed,
            } => self.wrap(|key| E::Field {
                object,
                key,
                field,
                primed,
            })?,
            Combine::Narrow { lo, hi } => {
                let mut inner = self.operand()?;
                let exprs = &mut self.lowerer.exprs;
                let lo = exprs.add(E::Int(lo));
                let hi = exprs.add(E::Int(hi));
                let above = exprs.add(E::Bin(Op::Le, lo, inner.value));
                let below = exprs.add(E::Bin(Op::Le, inner.value, hi));
                inner.defined.push(exprs.add(E::Bin(Op::And, above, below)));
                inner
            }
        };
        self.lows.push(low);
        Ok(())
    }

    /// The one operand's value inside `node`, with its definedness.
    fn wrap(&mut self, node: impl FnOnce(ExprId) -> E) -> Lower<Low> {
        let inner = self.operand()?;
        Ok(Low {
            value: self.lowerer.exprs.add(node(inner.value)),
            defined: inner.defined,
        })
    }
}

impl Lowerer<'_> {
    fn object_of(&self, id: &str) -> Option<usize> {
        self.objects.iter().position(|object| object.id == id)
    }

    /// Lower the expression `visit` and everything it reaches.
    fn lower_visit(&mut self, visit: Visit) -> Lower<Low> {
        let mut lowering = Lowering {
            lowerer: self,
            lows: Vec::new(),
        };
        if let ControlFlow::Break(refusal) = walk(&mut lowering, visit) {
            return Err(refusal);
        }
        lowering.operand()
    }

    fn term(&mut self, term: &Value, primed: bool) -> Lower<Low> {
        self.lower_visit(Visit::Term {
            term: term.clone(),
            primed,
        })
    }

    fn node(&mut self, position: usize, primed: bool) -> Lower<Low> {
        self.lower_visit(Visit::Node { position, primed })
    }

    fn literal(&self, term: &Value) -> Lower<E> {
        let value = &term["value"];
        let value = match str_of(term, "value_kind") {
            Some("integer") => value.as_str().map(|text| E::Int(text.to_owned())),
            Some("float64_bits") => value.as_str().map(|bits| E::Float64Bits(bits.to_owned())),
            Some("boolean") => value
                .as_bool()
                .or_else(|| value.as_str().map(|text| text == "true"))
                .map(E::Bool),
            Some("enum") => {
                let owner = self
                    .graph
                    .position_of(&term["type"])
                    .and_then(|position| self.graph.node(position).declaration.as_ref())
                    .and_then(|declaration| declaration.qualified_name.last())
                    .map(|name| name.to_string());
                value.as_str().zip(owner).map(|(case, owner)| E::Enum {
                    owner,
                    case: case.to_owned(),
                })
            }
            _ => None,
        };
        value.map_or_else(
            || {
                refuse(
                    "value/literal",
                    "a literal that is not supported by the state mapper",
                )
            },
            Ok,
        )
    }

    /// Whether the parameter node at `position` is typed by an object type.
    fn is_object_typed(&self, position: usize) -> bool {
        let node = self.graph.node(position);
        let Some(ty) = self.graph.position(&node.semantic_type) else {
            return false;
        };
        let Ok(body) = serde_json::to_value(&self.graph.node(ty).body) else {
            return false;
        };
        let path = &body["members"][0]["target"]["path"];
        path[0] == "model" && path[1] == "object_type"
    }

    /// The field a projection reads, checked before its key is lowered.
    fn project(&self, construct: &str, body: &Value, primed: bool) -> Lower<Combine> {
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
        Ok(Combine::Field {
            object,
            field,
            primed,
        })
    }

    /// The bounds of a conversion, checked before its operand is lowered.
    fn narrow(&self, construct: &str, body: &Value) -> Lower<Combine> {
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
        Ok(Combine::Narrow { lo, hi })
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
        .ok_or_else(|| fail("the `over` binder has no type"))?;
    let object_type = target["path"][0] == "model" && target["path"][1] == "object_type";
    let id = target["path"].get(2).and_then(Value::as_str);
    let (true, Some(id)) = (object_type, id) else {
        return defer_at(
            position,
            "temporal/subject",
            "a clause over a subject that is no object type, such as a protocol subject",
        );
    };
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
            return defer_at(
                node,
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
                .ok_or_else(|| refusal("temporal/fairness", "a fairness names no declaration"))?
                .to_owned(),
            operation: str_of(target, "name")
                .ok_or_else(|| refusal("temporal/fairness", "a fairness names no operation"))?
                .to_owned(),
            node,
        });
    }
    Ok(reads)
}

// ---- Rendering -----------------------------------------------------------

/// One operation's lowered clauses.
struct OpLow {
    pre: Vec<ExprId>,
    post: Vec<ExprId>,
    node: usize,
}

/// What the renderers read.
struct Input<'a> {
    objects: &'a [Object],
    operations: &'a [Vec<OpLow>],
    snapshot: Option<&'a Value>,
    show: &'a Show<'a>,
    first: usize,
    clauses: &'a [(String, usize, usize, ExprId)],
    temporals: &'a [ClauseLow],
    dense: Option<&'a dense::Model>,
}

struct Show<'a> {
    target: Target,
    objects: &'a [Object],
    exprs: &'a Exprs,
}

fn int_text(value: &str) -> String {
    if value.starts_with('-') {
        format!("({value})")
    } else {
        value.to_owned()
    }
}

fn float64_nan_bits_are_canonical(bits: &str) -> bool {
    if bits.len() != 16
        || !bits
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return false;
    }
    let Some(value) = u64::from_str_radix(bits, 16).ok() else {
        return false;
    };
    let exponent = (value >> 52) & 0x7ff;
    let fraction = value & ((1_u64 << 52) - 1);
    exponent == 0x7ff && fraction != 0
}

/// FR-812's lossless binary64 representation. Finite values retain their
/// sign, integer significand and power-of-two exponent; NaNs retain all bits.
fn float64_value(bits: &str, target: Target) -> String {
    let Some(value) = u64::from_str_radix(bits, 16).ok() else {
        return "[tag |-> \"nan\", bits |-> \"7ff8000000000000\"]".to_owned();
    };
    let (sign, lean_sign) = if value >> 63 == 0 {
        ("positive", "false")
    } else {
        ("negative", "true")
    };
    let exponent = (value >> 52) & 0x7ff;
    let fraction = value & ((1_u64 << 52) - 1);
    if exponent == 0x7ff && fraction != 0 {
        if !float64_nan_bits_are_canonical(bits) {
            return match target {
                Target::Tla => "[tag |-> \"nan\", bits |-> \"7ff8000000000000\"]".to_owned(),
                Target::Lean => "F.nan \"7ff8000000000000\"".to_owned(),
            };
        }
        return match target {
            Target::Tla => format!("[tag |-> \"nan\", bits |-> \"{bits}\"]"),
            Target::Lean => format!("F.nan \"{bits}\""),
        };
    }
    if exponent == 0x7ff {
        return match target {
            Target::Tla => format!("[tag |-> \"inf\", sign |-> \"{sign}\"]"),
            Target::Lean => format!("F.inf {lean_sign}"),
        };
    }
    let (significand, power) = if exponent == 0 {
        (fraction, -1074_i32)
    } else {
        ((1_u64 << 52) | fraction, exponent as i32 - 1075)
    };
    match target {
        Target::Tla => format!(
            "[tag |-> \"finite\", sign |-> \"{sign}\", significand |-> {significand}, exponent |-> {power}]"
        ),
        Target::Lean => format!("F.finite {lean_sign} {significand} ({power})"),
    }
}

impl Show<'_> {
    fn exprs(&self, items: &[ExprId]) -> Lower<Vec<String>> {
        items.iter().map(|&item| self.expr(item)).collect()
    }

    fn expr(&self, e: ExprId) -> Lower<String> {
        render(e, |e, out| self.pieces(e, out))
    }

    /// The pieces of one expression node, its operands in written order.
    fn pieces(&self, e: ExprId, out: &mut Vec<Piece<ExprId>>) -> Lower<()> {
        let tla = self.target == Target::Tla;
        match self.exprs.get(e)? {
            E::Int(value) => out.push(Piece::text(int_text(value))),
            E::Float64Bits(bits) => out.push(Piece::text(float64_value(bits, self.target))),
            E::Bool(true) => out.push(Piece::text(if tla { "TRUE" } else { "True" })),
            E::Bool(false) => out.push(Piece::text(if tla { "FALSE" } else { "False" })),
            E::Enum { owner, case } => {
                let member = enum_member_name(owner, case);
                out.push(Piece::text(if tla {
                    format!("\"{member}\"")
                } else {
                    format!("{}.{}", enum_type_name(owner), member)
                }));
            }
            E::Param(name) => out.push(Piece::text(name.clone())),
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
                let name = object.field(field);
                if tla {
                    let tick = if *primed { "'" } else { "" };
                    out.extend([
                        Piece::text(format!("{}{tick}[", object.variable())),
                        Piece::Node(*key),
                        Piece::text(format!("].{name}")),
                    ]);
                } else {
                    let state = if *primed { "s'" } else { "s" };
                    let (open, close) = match field.kind {
                        Kind::Bool => ("(", " = true)"),
                        Kind::Int { .. } | Kind::Float64 | Kind::Enum { .. } => ("", ""),
                    };
                    out.extend([
                        Piece::text(format!("{open}({} ", mangle("F_get_", &object.name))),
                        Piece::Node(*key),
                        Piece::text(format!(" {state}.{}).{name}{close}", object.variable())),
                    ]);
                }
            }
            E::Neg(inner) => out.extend([Piece::text("(-"), Piece::Node(*inner), Piece::text(")")]),
            E::FloatNeg(inner) => out.extend([
                Piece::text("F_float64_neg("),
                Piece::Node(*inner),
                Piece::text(")"),
            ]),
            E::Not(inner) => out.extend([
                Piece::text(if tla { "~" } else { "Not " }),
                Piece::Node(*inner),
            ]),
            E::Bin(op, left, right) => {
                let (open, between) = match (op, tla) {
                    (Op::EqFloat, _) => ("F_float64_eq(", ", "),
                    (Op::NeBool, true) => ("~(", " <=> "),
                    (Op::NeInt, false) => ("(Ne ", " "),
                    (Op::NeEnum, false) => ("(Ne ", " "),
                    (Op::NeBool, false) => ("Not (", " <-> "),
                    (Op::Add, _) => ("(", " + "),
                    (Op::Sub, _) => ("(", " - "),
                    (Op::Mul, _) => ("(", " * "),
                    (Op::Lt, _) => ("(", " < "),
                    (Op::Le, _) => ("(", " <= "),
                    (Op::Gt, _) => ("(", " > "),
                    (Op::Ge, _) => ("(", " >= "),
                    (Op::EqInt, _) => ("(", " = "),
                    (Op::EqEnum, _) => ("(", " = "),
                    (Op::And, _) => ("(", " /\\ "),
                    (Op::Or, _) => ("(", " \\/ "),
                    (Op::NeInt, true) => ("(", " /= "),
                    (Op::NeEnum, true) => ("(", " /= "),
                    (Op::Implies, true) => ("(", " => "),
                    (Op::Implies, false) => ("(", " -> "),
                    (Op::EqBool, true) => ("(", " <=> "),
                    (Op::EqBool, false) => ("(", " <-> "),
                };
                out.extend([
                    Piece::text(open),
                    Piece::Node(*left),
                    Piece::text(between),
                    Piece::Node(*right),
                    Piece::text(")"),
                ]);
            }
            E::Conj(items) => {
                out.push(Piece::text("("));
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(Piece::text(" /\\ "));
                    }
                    out.push(Piece::Node(*item));
                }
                out.push(Piece::text(")"));
            }
        }
        Ok(())
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

struct NameRecord {
    name: String,
    node: usize,
    binding: Option<String>,
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
        Kind::Enum { name, cases, .. } => {
            let members = cases
                .iter()
                .map(|case| format!("\"{}\"", enum_member_name(name, case)))
                .collect::<Vec<_>>();
            if tla {
                vec![format!("{path} \\in {{{}}}", members.join(", "))]
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
        Kind::Float64 => match target {
            Target::Tla => vec![format!("{path} \\in F_float64_values")],
            Target::Lean => vec![format!("F_float64_values {path}")],
        },
    }
}

/// The parameter's domain in TLA+: an explicit range, `Int` or `BOOLEAN`.
fn tla_domain(kind: &Kind) -> String {
    match kind {
        Kind::Bool => "BOOLEAN".to_owned(),
        Kind::Enum { name, cases, .. } => format!(
            "{{{}}}",
            cases
                .iter()
                .map(|case| format!("\"{}\"", enum_member_name(name, case)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Kind::Int {
            lo: Some(lo),
            hi: Some(hi),
        } => format!("{}..{}", int_text(lo), int_text(hi)),
        Kind::Int { .. } => "Int".to_owned(),
        Kind::Float64 => "F_float64_values".to_owned(),
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
    let timed_models = fsl_package::CheckedTimedModel::from_admitted(graph.package)
        .map_err(|message| refusal("checked package", message))?;
    for (name, position) in wanted {
        if &*graph.node(*position).node_tag == "temporal" {
            let read = temporal::read(graph, name, *position, &timed_models, target).map_err(
                |mut refusal| {
                    refusal.obligation = Some(name.clone());
                    refusal.node.get_or_insert(*position);
                    refusal
                },
            )?;
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
        let ObjectParts {
            fields,
            clocks,
            operations,
        } = object.body.map_err(attribute)?;
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
            clocks,
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
        exprs: Exprs::default(),
    };
    let dense = dense::Model::read(&mut lowerer, &reads, &timed_models, wanted)?;
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
                    "precondition" => lowered.pre.push(low.clause(&mut lowerer.exprs)),
                    "postcondition" => lowered.post.push(low.clause(&mut lowerer.exprs)),
                    other => {
                        return Err(attribute(refusal(
                            "state/state_clause",
                            &format!("a `{other}` operation clause"),
                        )));
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
        let formula = low.clause(&mut lowerer.exprs);
        lowered_clauses.push((name.clone(), *position, object, formula));
    }
    let mut temporals = Vec::new();
    for read in reads {
        let attribute = |mut refusal: Refusal| {
            refusal.obligation = Some(read.name.clone());
            refusal.node.get_or_insert(read.position);
            refusal
        };
        let object = objects
            .iter()
            .position(|object| object.id == read.context)
            .ok_or_else(|| refusal("model/object_type", "an unexported clause context"))
            .map_err(attribute)?;
        let (mut cur, mut next) = (Vec::new(), Vec::new());
        // The binder as the lowerer names it, then as a clause binder (`o_`).
        let over = lowerer.node(read.over, false).map_err(attribute)?.value;
        let lowered_binder = match lowerer.exprs.get(over).map_err(attribute)? {
            E::Param(name) => name.clone(),
            _ => {
                return Err(attribute(refusal(
                    "temporal/temporal_clause",
                    "the `over` binder does not lower to a parameter",
                )));
            }
        };
        let binder = format!("o_{}", read.binder.trim_start_matches("p_"));
        let mut defs: Vec<ExprId> = Vec::new();
        for &leaf in &read.atoms {
            for (primed, atoms) in [(false, &mut cur), (true, &mut next)] {
                let low = lowerer.node(leaf, primed).map_err(attribute)?;
                if !primed {
                    for def in low.defined {
                        let def = lowerer.exprs.rename_param(def, &lowered_binder, &binder);
                        if !defs.iter().any(|seen| seen == &def) {
                            defs.push(def);
                        }
                    }
                }
                atoms.push(
                    lowerer
                        .exprs
                        .rename_param(low.value, &lowered_binder, &binder),
                );
            }
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
            let (fair_object, operation) = match found {
                Some((index, operation)) if index == object => (index, operation),
                _ => {
                    return defer_at(
                        fair.node,
                        "temporal/fairness-target",
                        "a fairness over an operation of another object type than the clause's subject",
                    )
                    .map_err(attribute);
                }
            };
            fairness.push(FairLow {
                strong: fair.strong,
                each: fair.each,
                object: fair_object,
                operation,
            });
        }
        temporals.push(ClauseLow {
            name: read.name,
            position: read.position,
            object,
            binder: binder.clone(),
            formulas: read.formulas,
            tf: read.tf,
            formula_sites: read.formula_sites,
            refuse_at: read.refuse_at,
            defs,
            cur,
            next,
            fairness,
        });
    }

    let exprs = lowerer.exprs;
    let show = Show {
        target,
        objects: &objects,
        exprs: &exprs,
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
        dense: dense.as_ref(),
    };
    let extra = match target {
        Target::Tla => tla(&input, &mut definitions)?,
        Target::Lean => {
            lean(&input, &mut definitions)?;
            let mut extra = Extra::default();
            if let Some(model) = &dense {
                extra.aux_names.extend(model.clock_names());
            }
            extra
        }
    };
    assemble(graph, target, module, &objects, &extra, definitions)
}

// ---- Snapshot ------------------------------------------------------------

enum Lit {
    Int(String),
    Bool(bool),
    Float64Bits(String),
    Enum { owner: String, case: String },
}

impl Lit {
    fn text(&self, target: Target) -> String {
        match (self, target) {
            (Self::Int(value), _) => int_text(value),
            (Self::Bool(true), Target::Tla) => "TRUE".to_owned(),
            (Self::Bool(false), Target::Tla) => "FALSE".to_owned(),
            (Self::Bool(value), Target::Lean) => value.to_string(),
            (Self::Float64Bits(bits), target) => float64_value(bits, target),
            (Self::Enum { owner, case }, Target::Tla) => {
                format!("\"{}\"", enum_member_name(owner, case))
            }
            (Self::Enum { owner, case }, Target::Lean) => {
                format!(
                    "{}.{}",
                    enum_type_name(owner),
                    enum_member_name(owner, case)
                )
            }
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
                    Kind::Float64 => value["bits"]
                        .as_str()
                        .filter(|_| value["type"] == "float64")
                        .map(|bits| Lit::Float64Bits(bits.to_owned())),
                    Kind::Enum { name, .. } => {
                        value["variant"]["case"].as_str().map(|case| Lit::Enum {
                            owner: name.clone(),
                            case: case.to_owned(),
                        })
                    }
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
            let name = object.field(field);
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
    std::iter::once(("r_self".to_owned(), None))
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

fn tla(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<Extra> {
    let target = Target::Tla;
    let objects = input.objects;
    let has_float64 = objects.iter().any(|object| {
        object
            .fields
            .iter()
            .any(|field| matches!(field.kind, Kind::Float64))
            || object.operations.iter().any(|operation| {
                operation
                    .params
                    .iter()
                    .any(|param| matches!(param.kind, Kind::Float64))
            })
    }) || input
        .operations
        .iter()
        .flatten()
        .flat_map(|operation| operation.pre.iter().chain(&operation.post))
        .any(|&e| input.show.exprs.uses_float64(e))
        || input
            .clauses
            .iter()
            .any(|&(_, _, _, clause)| input.show.exprs.uses_float64(clause))
        || input.temporals.iter().any(|clause| {
            clause
                .defs
                .iter()
                .chain(&clause.cur)
                .chain(&clause.next)
                .any(|&e| input.show.exprs.uses_float64(e))
        });
    if has_float64 {
        out.push(define(
            input.first,
            Some("F_float64_hex_digits".to_owned()),
            "F_float64_hex_digits == {\"0\", \"1\", \"2\", \"3\", \"4\", \"5\", \"6\", \"7\", \"8\", \"9\", \"a\", \"b\", \"c\", \"d\", \"e\", \"f\"}".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_nan_bits".to_owned()),
            "F_float64_nan_bits(bits) ==\n  Len(bits) = 16\n  /\\ \\A i \\in 1..16 : SubSeq(bits, i, i) \\in F_float64_hex_digits\n  /\\ SubSeq(bits, 1, 1) \\in {\"7\", \"f\"}\n  /\\ SubSeq(bits, 2, 3) = \"ff\"\n  /\\ \\E i \\in 4..16 : SubSeq(bits, i, i) # \"0\"".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_values".to_owned()),
            "F_float64_values ==\n  [tag : {\"finite\"}, sign : {\"positive\", \"negative\"}, significand : {0}, exponent : {-1074}]\n  \\cup [tag : {\"finite\"}, sign : {\"positive\", \"negative\"}, significand : 1..4503599627370495, exponent : {-1074}]\n  \\cup [tag : {\"finite\"}, sign : {\"positive\", \"negative\"}, significand : 4503599627370496..9007199254740991, exponent : -1074..971]\n  \\cup [tag : {\"inf\"}, sign : {\"positive\", \"negative\"}]\n  \\cup { [tag |-> \"nan\", bits |-> bits] : bits \\in STRING /\\ F_float64_nan_bits(bits) }".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_eq".to_owned()),
            "F_float64_eq(a, b) ==\n  IF a.tag = \"nan\" \\/ b.tag = \"nan\" THEN FALSE\n  ELSE IF a.tag = \"finite\" /\\ b.tag = \"finite\" /\\ a.significand = 0 /\\ b.significand = 0 THEN TRUE\n  ELSE a = b".to_owned(),
            None,
        ));
        let sign_cases = "0123456789abcdef"
            .chars()
            .enumerate()
            .map(|(index, nibble)| format!("n = \"{nibble}\" -> \"{:x}\"", index ^ 8))
            .collect::<Vec<_>>()
            .join(" [] ");
        out.push(define(
            input.first,
            Some("F_float64_flip_sign_nibble".to_owned()),
            format!("F_float64_flip_sign_nibble(n) == CASE {sign_cases}"),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_flip_nan_sign".to_owned()),
            "F_float64_flip_nan_sign(bits) ==\n  F_float64_flip_sign_nibble(SubSeq(bits, 1, 1)) \\o SubSeq(bits, 2, Len(bits))".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_neg".to_owned()),
            "F_float64_neg(x) ==\n  IF x.tag = \"nan\"\n  THEN [x EXCEPT !.bits = F_float64_flip_nan_sign(@)]\n  ELSE [x EXCEPT !.sign = IF @ = \"positive\" THEN \"negative\" ELSE \"positive\"]".to_owned(),
            None,
        ));
    }
    let variables: Vec<String> = objects
        .iter()
        .map(Object::variable)
        .chain(input.dense.into_iter().flat_map(dense::Model::variables))
        .collect();
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
        if input.snapshot.is_none()
            && input
                .temporals
                .iter()
                .any(|clause| objects[clause.object].node == object.node)
        {
            type_ok.push(format!(
                "DOMAIN {v} \\subseteq {}",
                mangle("Keys_", &object.name)
            ));
        }
        let fields: Vec<String> = object
            .fields
            .iter()
            .flat_map(|field| {
                range_conditions(
                    &format!("{v}[k].{}", object.field(field)),
                    &field.kind,
                    target,
                )
            })
            .collect();
        if !fields.is_empty() {
            type_ok.push(format!("\\A k \\in DOMAIN {v} : {}", fields.join(" /\\ ")));
        }
    }
    if let Some(model) = input.dense {
        type_ok.extend(model.tla_types(&objects[0].variable()));
    }
    out.push(define(
        input.first,
        None,
        format!("TypeOK ==\n{}", bullets(&type_ok)),
        None,
    ));
    if let Some(model) = input.dense {
        model.tla_delay_bounds(input, out)?;
    }
    let init = match input.snapshot {
        None if input.dense.is_none() => "Init == TypeOK".to_owned(),
        None => {
            let mut parts = input
                .dense
                .map(|model| model.tla_init(&objects[0].variable()))
                .unwrap_or_default();
            parts.push("TypeOK".to_owned());
            parts.push("TimeInv(0)".to_owned());
            format!("Init ==\n{}", bullets(&parts))
        }
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
            if let Some(model) = input.dense {
                parts.extend(model.tla_init(&objects[0].variable()));
                parts.push("TimeInv(0)".to_owned());
            }
            format!("Init ==\n{}", bullets(&parts))
        }
    };
    out.push(define(input.first, None, init, None));
    let mut disjuncts = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let v = object.variable();
        for (at, (operation, lowered)) in object
            .operations
            .iter()
            .zip(&input.operations[index])
            .enumerate()
        {
            let name = object.operation(operation);
            let params = parameters(operation);
            let names: Vec<&str> = params.iter().map(|(name, _)| name.as_str()).collect();
            let mut lines = vec![format!("r_self \\in DOMAIN {v}")];
            for (param, kind) in params.iter().skip(1) {
                if let Some(kind) = kind {
                    lines.extend(range_conditions(param, kind, target));
                }
            }
            lines.extend(input.show.exprs(&lowered.pre)?);
            if let Some(model) = input.dense {
                lines.extend(model.tla_action(input, at)?);
            }
            lines.extend(input.show.exprs(&lowered.post)?);
            lines.push(format!("DOMAIN {v}' = DOMAIN {v}"));
            for (field, receiver) in frame_fields(object, operation) {
                let f = object.field(field);
                let guard = if receiver { "k /= r_self => " } else { "" };
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
    let next_name = if let Some(model) = input.dense {
        model.tla_steps(input, out)?;
        "TNext"
    } else {
        "Next"
    };
    out.push(define(
        input.first,
        None,
        format!("Spec == Init /\\ [][{next_name}]_vars /\\ WF_vars({next_name})"),
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
                "{clause} == \\A r_self \\in DOMAIN {} : {}",
                object.variable(),
                input.show.expr(*formula)?
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
    let mut extra = temporal::tla(input, out)?;
    if let Some(model) = input.dense {
        extra.dense = true;
        extra.variables.splice(0..0, model.variables());
        extra.aux_names.extend(model.clock_names());
    }
    Ok(extra)
}

// ---- Lean 4 --------------------------------------------------------------

fn lean(input: &Input<'_>, out: &mut Vec<Definition>) -> Lower<()> {
    let target = Target::Lean;
    let objects = input.objects;
    let has_float64 = objects.iter().any(|object| {
        object
            .fields
            .iter()
            .any(|field| matches!(field.kind, Kind::Float64))
            || object.operations.iter().any(|operation| {
                operation
                    .params
                    .iter()
                    .any(|param| matches!(param.kind, Kind::Float64))
            })
    }) || input
        .operations
        .iter()
        .flatten()
        .flat_map(|operation| operation.pre.iter().chain(&operation.post))
        .any(|&e| input.show.exprs.uses_float64(e))
        || input
            .clauses
            .iter()
            .any(|&(_, _, _, clause)| input.show.exprs.uses_float64(clause))
        || input.temporals.iter().any(|clause| {
            clause
                .defs
                .iter()
                .chain(&clause.cur)
                .chain(&clause.next)
                .any(|&e| input.show.exprs.uses_float64(e))
        });
    if has_float64 {
        out.push(define(
            input.first,
            Some("F".to_owned()),
            "inductive F where\n  | finite (negative : Bool) (significand : Nat) (exponent : Int)\n  | inf (negative : Bool)\n  | nan (bits : String)\n  deriving BEq, DecidableEq".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_hex_lower".to_owned()),
            "def F_float64_hex_lower (c : Char) : Prop := c ∈ \"0123456789abcdef\".toList"
                .to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_nan_bits".to_owned()),
            "def F_float64_nan_bits (bits : String) : Prop :=\n  match bits.toList with\n  | [sign, e1, e2, e3, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12] =>\n      F_float64_hex_lower sign /\\ sign ∈ \"7f\".toList /\\ e1 = 'f' /\\ e2 = 'f' /\\ F_float64_hex_lower e3 /\\\n      F_float64_hex_lower f1 /\\ F_float64_hex_lower f2 /\\ F_float64_hex_lower f3 /\\ F_float64_hex_lower f4 /\\\n      F_float64_hex_lower f5 /\\ F_float64_hex_lower f6 /\\ F_float64_hex_lower f7 /\\ F_float64_hex_lower f8 /\\\n      F_float64_hex_lower f9 /\\ F_float64_hex_lower f10 /\\ F_float64_hex_lower f11 /\\ F_float64_hex_lower f12 /\\\n      (e3 ≠ '0' \\/ f1 ≠ '0' \\/ f2 ≠ '0' \\/ f3 ≠ '0' \\/ f4 ≠ '0' \\/ f5 ≠ '0' \\/ f6 ≠ '0' \\/ f7 ≠ '0' \\/ f8 ≠ '0' \\/ f9 ≠ '0' \\/ f10 ≠ '0' \\/ f11 ≠ '0' \\/ f12 ≠ '0')\n  | _ => False".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_flip_sign_char".to_owned()),
            "def F_float64_flip_sign_char (sign : Char) : Char :=\n  match sign with\n  | '0' => '8' | '1' => '9' | '2' => 'a' | '3' => 'b'\n  | '4' => 'c' | '5' => 'd' | '6' => 'e' | '7' => 'f'\n  | '8' => '0' | '9' => '1' | 'a' => '2' | 'b' => '3'\n  | 'c' => '4' | 'd' => '5' | 'e' => '6' | 'f' => '7'\n  | other => other".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_neg_nan".to_owned()),
            "def F_float64_neg_nan (bits : String) : String :=\n  match bits.toList with\n  | sign :: rest => String.ofList (F_float64_flip_sign_char sign :: rest)\n  | [] => bits".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_values".to_owned()),
            "def F_float64_values (x : F) : Prop :=\n  match x with\n  | .finite _ significand exponent =>\n      (significand = 0 /\\ exponent = -1074) \\/\n      (1 <= significand /\\ significand < 4503599627370496 /\\ exponent = -1074) \\/\n      (4503599627370496 <= significand /\\ significand <= 9007199254740991 /\\ -1074 <= exponent /\\ exponent <= 971)\n  | .inf _ => True\n  | .nan bits => F_float64_nan_bits bits".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_eq".to_owned()),
            "def F_float64_eq (a b : F) : Prop :=\n  match a, b with\n  | .nan _, _ | _, .nan _ => False\n  | .finite signA sigA expA, .finite signB sigB expB =>\n      if sigA = 0 /\\ sigB = 0 then True else signA = signB /\\ sigA = sigB /\\ expA = expB\n  | .inf signA, .inf signB => signA = signB\n  | _, _ => False".to_owned(),
            None,
        ));
        out.push(define(
            input.first,
            Some("F_float64_neg".to_owned()),
            "def F_float64_neg (x : F) : F :=\n  match x with\n  | .finite negative significand exponent => .finite (!negative) significand exponent\n  | .inf negative => .inf (!negative)\n  | .nan bits => .nan (F_float64_neg_nan bits)".to_owned(),
            None,
        ));
    }
    let mut enums = BTreeMap::new();
    for kind in objects
        .iter()
        .flat_map(|object| object.fields.iter().map(|field| &field.kind))
        .chain(
            objects
                .iter()
                .flat_map(|object| object.operations.iter())
                .flat_map(|operation| operation.params.iter().map(|param| &param.kind)),
        )
    {
        if let Kind::Enum { id, name, cases } = kind {
            enums
                .entry(id.clone())
                .or_insert_with(|| (name.clone(), cases.clone()));
        }
    }
    for (name, cases) in enums.values() {
        let constructors = cases
            .iter()
            .map(|case| format!("  | {}", enum_member_name(name, case)))
            .collect::<Vec<_>>();
        out.push(define(
            input.first,
            Some(enum_type_name(name)),
            format!(
                "inductive {} where\n{}\nderiving DecidableEq",
                enum_type_name(name),
                constructors.join("\n")
            ),
            None,
        ));
    }
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
                let ty = match &field.kind {
                    Kind::Int { .. } => "Int".to_owned(),
                    Kind::Bool => "Bool".to_owned(),
                    Kind::Float64 => "F".to_owned(),
                    Kind::Enum { name, .. } => enum_type_name(name),
                };
                format!("  {} : {ty}", object.field(field))
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
                let value = match &field.kind {
                    Kind::Int { .. } => "0".to_owned(),
                    Kind::Bool => "false".to_owned(),
                    Kind::Float64 => "F.finite false 0 (-1074)".to_owned(),
                    Kind::Enum { name, cases, .. } => cases.first().map_or_else(
                        || "".to_owned(),
                        |case| format!("{}.{}", enum_type_name(name), enum_member_name(name, case)),
                    ),
                };
                format!("{} := {value}", object.field(field))
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
                range_conditions(&format!("e.2.{}", object.field(field)), &field.kind, target)
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
    let (clock_types, clock_args) = if let Some(model) = input.dense {
        model.lean_clock_types(input, out);
        (" (c c' : Clocks)", " c c'")
    } else {
        ("", "")
    };
    let mut steps = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let v = object.variable();
        let has = mangle("F_has_", &object.name);
        let get = mangle("F_get_", &object.name);
        for (at, (operation, lowered)) in object
            .operations
            .iter()
            .zip(&input.operations[index])
            .enumerate()
        {
            let name = object.operation(operation);
            let params = parameters(operation);
            let typed: Vec<String> = params
                .iter()
                .map(|(param, kind)| {
                    let ty = match kind {
                        None => "List Nat".to_owned(),
                        Some(Kind::Bool) => "Bool".to_owned(),
                        Some(Kind::Int { .. }) => "Int".to_owned(),
                        Some(Kind::Float64) => "F".to_owned(),
                        Some(Kind::Enum { name, .. }) => enum_type_name(name),
                    };
                    format!("({param} : {ty})")
                })
                .collect();
            let mut lines = vec![format!("{has} r_self s.{v}")];
            for (param, kind) in params.iter().skip(1) {
                if let Some(kind) = kind {
                    lines.extend(range_conditions(param, kind, target));
                }
            }
            lines.extend(input.show.exprs(&lowered.pre)?);
            if let Some(model) = input.dense {
                lines.extend(model.lean_action(input, at)?);
            }
            lines.extend(input.show.exprs(&lowered.post)?);
            lines.push(format!(
                "List.map (fun e => e.1) s'.{v} = List.map (fun e => e.1) s.{v}"
            ));
            for (field, receiver) in frame_fields(object, operation) {
                let f = object.field(field);
                let guard = if receiver { "Not (k = r_self) -> " } else { "" };
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
                    "def {name} (s : State) (s' : State){clock_types} {} : Prop :=\n  {}",
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
                "({} {name} s s'{clock_args} {})",
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
        format!("def Step (s : State) (s' : State){clock_types} : Prop :=\n  {step}"),
        None,
    ));
    if let Some(model) = input.dense {
        model.lean_steps(input, out)?;
    } else {
        out.push(define(
        input.first,
        None,
        "structure Behaviour where\n  st : Nat -> State\n  init : Init (st 0)\n  next : forall n, Step (st n) (st (n + 1)) \\/ (Not (exists t, Step (st n) t) /\\ st (n + 1) = st n)"
            .to_owned(),
        None,
        ));
    }
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
                "def {clause} (s : State) : Prop :=\n  forall r_self, {} r_self s.{} -> {}",
                mangle("F_has_", &object.name),
                object.variable(),
                input.show.expr(*formula)?
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
    extra: &Extra,
    definitions: Vec<Definition>,
) -> Lower<Lowered> {
    let state_clauses = state_clauses(graph)?;
    let first = objects.first().map_or(0, |object| object.node);
    let mut text = match target {
        Target::Tla => {
            let variables: Vec<String> = objects
                .iter()
                .map(Object::variable)
                .chain(extra.variables.iter().cloned())
                .collect();
            format!(
                "---- MODULE {module} ----\nEXTENDS Naturals, Integers, Sequences, FiniteSets, TLC{}\n\n{}\n{}VARIABLES {}\n",
                if extra.dense { ", Reals" } else { "" },
                comment(graph, first, target)?,
                if extra.constants.is_empty() {
                    String::new()
                } else {
                    let assumed: Vec<String> = extra
                        .constants
                        .iter()
                        .map(|keys| {
                            format!("ASSUME {keys} \\subseteq Seq(Nat) /\\ IsFiniteSet({keys})\n")
                        })
                        .collect();
                    format!(
                        "CONSTANTS {}\n{}",
                        extra.constants.join(", "),
                        assumed.concat()
                    )
                },
                variables.join(", ")
            )
        }
        Target::Lean => format!("namespace {module}\n"),
    };
    let mut name_records = Vec::new();
    name_records.extend(extra.aux_names.iter().map(|(name, node)| NameRecord {
        name: name.clone(),
        node: *node,
        binding: None,
    }));
    for object in objects {
        name_records.push(NameRecord {
            name: object.variable(),
            node: object.node,
            binding: None,
        });
        for field in &object.fields {
            name_records.push(NameRecord {
                name: object.field(field),
                node: object.node,
                binding: Some(node_id_text(graph, object.node)?),
            });
        }
        for operation in &object.operations {
            let position = operation_anchor(graph, object, operation).unwrap_or(object.node);
            name_records.push(NameRecord {
                name: object.operation(operation),
                node: position,
                binding: None,
            });
            for parameter in &operation.params {
                let node =
                    operation_parameter_node(graph, &state_clauses, position, &parameter.name)
                        .unwrap_or(position);
                name_records.push(NameRecord {
                    name: mangle("p_", &parameter.name),
                    node,
                    binding: Some(node_id_text(graph, position)?),
                });
            }
        }
    }
    for object in objects {
        let owner = node_id_text(graph, object.node)?;
        for kind in object.fields.iter().map(|field| &field.kind).chain(
            object
                .operations
                .iter()
                .flat_map(|operation| operation.params.iter().map(|param| &param.kind)),
        ) {
            if let Kind::Enum { name, cases, .. } = kind {
                for case in cases {
                    name_records.push(NameRecord {
                        name: enum_member_name(name, case),
                        node: object.node,
                        binding: Some(owner.clone()),
                    });
                }
            }
        }
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
            name_records.push(NameRecord {
                name,
                node: definition.node,
                binding: None,
            });
        }
    }
    match target {
        Target::Tla => text.push_str("\n====\n"),
        Target::Lean => text.push_str(&format!("\nend {module}\n")),
    }
    name_records.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.binding.cmp(&right.binding))
    });
    name_records.dedup_by(|right, left| right.name == left.name && right.binding == left.binding);
    let mut name_map = Vec::with_capacity(name_records.len());
    for record in name_records {
        let region = source_region(graph, record.node)
            .map_or(Value::Null, |(_, start, end)| json!([start, end]));
        let mut entry = json!({
            "name": record.name,
            "node": node_id_text(graph, record.node)?,
            "region": region,
        });
        if let Some(binding) = record.binding {
            entry["binding"] = json!(binding);
        }
        name_map.push(entry);
    }
    let mut conditions: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for cond in &extra.conditional {
        let node = node_id_text(graph, cond.node)?;
        let value = match &cond.type_name {
            Some(type_name) => json!({"type": cond.kind, "type_name": type_name, "node": node}),
            None => json!({"type": cond.kind, "clause": cond.obligation, "node": node}),
        };
        conditions
            .entry(cond.obligation.clone())
            .or_default()
            .push(value);
    }
    Ok(Lowered {
        text,
        name_map,
        regions,
        conditions,
        definitions: count,
    })
}

fn operation_anchor(graph: &Graph<'_>, object: &Object, operation: &Operation) -> Option<usize> {
    graph
        .package
        .resolved_graph()
        .nodes
        .iter()
        .enumerate()
        .find_map(|(position, node)| {
            if &*node.node_tag != "state" || &*node.semantic_form != "operation_anchor" {
                return None;
            }
            let body = serde_json::to_value(&node.body).ok()?;
            let members = items(&body, "members");
            let context = members
                .iter()
                .find(|member| str_of(member, "name") == Some("context"))?;
            let operation_member = members
                .iter()
                .find(|member| str_of(member, "name") == Some("operation"))?;
            let context_path = context["value"]["target"]["path"].as_array()?;
            let context_identity = context_path.get(2)?.as_str()?;
            let operation_name = operation_member["value"]["value"].as_str()?;
            (context_identity == object.id
                && (operation_name == operation.name || operation_name == operation.id))
                .then_some(position)
        })
}

fn operation_parameter_node(
    graph: &Graph<'_>,
    clauses: &[StateClause],
    anchor: usize,
    parameter: &str,
) -> Option<usize> {
    clauses
        .iter()
        .filter(|clause| clause.anchor == Some(anchor))
        .flat_map(|clause| graph.closure(clause.position))
        .find(|&position| {
            let node = graph.node(position);
            &*node.node_tag == "value"
                && &*node.semantic_form == "parameter"
                && parameter_name(graph, position).as_deref() == Some(parameter)
        })
}

#[cfg(test)]
mod float64_nan_tests {
    use super::float64_nan_bits_are_canonical;
    use ix_trace_rs::trace;

    #[test]
    #[trace("FSpec-FR-812-AC-8")]
    fn nan_bit_strings_are_exact_canonical_ieee_encodings() {
        assert!(float64_nan_bits_are_canonical("7ff8000000000000"));
        assert!(float64_nan_bits_are_canonical("7ff0000000000001"));
        assert!(float64_nan_bits_are_canonical("fff8000000000001"));
        for rejected in [
            "7ff800000000000",  // short
            "7FF8000000000000", // uppercase
            "7ff0000000000000", // infinity, not NaN / zero fraction
            "3ff0000000000000", // finite, not NaN
            "0ff8000000000001", // first nibble gives a finite exponent
            "7ff800000000000g", // non-hex character
        ] {
            assert!(
                !float64_nan_bits_are_canonical(rejected),
                "must reject {rejected}"
            );
        }
    }
}
