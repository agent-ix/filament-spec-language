// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Applications and transition identities": the finite value domain
//! of a declared type, indexed rather than materialized so a caller walks a
//! product of domains one element at a time.

use filament_exact::{
    from_admitted, from_admitted_slots, CollectionKind, CollectionType, EnumMember, FieldValue,
    Integer, NodeKey, ObjectReference, OptionValue, Presence, TextType, UnionMember, UnionValue,
    Value, ValueType,
};
use filament_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, TypeEnvironment, UnionMemberDeclaration,
};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};

/// Selector maxima authenticated by a timed subject's live-clock bound.
pub(super) type ClockSelectorMaxima = BTreeMap<NodeKey, Vec<u64>>;

/// Each record type's slot indices in declaration order, the first the most
/// significant component of its enumerated domain (FR-120, FR-215). A record
/// it does not list enumerates in slot order.
pub(crate) type RecordOrders = BTreeMap<NodeKey, Vec<usize>>;

/// The slot indices of each record of `types` in the declaration order
/// `declared` names its fields, then any slot it does not name in slot
/// order.
pub(crate) fn record_orders(
    types: &TypeEnvironment,
    declared: &BTreeMap<NodeKey, Vec<String>>,
) -> RecordOrders {
    declared
        .iter()
        .filter_map(|(declaration, names)| {
            let CompositeShape::Record(fields) = types.composite(declaration.clone())?.shape()
            else {
                return None;
            };
            let mut order: Vec<usize> = names
                .iter()
                .filter_map(|name| fields.iter().position(|field| field.name() == name))
                .collect();
            let rest: Vec<usize> = (0..fields.len())
                .filter(|slot| !order.contains(slot))
                .collect();
            order.extend(rest);
            Some((declaration.clone(), order))
        })
        .collect()
}

