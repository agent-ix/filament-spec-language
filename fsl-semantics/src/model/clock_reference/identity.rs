// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230's place/v2 grammar. Declaration authentication belongs to the
//! checked-source and admitted-package adapters, not to decoded strings.

use quire_canonical::{Encode, Limits, Node, NodeRef, Sink, Writer};

/// A canonical nonnegative decimal string. No JSON number conversion occurs.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Decimal(String);

impl Decimal {
    /// Validate an exact decimal spelling, without imposing an integer ceiling.
    pub fn new(text: &str) -> Result<Self, IdentityError> {
        if text.is_empty()
            || (text.len() > 1 && text.starts_with('0'))
            || !text.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(IdentityError::Malformed);
        }
        Ok(Self(text.to_owned()))
    }

    /// The canonical decimal spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<u64> for Decimal {
    fn from(value: u64) -> Self {
        Self(value.to_string())
    }
}

/// The original declaration scope, selected by provenance rather than route.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeclarationScope {
    /// A declaration retaining its authenticated SourceOwner.
    Source {
        /// Original source authority.
        authority: String,
        /// Original source identity.
        identity: String,
        /// Original type or population declaration URI.
        declaration: String,
    },
    /// An admitted original Model declaration without a SourceOwner.
    Model {
        /// Selected model identity.
        identity: String,
        /// Original type or population declaration URI.
        declaration: String,
    },
}

/// Positional collection kinds; maps deliberately have no identity encoding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CollectionKind {
    /// An ordered sequence with repetitions.
    Sequence,
    /// An unordered set.
    Set,
    /// An unordered collection with repetitions.
    Bag,
    /// An ordered set.
    OrderedSet,
}

impl CollectionKind {
    fn text(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Set => "set",
            Self::Bag => "bag",
            Self::OrderedSet => "ordered-set",
        }
    }

    fn read(text: &str) -> Result<Self, IdentityError> {
        match text {
            "sequence" => Ok(Self::Sequence),
            "set" => Ok(Self::Set),
            "bag" => Ok(Self::Bag),
            "ordered-set" => Ok(Self::OrderedSet),
            _ => Err(IdentityError::Malformed),
        }
    }
}

/// An authenticated declaration's positional collection shape.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CollectionShape {
    /// The positional collection kind.
    pub kind: CollectionKind,
    /// Declared inclusive minimum.
    pub minimum: Decimal,
    /// Declared inclusive maximum, absent when unbounded.
    pub maximum: Option<Decimal>,
}

/// A wrapper on the selected clock carrier. The flat list avoids recursive
/// construction, encoding and dropping for deeply nested declared types.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CarrierLayer {
    /// One semantic optional-presence layer.
    Option,
    /// One positional collection layer.
    Collection(CollectionShape),
}

/// The alias-erased leaf of a selected clock-path carrier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CarrierLeaf {
    /// The fixed validated native Clock leaf.
    Clock,
    /// A named non-alias carrier's original declaration.
    Declared(DeclarationScope),
    /// An anonymous tuple identified by the preceding typed path.
    Tuple,
    /// An anonymous union identified by the preceding typed path.
    Union,
    /// An anonymous record identified by the preceding typed path.
    Record,
}

/// A selected member or payload carrier, outermost wrapper first.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Carrier {
    /// The semantic wrapper layers, in declaration order.
    pub layers: Vec<CarrierLayer>,
    /// The alias-erased carrier leaf.
    pub leaf: CarrierLeaf,
}

/// One entry of the complete normalized typed path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PathEntry {
    /// A field retaining its original declaring scope and member URI.
    Field {
        /// The original declaring object or record scope.
        declaring: DeclarationScope,
        /// The original member URI, after declaration normalization.
        member: String,
        /// The selected declared member carrier.
        carrier: Carrier,
    },
    /// A tuple position, independent of source pN or selected tuple/N spelling.
    Tuple {
        /// Zero-based tuple position.
        position: Decimal,
        /// The selected payload carrier.
        carrier: Carrier,
    },
    /// A named union-case selection.
    Case {
        /// The declared case name.
        name: String,
        /// The selected payload carrier.
        carrier: Carrier,
    },
    /// A positional collection element.
    Collection {
        /// The declared collection shape.
        shape: CollectionShape,
        /// The selected element carrier.
        carrier: Carrier,
    },
    /// One semantic optional-presence layer on the selected path.
    Option,
}

/// A declared template contains no concrete runtime selector or allocation id.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PlaceTemplate {
    /// Original declaring type/model scope.
    pub scope: DeclarationScope,
    /// The complete normalized typed path.
    pub path: Vec<PathEntry>,
}

