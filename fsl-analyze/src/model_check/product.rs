// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 SM-3, SM-4 and EN-1: a product over the existing transition seam.

use fsl_eval::simulation::{
    Expansion, ExpansionStop, Footprint, OfferedReductions, TransitionSystem,
};
use quire_canonical::{Encode, Sink, Writer};
use quire_canonical_api as canonical_api;

/// Both components of a product state, preserved independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductState<M, P> {
    /// The original model state.
    pub model: M,
    /// The property state after reading this position.
    pub property: P,
}

/// Independently supplied canonical keys of the two product components.
pub struct ProductKey<M, P> {
    /// Model component's full key.
    pub model: M,
    /// Property component's full key, independent of materialization order.
    pub property: P,
}

trait ProductKeyWriter<M, P> {
    type Error;

    fn begin_array(&mut self) -> Result<(), Self::Error>;
    fn model(&mut self, model: &M) -> Result<(), Self::Error>;
    fn property(&mut self, property: &P) -> Result<(), Self::Error>;
    fn end_array(&mut self) -> Result<(), Self::Error>;
}

fn encode_product_key<W, M, P>(writer: &mut W, model: &M, property: &P) -> Result<(), W::Error>
where
    W: ProductKeyWriter<M, P>,
{
    writer.begin_array()?;
    writer.model(model)?;
    writer.property(property)?;
    writer.end_array()
}

impl<S: Sink + ?Sized, M: Encode, P: Encode> ProductKeyWriter<M, P>
    for quire_canonical::Writer<'_, S>
{
    type Error = quire_canonical::Error;

    fn begin_array(&mut self) -> Result<(), Self::Error> {
        Writer::begin_array(self)
    }

    fn model(&mut self, model: &M) -> Result<(), Self::Error> {
        model.encode_into(self)
    }

    fn property(&mut self, property: &P) -> Result<(), Self::Error> {
        property.encode_into(self)
    }

    fn end_array(&mut self) -> Result<(), Self::Error> {
        Writer::end_array(self)
    }
}

impl<S: canonical_api::Sink + ?Sized, M: canonical_api::Encode, P: canonical_api::Encode>
    ProductKeyWriter<M, P> for canonical_api::Writer<'_, S>
{
    type Error = canonical_api::Error;

    fn begin_array(&mut self) -> Result<(), Self::Error> {
        canonical_api::Writer::begin_array(self)
    }

    fn model(&mut self, model: &M) -> Result<(), Self::Error> {
        model.encode_into(self)
    }

    fn property(&mut self, property: &P) -> Result<(), Self::Error> {
        property.encode_into(self)
    }

    fn end_array(&mut self) -> Result<(), Self::Error> {
        canonical_api::Writer::end_array(self)
    }
}

impl<M: Encode, P: Encode> Encode for ProductKey<M, P> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        encode_product_key(writer, &self.model, &self.property)
    }
}

impl<M: canonical_api::Encode, P: canonical_api::Encode> canonical_api::Encode
    for ProductKey<M, P>
{
    fn encode_into<S: canonical_api::Sink + ?Sized>(
        &self,
        writer: &mut canonical_api::Writer<'_, S>,
    ) -> Result<(), canonical_api::Error> {
        encode_product_key(writer, &self.model, &self.property)
    }
}

#[derive(Clone, Copy)]
struct CanonicalValueRef<'a>(&'a canonical_api::CanonicalValue);

impl canonical_api::Encode for CanonicalValueRef<'_> {
    fn encode_into<S: canonical_api::Sink + ?Sized>(
        &self,
        writer: &mut canonical_api::Writer<'_, S>,
    ) -> Result<(), canonical_api::Error> {
        writer.canonical_value(self.0)
    }
}

impl<P: canonical_api::Encode> ProductKey<canonical_api::CanonicalValue, P> {
    /// Encode a product key through its typed encoder, reusing a validated
    /// canonical model key.
    pub(crate) fn canonical_bytes_with_model_value(
        model: &canonical_api::CanonicalValue,
        property: P,
        max_bytes: u64,
    ) -> Result<Vec<u8>, quire_canonical::Error> {
        let key = ProductKey {
            model: CanonicalValueRef(model),
            property,
        };
        let mut bytes = Vec::new();
        canonical_api::encode_in_order_with_reservation(
            &mut bytes,
            &key,
            canonical_api::Limits::new(max_bytes),
            &mut Unmetered,
        )
        .map_err(legacy_error)?;
        Ok(bytes)
    }
}

