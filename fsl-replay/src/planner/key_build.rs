// SPDX-License-Identifier: AGPL-3.0-or-later
//! One canonical projection for source and admitted composition slot identities.

use std::cell::RefCell;

use filament_contract_model::{
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge, NativeJoinLocus,
};
use filament_exact::{NodeKey, NodePathIter};
use fsl_foundation::node_wire::{project_node_key, NodeIdentitySink};
use quire_canonical::{Encode, Sink, Writer};

use super::from_package::ItemKeyInput;
use super::key::{ClaimKind, FairnessKey, ItemKey, KeyWireError, TemporalItemKind, TemporalKey};

/// Native materialization never turns an unpaid charge into a wire refusal.
#[derive(Debug, thiserror::Error)]
pub(super) enum KeyBuildRefusal {
    #[error("native key materialization exceeded {denied:?} at {locus:?}")]
    Native {
        denied: NativeJoinDeniedCharge,
        locus: Option<NativeJoinLocus>,
        configured_bound: u64,
    },
    #[error(transparent)]
    Canonical(#[from] quire_canonical::Error),
    #[error(transparent)]
    Wire(#[from] KeyWireError),
}

struct Scope<'a> {
    budget: RefCell<Option<&'a mut NativeJoinBudget>>,
    locus: NativeJoinLocus,
}
impl<'a> Scope<'a> {
    fn new(
        budget: Option<&'a mut NativeJoinBudget>,
        locus: NativeJoinLocus,
    ) -> Result<Self, KeyBuildRefusal> {
        if let Some(budget) = budget.as_deref() {
            if let Some(denied) = budget.first_unpaid() {
                return Err(Self::denied(budget, denied));
            }
        }
        let scope = Self {
            budget: RefCell::new(budget),
            locus,
        };
        if let Some(budget) = &mut *scope.budget.borrow_mut() {
            budget.resume();
        }
        Ok(scope)
    }
    fn denied(budget: &NativeJoinBudget, denied: NativeJoinDeniedCharge) -> KeyBuildRefusal {
        KeyBuildRefusal::Native {
            configured_bound: match denied.kind {
                NativeJoinChargeKind::Work => budget.work_limit(),
                NativeJoinChargeKind::Storage => budget.storage_limit(),
            },
            locus: budget.first_unpaid_locus(),
            denied,
        }
    }
    fn charge(
        &self,
        kind: NativeJoinChargeKind,
        amount: u128,
    ) -> Result<(), quire_canonical::Error> {
        if let Some(budget) = &mut *self.budget.borrow_mut() {
            budget
                .accounted_charge(kind, amount, self.locus)
                .map_err(|denied| quire_canonical::Error::ReservationDenied {
                    requested: usize::try_from(denied.amount).unwrap_or(usize::MAX),
                })?;
        }
        Ok(())
    }
    fn work(&self, amount: u128) -> Result<(), quire_canonical::Error> {
        self.charge(NativeJoinChargeKind::Work, amount)
    }
    fn text(&self, text: &str) -> Result<(), quire_canonical::Error> {
        self.work(1 + text.len() as u128)
    }
    fn finish<T>(&self, result: Result<T, quire_canonical::Error>) -> Result<T, KeyBuildRefusal> {
        if let Some(budget) = &*self.budget.borrow() {
            if let Some(denied) = budget.first_unpaid() {
                return Err(Self::denied(budget, denied));
            }
        }
        Ok(result?)
    }
    fn metered(&self) -> bool {
        self.budget.borrow().is_some()
    }
}

struct Reservation<'s, 'b>(&'s Scope<'b>);
impl quire_canonical::Reservation for Reservation<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
        self.0
            .charge(NativeJoinChargeKind::Storage, bytes as u128)
            .map_err(|_| quire_canonical::ReservationDenied)
    }
}
struct Output<'s, 'b> {
    scope: &'s Scope<'b>,
    bytes: Vec<u8>,
}
impl Sink for Output<'_, '_> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), quire_canonical::Error> {
        self.scope.work(bytes.len() as u128)?;
        let needed = self.bytes.len().checked_add(bytes.len()).ok_or(
            quire_canonical::Error::Allocation {
                requested: usize::MAX,
            },
        )?;
        if needed > self.bytes.capacity() {
            let target = needed.max(self.bytes.capacity().saturating_mul(2));
            self.scope
                .charge(NativeJoinChargeKind::Storage, 1 + target as u128)?;
            self.bytes
                .try_reserve_exact(target - self.bytes.len())
                .map_err(|_| quire_canonical::Error::Allocation { requested: target })?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

struct NodeProjection<'w, 's, 'b, S: Sink + ?Sized> {
    writer: &'w mut Writer<'s, S>,
    scope: &'w Scope<'b>,
}
impl<S: Sink + ?Sized> NodeIdentitySink for NodeProjection<'_, '_, '_, S> {
    type Error = quire_canonical::Error;
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.scope.work(1)
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.writer.begin_object()
    }
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        self.scope.text(name)?;
        self.writer.name(name)
    }
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error> {
        self.scope.work(1)?;
        self.writer.integer(i128::from(ordinal))
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.scope.text(text)?;
        self.writer.string(text)
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        self.writer.begin_array()
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        self.writer.end_array()
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.writer.end_object()
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.writer.reserve_list(stack, 1)?;
        stack.push(path);
        Ok(())
    }
}
struct Subject<'a, 'b> {
    identity: &'a str,
    assumptions: &'a [NodeKey],
    scope: &'a Scope<'b>,
}
impl Encode for Subject<'_, '_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        self.scope.work(1)?;
        writer.begin_object()?;
        if !self.assumptions.is_empty() {
            writer.name("assumptions")?;
            writer.begin_array()?;
            for node in self.assumptions {
                project_node_key(
                    node,
                    &mut NodeProjection {
                        writer,
                        scope: self.scope,
                    },
                )?;
            }
            writer.end_array()?;
        }
        writer.name("identity")?;
        self.scope.text(self.identity)?;
        writer.string(self.identity)?;
        for name in ["initial_states", "proof_bounds", "universes"] {
            self.scope.work(1)?;
            writer.name(name)?;
            writer.begin_array()?;
            writer.end_array()?;
        }
        writer.end_object()
    }
}
fn encode(value: &impl Encode, scope: &Scope<'_>) -> Result<String, KeyBuildRefusal> {
    let mut output = Output {
        scope,
        bytes: Vec::new(),
    };
    let result = if scope.metered() {
        quire_canonical::encode_with_reservation(
            &mut output,
            value,
            quire_canonical::Limits::new(u64::MAX),
            &mut Reservation(scope),
        )
    } else {
        quire_canonical::encode(&mut output, value, quire_canonical::Limits::new(u64::MAX))
    };
    scope.finish(result)?;
    scope.finish(scope.work(output.bytes.len() as u128))?;
    String::from_utf8(output.bytes).map_err(|_| KeyWireError.into())
}