/// A concrete positional selector tagged with its canonical path position.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SlotIndex {
    /// The position of the collection entry in the canonical typed path.
    pub path_position: Decimal,
    /// The nonnegative concrete positional selector.
    pub index: Decimal,
}

/// Decoded identity components. Their membership and provenance must still be
/// validated against the actual model's declared table and owner context.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PlaceIdentity {
    /// Authenticated original population descriptor P.
    pub population: DeclarationScope,
    /// The owner's local key K.
    pub owner: String,
    /// Original scope S and complete typed path T.
    pub template: PlaceTemplate,
    /// The ordered tagged positional selectors J.
    pub indices: Vec<SlotIndex>,
}

/// A malformed identity or the existing canonical reader/writer's refusal.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// The text or decoded component shape violates the place/v2 grammar.
    #[error("malformed canonical clock-place identity")]
    Malformed,
    /// The shared canonical JSON writer refused, retaining its resource cause.
    #[error(transparent)]
    Canonical(#[from] quire_canonical::Error),
    /// The shared flat JSON reader refused, retaining its resource cause.
    #[error(transparent)]
    Read(#[from] quire_canonical::ReadError),
}

impl PlaceIdentity {
    /// Encode the complete canonical text under the caller's byte ceiling.
    pub fn encode(&self, limits: Limits) -> Result<String, IdentityError> {
        self.validate_indices()?;
        let components = [
            quire_canonical::to_vec(&self.population, limits)?,
            quire_canonical::to_vec(&self.owner, limits)?,
            quire_canonical::to_vec(&self.template.scope, limits)?,
            quire_canonical::to_vec(&Path(&self.template.path), limits)?,
            quire_canonical::to_vec(&Indices(&self.indices), limits)?,
        ];
        let mut text = String::from("place/v2");
        for component in components {
            let required = text
                .len()
                .saturating_add(1)
                .saturating_add(component.len().saturating_mul(2));
            check_bytes(required, limits)?;
            text.push('/');
            for byte in component {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                text.push(char::from(HEX[usize::from(byte >> 4)]));
                text.push(char::from(HEX[usize::from(byte & 15)]));
            }
        }
        Ok(text)
    }

    /// Decode only v2, preserving exact decimal strings and requiring a byte
    /// equal canonical round trip. Declaration admission is a separate check.
    pub fn decode(text: &str, limits: Limits) -> Result<Self, IdentityError> {
        check_bytes(text.len(), limits)?;
        let pieces: Vec<_> = text
            .strip_prefix("place/v2/")
            .ok_or(IdentityError::Malformed)?
            .split('/')
            .collect();
        let [population, owner, scope, path, indices] = pieces.as_slice() else {
            return Err(IdentityError::Malformed);
        };
        let population = read_component(population, limits)?;
        let owner = read_component(owner, limits)?;
        let scope = read_component(scope, limits)?;
        let path = read_component(path, limits)?;
        let indices = read_component(indices, limits)?;
        let identity = Self {
            population: DeclarationScope::read(population.root())?,
            owner: string(owner.root())?.to_owned(),
            template: PlaceTemplate {
                scope: DeclarationScope::read(scope.root())?,
                path: array(path.root())?
                    .into_iter()
                    .map(PathEntry::read)
                    .collect::<Result<_, _>>()?,
            },
            indices: array(indices.root())?
                .into_iter()
                .map(SlotIndex::read)
                .collect::<Result<_, _>>()?,
        };
        if identity.encode(limits)? != text {
            return Err(IdentityError::Malformed);
        }
        Ok(identity)
    }

    fn validate_indices(&self) -> Result<(), IdentityError> {
        if self.owner.is_empty() || self.template.path.is_empty() {
            return Err(IdentityError::Malformed);
        }
        let mut indices = self.indices.iter();
        for (position, entry) in self.template.path.iter().enumerate() {
            if matches!(entry, PathEntry::Collection { .. }) {
                let index = indices.next().ok_or(IdentityError::Malformed)?;
                if index.path_position.as_str() != position.to_string() {
                    return Err(IdentityError::Malformed);
                }
            }
        }
        if indices.next().is_some() {
            return Err(IdentityError::Malformed);
        }
        Ok(())
    }
}

fn check_bytes(required: usize, limits: Limits) -> Result<(), IdentityError> {
    let required = u64::try_from(required).unwrap_or(u64::MAX);
    if required > limits.max_bytes() {
        return Err(
            quire_canonical::Error::Limit(quire_canonical::LimitExceeded {
                kind: quire_canonical::LimitKind::CanonicalBytes,
                bound: limits.max_bytes(),
                required,
            })
            .into(),
        );
    }
    Ok(())
}

fn read_component(text: &str, limits: Limits) -> Result<quire_canonical::Document, IdentityError> {
    let mut bytes = Vec::new();
    let mut digits = text.bytes();
    while let Some(high) = digits.next() {
        let low = digits.next().ok_or(IdentityError::Malformed)?;
        let digit = |byte| match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            _ => Err(IdentityError::Malformed),
        };
        bytes.push((digit(high)? << 4) | digit(low)?);
    }
    Ok(quire_canonical::read(&bytes, limits.max_bytes())?)
}