/// Positions of clock-reference leaves in a member's checked data type.
/// Composite cycles are visited once; their ordinary extent check owns recursion.
pub(super) fn clock_reference_paths(
    value_type: &ValueType,
    types: &TypeEnvironment,
) -> Vec<Vec<u32>> {
    let mut paths = Vec::new();
    let mut pending = vec![(value_type, Vec::new(), Vec::new())];
    while let Some((value_type, path, mut entered)) = pending.pop() {
        let children: Vec<&ValueType> = match value_type {
            ValueType::Reference(target)
                if fsl_semantics::model::clock_reference::is_clock_type(target) =>
            {
                paths.push(path);
                continue;
            }
            ValueType::Option(payload) => vec![payload],
            ValueType::Collection(collection) => vec![collection.element()],
            ValueType::Map(map) => vec![map.values()],
            ValueType::Composite(key) if !entered.contains(key) => {
                entered.push(key.clone());
                match types
                    .composite(key.clone())
                    .map(CompositeDeclaration::shape)
                {
                    Some(CompositeShape::Record(fields)) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    Some(CompositeShape::Tuple(positions)) => positions.iter().collect(),
                    Some(CompositeShape::Union(members)) => members
                        .iter()
                        .flat_map(|member| member.positions())
                        .collect(),
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        for (at, child) in children.into_iter().enumerate().rev() {
            let Ok(at) = u32::try_from(at) else {
                continue;
            };
            let mut child_path = path.clone();
            child_path.push(at);
            pending.push((child, child_path, entered.clone()));
        }
    }
    paths
}

/// Whether this module can enumerate `value_type`: `Boolean`, `Int[..]`,
/// `Option<T>`, `Reference<T>` and a sequence with a declared maximum over
/// one of them. Every other type is an unbounded domain (FR-120 "Bounds").
pub(crate) fn enumerable(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Boolean | ValueType::Int(_) | ValueType::Reference(_) => true,
        ValueType::Text(_) => true,
        ValueType::Option(payload) => enumerable(payload),
        ValueType::Collection(collection) => {
            collection.kind() == CollectionKind::Sequence
                && collection.bound().is_some()
                && enumerable(collection.element())
        }
        ValueType::Integer
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::CarriedQuantity(_)
        | ValueType::Map(_)
        | ValueType::String
        | ValueType::Bytes
        | ValueType::JsonObject
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Composite(_)
        | ValueType::Population(_) => false,
    }
}

/// The finite space of a template's attempt type (ADR-027 ST-11): a type FR-120
/// enumerates, an enum, a record with optional fields, or a record or tuple of
/// such, indexed in declaration order with the last field moving fastest and
/// an absent optional field first.
pub(super) enum Space {
    Plain(Domain),
    Enum(Vec<Value>),
    Composite {
        declaration: NodeKey,
        fields: Vec<(bool, Space)>,
        /// The slot indices, most significant first.
        order: Vec<usize>,
    },
    /// A union: its members in declaration order, each with the spaces of
    /// its payload positions.
    Union(Vec<(UnionMember, Vec<Space>)>),
}

impl Space {
    /// The space of `value_type`; `None` for a type with no finite space
    /// here (a recursive record, or a field FR-120 does not enumerate).
    fn of(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
        orders: &RecordOrders,
        visiting: &mut Vec<NodeKey>,
    ) -> Option<Self> {
        match value_type {
            ValueType::Enum(shape) => Some(Self::Enum(
                shape
                    .variants()
                    .enumerate()
                    .map(|(rank, variant)| {
                        u32::try_from(rank)
                            .ok()
                            .map(|rank| Value::Enum(EnumMember::new(variant.clone(), rank)))
                    })
                    .collect::<Option<Vec<Value>>>()?,
            )),
            ValueType::Composite(declaration) => {
                if visiting.contains(declaration) {
                    return None;
                }
                let members: Vec<(bool, &ValueType)> = match types
                    .composite(declaration.clone())
                    .map(CompositeDeclaration::shape)?
                {
                    CompositeShape::Record(fields) => fields
                        .iter()
                        .map(|field| (field.presence() == Presence::Optional, field.value_type()))
                        .collect(),
                    CompositeShape::Tuple(positions) => {
                        positions.iter().map(|position| (false, position)).collect()
                    }
                    CompositeShape::Union(declared) => {
                        visiting.push(declaration.clone());
                        // FR-120: cases in ascending UTF-8 bytes of their
                        // names.
                        let mut declared: Vec<_> = declared.iter().collect();
                        declared.sort_by(|left, right| {
                            left.identifier()
                                .as_bytes()
                                .cmp(right.identifier().as_bytes())
                        });
                        let spaces = declared
                            .into_iter()
                            .map(|member| {
                                let verified = member.member()?.clone();
                                let positions = member
                                    .positions()
                                    .iter()
                                    .map(|position| {
                                        Self::of(position, objects, types, orders, visiting)
                                    })
                                    .collect::<Option<Vec<_>>>()?;
                                Some((verified, positions))
                            })
                            .collect::<Option<Vec<_>>>();
                        visiting.pop();
                        return spaces.map(Self::Union);
                    }
                };
                visiting.push(declaration.clone());
                let fields = members
                    .into_iter()
                    .map(|(optional, member)| {
                        Self::of(member, objects, types, orders, visiting)
                            .map(|space| (optional, space))
                    })
                    .collect::<Option<Vec<_>>>()?;
                visiting.pop();
                let order = orders
                    .get(declaration)
                    .filter(|order| order.len() == fields.len())
                    .cloned()
                    .unwrap_or_else(|| (0..fields.len()).collect());
                Some(Self::Composite {
                    declaration: declaration.clone(),
                    fields,
                    order,
                })
            }
            other => Domain::of(other, objects, types, orders).map(Self::Plain),
        }
    }

    /// How many values the space holds, saturating at `u128::MAX`.
    fn len(&self) -> u128 {
        match self {
            Self::Plain(domain) => domain.len(),
            Self::Enum(values) => values.len() as u128,
            Self::Union(members) => members.iter().fold(0u128, |total, (_, positions)| {
                total.saturating_add(
                    positions
                        .iter()
                        .fold(1u128, |product, space| product.saturating_mul(space.len())),
                )
            }),
            Self::Composite { fields, .. } => {
                fields.iter().fold(1u128, |total, (optional, space)| {
                    total.saturating_mul(space.len().saturating_add(u128::from(*optional)))
                })
            }
        }
    }

    /// The value at `index`; `None` past the end.
    fn nth(&self, index: u128) -> Option<Value> {
        match self {
            Self::Plain(domain) => domain.nth(index),
            Self::Enum(values) => values.get(usize::try_from(index).ok()?).cloned(),
            Self::Union(members) => {
                let mut remaining = index;
                for (member, positions) in members {
                    let count = positions
                        .iter()
                        .fold(1u128, |product, space| product.saturating_mul(space.len()));
                    if remaining >= count {
                        remaining -= count;
                        continue;
                    }
                    let digits = product_digits(remaining, positions.iter().map(Self::len))?;
                    let payload = positions
                        .iter()
                        .zip(digits)
                        .map(|(space, digit)| space.nth(digit))
                        .collect::<Option<Vec<_>>>()?;
                    return Some(UnionValue::from_admitted(member.clone(), payload));
                }
                None
            }
            Self::Composite {
                declaration,
                fields,
                order,
            } => {
                if index >= self.len() {
                    return None;
                }
                let mut remaining = index;
                let mut slots: Vec<Option<FieldValue>> = vec![None; fields.len()];
                for slot in order.iter().rev() {
                    let (optional, space) = fields.get(*slot)?;
                    let radix = space.len().saturating_add(u128::from(*optional));
                    let digit = remaining.checked_rem(radix)?;
                    remaining = remaining.checked_div(radix)?;
                    *slots.get_mut(*slot)? = Some(match (optional, digit.checked_sub(1)) {
                        (true, None) => FieldValue::Absent,
                        (true, Some(inner)) => FieldValue::Present(space.nth(inner)?),
                        (false, _) => FieldValue::Present(space.nth(digit)?),
                    });
                }
                let slots = slots.into_iter().collect::<Option<Vec<_>>>()?;
                Some(from_admitted_slots(
                    declaration.clone(),
                    slots.into_boxed_slice(),
                ))
            }
        }
    }
}

/// Whether `value_type` has a finite FR-120 value domain ([`Domain::of`]).
pub(crate) fn finite(value_type: &ValueType, types: &TypeEnvironment) -> bool {
    Domain::of(value_type, &[], types, &RecordOrders::new()).is_some()
}

/// Whether [`composite_values`] and [`Domain`] together enumerate `value_type`:
/// a type FR-120 enumerates, or an enum, record, tuple or union of finite
/// fields (ADR-027 ST-11: the records of a template's attempt type).
pub(crate) fn enumerable_in(value_type: &ValueType, types: &TypeEnvironment) -> bool {
    match value_type {
        ValueType::Composite(_) | ValueType::Enum(_) => Space::of(
            value_type,
            &[],
            types,
            &RecordOrders::new(),
            &mut Vec::new(),
        )
        .is_some(),
        other => enumerable(other),
    }
}

/// The child-index paths (the classifier's: record field, tuple position,
/// flattened union payload position, each from 0) of the leaves of
/// `value_type` this module cannot enumerate; the type's own path when no
/// leaf is to blame.
pub(crate) fn unenumerable_paths(value_type: &ValueType, types: &TypeEnvironment) -> Vec<Vec<u32>> {
    fn walk(
        value_type: &ValueType,
        types: &TypeEnvironment,
        path: &mut Vec<u32>,
        visiting: &mut Vec<NodeKey>,
        found: &mut Vec<Vec<u32>>,
    ) {
        match value_type {
            ValueType::Enum(_) => {}
            ValueType::Composite(declaration) => {
                if visiting.contains(declaration) {
                    return;
                }
                let Some(declared) = types.composite(declaration.clone()) else {
                    found.push(path.clone());
                    return;
                };
                let members: Vec<&ValueType> = match declared.shape() {
                    CompositeShape::Record(fields) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    CompositeShape::Tuple(positions) => positions.iter().collect(),
                    CompositeShape::Union(members) => members
                        .iter()
                        .flat_map(UnionMemberDeclaration::positions)
                        .collect(),
                };
                visiting.push(declaration.clone());
                for (index, member) in members.into_iter().enumerate() {
                    let Ok(index) = u32::try_from(index) else {
                        break;
                    };
                    path.push(index);
                    walk(member, types, path, visiting, found);
                    path.pop();
                }
                visiting.pop();
            }
            other => {
                if Domain::of(other, &[], types, &RecordOrders::new()).is_none() {
                    found.push(path.clone());
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(
        value_type,
        types,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut found,
    );
    if found.is_empty() && !enumerable_in(value_type, types) {
        found.push(Vec::new());
    }
    found
}

/// Every value of the composite type `value_type`, in index order; `charge`
/// is called once per value. `None` when the type has no finite space.
pub(super) fn composite_values<E>(
    value_type: &ValueType,
    objects: &[ObjectReference],
    types: &TypeEnvironment,
    orders: &RecordOrders,
    mut charge: impl FnMut() -> Result<(), E>,
) -> Result<Option<Vec<Value>>, E> {
    let Some(space) = Space::of(value_type, objects, types, orders, &mut Vec::new()) else {
        return Ok(None);
    };
    let mut values = Vec::new();
    let mut index = 0u128;
    while let Some(value) = space.nth(index) {
        charge()?;
        values.push(value);
        index = index.saturating_add(1);
    }
    Ok(Some(values))
}

/// A product's component indices, the last component moving fastest.
fn product_digits(
    mut index: u128,
    radices: impl DoubleEndedIterator<Item = u128>,
) -> Option<Vec<u128>> {
    let mut digits = Vec::new();
    for radix in radices.rev() {
        digits.push(index.checked_rem(radix)?);
        index = index.checked_div(radix)?;
    }
    if index != 0 {
        return None;
    }
    digits.reverse();
    Some(digits)
}

/// The finite domain of one declared type over a set of objects, in
/// FR-120's order: `false` then `true`; the integers ascending; absent then
/// each payload; the objects in the order given; sequences by length then
/// by element index, the first element most significant.
pub(super) enum Domain {
    Boolean,
    /// An unbounded integer type restricted to FR-190's size-ordered `S_b`.
    SizeInteger {
        bits: u8,
    },
    Int {
        lower: Integer,
        len: u128,
    },
    Option {
        payload_type: ValueType,
        inner: Box<Domain>,
    },
    Reference(Vec<ObjectReference>),
    Sequence {
        collection: CollectionType,
        element: Box<Domain>,
        min: u64,
        max: u64,
    },
    /// A `Set` (`repeat` false) or a `Bag` (`repeat` true) with a declared
    /// maximum (FR-120-AC-26): its values by element count, then each
    /// count's combinations of ascending element indices, the smallest
    /// element most significant.
    Combinations {
        collection: CollectionType,
        element: Box<Domain>,
        min: u64,
        max: u64,
        repeat: bool,
    },
    /// An `OrderedSet` with a declared maximum (FR-120-AC-28): its values by
    /// length, then each length's sequences of distinct elements in
    /// lexicographic element order, the first element most significant.
    Arrangements {
        collection: CollectionType,
        element: Box<Domain>,
        min: u64,
        max: u64,
    },
    /// A bounded FR-141 text domain in FR-804 retained-value order.
    Text(TextType),
    /// A random parameter's support, in declared order (FR-187, ADR-024
    /// PM-1): its finite domain whatever its declared type.
    Listed(Vec<Value>),
    /// An enum, or a record or tuple of finite components.
    Space(Box<Space>),
}

impl Domain {
    /// The domain of `value_type`, where a `Reference<T>` ranges over the
    /// members of `objects` whose most-specific type conforms to `T`.
    /// `None` when `value_type` is not [`enumerable`].
    pub(super) fn of(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
        orders: &RecordOrders,
    ) -> Option<Self> {
        match value_type {
            ValueType::Boolean => Some(Self::Boolean),
            ValueType::Text(text_type) => Some(Self::Text(*text_type)),
            ValueType::Int(interval) => {
                let span = interval.upper().sub(interval.lower()).add(&Integer::one());
                Some(Self::Int {
                    lower: interval.lower().clone(),
                    len: span.to_u64().map_or(u128::MAX, u128::from),
                })
            }
            ValueType::Option(payload) => Some(Self::Option {
                payload_type: (**payload).clone(),
                inner: Box::new(Self::of(payload, objects, types, orders)?),
            }),
            // FR-120 "Applications" (STD-508): an enum, a record or tuple
            // whose components all have a finite domain, the first declared
            // most significant, and a model-variant union whose payload
            // positions all have one; a clock component is a place, not a
            // component of the checked value.
            ValueType::Enum(_) | ValueType::Composite(_) => {
                Space::of(value_type, objects, types, orders, &mut Vec::new())
                    .map(|space| Self::Space(Box::new(space)))
            }
            ValueType::Reference(target)
                if fsl_semantics::model::clock_reference::is_clock_type(target) =>
            {
                // Timed declarations and selector bounds do not authenticate
                // the original population of a generated clock reference.
                // The model admission boundary reports this unsupported domain.
                None
            }
            ValueType::Reference(target) => Some(Self::Reference(
                objects
                    .iter()
                    .filter(|object| types.conforms(object.object_type().clone(), target.clone()))
                    .cloned()
                    .collect(),
            )),
            // FR-120-AC-26: a sequence, set or bag with a declared maximum
            // over a finite element, an ordered set by its arrangements.
            ValueType::Collection(collection) => {
                let bound = collection.bound()?;
                let element = Box::new(Self::of(collection.element(), objects, types, orders)?);
                let (collection, min, max) =
                    ((**collection).clone(), bound.minimum(), bound.maximum());
                match collection.kind() {
                    CollectionKind::Sequence => Some(Self::Sequence {
                        collection,
                        element,
                        min,
                        max,
                    }),
                    CollectionKind::Set | CollectionKind::Bag => Some(Self::Combinations {
                        repeat: collection.kind() == CollectionKind::Bag,
                        collection,
                        element,
                        min,
                        max,
                    }),
                    CollectionKind::OrderedSet => Some(Self::Arrangements {
                        collection,
                        element,
                        min,
                        max,
                    }),
                }
            }
            _ => None,
        }
    }

    /// The domain of `value_type` under FR-190's size bound for an otherwise
    /// unbounded integer. Other unbounded carriers remain unavailable until
    /// their canonical size measures are implemented.
    pub(super) fn of_bounded(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
        orders: &RecordOrders,
        max_value_bits: Option<u8>,
    ) -> Option<Self> {
        match value_type {
            ValueType::Integer => Some(Self::SizeInteger {
                bits: max_value_bits?,
            }),
            _ => Self::of(value_type, objects, types, orders),
        }
    }

    /// How many values a `Map<K, V>[min, max]` of FR-120 holds without
    /// enumerating them, saturating at `u128::MAX`: a map is a set of
    /// distinct keys each with a value, so with `|K|` keys and `|V|` values it
    /// holds the sum over entry counts `k` from `min` to `max` of
    /// C(`|K|`, `k`) * `|V|`^`k`. Its values order by entry count, then by
    /// keys ascending in `K`'s domain order, then by values in `V`'s. The
    /// value kernel has no map value, so the domain is counted and never
    /// materialized: [`Self::map_size`] is what a limit check reads.
    pub(super) fn map_size(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
        orders: &RecordOrders,
    ) -> Option<u128> {
        let ValueType::Map(map) = value_type else {
            return None;
        };
        let bound = map.bound()?;
        let size = |member: &ValueType| match member {
            ValueType::Map(_) => Self::map_size(member, objects, types, orders),
            other => Self::of(other, objects, types, orders).map(|domain| domain.len()),
        };
        let (keys, values) = (size(map.key())?, size(map.values())?);
        Some(entries_size(keys, values, bound.minimum(), bound.maximum()))
    }

    /// How many values the domain holds, saturating at `u128::MAX`.
    pub(super) fn len(&self) -> u128 {
        match self {
            Self::Boolean => 2,
            Self::SizeInteger { bits } => (1_u128 << (u32::from(*bits) + 1)) - 1,
            Self::Int { len, .. } => *len,
            Self::Option { inner, .. } => inner.len().saturating_add(1),
            Self::Reference(objects) => objects.len() as u128,
            Self::Listed(values) => values.len() as u128,
            Self::Space(space) => space.len(),
            Self::Sequence {
                element, min, max, ..
            } => {
                let base = element.len();
                (*min..=*max).fold(0u128, |total, length| {
                    total.saturating_add(power(base, length))
                })
            }
            Self::Combinations {
                element,
                min,
                max,
                repeat,
                ..
            } => {
                let base = element.len();
                (*min..=*max).fold(0u128, |total, count| {
                    total.saturating_add(combinations(base, u128::from(count), *repeat))
                })
            }
            Self::Arrangements {
                element, min, max, ..
            } => {
                let base = element.len();
                (*min..=*max).fold(0u128, |total, length| {
                    total.saturating_add(arrangements(base, u128::from(length)))
                })
            }
            Self::Text(text_type) => text_domain_count(text_type)
                .and_then(|count| count.to_string().parse::<u128>().ok())
                .unwrap_or(u128::MAX),
        }
    }

    /// The value at `index`; `None` past the end.
    pub(super) fn nth(&self, index: u128) -> Option<Value> {
        match self {
            Self::Boolean => match index {
                0 => Some(Value::Boolean(false)),
                1 => Some(Value::Boolean(true)),
                _ => None,
            },
            Self::SizeInteger { bits } => size_integer_nth(*bits, index),
            Self::Int { lower, len } => {
                if index >= *len {
                    return None;
                }
                let offset = u64::try_from(index).ok()?;
                Some(Value::Integer(lower.add(&Integer::from(offset))))
            }
            Self::Option {
                payload_type,
                inner,
            } => match index.checked_sub(1) {
                None => Some(OptionValue::none(payload_type.clone())),
                Some(payload) => Some(OptionValue::from_admitted(
                    payload_type.clone(),
                    Some(inner.nth(payload)?),
                )),
            },
            Self::Reference(objects) => objects
                .get(usize::try_from(index).ok()?)
                .cloned()
                .map(Value::Reference),
            Self::Listed(values) => values.get(usize::try_from(index).ok()?).cloned(),
            Self::Space(space) => space.nth(index),
            Self::Sequence {
                collection,
                element,
                min,
                max,
            } => {
                let base = element.len();
                let mut remaining = index;
                for length in *min..=*max {
                    let count = power(base, length);
                    if remaining < count {
                        let mut digits = Vec::new();
                        for _ in 0..length {
                            digits.push(remaining % base);
                            remaining /= base;
                        }
                        digits.reverse();
                        let elements = digits
                            .into_iter()
                            .map(|digit| element.nth(digit))
                            .collect::<Option<Vec<Value>>>()?;
                        return Some(from_admitted(collection.clone(), elements));
                    }
                    remaining -= count;
                }
                None
            }
            Self::Combinations {
                collection,
                element,
                min,
                max,
                repeat,
            } => {
                let base = element.len();
                let mut remaining = index;
                for count in *min..=*max {
                    let count = u128::from(count);
                    let total = combinations(base, count, *repeat);
                    if remaining >= total {
                        remaining -= total;
                        continue;
                    }
                    // Unrank the ascending index combination: each position
                    // takes the least index whose completions cover
                    // `remaining`.
                    let mut indices = Vec::new();
                    let mut least = 0u128;
                    for position in 0..count {
                        let left = count - position - 1;
                        let mut candidate = least;
                        loop {
                            if candidate >= base {
                                return None;
                            }
                            let after = if *repeat { candidate } else { candidate + 1 };
                            let completions = combinations(base - after, left, *repeat);
                            if remaining < completions {
                                break;
                            }
                            remaining -= completions;
                            candidate += 1;
                        }
                        indices.push(candidate);
                        least = if *repeat { candidate } else { candidate + 1 };
                    }
                    let elements = indices
                        .into_iter()
                        .map(|digit| element.nth(digit))
                        .collect::<Option<Vec<Value>>>()?;
                    return Some(from_admitted(collection.clone(), elements));
                }
                None
            }
            Self::Arrangements {
                collection,
                element,
                min,
                max,
            } => {
                let base = element.len();
                let mut remaining = index;
                for length in *min..=*max {
                    let length = u128::from(length);
                    let total = arrangements(base, length);
                    if remaining >= total {
                        remaining -= total;
                        continue;
                    }
                    // Each position takes the least unused element whose
                    // completions cover `remaining`.
                    let mut used: Vec<u128> = Vec::new();
                    let mut elements = Vec::new();
                    for position in 0..length {
                        let block = arrangements(base - position - 1, length - position - 1);
                        // The `rank`-th unused index, counting past the
                        // used ones (kept ascending).
                        let mut digit = remaining / block;
                        remaining %= block;
                        for taken in &used {
                            if *taken <= digit {
                                digit += 1;
                            }
                        }
                        used.push(digit);
                        used.sort_unstable();
                        elements.push(element.nth(digit)?);
                    }
                    return Some(from_admitted(collection.clone(), elements));
                }
                None
            }
            Self::Text(text_type) => {
                text_unrank(text_type, &Integer::from_str(&index.to_string()).ok()?)
                    .and_then(|text| admitted_text_value(text_type, &text))
            }
        }
    }

    /// The exact number of values when this domain has a supported exact
    /// count. Text counts may exceed the machine integer used by iteration.
    pub(super) fn exact_len(&self) -> Option<Integer> {
        match self {
            Self::Text(text_type) => text_domain_count(text_type),
            _ => Integer::from_str(&self.len().to_string()).ok(),
        }
    }

    /// The value at an exact integer index, including Text domains wider than
    /// the machine index used by finite graph enumeration.
    pub(super) fn nth_exact(&self, index: &Integer) -> Option<Value> {
        match self {
            Self::Text(text_type) => {
                text_unrank(text_type, index).and_then(|text| admitted_text_value(text_type, &text))
            }
            _ => self.nth(index.to_string().parse().ok()?),
        }
    }

    /// The zero-based FR-120/FR-190 domain index of `value`.
    pub(super) fn index_of(&self, value: &Value) -> Option<u128> {
        match (self, value) {
            (Self::Boolean, Value::Boolean(false)) => Some(0),
            (Self::Boolean, Value::Boolean(true)) => Some(1),
            (Self::Int { lower, len }, Value::Integer(value)) => {
                let offset = value.sub(lower).to_string().parse::<u128>().ok()?;
                (offset < *len).then_some(offset)
            }
            (Self::SizeInteger { bits }, Value::Integer(value)) => {
                if value.is_zero() {
                    return Some(0);
                }
                let magnitude = value.abs().to_u64()? as u128;
                let level = u8::try_from(u128::BITS - magnitude.leading_zeros()).ok()?;
                if level > *bits {
                    return None;
                }
                let start = (1_u128.checked_shl(u32::from(level))?).checked_sub(1)?;
                let half = 1_u128.checked_shl(u32::from(level - 1))?;
                let offset = if value.is_negative() {
                    (half.checked_mul(2)?.checked_sub(1)?).checked_sub(magnitude)?
                } else {
                    half.checked_add(magnitude.checked_sub(half)?)?
                };
                start.checked_add(offset)
            }
            _ => {
                let mut index = 0_u128;
                while let Some(candidate) = self.nth(index) {
                    if super::probabilistic::same_value(&candidate, value) {
                        return Some(index);
                    }
                    index = index.checked_add(1)?;
                }
                None
            }
        }
    }
}

/// `Integer` at the zero-based index in FR-190's size order.
fn text_domain_count(text_type: &TextType) -> Option<Integer> {
    let min = text_type.min();
    let max = text_type.max();
    let profile = text_type.profile();
    match profile {
        filament_exact::TextProfile::UnicodeScalars => {
            let alphabet = Integer::from(1_112_064_u64);
            Some((min..=max).fold(Integer::zero(), |total, length| {
                total.add(&alphabet.pow(&Integer::from(length)))
            }))
        }
        filament_exact::TextProfile::BinaryUtf8 => {
            let max = usize::try_from(max).ok()?;
            let mut weights = Vec::with_capacity(max.saturating_add(1));
            for length in 0..=max {
                let count = match length {
                    0 => Integer::one(),
                    _ => {
                        let mut total = Integer::zero();
                        for (width, alphabet) in
                            [128_u64, 1_920, 61_440, 1_048_576].into_iter().enumerate()
                        {
                            if let Some(previous) = length.checked_sub(width + 1) {
                                total = total.add(&Integer::from(alphabet).mul(&weights[previous]));
                            }
                        }
                        total
                    }
                };
                weights.push(count);
            }
            let mut total = Integer::zero();
            for length in usize::try_from(min).ok()?..=max {
                total = total.add(weights.get(length)?);
            }
            Some(total)
        }
        filament_exact::TextProfile::Nfd | filament_exact::TextProfile::Nfkd => {
            let table = nfd_suffix_counts(profile, max)?;
            let first = usize::try_from(min).ok()?;
            let last = usize::try_from(max).ok()?;
            Some((first..=last).fold(Integer::zero(), |total, length| {
                total.add(&table[length][0])
            }))
        }
        filament_exact::TextProfile::Nfc | filament_exact::TextProfile::Nfkc => {
            let table = composed_suffix_counts(profile, max)?;
            let first = usize::try_from(min).ok()?;
            let last = usize::try_from(max).ok()?;
            Some((first..=last).fold(Integer::zero(), |total, length| {
                total.add(&table[length][0][0])
            }))
        }
    }
}

fn text_unrank(text_type: &TextType, index: &Integer) -> Option<String> {
    if index.is_negative() {
        return None;
    }
    if index.is_zero() {
        if text_type.min() == 0 {
            return Some(String::new());
        }
        let count = usize::try_from(text_type.min()).ok()?;
        return Some(std::iter::repeat_n('\0', count).collect());
    }
    if *index >= text_domain_count(text_type)? {
        return None;
    }
    match text_type.profile() {
        filament_exact::TextProfile::UnicodeScalars => {
            let mut prefix = String::new();
            let mut remaining = index.clone();
            loop {
                let length = u64::try_from(prefix.chars().count()).ok()?;
                if (text_type.min()..=text_type.max()).contains(&length) {
                    if remaining.is_zero() {
                        return Some(prefix);
                    }
                    remaining = remaining.sub(&Integer::one());
                }
                let child_count = unicode_prefix_count(length.checked_add(1)?, text_type)?;
                let alphabet = Integer::from(1_112_064_u64);
                let scalar_index = remaining
                    .div_mod_floor(filament_exact::NonZeroInteger::try_from(&child_count).ok()?);
                let scalar_rank = u32::try_from(scalar_index.0.to_u64()?).ok()?;
                if scalar_rank >= 1_112_064 {
                    return None;
                }
                let _ = alphabet;
                remaining = scalar_index.1;
                prefix.push(scalar_from_rank(scalar_rank)?);
            }
        }
        filament_exact::TextProfile::BinaryUtf8 => {
            let mut prefix = String::new();
            let mut remaining = index.clone();
            let mut weights = Vec::new();
            let max = usize::try_from(text_type.max()).ok()?;
            for length in 0..=max {
                let count = match length {
                    0 => Integer::one(),
                    _ => {
                        let mut total = Integer::zero();
                        for (width, alphabet) in
                            [128_u64, 1_920, 61_440, 1_048_576].into_iter().enumerate()
                        {
                            if let Some(previous) = length.checked_sub(width + 1) {
                                total = total.add(&Integer::from(alphabet).mul(&weights[previous]));
                            }
                        }
                        total
                    }
                };
                weights.push(count);
            }
            loop {
                let byte_length = u64::try_from(prefix.len()).ok()?;
                if (text_type.min()..=text_type.max()).contains(&byte_length) {
                    if remaining.is_zero() {
                        return Some(prefix);
                    }
                    remaining = remaining.sub(&Integer::one());
                }
                let mut found = None;
                for (width, start, end) in [
                    (1_usize, 0_u32, 0x7f_u32),
                    (2, 0x80, 0x7ff),
                    (3, 0x800, 0xffff),
                    (4, 0x10000, 0x10ffff),
                ] {
                    let Some(next_bytes) = prefix.len().checked_add(width) else {
                        continue;
                    };
                    let mut child_count = Integer::zero();
                    for length in usize::try_from(text_type.min()).ok()?.max(next_bytes)..=max {
                        let left = length.checked_sub(next_bytes)?;
                        child_count = child_count.add(weights.get(left)?);
                    }
                    if child_count.is_zero() {
                        continue;
                    }
                    let width_count = if width == 3 {
                        u64::from(0xd7ff - start + 1) + u64::from(end - 0xe000 + 1)
                    } else {
                        u64::from(end - start + 1)
                    };
                    let group_count = child_count.mul(&Integer::from(width_count));
                    if remaining >= group_count {
                        remaining = remaining.sub(&group_count);
                        continue;
                    }
                    let (rank, suffix) = remaining.div_mod_floor(
                        filament_exact::NonZeroInteger::try_from(&child_count).ok()?,
                    );
                    let rank = u32::try_from(rank.to_u64()?).ok()?;
                    let surrogate_gap = 0xd800 - start;
                    let scalar = if width == 3 && rank >= surrogate_gap {
                        0xe000 + (rank - surrogate_gap)
                    } else {
                        start + rank
                    };
                    found = Some((scalar_from_rank_or_scalar(scalar)?, suffix));
                    break;
                }
                let (character, suffix) = found?;
                prefix.push(character);
                remaining = suffix;
            }
        }
        filament_exact::TextProfile::Nfd | filament_exact::TextProfile::Nfkd => {
            let mut prefix = String::new();
            let mut remaining = index.clone();
            let max = usize::try_from(text_type.max()).ok()?;
            let table = nfd_suffix_counts(text_type.profile(), text_type.max())?;
            let alphabet = normalized_scalar_alphabet(text_type.profile())?;
            let mut last_ccc = 0_u8;
            loop {
                let length = prefix.chars().count();
                if (usize::try_from(text_type.min()).ok()?..=max).contains(&length) {
                    if remaining.is_zero() {
                        return Some(prefix);
                    }
                    remaining = remaining.sub(&Integer::one());
                }
                if length >= max {
                    return None;
                }
                let mut selected = None;
                for (character, ccc) in alphabet {
                    if *ccc != 0 && last_ccc > *ccc {
                        continue;
                    }
                    let new_length = length.checked_add(1)?;
                    let first = usize::try_from(text_type.min()).ok()?.max(new_length);
                    let child_count = (first..=max).fold(Integer::zero(), |total, target| {
                        total.add(&table[target - new_length][usize::from(*ccc)])
                    });
                    if child_count.is_zero() {
                        continue;
                    }
                    if remaining < child_count {
                        selected = Some((*character, *ccc));
                        break;
                    }
                    remaining = remaining.sub(&child_count);
                }
                let (character, ccc) = selected?;
                prefix.push(character);
                last_ccc = ccc;
            }
        }
        filament_exact::TextProfile::Nfc | filament_exact::TextProfile::Nfkc => {
            let table = composed_suffix_counts(text_type.profile(), text_type.max())?;
            let alphabet = composed_scalar_alphabet(text_type.profile())?;
            let mut prefix = String::new();
            let mut remaining = index.clone();
            let mut starter = 0_usize;
            let mut last_ccc = 0_u8;
            let max = usize::try_from(text_type.max()).ok()?;
            loop {
                let length = prefix.chars().count();
                if (usize::try_from(text_type.min()).ok()?..=max).contains(&length) {
                    if remaining.is_zero() {
                        return Some(prefix);
                    }
                    remaining = remaining.sub(&Integer::one());
                }
                if length >= max {
                    return None;
                }
                let mut selected = None;
                for (character, ccc, next_starter) in &alphabet.alphabet {
                    if *ccc != 0 && last_ccc > *ccc {
                        continue;
                    }
                    if (last_ccc == 0 || last_ccc < *ccc)
                        && composed_pair_exists(alphabet, starter, *character)
                    {
                        continue;
                    }
                    let new_length = length.checked_add(1)?;
                    let first = usize::try_from(text_type.min()).ok()?.max(new_length);
                    let ccc_index = alphabet.ccc_indexes[usize::from(*ccc)];
                    let child_count = (first..=max).fold(Integer::zero(), |total, target| {
                        total.add(&table[target - new_length][*next_starter][ccc_index])
                    });
                    if child_count.is_zero() {
                        continue;
                    }
                    if remaining < child_count {
                        selected = Some((*character, *ccc, *next_starter));
                        break;
                    }
                    remaining = remaining.sub(&child_count);
                }
                let (character, ccc, next_starter) = selected?;
                prefix.push(character);
                if ccc == 0 {
                    starter = next_starter;
                }
                last_ccc = ccc;
            }
        }
    }
}

fn unicode_prefix_count(prefix_length: u64, text_type: &TextType) -> Option<Integer> {
    if prefix_length > text_type.max() {
        return Some(Integer::zero());
    }
    let alphabet = Integer::from(1_112_064_u64);
    let first = prefix_length.max(text_type.min());
    Some(
        (first..=text_type.max()).fold(Integer::zero(), |total, length| {
            total.add(&alphabet.pow(&Integer::from(length - prefix_length)))
        }),
    )
}

fn scalar_from_rank(rank: u32) -> Option<char> {
    let scalar = if rank < 0xd800 {
        rank
    } else {
        rank.checked_add(0x800)?
    };
    char::from_u32(scalar)
}

fn scalar_from_rank_or_scalar(scalar: u32) -> Option<char> {
    char::from_u32(scalar)
}

fn is_normalized(profile: filament_exact::TextProfile, text: &str) -> bool {
    let Some(form) = profile.normalization() else {
        return true;
    };
    form.apply(text).eq(text.chars())
}

fn normalized_scalar_alphabet(
    profile: filament_exact::TextProfile,
) -> Option<&'static [(char, u8)]> {
    static NFD: OnceLock<Vec<(char, u8)>> = OnceLock::new();
    static NFKD: OnceLock<Vec<(char, u8)>> = OnceLock::new();
    let cache = match profile {
        filament_exact::TextProfile::Nfd => &NFD,
        filament_exact::TextProfile::Nfkd => &NFKD,
        _ => return None,
    };
    Some(cache.get_or_init(|| {
        (0..1_112_064_u32)
            .filter_map(|rank| {
                let character = scalar_from_rank(rank)?;
                is_normalized(profile, &character.to_string()).then_some((
                    character,
                    unicode_normalization::char::canonical_combining_class(character),
                ))
            })
            .collect()
    }))
}

type DecomposedCounts = Vec<Vec<Integer>>;
type ComposedCounts = Vec<Vec<Vec<Integer>>>;
type CountCache<T> = OnceLock<Mutex<BTreeMap<usize, Arc<T>>>>;

fn nfd_suffix_counts(
    profile: filament_exact::TextProfile,
    max: u64,
) -> Option<Arc<DecomposedCounts>> {
    static NFD: CountCache<DecomposedCounts> = OnceLock::new();
    static NFKD: CountCache<DecomposedCounts> = OnceLock::new();
    let cache = match profile {
        filament_exact::TextProfile::Nfd => &NFD,
        filament_exact::TextProfile::Nfkd => &NFKD,
        _ => return None,
    };
    let max = usize::try_from(max).ok()?;
    if max <= 4 {
        if let Some(counts) = cache
            .get_or_init(Default::default)
            .lock()
            .ok()?
            .get(&max)
            .cloned()
        {
            return Some(counts);
        }
    }
    let alphabet = normalized_scalar_alphabet(profile)?;
    let mut by_ccc = [0_u64; 256];
    for (_, ccc) in alphabet {
        by_ccc[usize::from(*ccc)] = by_ccc[usize::from(*ccc)].saturating_add(1);
    }
    let mut counts = vec![vec![Integer::zero(); 256]; max.saturating_add(1)];
    counts[0].fill(Integer::one());
    for remaining in 1..=max {
        for previous in 0..=u8::MAX {
            let mut total = Integer::from(by_ccc[0]).mul(&counts[remaining - 1][0]);
            for ccc in 1..=u8::MAX {
                if previous <= ccc {
                    total = total.add(
                        &Integer::from(by_ccc[usize::from(ccc)])
                            .mul(&counts[remaining - 1][usize::from(ccc)]),
                    );
                }
            }
            counts[remaining][usize::from(previous)] = total;
        }
    }
    let counts = Arc::new(counts);
    if max <= 4 {
        cache
            .get_or_init(Default::default)
            .lock()
            .ok()?
            .insert(max, counts.clone());
    }
    Some(counts)
}

struct ComposedTextTables {
    alphabet: Vec<(char, u8, usize)>,
    starters: Vec<char>,
    compositions: BTreeMap<char, Vec<char>>,
    by_ccc: [u64; 256],
    ccc_values: Vec<u8>,
    ccc_indexes: [usize; 256],
    by_starter: Vec<u64>,
}

fn composed_scalar_alphabet(
    profile: filament_exact::TextProfile,
) -> Option<&'static ComposedTextTables> {
    static NFC: OnceLock<ComposedTextTables> = OnceLock::new();
    static NFKC: OnceLock<ComposedTextTables> = OnceLock::new();
    let cache = match profile {
        filament_exact::TextProfile::Nfc => &NFC,
        filament_exact::TextProfile::Nfkc => &NFKC,
        _ => return None,
    };
    Some(cache.get_or_init(|| {
        use unicode_normalization::char::{
            canonical_combining_class, compose, decompose_canonical,
        };

        let alphabet: Vec<(char, u8)> = (0..1_112_064_u32)
            .filter_map(|rank| {
                let character = scalar_from_rank(rank)?;
                is_normalized(profile, &character.to_string())
                    .then_some((character, canonical_combining_class(character)))
            })
            .collect();
        let members: std::collections::BTreeSet<char> =
            alphabet.iter().map(|(character, _)| *character).collect();
        let mut compositions = BTreeMap::<char, Vec<char>>::new();
        for (output, _) in &alphabet {
            let mut decomposition = Vec::new();
            decompose_canonical(*output, |character| decomposition.push(character));
            let Some(mut starter) = decomposition.first().copied() else {
                continue;
            };
            for character in decomposition.into_iter().skip(1) {
                if let Some(composed) = compose(starter, character) {
                    if members.contains(&character) {
                        compositions.entry(starter).or_default().push(character);
                    }
                    starter = composed;
                } else if canonical_combining_class(character) == 0 {
                    starter = character;
                }
            }
        }
        for candidates in compositions.values_mut() {
            candidates.sort_unstable();
            candidates.dedup();
        }
        compositions.retain(|_, candidates| !candidates.is_empty());
        let starters: Vec<char> = compositions.keys().copied().collect();
        let state_of = |character: char| {
            starters
                .binary_search(&character)
                .map_or(0, |index| index + 1)
        };
        let mut by_ccc = [0_u64; 256];
        let mut by_starter = vec![0_u64; starters.len() + 1];
        let alphabet = alphabet
            .into_iter()
            .map(|(character, ccc)| {
                by_ccc[usize::from(ccc)] = by_ccc[usize::from(ccc)].saturating_add(1);
                let next_starter = if ccc == 0 { state_of(character) } else { 0 };
                if ccc == 0 {
                    by_starter[next_starter] = by_starter[next_starter].saturating_add(1);
                }
                (character, ccc, next_starter)
            })
            .collect();
        let ccc_values: Vec<u8> = (0..=u8::MAX)
            .filter(|ccc| by_ccc[usize::from(*ccc)] > 0)
            .collect();
        let mut ccc_indexes = [0_usize; 256];
        for (index, ccc) in ccc_values.iter().copied().enumerate() {
            ccc_indexes[usize::from(ccc)] = index;
        }
        ComposedTextTables {
            alphabet,
            starters,
            compositions,
            by_ccc,
            ccc_values,
            ccc_indexes,
            by_starter,
        }
    }))
}

fn composed_pair_exists(tables: &ComposedTextTables, starter: usize, following: char) -> bool {
    starter > 0
        && tables
            .starters
            .get(starter - 1)
            .and_then(|starter| tables.compositions.get(starter))
            .is_some_and(|candidates| candidates.binary_search(&following).is_ok())
}

fn composed_suffix_counts(
    profile: filament_exact::TextProfile,
    max: u64,
) -> Option<Arc<ComposedCounts>> {
    static NFC: CountCache<ComposedCounts> = OnceLock::new();
    static NFKC: CountCache<ComposedCounts> = OnceLock::new();
    let cache = match profile {
        filament_exact::TextProfile::Nfc => &NFC,
        filament_exact::TextProfile::Nfkc => &NFKC,
        _ => return None,
    };
    let max = usize::try_from(max).ok()?;
    if max <= 4 {
        if let Some(counts) = cache
            .get_or_init(Default::default)
            .lock()
            .ok()?
            .get(&max)
            .cloned()
        {
            return Some(counts);
        }
    }
    let tables = composed_scalar_alphabet(profile)?;
    let states = tables.starters.len() + 1;
    let mut counts =
        vec![vec![vec![Integer::zero(); tables.ccc_values.len()]; states]; max.saturating_add(1)];
    for starter in &mut counts[0] {
        starter.fill(Integer::one());
    }
    for remaining in 1..=max {
        let mut zero_total = Integer::zero();
        for (next_starter, amount) in tables.by_starter.iter().enumerate() {
            zero_total = zero_total
                .add(&Integer::from(*amount).mul(&counts[remaining - 1][next_starter][0]));
        }
        for starter in 0..states {
            for (previous_index, previous_ccc) in tables.ccc_values.iter().copied().enumerate() {
                let mut total = zero_total.clone();
                if previous_ccc == 0 && starter > 0 {
                    let character = tables.starters[starter - 1];
                    if let Some(composing) = tables.compositions.get(&character) {
                        for following in composing {
                            if unicode_normalization::char::canonical_combining_class(*following)
                                == 0
                            {
                                let following_starter = tables
                                    .starters
                                    .binary_search(following)
                                    .map_or(0, |index| index + 1);
                                let ccc_zero = tables.ccc_indexes[0];
                                total =
                                    total.sub(&counts[remaining - 1][following_starter][ccc_zero]);
                            }
                        }
                    }
                }
                for ccc in tables.ccc_values.iter().copied().filter(|ccc| *ccc > 0) {
                    if previous_ccc > ccc || tables.by_ccc[usize::from(ccc)] == 0 {
                        continue;
                    }
                    let mut allowed = tables.by_ccc[usize::from(ccc)];
                    if previous_ccc < ccc && starter > 0 {
                        let character = tables.starters[starter - 1];
                        if let Some(composing) = tables.compositions.get(&character) {
                            let composed = composing
                                .iter()
                                .filter(|following| {
                                    unicode_normalization::char::canonical_combining_class(
                                        **following,
                                    ) == ccc
                                })
                                .count() as u64;
                            allowed = allowed.saturating_sub(composed);
                        }
                    }
                    total = total.add(&Integer::from(allowed).mul(
                        &counts[remaining - 1][starter][tables.ccc_indexes[usize::from(ccc)]],
                    ));
                }
                counts[remaining][starter][previous_index] = total;
            }
        }
    }
    let counts = Arc::new(counts);
    if max <= 4 {
        cache
            .get_or_init(Default::default)
            .lock()
            .ok()?
            .insert(max, counts.clone());
    }
    Some(counts)
}

pub(super) fn admitted_text_value(text_type: &TextType, text: &str) -> Option<Value> {
    use filament_exact::{admit_text, Meter, Outcome, TextPayload};
    let payload = TextPayload::from_utf8(text.as_bytes()).ok()?;
    let unlimited = filament_exact::ScalarLimits {
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
    };
    match admit_text(&payload, text_type, &mut Meter::new(unlimited)) {
        Outcome::Completed(value) => Some(Value::Text(value)),
        Outcome::Undefined(_) | Outcome::Refused(_) | Outcome::Incomplete(_) => None,
    }
}

fn size_integer_nth(bits: u8, index: u128) -> Option<Value> {
    if index == 0 {
        return Some(Value::Integer(Integer::from(0_i64)));
    }
    let level = u8::try_from(127_u32.checked_sub((index + 1).leading_zeros())?).ok()?;
    if level == 0 || level > bits {
        return None;
    }
    let start = (1_u128.checked_shl(u32::from(level))?).checked_sub(1)?;
    let offset = index.checked_sub(start)?;
    let half = 1_u128.checked_shl(u32::from(level - 1))?;
    let magnitude = if offset < half {
        (half.checked_mul(2)?).checked_sub(1)?.checked_sub(offset)?
    } else {
        half.checked_add(offset.checked_sub(half)?)?
    };
    let magnitude = Integer::from(u64::try_from(magnitude).ok()?);
    let value = if offset < half {
        magnitude.mul(&Integer::from(-1_i64))
    } else {
        magnitude
    };
    Some(Value::Integer(value))
}

/// The number of maps of `min` to `max` entries over `keys` possible keys and
/// `values` possible values: the sum over entry counts `k` of
/// C(`keys`, `k`) * `values`^`k`, saturating at `u128::MAX`. A map holds at
/// most one entry for each key, so the terms above `keys` are 0 and the sum
/// covers `min` to `min(max, keys)` only: a term outside the declared range
/// is never computed, so a binomial that overflows below `min` does not
/// saturate the sum. Each term is exact or saturates, and a saturated term
/// saturates the sum, so a declared bound as wide as `u64::MAX` costs no
/// more than the terms up to the first that no longer fits.
fn entries_size(keys: u128, values: u128, min: u64, max: u64) -> u128 {
    if values == 0 {
        // Only the empty map exists.
        return u128::from(min == 0);
    }
    let top = max.min(u64::try_from(keys).unwrap_or(u64::MAX));
    let mut total = 0u128;
    for count in min..=top {
        let term = combinations(keys, u128::from(count), false)
            .saturating_mul(values.saturating_pow(u32::try_from(count).unwrap_or(u32::MAX)));
        total = total.saturating_add(term);
        if total == u128::MAX {
            break;
        }
    }
    total
}

/// The number of `length`-element sequences of distinct values drawn from
/// `base` values, P(`base`, `length`), saturating at `u128::MAX`; 0 when
/// `length` exceeds `base`.
fn arrangements(base: u128, length: u128) -> u128 {
    if length > base {
        return 0;
    }
    (0..length)
        .try_fold(1u128, |total, step| total.checked_mul(base - step))
        .unwrap_or(u128::MAX)
}

/// The number of `count`-element subsets of `base` values, or of
/// `count`-element multisets when `repeat`, saturating at `u128::MAX`.
fn combinations(base: u128, count: u128, repeat: bool) -> u128 {
    let pool = if repeat {
        match (base + count).checked_sub(1) {
            Some(pool) => pool,
            None => return u128::from(count == 0),
        }
    } else {
        base
    };
    if count > pool {
        return 0;
    }
    let count = count.min(pool - count);
    let mut total = 1u128;
    for step in 0..count {
        // Exact at every step: the product of `step + 1` consecutive
        // integers is divisible by `(step + 1)!`.
        total = match total.checked_mul(pool - step) {
            Some(product) => product / (step + 1),
            None => return u128::MAX,
        };
    }
    total
}

/// Every combination of digits `0..radix` for a list of radices, the last
/// digit moving fastest, produced one at a time. No radices is one empty
/// combination; a zero radix is none.
pub(super) struct Odometer {
    radices: Vec<u128>,
    digits: Vec<u128>,
    started: bool,
    exhausted: bool,
}

impl Odometer {
    /// An odometer over `radices`.
    pub(super) fn new(radices: Vec<u128>) -> Self {
        Self {
            digits: vec![0; radices.len()],
            radices,
            started: false,
            exhausted: false,
        }
    }

    /// The next combination, or `None` once every one has been produced.
    pub(super) fn next_digits(&mut self) -> Option<Vec<u128>> {
        if self.exhausted {
            return None;
        }
        if !self.started {
            self.started = true;
            if self.radices.contains(&0) {
                self.exhausted = true;
                return None;
            }
            return Some(self.digits.clone());
        }
        let mut position = self.digits.len();
        loop {
            let Some(previous) = position.checked_sub(1) else {
                self.exhausted = true;
                return None;
            };
            position = previous;
            self.digits[position] += 1;
            if self.digits[position] < self.radices[position] {
                return Some(self.digits.clone());
            }
            self.digits[position] = 0;
        }
    }
}

/// `base` to the `exponent`, saturating at `u128::MAX`.
fn power(base: u128, exponent: u64) -> u128 {
    (0..exponent).fold(1u128, |product, _| product.saturating_mul(base))
}

#[cfg(test)]
mod tests {
    use filament_exact::{CardinalityBound, IntegerInterval};
    use ix_trace_rs::trace;
    use unicode_normalization::UnicodeNormalization;

    use super::*;

    fn types() -> TypeEnvironment {
        TypeEnvironment::new([], []).expect("an empty environment admits")
    }

    fn int(lower: i64, upper: i64) -> ValueType {
        ValueType::Int(
            IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("nonempty"),
        )
    }

    fn integer(value: &Value) -> String {
        match value {
            Value::Integer(integer) => integer.to_string(),
            other => panic!("expected an integer, got {other:?}"),
        }
    }

    /// Timed declarations alone cannot authenticate generated clock references;
    /// ordinary object references still enumerate their actual owners.
    #[trace("TC-3084", "FR-1040-AC-8")]
    #[test]
    fn legacy_timed_context_does_not_enumerate_clock_reference_domains() {
        use filament_semantic_value::checking::{CheckMode, CheckingLimits};
        use fsl_semantics::model::accounting::ModelNormalizationLimits;
        use fsl_semantics::model::intake::{admit_unit, package_input};
        use fsl_semantics::model::state::StateModel;

        let source = "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"filament.model.complete/v1\";\n\
            profile t = \"filament.temporal.complete/v1\";\n\
            dimension Time; unit ms : Time = rational(1, 1); time Call dense unit ms;\n\
            object Call { q: Sequence<Clock>[0, 2]; x: Clock; }\n";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "clock-reference-domain"),
            "clock-domain.native",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source parses");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("forms build");
        let packages =
            package_input(std::iter::empty::<&[u8]>()).expect("source-only package input");
        let models = admit_unit(
            &unit.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("model selections");
        let graph = fsl_semantics::check::PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "clock-reference-domain".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("declarations assemble")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("source clock model checks: {refusals:?}"));
        let [timed] = graph.timed_models() else {
            panic!("one checked timed model")
        };
        let templates: Vec<_> = timed.clocks().iter().zip(timed.place_nodes()).collect();
        let [(collection, _), (scalar, _)] = templates.as_slice() else {
            panic!("q and x templates")
        };
        assert_eq!(
            collection
                .slots()
                .map(|shape| shape.max)
                .collect::<Vec<_>>(),
            [Some(2)]
        );
        assert_eq!(scalar.slots().count(), 0);
        let model = StateModel::new(
            graph.model_inputs(),
            graph.scope().types(),
            &packages,
            ModelNormalizationLimits::default(),
        )
        .expect("checked source model");
        let owner_type = model
            .type_key(collection.owner())
            .expect("declared owner type");
        let c1 = model
            .object_reference(owner_type, "c1")
            .expect("first owner");
        let c2 = model
            .object_reference(owner_type, "c2")
            .expect("second owner");
        let references = [c1.clone(), c2.clone()];
        let target = graph
            .scope()
            .types()
            .object_types()
            .find(|declaration| {
                fsl_semantics::model::clock_reference::is_clock_type(&declaration.key())
            })
            .expect("checked native clock reference target")
            .key();
        let value_type = ValueType::Reference(target);
        assert!(
            Domain::of_bounded(
                &value_type,
                &references,
                graph.scope().types(),
                &RecordOrders::new(),
                None,
            )
            .is_none(),
            "checked templates and real owners do not supply original population provenance"
        );
        let ordinary = Domain::of(
            &ValueType::Reference(collection.owner()),
            &references,
            graph.scope().types(),
            &RecordOrders::new(),
        )
        .expect("ordinary references remain enumerable");
        assert_eq!(ordinary.len(), 2);
        for (index, expected) in [(0, c1), (1, c2)] {
            let Some(Value::Reference(actual)) = ordinary.nth(index) else {
                panic!("the ordinary reference domain retains both real owners");
            };
            assert_eq!(actual, expected);
        }
        assert!(ordinary.nth(2).is_none());
    }

    /// An unauthenticated clock leaf cannot leave an empty or partial domain
    /// through optional values, collections, records, tuples or union cases.
    #[trace("TC-3084", "FR-1040-AC-8")]
    #[test]
    fn unauthenticated_clock_reference_domains_refuse_nested_carriers() {
        use filament_semantic_value::declaration::{FieldDeclaration, ObjectTypeDeclaration};

        let target = filament_exact::EffectiveId::new(
            None,
            filament_exact::OriginalDeclarationKey::new("filament/native", "Clock"),
        );
        let clock = ValueType::Reference(target.clone());
        let record = crate::test_ids::node_key([21]);
        let tuple = crate::test_ids::node_key([22]);
        let union = crate::test_ids::node_key([23]);
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    record.clone(),
                    "Record",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("clock", clock.clone(), Presence::Optional),
                        FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required),
                    ]),
                ),
                CompositeDeclaration::new(
                    tuple.clone(),
                    "Tuple",
                    CompositeShape::Tuple(vec![ValueType::Boolean, clock.clone()]),
                ),
                CompositeDeclaration::new(
                    union.clone(),
                    "Choice",
                    CompositeShape::Union(vec![
                        UnionMemberDeclaration::from_verified(
                            UnionMember::from_admitted(union.clone(), "Flag").expect("case name"),
                            vec![ValueType::Boolean],
                        ),
                        UnionMemberDeclaration::from_verified(
                            UnionMember::from_admitted(union.clone(), "Clock").expect("case name"),
                            vec![clock.clone()],
                        ),
                    ]),
                ),
            ],
            [ObjectTypeDeclaration::new(
                target,
                "ClockReference",
                Vec::new(),
            )],
        )
        .expect("carrier declarations admit");
        let sequence = ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            clock.clone(),
            Some(CardinalityBound::new(0, 2).expect("bounded collection")),
        ));
        for value_type in [
            clock.clone(),
            ValueType::option(clock),
            ValueType::option(sequence.clone()),
            sequence,
            ValueType::Composite(record),
            ValueType::Composite(tuple),
            ValueType::Composite(union),
        ] {
            assert!(
                Domain::of(&value_type, &[], &types, &RecordOrders::new()).is_none(),
                "no authenticated clock domain for {value_type:?}"
            );
        }
    }

    /// FR-120 "Applications": an `Int[lower, upper]` domain is every integer
    /// in the range, ascending, and `Option<T>` is absent then each value.
    #[trace("TC-471", "FR-120-AC-2")]
    #[test]
    fn integers_ascend_and_an_option_starts_absent() {
        let domain = Domain::of(&int(-1, 1), &[], &types(), &RecordOrders::new())
            .expect("Int is enumerable");
        let values: Vec<String> = (0..domain.len())
            .map(|index| integer(&domain.nth(index).expect("in range")))
            .collect();
        assert_eq!(values, ["-1", "0", "1"]);
        assert!(domain.nth(3).is_none());

        let option = Domain::of(
            &ValueType::option(int(0, 1)),
            &[],
            &types(),
            &RecordOrders::new(),
        )
        .expect("enumerable");
        assert_eq!(option.len(), 3);
        let Some(Value::Option(none)) = option.nth(0) else {
            panic!("index 0 is an option")
        };
        assert!(none.payload().is_none());
    }

    /// FSpec FR-190's size domain is ordered least-magnitude first and has
    /// exactly `2^(bits + 1) - 1` signed integers.
    #[trace("FSpec-TC-1600", "FSpec-FR-190-AC-9")]
    #[test]
    fn size_bounded_integer_uses_the_pinned_size_order() {
        let domain = Domain::of_bounded(
            &ValueType::Integer,
            &[],
            &types(),
            &RecordOrders::new(),
            Some(3),
        )
        .expect("max_value_bits bounds Integer");
        let values: Vec<String> = (0..domain.len())
            .map(|index| integer(&domain.nth(index).expect("in range")))
            .collect();
        assert_eq!(
            values,
            ["0", "-1", "1", "-3", "-2", "2", "3", "-7", "-6", "-5", "-4", "4", "5", "6", "7"]
        );
        assert_eq!(domain.len(), 15);
        assert!(domain.nth(15).is_none());
    }

    /// FR-120: a sequence with a declared maximum ranges over every length
    /// from its minimum to its maximum; with `Int[0, 1]` elements and bound
    /// `[0, 2]` that is 1 + 2 + 4 sequences.
    #[trace("TC-471", "FR-120-AC-2")]
    #[test]
    fn a_bounded_sequence_ranges_over_every_length() {
        let sequence = ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            int(0, 1),
            Some(CardinalityBound::new(0, 2).expect("bound")),
        ));
        let domain =
            Domain::of(&sequence, &[], &types(), &RecordOrders::new()).expect("enumerable");
        assert_eq!(domain.len(), 7);
        let lengths: Vec<usize> = (0..7)
            .map(|index| match domain.nth(index) {
                Some(Value::Collection(collection)) => collection.elements().len(),
                other => panic!("expected a sequence, got {other:?}"),
            })
            .collect();
        assert_eq!(lengths, [0, 1, 1, 2, 2, 2, 2]);
    }

    /// The odometer visits every combination once, last digit fastest.
    #[trace("TC-471", "FR-120-AC-3")]
    #[test]
    fn an_odometer_visits_every_combination_once() {
        let mut odometer = Odometer::new(vec![2, 3]);
        let mut seen = Vec::new();
        while let Some(digits) = odometer.next_digits() {
            seen.push(digits);
        }
        assert_eq!(seen.len(), 6);
        assert_eq!(seen[0], [0, 0]);
        assert_eq!(seen[1], [0, 1]);
        assert_eq!(seen[5], [1, 2]);
        assert_eq!(Odometer::new(vec![]).next_digits(), Some(vec![]));
        assert_eq!(Odometer::new(vec![2, 0]).next_digits(), None);
    }

    /// Every other type is unbounded here.
    #[trace("TC-472", "FR-120-AC-7")]
    #[test]
    fn an_unenumerable_type_has_no_domain() {
        assert!(!enumerable(&ValueType::Integer));
        assert!(Domain::of(&ValueType::String, &[], &types(), &RecordOrders::new()).is_none());
    }

    /// The type environment of `Inner { e: Boolean, o?: Boolean }` and
    /// `Outer { inner: Inner, flag: Boolean }`.
    fn nested() -> (TypeEnvironment, NodeKey, NodeKey) {
        use filament_semantic_value::declaration::FieldDeclaration;
        let inner = crate::test_ids::node_key([2]);
        let outer = crate::test_ids::node_key([3]);
        let declarations = [
            CompositeDeclaration::new(
                inner.clone(),
                "Inner",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("e", ValueType::Boolean, Presence::Required),
                    FieldDeclaration::new("o", ValueType::Boolean, Presence::Optional),
                ]),
            ),
            CompositeDeclaration::new(
                outer.clone(),
                "Outer",
                CompositeShape::Record(vec![
                    FieldDeclaration::new(
                        "inner",
                        ValueType::Composite(inner.clone()),
                        Presence::Required,
                    ),
                    FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required),
                ]),
            ),
        ];
        (
            TypeEnvironment::new(declarations, []).expect("the declarations admit"),
            inner,
            outer,
        )
    }

    /// ADR-027 ST-11: the records of an attempt type range over its whole
    /// finite domain: an optional field (absent first) and a nested record of
    /// finite fields enumerate, each combination once.
    #[test]
    fn an_attempt_type_of_finite_fields_enumerates_every_record() {
        let (types, inner, outer) = nested();
        let mut charged = 0;
        let values = |declaration, charged: &mut u32| {
            composite_values::<()>(
                &ValueType::Composite(declaration),
                &[],
                &types,
                &RecordOrders::new(),
                || {
                    *charged += 1;
                    Ok(())
                },
            )
            .expect("no stop")
            .expect("a finite domain")
        };
        assert_eq!(values(inner, &mut charged).len(), 6);
        assert_eq!(values(outer.clone(), &mut charged).len(), 12);
        assert_eq!(charged, 18);
        let all = values(outer, &mut charged);
        let distinct: std::collections::BTreeSet<String> = all
            .iter()
            .map(|value| {
                format!(
                    "{:?}",
                    super::super::encode::typed_value_enc(value, &types, &[])
                )
            })
            .collect();
        assert_eq!(distinct.len(), 12);
    }

    /// ADR-027 ST-11: a union over finite members enumerates every member
    /// with every combination of its payload, in declaration order, and is
    /// written as FR-120's union form.
    #[test]
    fn a_union_attempt_type_enumerates_every_member() {
        use filament_exact::UnionMember;
        use filament_semantic_value::declaration::UnionMemberDeclaration;
        let union = crate::test_ids::node_key([5]);
        let member = |name: &str, _key: u8, positions: Vec<ValueType>| {
            UnionMemberDeclaration::from_verified(
                UnionMember::from_admitted(union.clone(), name).expect("a member name"),
                positions,
            )
        };
        let declaration = CompositeDeclaration::new(
            union.clone(),
            "Choice",
            CompositeShape::Union(vec![
                member("Flag", 6, vec![ValueType::Boolean]),
                member("Pair", 7, vec![ValueType::Boolean, int(0, 1)]),
                member("Nothing", 8, vec![]),
            ]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        assert!(enumerable_in(&ValueType::Composite(union.clone()), &types));
        let values = composite_values::<()>(
            &ValueType::Composite(union),
            &[],
            &types,
            &RecordOrders::new(),
            || Ok(()),
        )
        .expect("no stop")
        .expect("a finite domain");
        assert_eq!(values.len(), 2 + 4 + 1);
        let written = quire_canonical::to_vec(
            &super::super::encode::typed_value_enc(&values[1], &types, &[]),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("encodes");
        assert_eq!(
            String::from_utf8(written).expect("UTF-8"),
            r#"{"components":[{"type":"boolean","value":true}],"member":"Flag","name":"{\"node\":\"[5]\",\"package\":\"fixture\"}","type":"union"}"#
        );
    }

    /// FR-120 "Applications": a model-variant union's cases are ordered by
    /// ascending UTF-8 bytes of their names, whatever their declared order:
    /// `Zed` declared before `Alpha` still lists `Alpha` first.
    #[trace("TC-1270")]
    #[test]
    fn a_variant_union_orders_its_cases_by_name() {
        use filament_exact::UnionMember;
        use filament_semantic_value::declaration::UnionMemberDeclaration;
        let union = crate::test_ids::node_key([12]);
        let case = |_byte: u8, name: &str| {
            UnionMember::from_admitted(union.clone(), name).expect("a member name")
        };
        let (zed, alpha) = (case(13, "Zed"), case(14, "Alpha"));
        let declaration = CompositeDeclaration::new(
            union.clone(),
            "Ordered",
            CompositeShape::Union(vec![
                UnionMemberDeclaration::from_verified(zed.clone(), vec![]),
                UnionMemberDeclaration::from_verified(alpha.clone(), vec![]),
            ]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        let domain = Domain::of(
            &ValueType::Composite(union),
            &[],
            &types,
            &RecordOrders::new(),
        )
        .expect("a finite domain");
        let names: Vec<String> = (0..domain.len())
            .map(|index| match domain.nth(index) {
                Some(Value::Union(value)) => value.member().identifier().to_owned(),
                other => panic!("a union value: {other:?}"),
            })
            .collect();
        assert_eq!(names, ["Alpha", "Zed"]);
    }

    /// FR-120-AC-25 / TC-1270: an admitted model variant union adds the
    /// nullary case's one value to each value of a finite Boolean payload.
    #[trace("TC-1270", "FR-120-AC-25")]
    #[test]
    fn a_variant_union_adds_nullary_and_payload_case_domains() {
        use filament_exact::UnionMember;
        use filament_semantic_value::declaration::UnionMemberDeclaration;
        let union = crate::test_ids::node_key([9]);
        let empty = UnionMember::from_admitted(union.clone(), "Empty").expect("a member name");
        let enabled = UnionMember::from_admitted(union.clone(), "Enabled").expect("a member name");
        let declaration = CompositeDeclaration::new(
            union.clone(),
            "Choice",
            CompositeShape::Union(vec![
                UnionMemberDeclaration::from_verified(empty.clone(), vec![]),
                UnionMemberDeclaration::from_verified(enabled.clone(), vec![ValueType::Boolean]),
            ]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        assert!(enumerable_in(&ValueType::Composite(union.clone()), &types));
        let values = composite_values::<()>(
            &ValueType::Composite(union),
            &[],
            &types,
            &RecordOrders::new(),
            || Ok(()),
        )
        .expect("no stop")
        .expect("the union has a finite domain");
        assert_eq!(values.len(), 3);
        let Value::Union(empty_value) = &values[0] else {
            panic!("the nullary case is first")
        };
        assert_eq!(empty_value.member().variant(), empty.variant());
        assert!(empty_value.payload().is_empty());
        for (index, expected) in [(1, false), (2, true)] {
            let Value::Union(enabled_value) = &values[index] else {
                panic!("the payload case follows the nullary case")
            };
            assert_eq!(enabled_value.member().variant(), enabled.variant());
            assert!(
                matches!(enabled_value.payload(), [Value::Boolean(value)] if *value == expected)
            );
        }
    }

    /// FR-120-AC-22 / TC-1270: a tuple's finite domain is the ordered
    /// Cartesian product of its positions, with the first position most
    /// significant.
    #[trace("TC-1270", "FR-120-AC-22")]
    #[test]
    fn a_tuple_value_domain_is_the_ordered_product_of_its_positions() {
        let tuple = crate::test_ids::node_key([12]);
        let declaration = CompositeDeclaration::new(
            tuple.clone(),
            "Pair",
            CompositeShape::Tuple(vec![ValueType::Boolean, int(0, 1)]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        assert!(enumerable_in(&ValueType::Composite(tuple.clone()), &types));
        let values = composite_values::<()>(
            &ValueType::Composite(tuple),
            &[],
            &types,
            &RecordOrders::new(),
            || Ok(()),
        )
        .expect("no stop")
        .expect("both positions have finite domains");
        assert_eq!(values.len(), 4);
        let rendered = values
            .iter()
            .map(|value| {
                let Value::Composite(tuple_value) = value else {
                    panic!("the tuple declaration yields tuple values")
                };
                let [
                    FieldValue::Present(Value::Boolean(first)),
                    FieldValue::Present(Value::Integer(second)),
                ] = tuple_value.slots()
                else {
                    panic!("both tuple positions retain their declared order")
                };
                (*first, second.to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rendered,
            [
                (false, "0".into()),
                (false, "1".into()),
                (true, "0".into()),
                (true, "1".into())
            ]
        );
    }

    /// FR-120's record form, written exactly: `type`, the DeclarationKey's
    /// RFC 8785 text as `name`, and one `{name, value}` entry per declared
    /// field with an absent optional field `null`.
    #[test]
    fn a_record_value_is_written_in_fr_120_s_record_form() {
        let (types, inner, _) = nested();
        let first = composite_values::<()>(
            &ValueType::Composite(inner.clone()),
            &[],
            &types,
            &RecordOrders::new(),
            || Ok(()),
        )
        .expect("no stop")
        .expect("a finite domain")
        .remove(0);
        let written = quire_canonical::to_vec(
            &super::super::encode::typed_value_enc(&first, &types, &[]),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("encodes");
        assert_eq!(
            String::from_utf8(written).expect("UTF-8"),
            r#"{"fields":[{"name":"e","value":{"type":"boolean","value":false}},{"name":"o","value":null}],"name":"{\"node\":\"[2]\",\"package\":\"fixture\"}","type":"record"}"#
        );
    }

    /// FR-120-AC-28: an `OrderedSet<E>[0, N]` is every sequence of up to `N`
    /// distinct elements, by length and then lexicographically: 1 + 2 + 2
    /// for `Boolean` and `[0, 2]`, and 1 + 3 + 6 + 6 = 16 for `Int[0, 2]`
    /// with `[0, 3]` and with `[0, 5]`.
    #[trace("TC-1270", "FR-120-AC-28")]
    #[test]
    fn a_bounded_ordered_set_ranges_over_distinct_arrangements() {
        let ordered = |element: ValueType, max: u64| {
            ValueType::collection(CollectionType::new(
                CollectionKind::OrderedSet,
                element,
                Some(CardinalityBound::new(0, max).expect("bound")),
            ))
        };
        let render = |value_type: &ValueType| -> Vec<String> {
            let domain = Domain::of(value_type, &[], &types(), &RecordOrders::new())
                .expect("a finite domain");
            let values: Vec<String> = (0..domain.len())
                .map(|index| format!("{:?}", domain.nth(index).expect("in range")))
                .collect();
            assert!(domain.nth(domain.len()).is_none());
            values
        };
        let booleans = render(&ordered(ValueType::Boolean, 2));
        assert_eq!(booleans.len(), 5);
        let flags = |value: &str| -> Vec<bool> {
            value
                .match_indices("Boolean(")
                .map(|(at, _)| value[at + 8..].starts_with("true"))
                .collect()
        };
        let shapes: Vec<Vec<bool>> = booleans.iter().map(|value| flags(value)).collect();
        assert_eq!(
            shapes,
            [
                vec![],
                vec![false],
                vec![true],
                vec![false, true],
                vec![true, false]
            ]
        );
        let small = render(&ordered(int(0, 2), 3));
        assert_eq!(small.len(), 16);
        let digits = |value: &str| -> String {
            value
                .match_indices("Integer(Integer(")
                .map(|(at, _)| value[at + 16..].chars().next().unwrap_or('?'))
                .collect()
        };
        let order: Vec<String> = small.iter().map(|value| digits(value)).collect();
        let wide: Vec<String> = render(&ordered(int(0, 2), 5))
            .iter()
            .map(|value| digits(value))
            .collect();
        assert_eq!(wide, order);
        assert_eq!(
            order,
            [
                "", "0", "1", "2", "01", "02", "10", "12", "20", "21", "012", "021", "102", "120",
                "201", "210"
            ]
        );
    }

    /// FR-120-AC-28: an `OrderedSet` whose size does not fit in 128 bits
    /// saturates at `u128::MAX`, which exceeds every limit, rather than
    /// wrapping to a small size.
    #[trace("TC-1270", "FR-120-AC-28")]
    #[test]
    fn an_overflowing_ordered_set_size_saturates() {
        let huge = Integer::from(u64::MAX).add(&Integer::from(u64::MAX));
        let element =
            ValueType::Int(IntegerInterval::new(Integer::from(0_i64), huge).expect("nonempty"));
        let ordered = ValueType::collection(CollectionType::new(
            CollectionKind::OrderedSet,
            element,
            Some(CardinalityBound::new(0, 4).expect("bound")),
        ));
        let domain =
            Domain::of(&ordered, &[], &types(), &RecordOrders::new()).expect("a finite domain");
        assert_eq!(domain.len(), u128::MAX);
        // Within 128 bits the sum is exact: 1 + 3 + 3 * 2 + 3 * 2 * 1.
        let small = ValueType::collection(CollectionType::new(
            CollectionKind::OrderedSet,
            int(0, 2),
            Some(CardinalityBound::new(0, 4).expect("bound")),
        ));
        let exact = Domain::of(&small, &[], &types(), &RecordOrders::new()).expect("finite");
        assert_eq!(exact.len(), 16);
    }

    /// FR-120-AC-28: a `Map<K, V>[min, max]` holds the sum over entry counts
    /// `k` of C(`|K|`, `k`) * `|V|`^`k` values, counted without enumeration
    /// and saturating rather than wrapping.
    #[trace("TC-1270", "FR-120-AC-28")]
    #[test]
    fn a_bounded_map_counts_its_entries_by_key_subsets_and_values() {
        let map = |keys: ValueType, values: ValueType, min: u64, max: u64| {
            ValueType::map(
                filament_exact::MapType::new(
                    keys,
                    values,
                    Some(CardinalityBound::new(min, max).expect("ordered")),
                )
                .expect("a leaf key"),
            )
        };
        let size = |value: &ValueType| Domain::map_size(value, &[], &types(), &RecordOrders::new());
        // |K| = 3, |V| = 2: 1 + 3*2 + 3*4 = 19.
        assert_eq!(size(&map(int(0, 2), ValueType::Boolean, 0, 2)), Some(19));
        // A lower bound drops the smaller entry counts: 6 + 12.
        assert_eq!(size(&map(int(0, 2), ValueType::Boolean, 1, 2)), Some(18));
        // More entries than keys add nothing.
        assert_eq!(size(&map(int(0, 1), ValueType::Boolean, 0, 5)), Some(9));
        // A key or value with no finite domain, or an unbounded map, has no size.
        assert_eq!(
            size(&map(ValueType::String, ValueType::Boolean, 0, 2)),
            None
        );
        assert_eq!(size(&ValueType::Boolean), None);
        // Overflow saturates.
        let wide = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(u64::MAX)).expect("nonempty"),
        );
        assert_eq!(size(&map(wide.clone(), wide, 0, 40)), Some(u128::MAX));
        // A declared bound far above the key count costs no more than the keys:
        // 1 + 2*2 + 1*4 = 9 for two Boolean keys and two values.
        let started = std::time::Instant::now();
        for upper in [10_000_000, u64::MAX] {
            assert_eq!(
                size(&map(ValueType::Boolean, ValueType::Boolean, 0, upper)),
                Some(9)
            );
        }
        let wide_keys = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(9_999_999_i64))
                .expect("nonempty"),
        );
        assert_eq!(
            size(&map(wide_keys, ValueType::Boolean, 0, u64::MAX)),
            Some(u128::MAX)
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        // A bound that excludes the large terms does not saturate: 200 keys
        // and one value held to exactly 200 entries is one map, and a lower
        // bound above the key count leaves none.
        let keys200 = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(199_i64)).expect("nonempty"),
        );
        let one_value = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(0_i64)).expect("nonempty"),
        );
        assert_eq!(
            size(&map(keys200.clone(), one_value.clone(), 200, 200)),
            Some(1)
        );
        assert_eq!(size(&map(keys200, one_value, 201, 300)), Some(0));
    }

    /// FR-190-AC-18: the unranked Text domains use exact profile lengths and
    /// FR-804 prefix-first scalar or UTF-8 byte order.
    #[trace("FSpec-TC-1600", "FSpec-FR-190-AC-18")]
    #[test]
    fn text_domains_unrank_unicode_and_binary_utf8_prefixes() {
        let text = |max, profile| {
            Domain::of(
                &ValueType::Text(TextType::new(0, max, profile).expect("Text bounds")),
                &[],
                &types(),
                &RecordOrders::new(),
            )
            .expect("finite Text domain")
        };
        let unicode = text(2, filament_exact::TextProfile::UnicodeScalars);
        assert_eq!(unicode.len(), 1_236_687_452_161);
        let text_at = |domain: &Domain, index| match domain.nth(index) {
            Some(Value::Text(text)) => text.retained().to_owned(),
            value => panic!("expected Text at {index}, got {value:?}"),
        };
        assert_eq!(text_at(&unicode, 0), "");
        assert_eq!(text_at(&unicode, 1), "\0");
        assert_eq!(text_at(&unicode, 2), "\0\0");
        assert_eq!(text_at(&unicode, 3), "\0\u{1}");
        assert_eq!(text_at(&unicode, 1_112_066), "\u{1}");
        assert_eq!(text_at(&unicode, 61_492_746_241), "\u{e000}");
        assert_eq!(text_at(&unicode, 1_236_687_452_160), "\u{10ffff}\u{10ffff}");
        assert_eq!(text_at(&unicode, 968_414_557_300), "\u{d51a9}\u{87ac9}");

        let binary = text(3, filament_exact::TextProfile::BinaryUtf8);
        assert_eq!(binary.len(), 2_668_545);
        assert_eq!(text_at(&binary, 18_434), "\u{1}");
        assert_eq!(text_at(&binary, 2_359_425), "\u{80}");
        assert_eq!(text_at(&binary, 2_607_105), "\u{800}");
        assert_eq!(text_at(&binary, 2_668_544), "\u{ffff}");

        let single = |profile| {
            Domain::of(
                &ValueType::Text(TextType::new(0, 1, profile).expect("Text bounds")),
                &[],
                &types(),
                &RecordOrders::new(),
            )
            .expect("finite one-scalar Text domain")
        };
        assert_eq!(
            text_at(&single(filament_exact::TextProfile::UnicodeScalars), 193),
            "\u{c0}"
        );
        assert_eq!(
            text_at(&single(filament_exact::TextProfile::Nfc), 193),
            "\u{c0}"
        );
        assert_eq!(
            text_at(&single(filament_exact::TextProfile::Nfd), 193),
            "\u{c6}"
        );
        assert_eq!(
            text_at(&single(filament_exact::TextProfile::Nfc), 161),
            "\u{a0}"
        );
        assert_eq!(
            text_at(&single(filament_exact::TextProfile::Nfkc), 161),
            "\u{a1}"
        );

        let nfc = text(2, filament_exact::TextProfile::Nfc);
        for index in [0, 1, 2, 3, 4, nfc.len() / 2, nfc.len() - 1] {
            let value = text_at(&nfc, index);
            assert!(value.chars().eq(value.chars().nfc()));
        }
    }
}
