// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrow the original protocol identity fields into an admitted output sink.
use super::super::borrowed_wire::{tagged, value, ProtocolWireSink};
use super::*;
use serde_json::Value as Json;

fn thread<S: ProtocolWireSink>(s: &mut S, path: &ThreadPath) -> Result<Json, S::Error> {
    let mut out = tagged(s, "thread-path")?;
    s.field(&mut out, "instance", |s| s.unsigned(path.instance))?;
    s.field(&mut out, "steps", |s| {
        let mut out = s.array(path.steps.len())?;
        for (index, step) in path.steps.iter().enumerate() {
            s.element(&mut out, index, |s| {
                let mut out = s.object()?;
                match step {
                    PathStep::Branch {
                        node,
                        branch,
                        ordinal,
                    } => {
                        s.field(&mut out, "node", |s| s.string(node))?;
                        s.field(&mut out, "branch", |s| s.string(branch))?;
                        s.field(&mut out, "ordinal", |s| s.unsigned(*ordinal))?;
                    }
                    PathStep::Compensation { template, ordinal } => {
                        s.field(&mut out, "template", |s| s.string(template))?;
                        s.field(&mut out, "ordinal", |s| s.unsigned(*ordinal))?;
                    }
                }
                Ok(out)
            })?;
        }
        Ok(out)
    })?;
    Ok(out)
}

fn message<S: ProtocolWireSink>(s: &mut S, message: &MessageOccurrence) -> Result<Json, S::Error> {
    let mut out = tagged(s, "message-occurrence")?;
    s.field(&mut out, "instance", |s| s.unsigned(message.instance))?;
    s.field(&mut out, "node", |s| s.string(&message.node))?;
    s.field(&mut out, "path", |s| thread(s, &message.path))?;
    s.field(&mut out, "ordinal", |s| s.unsigned(message.ordinal))?;
    Ok(out)
}

fn registration<S: ProtocolWireSink>(
    s: &mut S,
    registration: &RegistrationKey,
) -> Result<Json, S::Error> {
    let mut out = tagged(s, "registration")?;
    s.field(&mut out, "template", |s| s.string(&registration.template))?;
    s.field(&mut out, "thread", |s| thread(s, &registration.thread))?;
    s.field(&mut out, "ordinal", |s| s.unsigned(registration.ordinal))?;
    Ok(out)
}

fn arguments<S: ProtocolWireSink>(s: &mut S, arguments: &[Enc]) -> Result<Json, S::Error> {
    let mut out = s.array(arguments.len())?;
    for (index, argument) in arguments.iter().enumerate() {
        s.element(&mut out, index, |s| value(s, argument, 0))?;
    }
    Ok(out)
}