pub(super) fn subject_text(
    identity: &str,
    assumptions: &[NodeKey],
) -> Result<String, KeyWireError> {
    let scope = Scope::new(None, locus()).map_err(|_| KeyWireError)?;
    encode(
        &Subject {
            identity,
            assumptions,
            scope: &scope,
        },
        &scope,
    )
    .map_err(|_| KeyWireError)
}
pub(super) fn subject_text_with_budget(
    identity: &str,
    assumptions: &[NodeKey],
    budget: &mut NativeJoinBudget,
) -> Result<String, KeyBuildRefusal> {
    let scope = Scope::new(Some(budget), locus())?;
    encode(
        &Subject {
            identity,
            assumptions,
            scope: &scope,
        },
        &scope,
    )
}

fn locus() -> NativeJoinLocus {
    NativeJoinLocus {
        source: filament_contract_model::NativeJoinLocusSource::Package,
        form: "composition_item_key",
    }
}
pub(super) fn fairness_key(
    constraint: &filament_semantic_value::fairness::FairnessConstraint,
) -> Result<FairnessKey, KeyWireError> {
    let scope = Scope::new(None, locus()).map_err(|_| KeyWireError)?;
    Ok(FairnessKey::from_canonical(
        encode(constraint, &scope).map_err(|_| KeyWireError)?,
    ))
}
pub(super) fn fairness_keys_with_budget(
    constraints: &[filament_semantic_value::fairness::FairnessConstraint],
    budget: &mut NativeJoinBudget,
) -> Result<Vec<FairnessKey>, KeyBuildRefusal> {
    let scope = Scope::new(Some(budget), locus())?;
    if !constraints.is_empty() {
        scope.finish(scope.charge(NativeJoinChargeKind::Storage, 1 + constraints.len() as u128))?;
    }
    let mut keys = Vec::with_capacity(constraints.len());
    for constraint in constraints {
        scope.finish(scope.work(1))?;
        keys.push(FairnessKey::from_canonical(encode(constraint, &scope)?));
    }
    Ok(keys)
}