fn array<'d>(node: NodeRef<'d>) -> Result<Vec<NodeRef<'d>>, IdentityError> {
    match node.node() {
        Node::Array(items) => Ok(items.collect()),
        _ => Err(IdentityError::Malformed),
    }
}

fn string<'d>(node: NodeRef<'d>) -> Result<&'d str, IdentityError> {
    match node.node() {
        Node::String(text) => Ok(text),
        _ => Err(IdentityError::Malformed),
    }
}

impl DeclarationScope {
    fn read(node: NodeRef<'_>) -> Result<Self, IdentityError> {
        let parts = array(node)?;
        match parts.as_slice() {
            [tag, authority, identity, declaration] if string(*tag)? == "source" => {
                Ok(Self::Source {
                    authority: string(*authority)?.to_owned(),
                    identity: string(*identity)?.to_owned(),
                    declaration: string(*declaration)?.to_owned(),
                })
            }
            [tag, identity, declaration] if string(*tag)? == "model" => Ok(Self::Model {
                identity: string(*identity)?.to_owned(),
                declaration: string(*declaration)?.to_owned(),
            }),
            _ => Err(IdentityError::Malformed),
        }
    }
}

impl Encode for DeclarationScope {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::Source {
                authority,
                identity,
                declaration,
            } => {
                writer.string("source")?;
                writer.string(authority)?;
                writer.string(identity)?;
                writer.string(declaration)?;
            }
            Self::Model {
                identity,
                declaration,
            } => {
                writer.string("model")?;
                writer.string(identity)?;
                writer.string(declaration)?;
            }
        }
        writer.end_array()
    }
}

impl CollectionShape {
    fn write_fields<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.string(self.kind.text())?;
        writer.string(self.minimum.as_str())?;
        match &self.maximum {
            Some(maximum) => writer.string(maximum.as_str()),
            None => writer.null(),
        }
    }

    fn read(
        kind: NodeRef<'_>,
        minimum: NodeRef<'_>,
        maximum: NodeRef<'_>,
    ) -> Result<Self, IdentityError> {
        Ok(Self {
            kind: CollectionKind::read(string(kind)?)?,
            minimum: Decimal::new(string(minimum)?)?,
            maximum: if matches!(maximum.node(), Node::Null) {
                None
            } else {
                Some(Decimal::new(string(maximum)?)?)
            },
        })
    }
}

impl Encode for Carrier {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        for layer in &self.layers {
            writer.begin_array()?;
            match layer {
                CarrierLayer::Option => writer.string("option")?,
                CarrierLayer::Collection(shape) => {
                    writer.string("collection")?;
                    shape.write_fields(writer)?;
                }
            }
        }
        writer.begin_array()?;
        match &self.leaf {
            CarrierLeaf::Clock => writer.string("clock")?,
            CarrierLeaf::Declared(scope) => {
                writer.string("declared")?;
                scope.encode_into(writer)?;
            }
            CarrierLeaf::Tuple => writer.string("tuple")?,
            CarrierLeaf::Union => writer.string("union")?,
            CarrierLeaf::Record => writer.string("record")?,
        }
        writer.end_array()?;
        for _ in &self.layers {
            writer.end_array()?;
        }
        Ok(())
    }
}

impl Carrier {
    fn read(mut node: NodeRef<'_>) -> Result<Self, IdentityError> {
        let mut layers = Vec::new();
        loop {
            let parts = array(node)?;
            let leaf = match parts.as_slice() {
                [tag, next] if string(*tag)? == "option" => {
                    layers.push(CarrierLayer::Option);
                    node = *next;
                    continue;
                }
                [tag, kind, min, max, next] if string(*tag)? == "collection" => {
                    layers.push(CarrierLayer::Collection(CollectionShape::read(
                        *kind, *min, *max,
                    )?));
                    node = *next;
                    continue;
                }
                [tag, scope] if string(*tag)? == "declared" => {
                    CarrierLeaf::Declared(DeclarationScope::read(*scope)?)
                }
                [tag] => match string(*tag)? {
                    "clock" => CarrierLeaf::Clock,
                    "tuple" => CarrierLeaf::Tuple,
                    "union" => CarrierLeaf::Union,
                    "record" => CarrierLeaf::Record,
                    _ => return Err(IdentityError::Malformed),
                },
                _ => return Err(IdentityError::Malformed),
            };
            return Ok(Self { layers, leaf });
        }
    }
}

