// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-205: identity-label canonical states, separate from concrete execution.

use std::collections::{BTreeMap, BTreeSet};

use fsl_semantics::check::ProtocolNodeId;

use super::identity::{DeliveryIdentity, MessageOccurrence, ProtocolTransition, ThreadPath};
use super::memory::MemoryModel;
use super::{Entry, ProtocolRefusal, ProtocolState, ProtocolSystem};
use crate::simulation::model::encode::{value_enc, Enc, Typing};
use crate::simulation::{EncodingRefusal, StateKey, SubjectSystem};

type MessageLabels = BTreeMap<MessageOccurrence, MessageOccurrence>;
type DeliveryLabels = BTreeMap<(MessageOccurrence, u64), u64>;
type NextOrdinals = BTreeMap<(ProtocolNodeId, ThreadPath), (u64, u64)>;

struct CanonicalProtocol<M: MemoryModel> {
    state: ProtocolState<M>,
    message_labels: MessageLabels,
    delivery_labels: DeliveryLabels,
    next_ordinals: NextOrdinals,
    entries: Vec<(Enc, Enc)>,
}

impl<M: MemoryModel> CanonicalProtocol<M> {
    fn message(&self, message: &MessageOccurrence) -> MessageOccurrence {
        if let Some(canonical) = self.message_labels.get(message) {
            return canonical.clone();
        }
        self.next_ordinals
            .get(&(message.node(), message.path().clone()))
            .filter(|(concrete, _)| message.ordinal() == *concrete)
            .map_or_else(
                || message.clone(),
                |(_, canonical)| message.with_ordinal(*canonical),
            )
    }
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// Explore identity-label canonical states for explicit checked-item read
    /// sets, using qualified channel names such as `Echo::ch`. Execution
    /// systems remain concrete unless this is selected.
    #[must_use]
    pub fn with_cycle_keys(mut self, delivery_reads: BTreeSet<String>) -> Self {
        self.message_reads = Some(delivery_reads.clone());
        self.delivery_reads = Some(delivery_reads);
        self
    }

    /// Select explicit message and delivery read sets for cycle exploration.
    /// The delivery set is a subset of the message set by definition.
    #[must_use]
    pub fn with_cycle_read_sets(
        mut self,
        message_reads: BTreeSet<String>,
        delivery_reads: BTreeSet<String>,
    ) -> Self {
        let mut message_reads = message_reads;
        message_reads.extend(delivery_reads.iter().cloned());
        self.message_reads = Some(message_reads);
        self.delivery_reads = Some(delivery_reads);
        self
    }

    /// FR-205's canonical state for an explicit delivery read set. The
    /// legacy name is retained; it treats delivery reads as message reads too.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn delivery_canonical_state(
        &self,
        state: &ProtocolState<M>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<ProtocolState<M>, EncodingRefusal> {
        self.canonical_state_with_reads(state, delivery_reads, delivery_reads)
    }

    /// FR-205's canonical state for explicit message and delivery read sets.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn canonical_state_with_reads(
        &self,
        state: &ProtocolState<M>,
        message_reads: &BTreeSet<String>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<ProtocolState<M>, EncodingRefusal> {
        self.canonical_protocol(state, message_reads, delivery_reads)
            .map(|canonical| canonical.state)
    }

    /// FR-205's cycle key for an explicit delivery read set. The input state
    /// and its concrete `ProtocolKey` are unchanged.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn cycle_key(
        &self,
        state: &ProtocolState<M>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<StateKey, EncodingRefusal> {
        self.cycle_key_with_reads(state, delivery_reads, delivery_reads)
    }