pub(super) fn item_key(
    input: ItemKeyInput<'_>,
    budget: Option<&mut NativeJoinBudget>,
) -> Result<ItemKey, KeyBuildRefusal> {
    let scope = Scope::new(budget, locus())?;
    item(input, &scope)
}
fn item(mut input: ItemKeyInput<'_>, scope: &Scope<'_>) -> Result<ItemKey, KeyBuildRefusal> {
    scope.finish(scope.work(1))?;
    let temporal = if input.kind == ClaimKind::TemporalSatisfaction {
        if scope.metered() {
            // In-place insertion is the repository's bounded sorting pattern:
            // each actual UTF-8 comparison and movement can refuse immediately.
            for at in 1..input.fairness.len() {
                let mut next = at;
                while next > 0 {
                    let left = input.fairness[next - 1].canonical().as_bytes();
                    let right = input.fairness[next].canonical().as_bytes();
                    let mut order = std::cmp::Ordering::Equal;
                    let mut left = left.iter();
                    let mut right = right.iter();
                    loop {
                        scope.finish(scope.work(1))?;
                        match (left.next(), right.next()) {
                            (Some(a), Some(b)) => {
                                order = a.cmp(b);
                                if !order.is_eq() {
                                    break;
                                }
                            }
                            (None, None) => break,
                            (None, Some(_)) => {
                                order = std::cmp::Ordering::Less;
                                break;
                            }
                            (Some(_), None) => {
                                order = std::cmp::Ordering::Greater;
                                break;
                            }
                        }
                    }
                    if !order.is_gt() {
                        break;
                    }
                    scope.finish(scope.work(1))?;
                    input.fairness.swap(next - 1, next);
                    next -= 1;
                }
            }
        } else {
            input.fairness.sort();
        }
        Some(TemporalKey {
            subject: encode(
                &Subject {
                    identity: input.subject_identity,
                    assumptions: input.assumptions,
                    scope,
                },
                scope,
            )?,
            item_kind: TemporalItemKind::Formula,
            fairness: input.fairness,
            stutter_closure: input.stutter_closure,
        })
    } else {
        None
    };
    Ok(ItemKey {
        claim: input.claim,
        kind: input.kind,
        temporal,
        correspondence: input.correspondence,
        premises: Vec::new(),
        extent: input.extent,
    })
}

#[cfg(test)]
mod tests {
    use super::super::key::ExtentClass;
    use super::*;
    use filament_exact::{EffectiveId, Origin, OriginalDeclarationKey, PathSegment, Role};
    use filament_semantic_value::fairness::{
        FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
    };
    use fsl_foundation::source::provenance::OccurrenceKey;