struct Path<'a>(&'a [PathEntry]);

impl Encode for Path<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        for entry in self.0 {
            writer.begin_array()?;
            match entry {
                PathEntry::Field {
                    declaring,
                    member,
                    carrier,
                } => {
                    writer.string("field")?;
                    declaring.encode_into(writer)?;
                    writer.string(member)?;
                    carrier.encode_into(writer)?;
                }
                PathEntry::Tuple { position, carrier } => {
                    writer.string("tuple")?;
                    writer.string(position.as_str())?;
                    carrier.encode_into(writer)?;
                }
                PathEntry::Case { name, carrier } => {
                    writer.string("case")?;
                    writer.string(name)?;
                    carrier.encode_into(writer)?;
                }
                PathEntry::Collection { shape, carrier } => {
                    writer.string("collection")?;
                    shape.write_fields(writer)?;
                    carrier.encode_into(writer)?;
                }
                PathEntry::Option => writer.string("option")?,
            }
            writer.end_array()?;
        }
        writer.end_array()
    }
}

impl PathEntry {
    fn read(node: NodeRef<'_>) -> Result<Self, IdentityError> {
        let parts = array(node)?;
        match parts.as_slice() {
            [tag, declaring, member, carrier] if string(*tag)? == "field" => Ok(Self::Field {
                declaring: DeclarationScope::read(*declaring)?,
                member: string(*member)?.to_owned(),
                carrier: Carrier::read(*carrier)?,
            }),
            [tag, position, carrier] if string(*tag)? == "tuple" => Ok(Self::Tuple {
                position: Decimal::new(string(*position)?)?,
                carrier: Carrier::read(*carrier)?,
            }),
            [tag, name, carrier] if string(*tag)? == "case" => Ok(Self::Case {
                name: string(*name)?.to_owned(),
                carrier: Carrier::read(*carrier)?,
            }),
            [tag, kind, min, max, carrier] if string(*tag)? == "collection" => {
                Ok(Self::Collection {
                    shape: CollectionShape::read(*kind, *min, *max)?,
                    carrier: Carrier::read(*carrier)?,
                })
            }
            [tag] if string(*tag)? == "option" => Ok(Self::Option),
            _ => Err(IdentityError::Malformed),
        }
    }
}

struct Indices<'a>(&'a [SlotIndex]);

impl Encode for Indices<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        for index in self.0 {
            writer.begin_array()?;
            writer.string("slot")?;
            writer.string(index.path_position.as_str())?;
            writer.string(index.index.as_str())?;
            writer.end_array()?;
        }
        writer.end_array()
    }
}