    /// FR-205's cycle key for explicit message and delivery read sets.
    ///
    /// # Errors
    /// Refuses if a protocol identity cannot be encoded.
    pub fn cycle_key_with_reads(
        &self,
        state: &ProtocolState<M>,
        message_reads: &BTreeSet<String>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<StateKey, EncodingRefusal> {
        let canonical = self.canonical_state_with_reads(state, message_reads, delivery_reads)?;
        self.canonical_state_key(&canonical)
    }

    fn canonical_protocol(
        &self,
        state: &ProtocolState<M>,
        message_reads: &BTreeSet<String>,
        delivery_reads: &BTreeSet<String>,
    ) -> Result<CanonicalProtocol<M>, EncodingRefusal> {
        let mut message_reads = message_reads.clone();
        message_reads.extend(delivery_reads.iter().cloned());
        let mut canonical = state.clone();
        let mut message_labels = BTreeMap::new();
        let mut delivery_labels = BTreeMap::new();
        let mut next_ordinals = BTreeMap::new();
        let mut entries = Vec::new();

        for instance in canonical.instances.values_mut() {
            let pending_messages: BTreeSet<_> = instance
                .pending_sends
                .values()
                .map(|send| send.message.clone())
                .collect();
            let pending_groups: BTreeSet<_> = pending_messages
                .iter()
                .map(|message| (message.node(), message.path().clone()))
                .collect();

            // One message's first queued copy determines its canonical ordinal
            // order. Queue-map order is the deterministic order between keyed
            // FIFO queues; each queue's vector is its authored delivery order.
            let mut queued = BTreeMap::<MessageOccurrence, QueuedMessage>::new();
            let mut queue_position = 0usize;
            for (key, queue) in &instance.queues {
                let channel = &self.program.channels[key.channel];
                for entry in queue {
                    if !message_reads.contains(&channel.name)
                        && !pending_groups
                            .contains(&(entry.message.node(), entry.message.path().clone()))
                    {
                        let payload = encode(&value_enc(&entry.payload, self.model.typing()))?;
                        let info = queued
                            .entry(entry.message.clone())
                            .or_insert(QueuedMessage {
                                payload,
                                copies: 0,
                                first_position: queue_position,
                                ordered: key.fifo.is_some() || channel.send_ordered,
                            });
                        info.copies = info.copies.checked_add(1).ok_or_else(|| {
                            EncodingRefusal::NoEncoding(
                                "a canonical queued-copy count overflows".to_owned(),
                            )
                        })?;
                    }
                    queue_position = queue_position.saturating_add(1);
                }
            }

            let mut messages_by_group = BTreeMap::<
                (ProtocolNodeId, ThreadPath),
                Vec<(MessageOccurrence, QueuedMessage)>,
            >::new();
            for (message, info) in queued {
                messages_by_group
                    .entry((message.node(), message.path().clone()))
                    .or_default()
                    .push((message, info));
            }

            for ((node, path), messages) in &mut messages_by_group {
                let ordered = messages.first().is_some_and(|(_, info)| info.ordered);
                if ordered {
                    messages.sort_by_key(|(_, info)| info.first_position);
                } else {
                    messages.sort_by(|(left_message, left), (right_message, right)| {
                        (&left.payload, left.copies, left_message).cmp(&(
                            &right.payload,
                            right.copies,
                            right_message,
                        ))
                    });
                }
                for (ordinal, (message, _)) in messages.iter().enumerate() {
                    let ordinal = u64::try_from(ordinal).map_err(|_| {
                        EncodingRefusal::NoEncoding(
                            "a canonical message ordinal overflows".to_owned(),
                        )
                    })?;
                    message_labels.insert(message.clone(), message.with_ordinal(ordinal));
                }
                let _ = (node, path);
            }

            // Rewrite queued message occurrences first. Keep the concrete
            // entry identity alongside it so concrete replay can lift a
            // quotient edge without changing any reported label.
            let mut entry_origins =
                BTreeMap::<(MessageOccurrence, u64), (MessageOccurrence, u64, Enc)>::new();
            for queue in instance.queues.values_mut() {
                for entry in queue {
                    let concrete_message = entry.message.clone();
                    let concrete_delivery = entry.delivery;
                    let concrete_enc = entry.enc.clone();
                    let canonical_message = message_labels
                        .get(&concrete_message)
                        .cloned()
                        .unwrap_or_else(|| concrete_message.clone());
                    entry.enc = entry_with_labels(
                        entry,
                        &canonical_message,
                        concrete_delivery,
                        self.model.typing(),
                    );
                    entry.bytes = encode(&entry.enc)?;
                    entry.message = canonical_message.clone();
                    entry_origins.insert(
                        (canonical_message, concrete_delivery),
                        (concrete_message, concrete_delivery, concrete_enc),
                    );
                }
            }

            // Canonical message ordinal bounds count only messages with a
            // queued copy. A send node/path with a pending interval send keeps
            // its messages and counter concrete.
            let groups: BTreeSet<_> = instance
                .message_ordinals
                .keys()
                .cloned()
                .chain(messages_by_group.keys().cloned())
                .collect();
            let concrete_ordinals = std::mem::take(&mut instance.message_ordinals);
            for (node, path) in groups {
                let concrete = concrete_ordinals
                    .get(&(node, path.clone()))
                    .copied()
                    .unwrap_or(0);
                let group = (node, path.clone());
                let channel = self
                    .program
                    .messages
                    .get(&node)
                    .and_then(|send| self.program.channels.get(send.channel))
                    .ok_or_else(|| {
                        EncodingRefusal::NoEncoding(
                            "a message ordinal names no send channel".to_owned(),
                        )
                    })?;
                if message_reads.contains(&channel.name) || pending_groups.contains(&group) {
                    if concrete != 0 {
                        instance.message_ordinals.insert(group, concrete);
                    }
                    continue;
                }
                let count = messages_by_group.get(&group).map_or(0, Vec::len);
                let count = u64::try_from(count).map_err(|_| {
                    EncodingRefusal::NoEncoding("a canonical message ordinal overflows".to_owned())
                })?;
                next_ordinals.insert(group.clone(), (concrete, count));
                if count > 0 {
                    instance.message_ordinals.insert(group, count);
                }
            }

            // Delivery labels are canonicalized per message, independently
            // of whether message ordinals are read. Unordered queue copies
            // sort by entry bytes without the delivery member.
            let mut copies = BTreeMap::<MessageOccurrence, u64>::new();
            for (key, queue) in &mut instance.queues {
                let channel = &self.program.channels[key.channel];
                let delivery_read = delivery_reads.contains(&channel.name);
                if key.fifo.is_none() && !channel.send_ordered && !delivery_read {
                    let mut ordered = queue
                        .drain(..)
                        .map(|entry| Ok((encode(&without_delivery(&entry))?, entry)))
                        .collect::<Result<Vec<_>, EncodingRefusal>>()?;
                    ordered.sort_by(|left, right| left.0.cmp(&right.0));
                    *queue = ordered.into_iter().map(|(_, entry)| entry).collect();
                }
                for entry in queue.iter_mut() {
                    let pending_group = pending_groups
                        .contains(&(entry.message.node(), entry.message.path().clone()));
                    if delivery_read || pending_group {
                        let origin = entry_origins
                            .remove(&(entry.message.clone(), entry.delivery))
                            .ok_or_else(|| {
                                EncodingRefusal::NoEncoding(
                                    "a canonical queue entry lost its concrete identity".to_owned(),
                                )
                            })?;
                        entries.push((origin.2, entry.enc.clone()));
                        continue;
                    }
                    let next = copies.entry(entry.message.clone()).or_default();
                    let canonical_delivery = *next;
                    delivery_labels.insert(
                        (
                            entry_origins
                                .get(&(entry.message.clone(), entry.delivery))
                                .map(|origin| origin.0.clone())
                                .unwrap_or_else(|| entry.message.clone()),
                            entry.delivery,
                        ),
                        canonical_delivery,
                    );
                    let origin = entry_origins
                        .remove(&(entry.message.clone(), entry.delivery))
                        .ok_or_else(|| {
                            EncodingRefusal::NoEncoding(
                                "a canonical queue entry lost its concrete identity".to_owned(),
                            )
                        })?;
                    entry.delivery = canonical_delivery;
                    entry.enc = entry_with_labels(
                        entry,
                        &entry.message,
                        canonical_delivery,
                        self.model.typing(),
                    );
                    entry.bytes = encode(&entry.enc)?;
                    entries.push((origin.2, entry.enc.clone()));
                    *next = next.checked_add(1).ok_or_else(|| {
                        EncodingRefusal::NoEncoding(
                            "a canonical delivery count overflows".to_owned(),
                        )
                    })?;
                }
                if key.fifo.is_none() && !channel.send_ordered && !delivery_read {
                    queue.sort_by(|left, right| left.bytes.cmp(&right.bytes));
                }
            }

            // The next fresh label is the number of queued copies. Removed
            // messages have no retained bound in the quotient.
            let old_next = std::mem::take(&mut instance.next_deliveries);
            for (message, next) in old_next {
                let channel = self
                    .program
                    .messages
                    .get(&message.node())
                    .and_then(|send| self.program.channels.get(send.channel))
                    .ok_or_else(|| {
                        EncodingRefusal::NoEncoding(
                            "a next-delivery bound names no send channel".to_owned(),
                        )
                    })?;
                let pending_group =
                    pending_groups.contains(&(message.node(), message.path().clone()));
                if delivery_reads.contains(&channel.name) || pending_group {
                    instance.next_deliveries.insert(message, next);
                } else {
                    let canonical_message = message_labels
                        .get(&message)
                        .cloned()
                        .unwrap_or_else(|| message.clone());
                    let canonical_next = copies.get(&canonical_message).copied().unwrap_or(0);
                    if canonical_next > 0 {
                        delivery_labels.insert((message, next), canonical_next);
                    }
                }
            }
            for (message, count) in copies {
                if pending_groups.contains(&(message.node(), message.path().clone())) {
                    continue;
                }
                if count > 0 {
                    instance.next_deliveries.insert(message, count);
                }
            }
        }

        Ok(CanonicalProtocol {
            state: canonical,
            message_labels,
            delivery_labels,
            next_ordinals,
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
        if let (Some(message_reads), Some(delivery_reads)) =
            (&self.message_reads, &self.delivery_reads)
        {
            for (_, state) in &mut expansion.successors {
                *state = self
                    .canonical_state_with_reads(state, message_reads, delivery_reads)
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
        let (Some(message_reads), Some(delivery_reads)) =
            (&self.message_reads, &self.delivery_reads)
        else {
            return Ok(None);
        };
        if self.program.channels.iter().all(|channel| {
            message_reads.contains(&channel.name) && delivery_reads.contains(&channel.name)
        }) {
            return Ok(None);
        }
        let canonical = self
            .canonical_protocol(source, message_reads, delivery_reads)
            .map_err(|_| super::broken())?;
        let wanted = self
            .cycle_key_with_reads(target, message_reads, delivery_reads)
            .map_err(|_| super::broken())?;
        let expansion = self.expand_bounded(source)?.0;
        for (candidate, state) in expansion.successors {
            let renamed = candidate.relabel_cycle_labels(
                &canonical.entries,
                |message| canonical.message(message),
                |delivery| {
                    let message = canonical.message(delivery.message());
                    let index = canonical
                        .delivery_labels
                        .get(&(delivery.message().clone(), delivery.index()))
                        .copied()
                        .unwrap_or_else(|| delivery.index());
                    DeliveryIdentity::new(message, index)
                },
            );
            if renamed == *transition
                && self
                    .cycle_key_with_reads(&state, message_reads, delivery_reads)
                    .map_err(|_| super::broken())?
                    == wanted
            {
                return Ok(Some((candidate, state)));
            }
        }
        Err(ProtocolRefusal::fault("a-cycle-edge-has-a-concrete-successor").into())
    }
}

#[derive(Clone)]
struct QueuedMessage {
    payload: Vec<u8>,
    copies: u64,
    first_position: usize,
    ordered: bool,
}

fn without_delivery(entry: &Entry) -> Enc {
    let mut encoded = entry.enc.clone();
    if let Enc::Object(members) = &mut encoded {
        members.retain(|(name, _)| *name != "delivery");
    }
    encoded
}

fn entry_with_labels(
    entry: &Entry,
    message: &MessageOccurrence,
    delivery: u64,
    typing: Typing<'_>,
) -> Enc {
    Enc::tagged(
        "queue-entry",
        vec![
            ("message", message.enc()),
            ("delivery", Enc::text(delivery.to_string())),
            ("payload", value_enc(&entry.payload, typing)),
        ],
    )
}

fn encode(value: &Enc) -> Result<Vec<u8>, EncodingRefusal> {
    crate::simulation::key::canonical_bytes(value, super::identity_limits())
}
