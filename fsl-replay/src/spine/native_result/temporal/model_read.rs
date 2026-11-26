// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural model-arm members of FR-267's temporal result reader.

use base64::Engine;
use filament_exact::{ObjectId, ObjectReference, UniverseId};
use filament_semantic_value::model_transition::ModelTransition;
use serde::{Deserialize, Deserializer};

use super::EffectiveIdRead;

/// A model state key's canonical unpadded base64url spelling.
/// Reading it checks the encoding, not membership in the supplied model.
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StateRead {
    #[serde(deserialize_with = "state_key")]
    key: String,
}

impl StateRead {
    /// The exact canonical base64url spelling in the result.
    pub fn key(&self) -> &str {
        &self.key
    }
}

fn state_key<'de, D: Deserializer<'de>>(decoder: D) -> Result<String, D::Error> {
    let key = String::deserialize(decoder)?;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&key)
        .map_err(serde::de::Error::custom)?;
    Ok(key)
}

/// The existing canonical typed reference bound to a model clause's `over`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBindingRead {
    #[serde(rename = "type")]
    kind: ReferenceTag,
    #[serde(deserialize_with = "universe")]
    universe: UniverseId,
    object_type: EffectiveIdRead,
    #[serde(deserialize_with = "object_identity")]
    identity: ObjectId,
}