impl<P: canonical_api::Encode> ProductKey<canonical_api::CanonicalValue, P> {
    /// Validate bytes previously returned by the canonical state-key encoder.
    pub(crate) fn read_model_value(
        model: &[u8],
        max_input_bytes: u64,
    ) -> Result<canonical_api::CanonicalValue, quire_canonical::Error> {
        canonical_api::CanonicalValue::new(model, max_input_bytes).map_err(|error| match error {
            canonical_api::CanonicalValueError::Read(canonical_api::ReadError::Limit(limit)) => {
                legacy_error(canonical_api::Error::Limit(limit))
            }
            canonical_api::CanonicalValueError::Read(canonical_api::ReadError::Allocation {
                requested,
            }) => quire_canonical::Error::Allocation { requested },
            canonical_api::CanonicalValueError::Encode(error) => legacy_error(error),
            canonical_api::CanonicalValueError::Read(_)
            | canonical_api::CanonicalValueError::NotCanonical => {
                quire_canonical::Error::Internal {
                    invariant: "canonical-model-key-can-be-read",
                }
            }
            _ => quire_canonical::Error::Internal {
                invariant: "canonical-model-key-error-is-classified",
            },
        })
    }
}

fn legacy_error(error: canonical_api::Error) -> quire_canonical::Error {
    match error {
        canonical_api::Error::Limit(limit) => {
            quire_canonical::Error::Limit(quire_canonical::LimitExceeded {
                kind: match limit.kind {
                    canonical_api::LimitKind::CanonicalBytes => {
                        quire_canonical::LimitKind::CanonicalBytes
                    }
                    canonical_api::LimitKind::InputBytes => quire_canonical::LimitKind::InputBytes,
                    canonical_api::LimitKind::WorkingBytes => {
                        quire_canonical::LimitKind::WorkingBytes
                    }
                    canonical_api::LimitKind::ObjectBytes => {
                        quire_canonical::LimitKind::ObjectBytes
                    }
                    _ => quire_canonical::LimitKind::CanonicalBytes,
                },
                bound: limit.bound,
                required: limit.required,
            })
        }
        canonical_api::Error::Allocation { requested } => {
            quire_canonical::Error::Allocation { requested }
        }
        _ => quire_canonical::Error::Internal {
            invariant: "canonical-api-error-maps-to-legacy-refusal",
        },
    }
}

struct Unmetered;

impl canonical_api::Reservation for Unmetered {
    fn reserve(&mut self, _bytes: usize) -> Result<(), canonical_api::ReservationDenied> {
        Ok(())
    }
}

/// A graph-local edge identity; terminal stutter cannot alias a model step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductTransition<T> {
    /// An unchanged transition identity from the model.
    Model(T),
    /// SM-4's infinite terminal repetition, with no model operation.
    TerminalStutter,
    /// FR-380's abstract stutter closure, with no enabled model operation.
    StutterClosure,
}

impl<T: Encode> Encode for ProductTransition<T> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::Model(identity) => {
                writer.string("model")?;
                identity.encode_into(writer)?;
            }
            Self::TerminalStutter => writer.string("terminal-stutter")?,
            Self::StutterClosure => writer.string("stutter-closure")?,
        }
        writer.end_array()
    }
}

/// A finding retains which component produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductFinding<M, P> {
    /// A finding from the model's expansion.
    Model(M),
    /// A property finding, including closed-boundary findings.
    Property(P),
}

/// Adapter for a supplied property state machine, not a formula translator.
///
/// The adapter owns letter evaluation and state advancement. `initial` reads
/// position zero; `advance` reads the post-state with the actual edge identity.
/// Implementations must put every fact affecting future moves into their key.
pub trait PropertyComponent<M: TransitionSystem> {
    /// Property state after reading a position.
    type State;
    /// Full canonical property key.
    type Key: Encode;
    /// Findings exposed without interpretation by the product.
    type Finding: Clone + Eq + std::fmt::Debug;

