// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-205: delivery-label canonical states, separate from concrete execution.

use std::collections::{BTreeMap, BTreeSet};

use super::identity::{DeliveryIdentity, MessageOccurrence, ProtocolTransition};
use super::memory::MemoryModel;
use super::{Entry, ProtocolRefusal, ProtocolState, ProtocolSystem};
use crate::simulation::model::encode::Enc;
use crate::simulation::{EncodingRefusal, StateKey, SubjectSystem};

type Labels = BTreeMap<(MessageOccurrence, u64), u64>;

struct CanonicalDeliveries<M: MemoryModel> {
    state: ProtocolState<M>,
    labels: Labels,
    entries: Vec<(Enc, Enc)>,
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// Explore delivery-label canonical states for the explicit channel read
    /// set, using qualified channel names such as `Echo::ch`. Execution
    /// systems remain concrete unless this is selected.
    ///
    /// Checked temporal atoms currently read only model state (PB-2), so
    /// their delivery read set is empty. This API also preserves channels
    /// whose labels a caller reads; it does not introduce an obligation form.
    #[must_use]
    pub fn with_cycle_keys(mut self, delivery_reads: BTreeSet<String>) -> Self {
        self.delivery_reads = Some(delivery_reads);
        self
    }

    /// FR-205's canonical state, preserving every channel in `delivery_reads`
    /// and every message with a pending interval send.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn delivery_canonical_state(
        &self,
        state: &ProtocolState<M>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<ProtocolState<M>, EncodingRefusal> {
        self.canonical_deliveries(state, delivery_reads)
            .map(|canonical| canonical.state)
    }

    /// FR-205's cycle key for an explicit delivery read set. The input state
    /// and its concrete ProtocolKey are unchanged.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn cycle_key(
        &self,
        state: &ProtocolState<M>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<StateKey, EncodingRefusal> {
        let canonical = self.delivery_canonical_state(state, delivery_reads)?;
        self.canonical_state_key(&canonical)
    }

    fn canonical_deliveries(
        &self,
        state: &ProtocolState<M>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<CanonicalDeliveries<M>, EncodingRefusal> {
        let mut canonical = state.clone();
        let mut labels = BTreeMap::new();
        let mut entries = Vec::new();
        for instance in canonical.instances.values_mut() {
            let pending: BTreeSet<_> = instance
                .pending_sends
                .values()
                .map(|send| send.message.clone())
                .collect();
            let mut counts = BTreeMap::<MessageOccurrence, u64>::new();
            for (key, queue) in &mut instance.queues {
                let channel = &self.program.channels[key.channel];
                if delivery_reads.contains(&channel.name) {
                    continue;
                }
                // Ordinary unordered queues order copies by all members except
                // delivery. FIFO and send-ordered queues retain their positions.
                if key.fifo.is_none() && !channel.send_ordered {
                    let mut ordered = queue
                        .iter()
                        .map(|entry| Ok((encode(&without_delivery(entry))?, entry.clone())))
                        .collect::<Result<Vec<_>, EncodingRefusal>>()?;
                    ordered.sort_by(|left, right| left.0.cmp(&right.0));
                    *queue = ordered.into_iter().map(|(_, entry)| entry).collect();
                }
                for entry in queue.iter_mut() {
                    if pending.contains(&entry.message) {
                        continue;
                    }
                    let count = counts.entry(entry.message.clone()).or_default();
                    labels.insert((entry.message.clone(), entry.delivery), *count);
                    let encoded = entry_with_delivery(entry, *count);
                    entries.push((entry.enc.clone(), encoded.clone()));
                    entry.delivery = *count;
                    entry.bytes = encode(&encoded)?;
                    entry.enc = encoded;
                    *count = count.checked_add(1).ok_or_else(|| {
                        EncodingRefusal::NoEncoding(
                            "a canonical delivery count overflows".to_owned(),
                        )
                    })?;
                }
                if key.fifo.is_none() && !channel.send_ordered {
                    queue.sort_by(|left, right| left.bytes.cmp(&right.bytes));
                }
            }
            let mut empty = BTreeSet::new();
            for (message, next) in &mut instance.next_deliveries {
                let Some(send) = self.program.messages.get(&message.node()) else {
                    return Err(EncodingRefusal::NoEncoding(
                        "a message names no send".to_owned(),
                    ));
                };
                let channel = &self.program.channels[send.channel];
                if !delivery_reads.contains(&channel.name) && !pending.contains(message) {
                    let count = counts.get(message).copied().unwrap_or(0);
                    // The next fresh label participates in duplicate identities.
                    labels.insert((message.clone(), *next), count);
                    *next = count;
                    if count == 0 {
                        empty.insert(message.clone());
                    }
                }
            }
            instance
                .next_deliveries
                .retain(|message, _| !empty.contains(message));
        }
        Ok(CanonicalDeliveries {
            state: canonical,
            labels,
            entries,
        })
    }

    pub(super) fn canonical_successors(
        &self,
        expansion: &mut crate::simulation::Expansion<
            ProtocolTransition,
            ProtocolState<M>,
            crate::simulation::model::ModelFinding,
        >,
    ) -> Result<(), super::ExpansionStop> {
        if let Some(reads) = &self.delivery_reads {
            for (_, state) in &mut expansion.successors {
                *state = self
                    .delivery_canonical_state(state, reads)
                    .map_err(|_| super::broken())?;
            }
        }
        Ok(())
    }

    pub(super) fn concrete_cycle_step(
        &self,
        source: &ProtocolState<M>,
        transition: &ProtocolTransition,
        target: &ProtocolState<M>,
    ) -> Result<Option<(ProtocolTransition, ProtocolState<M>)>, super::ExpansionStop> {
        let Some(reads) = &self.delivery_reads else {
            return Ok(None);
        };
        // With no unread channel the graph already stores concrete states.
        if self
            .program
            .channels
            .iter()
            .all(|channel| reads.contains(&channel.name))
        {
            return Ok(None);
        }
        let canonical = self
            .canonical_deliveries(source, reads)
            .map_err(|_| super::broken())?;
        let wanted = self.cycle_key(target, reads).map_err(|_| super::broken())?;
        let expansion = self.expand_bounded(source)?.0;
        for (candidate, state) in expansion.successors {
            let renamed = candidate.relabel_deliveries(&canonical.entries, |delivery| {
                canonical
                    .labels
                    .get(&(delivery.message().clone(), delivery.index()))
                    .copied()
                    .map(|index| DeliveryIdentity::new(delivery.message().clone(), index))
                    .unwrap_or_else(|| delivery.clone())
            });
            if renamed == *transition
                && self.cycle_key(&state, reads).map_err(|_| super::broken())? == wanted
            {
                return Ok(Some((candidate, state)));
            }
        }
        Err(ProtocolRefusal::fault("a-cycle-edge-has-a-concrete-successor").into())
    }
}

fn without_delivery(entry: &Entry) -> Enc {
    let mut encoded = entry.enc.clone();
    if let Enc::Object(members) = &mut encoded {
        members.retain(|(name, _)| *name != "delivery");
    }
    encoded
}

fn entry_with_delivery(entry: &Entry, delivery: u64) -> Enc {
    // Preserve the already typed payload encoding, including its type identity.
    let mut encoded = entry.enc.clone();
    if let Enc::Object(members) = &mut encoded {
        for (name, value) in members {
            if *name == "delivery" {
                *value = Enc::text(delivery.to_string());
            }
        }
    }
    encoded
}

fn encode(value: &Enc) -> Result<Vec<u8>, EncodingRefusal> {
    crate::simulation::key::canonical_bytes(value, super::identity_limits())
}