    fn assumption() -> NodeKey {
        let child = NodeKey::new("child", vec![PathSegment::Text("μ".into())], 0).expect("child");
        NodeKey::new(
            "owner",
            vec![PathSegment::Text("clause".into()), PathSegment::Node(child)],
            3,
        )
        .expect("outer")
    }
    fn constraints() -> Vec<FairnessConstraint> {
        let operation = FairnessOperation::new(
            EffectiveId::new(None, OriginalDeclarationKey::new("model", "T")),
            "send".into(),
        );
        vec![
            FairnessConstraint::new(
                FairnessKind::Weak,
                operation.clone(),
                FairnessGranularity::Whole,
            ),
            FairnessConstraint::new(
                FairnessKind::Strong,
                operation.clone(),
                FairnessGranularity::Each,
            ),
            FairnessConstraint::new(FairnessKind::Strong, operation, FairnessGranularity::Each),
        ]
    }
    fn input<'a>(assumptions: &'a [NodeKey], fairness: Vec<FairnessKey>) -> ItemKeyInput<'a> {
        ItemKeyInput {
            claim: OccurrenceKey::new(assumptions[0].clone(), Origin::new(Role::new("claim"), 0)),
            kind: ClaimKind::TemporalSatisfaction,
            subject_identity: "library using model",
            assumptions,
            fairness,
            stutter_closure: true,
            correspondence: None,
            extent: ExtentClass::Unbounded,
        }
    }

    /// Trace: FR-1044-AC-10, FR-251-AC-49
    #[test]
    fn canonical_projection_matches_frozen_subject_and_constraint_bytes() {
        assert_eq!(
            subject_text("lib using \"model\"\nα", &[]).expect("quoted identity"),
            r#"{"identity":"lib using \"model\"\nα","initial_states":[],"proof_bounds":[],"universes":[]}"#
        );
        assert_eq!(
            subject_text("library using model", &[assumption()]).expect("nested assumption"),
            r#"{"assumptions":[{"ordinal":3,"package":"owner","path":["clause",{"ordinal":0,"package":"child","path":["μ"]}]}],"identity":"library using model","initial_states":[],"proof_bounds":[],"universes":[]}"#
        );
        let weak = FairnessConstraint::new(
            FairnessKind::Weak,
            FairnessOperation::new(
                EffectiveId::new(None, OriginalDeclarationKey::new("model", "T")),
                "send".into(),
            ),
            FairnessGranularity::Each,
        );
        assert_eq!(
            fairness_key(&weak).expect("constraint").canonical(),
            r#"{"granularity":"each","kind":"weak","operation":{"declaring_type":{"original":{"node":"T","package":"model"},"owner_effective_type":null},"name":"send"}}"#
        );
    }

    /// Trace: FR-1044-AC-10, FR-251-AC-49
    #[test]
    fn paid_keys_preserve_order_duplicates_and_first_unpaid_native_metadata() {
        let assumptions = [assumption()];
        let constraints = constraints();
        let project = |budget: &mut NativeJoinBudget| {
            let fairness = fairness_keys_with_budget(&constraints, budget)?;
            item_key(input(&assumptions, fairness), Some(budget))
        };
        let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
        let before_locus = NativeJoinLocus {
            source: filament_contract_model::NativeJoinLocusSource::ReachedRow(19),
            form: "already_joined_owner",
        };
        budget
            .accounted_charge(NativeJoinChargeKind::Work, 7, before_locus)
            .expect("previous work");
        budget
            .accounted_charge(NativeJoinChargeKind::Storage, 13, before_locus)
            .expect("previous storage");
        let actual = project(&mut budget).expect("paid full key");
        let paid = budget.consumed();
        let (work, storage) = (paid.work - 7, paid.storage - 13);
        assert!(work > 0 && storage > 0);
        let mut source_fairness: Vec<_> = constraints
            .iter()
            .map(|c| FairnessKey::of(c).expect("source fairness"))
            .collect();
        source_fairness.sort();
        let temporal = actual.temporal.as_ref().expect("temporal key");
        assert_eq!(temporal.fairness, source_fairness);
        assert_eq!(
            temporal.fairness.len(),
            3,
            "equal entries retain source tie/dedup behavior"
        );
        assert_eq!(temporal.fairness[0], temporal.fairness[1]);
        assert_eq!(
            temporal.subject,
            r#"{"assumptions":[{"ordinal":3,"package":"owner","path":["clause",{"ordinal":0,"package":"child","path":["μ"]}]}],"identity":"library using model","initial_states":[],"proof_bounds":[],"universes":[]}"#
        );
        let source =
            super::super::from_package::item_key_from_input(input(&assumptions, source_fairness))
                .expect("source common builder");
        assert_eq!(actual, source);
        let mut exact = NativeJoinBudget::new(work, storage);
        assert_eq!(project(&mut exact).expect("exact key allowance"), actual);
        assert_eq!(exact.consumed().work, work);
        assert_eq!(exact.consumed().storage, storage);
        for (work, storage, kind) in [
            (work - 1, 1_000_000, NativeJoinChargeKind::Work),
            (1_000_000, storage - 1, NativeJoinChargeKind::Storage),
            (0, 1_000_000, NativeJoinChargeKind::Work),
            (1_000_000, 0, NativeJoinChargeKind::Storage),
        ] {
            let mut short = NativeJoinBudget::new(work, storage);
            let KeyBuildRefusal::Native {
                denied,
                locus,
                configured_bound,
            } = project(&mut short).expect_err("unpaid key operation")
            else {
                panic!("native denial must not become canonical/wire failure");
            };
            assert_eq!(denied.kind, kind);
            assert_eq!(Some(denied), short.first_unpaid());
            assert_eq!(locus, short.first_unpaid_locus());
            assert_eq!(
                configured_bound,
                match kind {
                    NativeJoinChargeKind::Work => work,
                    NativeJoinChargeKind::Storage => storage,
                }
            );
            let before = short.consumed();
            assert!(
                matches!(project(&mut short), Err(KeyBuildRefusal::Native { denied: repeated, locus: repeated_locus, configured_bound: repeated_bound }) if repeated == denied && repeated_locus == locus && repeated_bound == configured_bound)
            );
            assert_eq!(short.consumed(), before);
        }
    }

    /// Trace: FR-251-AC-49
    #[test]
    fn ordering_refuses_before_the_first_unpaid_byte_or_movement() {
        let assumptions = [assumption()];
        let constraints = constraints();
        let fairness = constraints[..2]
            .iter()
            .map(|c| FairnessKey::of(c).expect("caller-owned canonical key"))
            .collect::<Vec<_>>();
        assert!(fairness[0] > fairness[1]);
        let prefix = fairness[0]
            .canonical()
            .bytes()
            .zip(fairness[1].canonical().bytes())
            .take_while(|(a, b)| a == b)
            .count() as u64;
        for bound in [1, 1 + prefix + 1] {
            let mut budget = NativeJoinBudget::new(bound, 0);
            let KeyBuildRefusal::Native {
                denied,
                locus,
                configured_bound,
            } = item_key(input(&assumptions, fairness.clone()), Some(&mut budget))
                .expect_err("comparison/movement denies before subject allocation")
            else {
                panic!("comparison must retain native work denial");
            };
            assert_eq!(denied.kind, NativeJoinChargeKind::Work);
            assert_eq!(denied.amount, 1);
            assert_eq!(configured_bound, bound);
            assert_eq!(Some(denied), budget.first_unpaid());
            assert_eq!(locus, budget.first_unpaid_locus());
            assert_eq!(budget.consumed().storage, 0);
        }
    }
}