    /// States after reading the model's initial position, yielded on demand.
    fn initial(&self, model: &M::State) -> impl Iterator<Item = Self::State> + use<Self, M>;
    /// Canonical key of a property state.
    fn key(&self, state: &Self::State) -> Self::Key;
    /// Property moves on a concrete model step or terminal repetition.
    ///
    /// # Errors
    /// Returns the original expansion stop when letter evaluation cannot run.
    fn advance(
        &self,
        state: &Self::State,
        source: &M::State,
        transition: &ProductTransition<M::TransitionId>,
        target: &M::State,
    ) -> Result<Vec<Self::State>, ExpansionStop>;
    /// Findings at this position.
    ///
    /// # Errors
    /// Returns the original property evaluation stop.
    fn findings(
        &self,
        state: &Self::State,
        model: &M::State,
    ) -> Result<Vec<Self::Finding>, ExpansionStop>;
    /// Close a bounded terminal execution with the profile's boundary rule.
    ///
    /// No model step is synthesized. The supplied monitor owns false-extension
    /// and constant handling, and reports its closure findings here.
    ///
    /// # Errors
    /// Returns the original monitor closure stop.
    fn close(
        &self,
        state: &Self::State,
        model: &M::State,
    ) -> Result<Vec<Self::Finding>, ExpansionStop>;
}

/// A model/property product using FR-101's actual `TransitionSystem` seam.
pub struct Product<'a, M, P> {
    model: &'a M,
    property: &'a P,
    infinite: bool,
}

impl<'a, M, P> Product<'a, M, P> {
    /// A bounded product: terminal executions close through the monitor.
    pub const fn bounded(model: &'a M, property: &'a P) -> Self {
        Self {
            model,
            property,
            infinite: false,
        }
    }
    /// An infinite product: terminal executions advance along stutter steps.
    pub const fn infinite(model: &'a M, property: &'a P) -> Self {
        Self {
            model,
            property,
            infinite: true,
        }
    }
}

impl<M, P> TransitionSystem for Product<'_, M, P>
where
    M: TransitionSystem,
    M::State: Clone,
    P: PropertyComponent<M>,
{
    type State = ProductState<M::State, P::State>;
    type TransitionId = ProductTransition<M::TransitionId>;
    type Key = ProductKey<M::Key, P::Key>;
    type Finding = ProductFinding<M::Finding, P::Finding>;

    type InitialError = M::InitialError;

    fn initial_iter(&self) -> impl Iterator<Item = Result<Self::State, Self::InitialError>> + '_ {
        self.model.initial_iter().flat_map(move |model| {
            let (states, refusal) = match model {
                Ok(model) => {
                    let properties = self.property.initial(&model);
                    let states = properties.map(move |property| {
                        Ok(ProductState {
                            model: model.clone(),
                            property,
                        })
                    });
                    (Some(states), None)
                }
                Err(refusal) => (None, Some(refusal)),
            };
            refusal
                .into_iter()
                .map(Err)
                .chain(states.into_iter().flatten())
        })
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        ProductKey {
            model: self.model.key(&state.model),
            property: self.property.key(&state.property),
        }
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        let expansion = self.model.successors(&state.model)?;
        let mut findings: Vec<_> = expansion
            .findings
            .into_iter()
            .map(ProductFinding::Model)
            .collect();
        findings.extend(
            self.property
                .findings(&state.property, &state.model)?
                .into_iter()
                .map(ProductFinding::Property),
        );
        let mut steps: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(identity, model)| (ProductTransition::Model(identity), model))
            .collect();
        if steps.is_empty() {
            if self.infinite {
                steps.push((ProductTransition::TerminalStutter, state.model.clone()));
            } else {
                findings.extend(
                    self.property
                        .close(&state.property, &state.model)?
                        .into_iter()
                        .map(ProductFinding::Property),
                );
            }
        }
        let mut successors = Vec::new();
        for (transition, model) in steps {
            for property in
                self.property
                    .advance(&state.property, &state.model, &transition, &model)?
            {
                successors.push((
                    transition.clone(),
                    ProductState {
                        model: model.clone(),
                        property,
                    },
                ));
            }
        }
        Ok(Expansion {
            successors,
            findings,
        })
    }

    /// ADR-021 EI-4: the product offers the partial-order reduction its
    /// model component offers, and nothing else: the property component is
    /// never reduced.
    fn offered_reductions(&self) -> OfferedReductions {
        OfferedReductions {
            partial_order: self.model.offered_reductions().partial_order,
            ..OfferedReductions::NONE
        }
    }

    /// FR-162: a model transition has its model component's footprint, and
    /// the terminal stutter edge has an empty one, so no ample-set
    /// candidate holds it.
    fn footprint(
        &self,
        transition: &Self::TransitionId,
    ) -> Option<Result<Footprint, ExpansionStop>> {
        match transition {
            ProductTransition::Model(transition) => self.model.footprint(transition),
            ProductTransition::TerminalStutter | ProductTransition::StutterClosure => {
                Some(Ok(Footprint::default()))
            }
        }
    }
}