impl ProtocolTransition {
    /// Project every wire identity field directly from this runtime identity.
    /// No cached/native key or retained encoding tree is changed or cloned.
    ///
    /// # Errors
    /// The sink's typed encoding/resource failure.
    pub fn write_borrowed<S: ProtocolWireSink>(&self, s: &mut S) -> Result<Json, S::Error> {
        let mut out = tagged(s, "protocol-transition")?;
        s.field(&mut out, "step", |s| s.string(self.kind.text()))?;
        if let Some(path) = &self.thread {
            s.field(&mut out, "thread", |s| thread(s, path))?;
        }
        if let Some(node) = &self.node {
            s.field(&mut out, "node", |s| s.string(node))?;
        }
        if let Some(actor) = &self.actor {
            s.field(&mut out, "actor", |s| s.reference(&actor.object))?;
        }
        if let Some(found) = &self.message {
            s.field(&mut out, "message", |s| message(s, found))?;
        }
        if let Some(delivery) = &self.delivery {
            s.field(&mut out, "delivery", |s| {
                let mut out = tagged(s, "delivery-identity")?;
                s.field(&mut out, "message", |s| message(s, &delivery.message))?;
                s.field(&mut out, "index", |s| s.unsigned(delivery.index))?;
                Ok(out)
            })?;
        }
        if let Some(effect) = &self.business_effect {
            s.field(&mut out, "business_effect", |s| {
                let mut out = tagged(s, "business-effect-occurrence")?;
                s.field(&mut out, "instance", |s| s.unsigned(effect.instance))?;
                s.field(&mut out, "node", |s| {
                    let node = u64::try_from(effect.node.0)
                        .map_err(|_| s.unsupported("business effect node ordinal"))?;
                    s.unsigned(node)
                })?;
                s.field(&mut out, "path", |s| thread(s, &effect.path))?;
                s.field(&mut out, "ordinal", |s| s.unsigned(effect.ordinal))?;
                Ok(out)
            })?;
        }
        match &self.members {
            Members::Attempt {
                operation,
                receiver,
                arguments: args,
                choice,
                ..
            } => {
                s.field(&mut out, "operation", |s| s.string(operation))?;
                s.field(&mut out, "receiver", |s| value(s, receiver, 0))?;
                s.field(&mut out, "arguments", |s| arguments(s, args))?;
                if let Some(choice) = choice {
                    s.field(&mut out, "memory_choice", |s| {
                        let mut out = s.object()?;
                        if let Some(depth) = choice.read_depth {
                            s.field(&mut out, "read_depth", |s| s.unsigned(depth))?;
                        }
                        if let Some(depth) = choice.slot_depth {
                            s.field(&mut out, "slot_depth", |s| s.unsigned(depth))?;
                        }
                        Ok(out)
                    })?;
                }
            }
            Members::Recommendation {
                role,
                operation,
                receiver,
                arguments: args,
            } => {
                s.field(&mut out, "recommendation", |s| {
                    let mut out = tagged(s, "protocol-recommendation")?;
                    s.field(&mut out, "node", |s| {
                        s.string(self.node.as_deref().unwrap_or_default())
                    })?;
                    s.field(&mut out, "role", |s| s.string(role))?;
                    s.field(&mut out, "instance", |s| match &self.thread {
                        Some(path) => s.unsigned(path.instance),
                        None => Err(s.unsupported("a-recommendation-has-a-thread")),
                    })?;
                    s.field(&mut out, "operation", |s| s.string(operation))?;
                    s.field(&mut out, "receiver", |s| value(s, receiver, 0))?;
                    s.field(&mut out, "arguments", |s| arguments(s, args))?;
                    if let Some(actor) = &self.actor {
                        s.field(&mut out, "actor", |s| s.reference(&actor.object))?;
                    }
                    Ok(out)
                })?;
            }
            Members::Control => {}
            Members::Record {
                record,
                recovery_summary,
            } if self.message.is_none()
                && self.delivery.is_none()
                && self.business_effect.is_none() =>
            {
                s.field(&mut out, "record", |s| value(s, record, 0))?;
                if let Some(summary) = recovery_summary {
                    s.field(&mut out, "recovery_summary", |s| s.string(summary))?;
                }
            }
            Members::Record { .. } => {}
            Members::Channel {
                name,
                instance,
                entry,
            } => {
                s.field(&mut out, "channel", |s| {
                    let mut out = tagged(s, "channel")?;
                    s.field(&mut out, "name", |s| s.string(name))?;
                    s.field(&mut out, "instance", |s| s.unsigned(*instance))?;
                    Ok(out)
                })?;
                s.field(&mut out, "entry", |s| value(s, entry, 0))?;
            }
            Members::Memory { identity } => {
                s.field(&mut out, "memory_step", |s| {
                    let mut out = tagged(s, "memory-step")?;
                    s.field(&mut out, "model", |s| s.string(identity.model.label()))?;
                    s.field(&mut out, "step", |s| s.string(identity.step))?;
                    s.field(&mut out, "thread", |s| thread(s, &identity.thread))?;
                    Ok(out)
                })?;
            }
            Members::Fence { ordering } => {
                s.field(&mut out, "ordering", |s| s.string(ordering.label()))?
            }
            Members::CAttempt {
                operation,
                receiver,
                arguments: args,
                record,
                template,
                registration: key,
                ..
            }
            | Members::COutcome {
                operation,
                receiver,
                arguments: args,
                record,
                template,
                registration: key,
                ..
            } => {
                s.field(&mut out, "operation", |s| s.string(operation))?;
                s.field(&mut out, "receiver", |s| value(s, receiver, 0))?;
                s.field(&mut out, "arguments", |s| arguments(s, args))?;
                s.field(&mut out, "record", |s| value(s, record, 0))?;
                s.field(&mut out, "template", |s| s.string(template))?;
                s.field(&mut out, "registration", |s| registration(s, key))?;
            }
            Members::CEnd {
                template,
                registration: key,
                record,
            } => {
                s.field(&mut out, "template", |s| s.string(template))?;
                s.field(&mut out, "registration", |s| registration(s, key))?;
                if let Some(record) = record {
                    s.field(&mut out, "record", |s| value(s, record, 0))?;
                }
            }
            Members::Role {
                role,
                object,
                instance,
            } => {
                s.field(&mut out, "role", |s| s.string(role))?;
                s.field(&mut out, "object", |s| s.reference(object))?;
                s.field(&mut out, "instance", |s| s.unsigned(*instance))?;
            }
        }
        Ok(out)
    }
}