impl SlotIndex {
    fn read(node: NodeRef<'_>) -> Result<Self, IdentityError> {
        let parts = array(node)?;
        match parts.as_slice() {
            [tag, position, index] if string(*tag)? == "slot" => Ok(Self {
                path_position: Decimal::new(string(*position)?)?,
                index: Decimal::new(string(*index)?)?,
            }),
            _ => Err(IdentityError::Malformed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    const VECTOR: &str = "place/v2/5b22736f75726365222c2261222c2269222c2269783a2f2f692f63616c6c73225d/22632f6422/5b22736f75726365222c2261222c2269222c2269783a2f2f692f43225d/5b5b226669656c64222c5b22736f75726365222c2261222c2269222c2269783a2f2f692f43225d2c2269783a2f2f692f432f78222c5b22636c6f636b225d5d5d/5b5d";

    fn scope(declaration: &str) -> DeclarationScope {
        DeclarationScope::Source {
            authority: "a".to_owned(),
            identity: "i".to_owned(),
            declaration: declaration.to_owned(),
        }
    }

    fn vector() -> PlaceIdentity {
        PlaceIdentity {
            population: scope("ix://i/calls"),
            owner: "c/d".to_owned(),
            template: PlaceTemplate {
                scope: scope("ix://i/C"),
                path: vec![PathEntry::Field {
                    declaring: scope("ix://i/C"),
                    member: "ix://i/C/x".to_owned(),
                    carrier: Carrier {
                        layers: Vec::new(),
                        leaf: CarrierLeaf::Clock,
                    },
                }],
            },
            indices: Vec::new(),
        }
    }

    /// The literal is the independently specified FR-230 vector, not a
    /// comparison between two invocations of this encoder.
    #[trace("TC-3084", "FR-230-AC-43")]
    #[test]
    fn delimiter_vector_has_the_exact_contract_text() {
        let limits = Limits::new(4096);
        assert_eq!(vector().encode(limits).expect("contract vector"), VECTOR);
        assert_eq!(
            PlaceIdentity::decode(VECTOR, limits).expect("canonical vector"),
            vector()
        );
    }

    #[trace("TC-464", "FR-230-AC-45")]
    #[test]
    fn noncanonical_texts_and_old_version_are_refused() {
        let limits = Limits::new(4096);
        for bad in [
            VECTOR.replacen("5b", "5B", 1),
            VECTOR.replacen("/5b5d", "/5b5b22736c6f74222c223031222c2230225d5d", 1),
            VECTOR.replacen("22632f6422", "2022632f6422", 1),
            format!("{VECTOR}/5b5d"),
            VECTOR.replacen("place/v2", "place/v1", 1),
        ] {
            assert!(
                matches!(
                    PlaceIdentity::decode(&bad, limits),
                    Err(IdentityError::Malformed)
                ),
                "{bad}"
            );
        }
    }

    #[trace("TC-3084", "FR-1040-AC-9", "FR-230-AC-49")]
    #[test]
    fn population_and_tagged_nested_slots_remain_distinct() {
        let limits = Limits::new(8192);
        let mut first = vector();
        let shape = CollectionShape {
            kind: CollectionKind::Sequence,
            minimum: 0.into(),
            maximum: Some(2.into()),
        };
        let PathEntry::Field { carrier, .. } = &mut first.template.path[0] else {
            panic!("field vector");
        };
        carrier.layers = vec![CarrierLayer::Collection(shape.clone())];
        carrier.leaf = CarrierLeaf::Tuple;
        first.template.path.extend([
            PathEntry::Collection {
                shape: shape.clone(),
                carrier: Carrier {
                    layers: Vec::new(),
                    leaf: CarrierLeaf::Tuple,
                },
            },
            PathEntry::Tuple {
                position: 0.into(),
                carrier: Carrier {
                    layers: vec![CarrierLayer::Collection(shape.clone())],
                    leaf: CarrierLeaf::Clock,
                },
            },
            PathEntry::Collection {
                shape,
                carrier: Carrier {
                    layers: Vec::new(),
                    leaf: CarrierLeaf::Clock,
                },
            },
        ]);
        first.indices = vec![
            SlotIndex {
                path_position: 1.into(),
                index: 0.into(),
            },
            SlotIndex {
                path_position: 3.into(),
                index: 0.into(),
            },
        ];
        let first_text = first.encode(limits).expect("two tagged slots");
        let mut second = first.clone();
        second.indices[1].index = 1.into();
        let second_text = second.encode(limits).expect("inner slot one");
        assert_ne!(first_text, second_text);
        assert_eq!(
            PlaceIdentity::decode(&first_text, limits).expect("slot zero"),
            first
        );
        assert_eq!(
            PlaceIdentity::decode(&second_text, limits).expect("slot one"),
            second
        );
        assert_eq!(first.indices[0], second.indices[0]);
        assert_eq!(first.template, second.template);
        let mut other_population = first.clone();
        other_population.population = scope("ix://i/archive");
        assert_ne!(
            other_population.encode(limits).expect("archive"),
            first_text
        );
        second.indices.reverse();
        assert!(matches!(
            second.encode(limits),
            Err(IdentityError::Malformed)
        ));
    }

    #[trace("TC-464", "FR-230-AC-45")]
    #[test]
    fn optional_kind_changes_the_identity_and_limits_remain_typed() {
        let limits = Limits::new(4096);
        let mut optional = vector();
        let PathEntry::Field { carrier, .. } = &mut optional.template.path[0] else {
            panic!("field vector");
        };
        carrier.layers.push(CarrierLayer::Option);
        optional.template.path.push(PathEntry::Option);
        let encoded = optional.encode(limits).expect("optional clock");
        assert_ne!(encoded, VECTOR);
        assert_eq!(
            PlaceIdentity::decode(&encoded, limits).expect("optional vector"),
            optional
        );
        assert!(matches!(
            vector().encode(Limits::new(1)),
            Err(IdentityError::Canonical(quire_canonical::Error::Limit(_)))
        ));
    }
}
