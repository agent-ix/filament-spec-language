// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source names retained for diagnostics, independent of checked identities.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::{
    CollectionKind, EffectiveId, IeeeWidth, NodeKey, QuantityCarrier, UnitDomain, UnitId, ValueType,
};

use super::domain_package::ValueTypeRef;

/// A source object's member position, independent of its displayed name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MemberSlot {
    /// A declared field.
    Field(DeclarationKey),
    /// An operation's parameter, in authored order.
    Parameter {
        /// The declaring operation.
        operation: DeclarationKey,
        /// The parameter's position in its declaration.
        position: usize,
    },
    /// An operation's result.
    Result(DeclarationKey),
}

/// One node of a flat alias tree. Child indices keep deep trees off the native stack.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AliasNode {
    pub alias: Option<String>,
    pub children: Vec<usize>,
}

/// The aliases written at each position of one checked member type.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AliasTree {
    pub nodes: Vec<AliasNode>,
}

/// The names needed to spell a source model's checked types. These maps are
/// diagnostic context; they do not contribute to model or type identities.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSpellings {
    nodes: BTreeMap<NodeKey, String>,
    objects: BTreeMap<EffectiveId, String>,
    units: BTreeMap<NodeKey, String>,
    aliases: BTreeMap<MemberSlot, AliasTree>,
}

impl SourceSpellings {
    /// Record, tuple, union or enum declaration name, if retained.
    pub fn node_name(&self, key: &NodeKey) -> Option<&str> {
        self.nodes.get(key).map(String::as_str)
    }
    /// Object type name, if retained.
    pub fn object_name(&self, key: &EffectiveId) -> Option<&str> {
        self.objects.get(key).map(String::as_str)
    }
    /// Declared unit name, if retained.
    pub fn unit_name(&self, key: &NodeKey) -> Option<&str> {
        self.units.get(key).map(String::as_str)
    }
    /// Set a declaration's diagnostic name without changing its checked key.
    pub fn name_node(&mut self, key: NodeKey, name: String) {
        self.nodes.insert(key, name);
    }
    /// Set an object type's diagnostic name.
    pub fn name_object(&mut self, key: EffectiveId, name: String) {
        self.objects.insert(key, name);
    }
    /// Set a declared unit's diagnostic name.
    pub fn name_unit(&mut self, key: NodeKey, name: String) {
        self.units.insert(key, name);
    }
    pub(crate) fn aliases(&mut self, slot: MemberSlot, tree: AliasTree) {
        if tree.nodes.iter().any(|node| node.alias.is_some()) {
            self.aliases.insert(slot, tree);
        }
    }
    pub(crate) fn name_type(&mut self, value_type: &ValueType, name: &str) {
        match value_type {
            ValueType::Composite(key) => self.name_node(key.clone(), name.to_owned()),
            ValueType::Enum(shape) => {
                for variant in shape.variants() {
                    self.name_node(variant.owner().clone(), name.to_owned());
                }
            }
            ValueType::Reference(key) => self.name_object(key.clone(), name.to_owned()),
            _ => {}
        }
    }
}

/// The FSL spelling of a member type, retaining the aliases of `slot` when
/// supplied. Domain member variants keep their existing identity spelling.
pub fn fsl_spelling(
    value_type: &ValueTypeRef,
    spellings: &SourceSpellings,
    slot: Option<&MemberSlot>,
) -> String {
    match value_type {
        ValueTypeRef::Resolved(value_type) => render(
            value_type,
            Some(spellings),
            slot.and_then(|slot| spellings.aliases.get(slot)),
        ),
        _ => value_type.to_string(),
    }
}

/// A checked type's internal identity text. It is never a user-facing spelling.
pub(crate) fn identity_spelling(value_type: &ValueType) -> String {
    format!("resolved:{}", render(value_type, None, None))
}