impl ModelBindingRead {
    /// Restore the canonical reference without asserting model membership.
    pub fn typed(&self) -> ObjectReference {
        let ReferenceTag::Reference = self.kind;
        ObjectReference::new(
            self.universe.clone(),
            self.object_type.typed(),
            self.identity.clone(),
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ReferenceTag {
    Reference,
}

fn object_identity<'de, D: Deserializer<'de>>(decoder: D) -> Result<ObjectId, D::Error> {
    ObjectId::new(String::deserialize(decoder)?).map_err(serde::de::Error::custom)
}

fn universe<'de, D: Deserializer<'de>>(decoder: D) -> Result<UniverseId, D::Error> {
    let value = serde_json::Value::deserialize(decoder)?;
    crate::spine::identity_form::read_universe(&value, &mut crate::spine::run_value::Path::new())
        .map_err(|_| serde::de::Error::custom("expected a closed universe node identity"))
}

/// One structurally checked source-model transition and its post-state key.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelStepRead {
    /// The source operation or terminal stutter identity.
    pub transition: ModelTransitionRead,
    /// The recorded post-state key, without successor authentication.
    pub post_state: StateRead,
}

/// The two transition identities admitted by an authored model clause run.
#[derive(Debug)]
pub enum ModelTransitionRead {
    /// The existing closed, typed source-operation identity.
    Transition(ModelTransition),
    /// The one terminal stutter identity, which carries no other members.
    TerminalStutter,
}

impl<'de> Deserialize<'de> for ModelTransitionRead {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(decoder)?;
        if value.get("type").and_then(serde_json::Value::as_str) == Some("terminal-stutter") {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Terminal {
                #[serde(rename = "type")]
                kind: TerminalTag,
            }
            #[derive(Deserialize)]
            enum TerminalTag {
                #[serde(rename = "terminal-stutter")]
                TerminalStutter,
            }
            let terminal: Terminal =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            let TerminalTag::TerminalStutter = terminal.kind;
            return Ok(Self::TerminalStutter);
        }
        crate::certificate::task_product::source_transition(&value, "")
            .map(Self::Transition)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::super::{decode, validate_counterexample, CounterexampleRead, TemporalRecord};
    use super::*;

    // These fixtures establish structural read behavior only. A valid key or
    // reference can still fail the model's separate semantic replay checks.
    fn binding() -> Value {
        let declaration = filament_checked::key::DeclarationKey {
            package: "test/counters".to_owned(),
            node: "test/counters/Counter".to_owned(),
        };
        let node = fsl_semantics::model::key::object_type_node_key(&declaration)
            .expect("the shared model declaration constructor admits this fixture");
        let reference = ObjectReference::new(
            UniverseId::new(node),
            filament_exact::EffectiveId::new(
                None,
                filament_exact::OriginalDeclarationKey::new(declaration.package, declaration.node),
            ),
            ObjectId::new("a").expect("a nonempty authored object identity"),
        );
        fsl_eval::simulation::model::value_json(&filament_exact::Value::Reference(reference))
            .expect("the existing canonical producer encodes the reference")
    }

    fn state(octets: &[u8]) -> Value {
        json!({"key": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(octets)})
    }

    fn step() -> Value {
        json!({"transition": {"type": "transition", "operation": "increment", "receiver": binding(),
            "arguments": [{"type": "integer", "value": "1"}]}, "post_state": state(b"[1,[]]")})
    }

    fn model() -> Value {
        json!({"trace": "model", "kind": "formula", "initial_state": 0,
            "prefix": [step()], "loop": [step()], "over_binding": binding(),
            "fairness": [], "interval": {"lower": "0", "upper": "1"}, "trace_position": 1})
    }

    fn read(
        counterexample: &Value,
        position: u64,
        reason: Option<&str>,
    ) -> Result<CounterexampleRead, String> {
        let kind = if reason.is_some() {
            "undefined"
        } else {
            "violated"
        };
        let record =
            decode(&json!({"kind": kind, "position": position, "counterexample": counterexample}))
                .map_err(|error| error.to_string())?;
        let counterexample = match record {
            TemporalRecord::Violated { counterexample, .. }
            | TemporalRecord::Undefined {
                counterexample: Some(counterexample),
                ..
            } => counterexample,
            other => panic!("expected a carried counterexample: {other:?}"),
        };
        validate_counterexample(&counterexample, position, reason)
            .map_err(|error| error.to_string())?;
        Ok(counterexample)
    }

    /// FR-267-AC-30, FR-120-AC-40: the model arm reads a source step's
    /// declaration identity text as that identity.
    #[test]
    #[trace("TC-1160", "FR-267-AC-30", "TC-471", "FR-120-AC-40")]
    fn a_source_step_reads_back_its_declaration_identity() {
        let identity =
            r#"{"node":"ix://test/counters/Counter/increment","package":"test/counters"}"#;
        let mut value = model();
        value["prefix"][0]["transition"] = json!({"type": "transition", "operation": identity,
            "receiver": null, "arguments": []});
        let read = read(&value, 1, None).expect("a structurally valid model counterexample");
        let step = read.model_prefix().next().expect("the prefix step");
        let ModelTransitionRead::Transition(transition) = &step.transition else {
            panic!("a transition");
        };
        assert_eq!(transition.operation(), identity);
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-30")]
    fn model_reader_retains_typed_steps_binding_boundary_and_report_position() {
        let value = model();
        let read = read(&value, 1, None).expect("a structurally valid model counterexample");
        assert!(read.is_model());
        assert_eq!(read.model_initial_state(), Some(0));
        assert_eq!(read.trace_position(), 1);
        assert_eq!(read.interval(), Some(("0", "1")));
        assert_eq!(read.prefix().count(), 0);
        assert_eq!(read.loop_documents().count(), 0);
        assert_eq!(read.model_prefix().count(), 1);
        assert_eq!(read.model_loop().count(), 1);
        let step = read
            .model_prefix()
            .next()
            .expect("the recorded prefix step");
        assert_eq!(
            step.post_state.key(),
            value["prefix"][0]["post_state"]["key"].as_str().unwrap()
        );
        assert!(matches!(
            step.transition,
            ModelTransitionRead::Transition(_)
        ));
        let reference = read
            .model_over_binding()
            .expect("the typed binding")
            .typed();
        let encoded = fsl_eval::simulation::model::reference_canonical_text(&reference)
            .expect("canonical reference");
        assert_eq!(serde_json::from_str::<Value>(&encoded).unwrap(), binding());
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-30")]
    fn model_reader_accepts_initial_terminal_empty_prefix() {
        let mut value = model();
        value["prefix"] = json!([]);
        value["loop"] =
            json!([{"transition": {"type": "terminal-stutter"}, "post_state": state(b"[0,[]]")}]);
        value["trace_position"] = json!(0);
        let read = read(&value, 0, None).expect("the initial state can be terminal");
        assert_eq!(read.model_prefix().count(), 0);
        let terminal = read.model_loop().next().expect("one terminal stutter");
        assert!(matches!(
            terminal.transition,
            ModelTransitionRead::TerminalStutter
        ));
        assert_eq!(
            terminal.post_state.key(),
            state(b"[0,[]]")["key"].as_str().unwrap()
        );
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-31")]
    fn model_reader_rejects_null_mistyped_missing_and_out_of_range_required_members() {
        let base = model();
        for member in [
            "trace",
            "kind",
            "initial_state",
            "prefix",
            "loop",
            "over_binding",
            "fairness",
            "interval",
            "trace_position",
        ] {
            let mut value = base.clone();
            value.as_object_mut().unwrap().remove(member);
            assert!(read(&value, 1, None).is_err(), "missing {member}");
        }
        for value in [
            Value::Null,
            json!("0"),
            json!(-1),
            json!(0.5),
            json!(super::super::MAX_COUNT + 1),
        ] {
            let mut counterexample = base.clone();
            counterexample["initial_state"] = value.clone();
            assert!(
                read(&counterexample, 1, None).is_err(),
                "initial_state={value}"
            );
        }
        for value in [
            Value::Null,
            json!({"type": "reference"}),
            json!(true),
            json!({"type": "integer", "value": "1"}),
        ] {
            let mut counterexample = base.clone();
            counterexample["over_binding"] = value.clone();
            assert!(
                read(&counterexample, 1, None).is_err(),
                "over_binding={value}"
            );
        }
        assert!(read(&base, 0, None).unwrap_err().contains("trace_position"));
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-31")]
    fn model_and_observed_steps_cannot_cross_arms_or_mix_members() {
        let document = json!({"document": {"authority": "test", "identity": "s0"}});
        for member in ["prefix", "loop"] {
            for foreign in [document.clone(), json!({"sample": "0"})] {
                let mut value = model();
                value[member] = json!([foreign]);
                assert!(read(&value, 1, None).unwrap_err().contains(member));
            }
            let mut mixed = model();
            mixed[member][0]["sample"] = json!("0");
            assert!(read(&mixed, 1, None).is_err(), "mixed {member} step");
        }
        let observed = json!({"trace": "observed", "kind": "formula", "initial_state": null,
            "prefix": [document], "loop": [], "over_binding": null,
            "fairness": [], "interval": null, "trace_position": 0});
        let parsed = read(&observed, 0, None).expect("the observed control");
        assert!(!parsed.is_model());
        assert_eq!(parsed.prefix().collect::<Vec<_>>(), [("test", "s0")]);
        for member in ["prefix", "loop"] {
            let mut value = observed.clone();
            value[member] = json!([step()]);
            assert!(
                read(&value, 0, None).is_err(),
                "observed {member} model step"
            );
        }
        for (member, replacement) in [("initial_state", json!(0)), ("over_binding", binding())] {
            let mut value = observed.clone();
            value[member] = replacement;
            assert!(read(&value, 0, None).is_err(), "observed {member}");
        }
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-31")]
    fn model_reader_rejects_malformed_keys_and_transition_payloads() {
        for key in ["YQ==", "YR", "+/8", "a", "*"] {
            let mut value = model();
            value["prefix"][0]["post_state"]["key"] = json!(key);
            assert!(
                read(&value, 1, None).unwrap_err().contains("key"),
                "key {key}"
            );
        }
        for member in ["type", "operation", "receiver", "arguments"] {
            let mut value = model();
            value["prefix"][0]["transition"]
                .as_object_mut()
                .unwrap()
                .remove(member);
            assert!(
                read(&value, 1, None).is_err(),
                "missing transition {member}"
            );
        }
        for (member, replacement) in [
            ("operation", json!("")),
            ("receiver", json!({"type":"integer","value":1})),
            ("arguments", json!([{"type":"unknown"}])),
        ] {
            let mut value = model();
            value["prefix"][0]["transition"][member] = replacement;
            assert!(
                read(&value, 1, None).is_err(),
                "malformed transition {member}"
            );
        }
        for pointer in [
            "/extra",
            "/prefix/0/extra",
            "/prefix/0/post_state/extra",
            "/prefix/0/transition/extra",
            "/over_binding/extra",
        ] {
            let mut value = model();
            let (parent, name) = pointer.rsplit_once('/').unwrap();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(name.to_owned(), json!(true));
            assert!(read(&value, 1, None).is_err(), "unknown member {pointer}");
        }
        let mut value = model();
        value["loop"][0]["transition"] = json!({"type": "terminal-stutter", "operation": "extra"});
        assert!(read(&value, 1, None).is_err(), "closed terminal identity");
    }

    #[test]
    #[trace("TC-1160", "FR-267-AC-30", "FR-267-AC-31")]
    fn model_undefined_reader_requires_its_finite_prefix_state_key_and_cause() {
        let mut value = model();
        value["kind"] = json!({"type": "undefined-evaluation", "where": {
            "position": 1, "state_key": state(b"[1,[]]"), "locus": {"formula_node": 0}}, "cause": "division-by-zero"});
        value["loop"] = json!([]);
        value["interval"] = Value::Null;
        let parsed =
            read(&value, 1, Some("division-by-zero")).expect("a finite model undefined prefix");
        assert_eq!(
            parsed.undefined_state().expect("the model locus").key(),
            state(b"[1,[]]")["key"].as_str().unwrap()
        );
        for identity in [
            json!({"document": {"authority": "test", "identity": "s0"}}),
            json!({"sample": "1"}),
        ] {
            let mut changed = value.clone();
            let point = changed["kind"]["where"].as_object_mut().unwrap();
            point.remove("state_key");
            point.extend(identity.as_object().unwrap().clone());
            assert!(
                read(&changed, 1, Some("division-by-zero")).is_err(),
                "observed undefined locus"
            );
        }
        let mut changed = value.clone();
        changed["kind"]["where"]["state_key"] = state(b"[2,[]]");
        assert!(
            read(&changed, 1, Some("division-by-zero")).is_err(),
            "wrong final prefix key"
        );
        assert!(
            read(&value, 1, Some("none-value")).is_err(),
            "wrong recorded cause"
        );
        changed = value.clone();
        changed["loop"] = json!([step()]);
        assert!(
            read(&changed, 1, Some("division-by-zero")).is_err(),
            "undefined loop"
        );
        changed = value;
        changed["prefix"] = json!([]);
        changed["trace_position"] = json!(0);
        changed["kind"]["where"]["position"] = json!(0);
        assert!(
            read(&changed, 0, Some("division-by-zero")).is_ok(),
            "undefined at the initial state"
        );
    }
}