impl SendRecord {
    /// Project payload/displacement leaves without cloning their stored trees.
    ///
    /// # Errors
    /// The sink's typed encoding/resource failure.
    pub fn write_borrowed<S: ProtocolWireSink>(&self, s: &mut S) -> Result<Json, S::Error> {
        let mut out = tagged(s, "send-record")?;
        s.field(&mut out, "channel", |s| s.string(&self.channel))?;
        s.field(&mut out, "payload", |s| value(s, &self.payload, 0))?;
        if let Some(interval) = &self.interval {
            s.field(&mut out, "count", |s| s.unsigned(interval.count))?;
            s.field(&mut out, "entries", |s| {
                let len = interval.outcomes.len().min(interval.displacements.len());
                let mut out = s.array(len)?;
                for (index, (outcome, displaced)) in interval
                    .outcomes
                    .iter()
                    .zip(&interval.displacements)
                    .enumerate()
                {
                    s.element(&mut out, index, |s| {
                        let mut out = s.object()?;
                        s.field(&mut out, "outcome", |s| s.string(outcome.text()))?;
                        s.field(&mut out, "displaced", |s| match displaced {
                            Some(displaced) => value(s, displaced, 0),
                            None => Ok(Json::Null),
                        })?;
                        Ok(out)
                    })?;
                }
                Ok(out)
            })?;
            s.field(&mut out, "pending", |s| s.unsigned(interval.pending))?;
        } else {
            s.field(&mut out, "outcome", |s| s.string(self.outcome.text()))?;
            s.field(&mut out, "displaced", |s| match &self.displaced {
                Some(displaced) => value(s, displaced, 0),
                None => Ok(Json::Null),
            })?;
        }
        Ok(out)
    }
}

impl BlockedThread {
    /// Project a native blocked-thread record without an intermediate tree.
    ///
    /// # Errors
    /// The sink's typed encoding/resource failure.
    pub fn write_borrowed<S: ProtocolWireSink>(&self, s: &mut S) -> Result<Json, S::Error> {
        let mut out = s.object()?;
        s.field(&mut out, "thread", |s| thread(s, &self.path))?;
        s.field(&mut out, "rest", |s| {
            let mut out = tagged(s, "rest")?;
            s.field(&mut out, "kind", |s| s.string(self.rest_point.kind.text()))?;
            s.field(&mut out, "node", |s| match &self.rest_point.node {
                Some(node) => s.string(node),
                None => Ok(Json::Null),
            })?;
            Ok(out)
        })?;
        s.field(&mut out, "cause", |s| {
            let (tag, field) = match &self.cause {
                WaitCause::Precondition { clause } => {
                    ("precondition", Some(("clause", clause.as_str())))
                }
                WaitCause::NoPostState => ("no-post-state", None),
                WaitCause::Constraint => ("constraint", None),
                WaitCause::EmptyQueue { channel } => {
                    ("empty-queue", Some(("channel", channel.as_str())))
                }
                WaitCause::HeadBlocked { channel } => {
                    ("head-blocked", Some(("channel", channel.as_str())))
                }
                WaitCause::FullQueue { channel } => {
                    ("full-queue", Some(("channel", channel.as_str())))
                }
                WaitCause::Displaced { channel } => {
                    ("displaced", Some(("channel", channel.as_str())))
                }
                WaitCause::CheckFalse { node } => ("check-false", Some(("node", node.as_str()))),
                WaitCause::JoinWaiting { .. } => ("join-waiting", None),
                WaitCause::FinishWaiting { .. } => ("finish-waiting", None),
                WaitCause::CommitClosed { template } => {
                    ("commit-closed", Some(("template", template.as_str())))
                }
                WaitCause::NoRoleInstance { role } => {
                    ("no-role-instance", Some(("role", role.as_str())))
                }
                WaitCause::Gate => ("gate", None),
            };
            let mut out = tagged(s, tag)?;
            if let Some((name, value)) = field {
                s.field(&mut out, name, |s| s.string(value))?;
            }
            if let WaitCause::JoinWaiting { threads } | WaitCause::FinishWaiting { threads } =
                &self.cause
            {
                s.field(&mut out, "threads", |s| {
                    let mut out = s.array(threads.len())?;
                    for (index, path) in threads.iter().enumerate() {
                        s.element(&mut out, index, |s| thread(s, path))?;
                    }
                    Ok(out)
                })?;
            }
            Ok(out)
        })?;
        Ok(out)
    }
}