#[derive(Clone, Copy)]
pub(crate) enum TypeView<'a> {
    Value(&'a ValueType),
    Carrier(&'a QuantityCarrier),
}

impl<'a> TypeView<'a> {
    pub(crate) fn children(self) -> Vec<Self> {
        match self {
            Self::Value(ValueType::Option(inner)) => vec![Self::Value(inner)],
            Self::Value(ValueType::Collection(collection)) => {
                vec![Self::Value(collection.element())]
            }
            Self::Value(ValueType::Map(map)) => {
                vec![Self::Value(map.key()), Self::Value(map.values())]
            }
            Self::Value(ValueType::CarriedQuantity(quantity)) => {
                vec![Self::Carrier(quantity.carrier())]
            }
            _ => Vec::new(),
        }
    }
}

fn render(
    value_type: &ValueType,
    spellings: Option<&SourceSpellings>,
    aliases: Option<&AliasTree>,
) -> String {
    enum Part<'a> {
        Type(TypeView<'a>, Option<usize>),
        Text(String),
    }
    let mut parts = vec![Part::Type(TypeView::Value(value_type), aliases.map(|_| 0))];
    let mut output = String::new();
    while let Some(part) = parts.pop() {
        let (view, alias) = match part {
            Part::Text(text) => {
                output.push_str(&text);
                continue;
            }
            Part::Type(view, alias) => (view, alias),
        };
        let alias = alias.and_then(|index| aliases?.nodes.get(index));
        if let Some(name) = alias.and_then(|node| node.alias.as_deref()) {
            output.push_str(name);
            continue;
        }
        let child = |position| alias.and_then(|node| node.children.get(position)).copied();
        let ty = match view {
            TypeView::Carrier(carrier) => {
                output.push_str(&carrier_spelling(carrier));
                continue;
            }
            TypeView::Value(value_type) => value_type,
        };
        match ty {
            ValueType::Boolean => output.push_str("Boolean"),
            ValueType::Integer => output.push_str("Integer"),
            ValueType::String => output.push_str("String"),
            ValueType::Uuid => output.push_str("UUID"),
            ValueType::Timestamp => output.push_str("Timestamp"),
            ValueType::Int(interval) => {
                output.push_str(&format!("Int[{}, {}]", interval.lower(), interval.upper()))
            }
            ValueType::Rational(domain) => output.push_str(&rational_spelling(domain)),
            ValueType::Decimal(decimal) => output.push_str(&decimal_spelling(decimal)),
            ValueType::Float(float) => output.push_str(&format!(
                "{}[{}]",
                match float.width() {
                    IeeeWidth::Binary32 => "Float32",
                    IeeeWidth::Binary64 => "Float64",
                },
                float.rounding().as_str()
            )),
            ValueType::Text(text) => output.push_str(&format!(
                "Text[{}, {}; {}]",
                text.min(),
                text.max(),
                match text.profile() {
                    filament_exact::TextProfile::Nfc => "nfc",
                    filament_exact::TextProfile::Nfd => "nfd",
                    filament_exact::TextProfile::Nfkc => "nfkc",
                    filament_exact::TextProfile::Nfkd => "nfkd",
                    filament_exact::TextProfile::UnicodeScalars => "unicode-scalars",
                    filament_exact::TextProfile::BinaryUtf8 => "binary-utf8",
                }
            )),
            ValueType::Composite(key) => output.push_str(&named_node(key, spellings)),
            ValueType::Enum(shape) => {
                let name = shape
                    .variants()
                    .next()
                    .and_then(|variant| spellings?.node_name(variant.owner()));
                if let Some(name) = name {
                    output.push_str(name);
                } else {
                    if spellings.is_some() {
                        output.push_str("resolved:");
                    }
                    output.push_str("Enum<");
                    for (index, variant) in shape.variants().enumerate() {
                        if index != 0 {
                            output.push_str(", ");
                        }
                        output.push_str(&variant.owner().to_string());
                        output.push_str("::");
                        output.push_str(variant.case());
                    }
                    output.push('>');
                }
            }
            ValueType::Reference(key) => {
                output.push_str("Reference<");
                if let Some(name) = spellings.and_then(|names| names.object_name(key)) {
                    output.push_str(name);
                } else {
                    if spellings.is_some() {
                        output.push_str("resolved:");
                    }
                    output.push_str(&effective_identity(key));
                }
                output.push('>');
            }
            ValueType::Quantity(unit) => {
                output.push_str(&format!("Quantity<{}>", unit_spelling(unit, spellings)))
            }
            ValueType::CarriedQuantity(quantity) => {
                output.push_str(&format!(
                    "Quantity<{}, ",
                    unit_spelling(quantity.unit(), spellings)
                ));
                parts.push(Part::Text(">".to_owned()));
                parts.push(Part::Type(TypeView::Carrier(quantity.carrier()), child(0)));
            }
            ValueType::BoundedQuantity(quantity) => {
                let unit = unit_spelling(quantity.unit(), spellings);
                let bound = |value: Option<&filament_exact::Quantity>| {
                    value.map_or_else(
                        || "*".to_owned(),
                        |value| format!("{} {unit}", rational_literal(value.magnitude())),
                    )
                };
                output.push_str(&format!(
                    "Quantity<{unit}>[{}, {}]",
                    bound(quantity.min()),
                    bound(quantity.max())
                ));
            }
            ValueType::Option(inner) => {
                output.push_str("Option<");
                parts.push(Part::Text(">".to_owned()));
                parts.push(Part::Type(TypeView::Value(inner), child(0)));
            }
            ValueType::Collection(collection) => {
                output.push_str(match collection.kind() {
                    CollectionKind::Sequence => "Sequence",
                    CollectionKind::Set => "Set",
                    CollectionKind::Bag => "Bag",
                    CollectionKind::OrderedSet => "OrderedSet",
                });
                output.push('<');
                parts.push(Part::Text(format!(">{}", bounds(collection.bound()))));
                parts.push(Part::Type(TypeView::Value(collection.element()), child(0)));
            }
            ValueType::Map(map) => {
                output.push_str("Map<");
                parts.push(Part::Text(format!(">{}", bounds(map.bound()))));
                parts.push(Part::Type(TypeView::Value(map.values()), child(1)));
                parts.push(Part::Text(", ".to_owned()));
                parts.push(Part::Type(TypeView::Value(map.key()), child(0)));
            }
            ValueType::Population(maximum) => {
                if spellings.is_some() {
                    output.push_str("resolved:");
                }
                output.push_str("Population");
                if let Some(maximum) = maximum {
                    output.push_str(&format!("[{maximum}]"));
                }
            }
        }
    }
    output
}

fn bounds(bound: Option<filament_exact::CardinalityBound>) -> String {
    bound.map_or_else(String::new, |bound| {
        format!("[{}, {}]", bound.minimum(), bound.maximum())
    })
}

fn named_node(key: &NodeKey, spellings: Option<&SourceSpellings>) -> String {
    spellings
        .and_then(|names| names.node_name(key))
        .map_or_else(
            || {
                if spellings.is_some() {
                    format!("resolved:{key}")
                } else {
                    key.to_string()
                }
            },
            str::to_owned,
        )
}

fn effective_identity(key: &EffectiveId) -> String {
    let declarations: Vec<_> = key
        .owner_chain()
        .iter()
        .map(|key| serde_json::json!({"package": key.package(), "node": key.node()}))
        .collect();
    serde_json::Value::Array(declarations).to_string()
}

fn unit_spelling(unit: &UnitId, spellings: Option<&SourceSpellings>) -> String {
    let name = |key: &NodeKey| {
        spellings
            .and_then(|names| names.unit_name(key))
            .map_or_else(
                || {
                    if spellings.is_some() {
                        format!("resolved:{key}")
                    } else {
                        key.to_string()
                    }
                },
                str::to_owned,
            )
    };
    match unit.domain() {
        UnitDomain::Declared => unit.declared_key().map(name).unwrap_or_default(),
        UnitDomain::Compound => unit
            .compound_terms()
            .unwrap_or_default()
            .iter()
            .map(|(key, exponent)| format!("{}^{exponent}", name(key)))
            .collect::<Vec<_>>()
            .join("*"),
    }
}

fn decimal_spelling(decimal: &filament_exact::DecimalType) -> String {
    format!(
        "Decimal[{}, {}; {}, {}; {}]",
        decimal.lower(),
        decimal.upper(),
        decimal.min_scale(),
        decimal.max_scale(),
        decimal.rounding().as_str()
    )
}

fn rational_spelling(domain: &filament_exact::RationalDomain) -> String {
    format!(
        "Rational[{}, {}; {}, {}]",
        domain.numerator().lower(),
        domain.numerator().upper(),
        domain.denominator().lower(),
        domain.denominator().upper()
    )
}

fn carrier_spelling(carrier: &QuantityCarrier) -> String {
    match carrier {
        QuantityCarrier::Decimal(decimal) => decimal_spelling(decimal),
        QuantityCarrier::Rational(rational) => rational_spelling(rational),
        QuantityCarrier::Integer { domain, rounding } => format!(
            "Int[{}, {}; {}]",
            domain.lower(),
            domain.upper(),
            rounding.as_str()
        ),
    }
}

fn rational_literal(value: &filament_exact::Rational) -> String {
    if value.denominator() == &filament_exact::Integer::from(1_i64) {
        value.numerator().to_string()
    } else {
        format!("rational({}, {})", value.numerator(), value.denominator())
    }
}
