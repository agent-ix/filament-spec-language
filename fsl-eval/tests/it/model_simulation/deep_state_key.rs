// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-735 step 2 (FR-262-AC-2) and TC-729 step 1 (FR-259-AC-3): the state
//! key of a 100,000-long `List` record, taken through the public
//! model-simulation snapshot path on a 512 KiB stack, equals the RFC 8785
//! text of its canonical JSON form and matches the key from an 8 MiB stack.
//!
//! The expected bytes are written out here by hand (sorted members, no
//! whitespace, `null` for an absent presence-optional field and the bare
//! encoded value for a present one, per FSpec FR-181), never taken from the
//! encoder under test.

use super::*;

const DEPTH: usize = 100_000;

fn list_ident() -> String {
    ident("List")
}

fn list_field(name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("List/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

/// The counters package with a record value type
/// `List { head: Level; tail: List? }` and a `Counter.list: List?` field.
fn list_document() -> Vec<u8> {
    let mut document: Json =
        serde_json::from_slice(&counters_document_with(Vec::new(), false, Vec::new()))
            .expect("fixture JSON");
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({
        "kind": {"module": PACKAGE, "name": "record"}, "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "record",
            "members": {"identityFields": "optional"},
            "meaning": meaning::RECORD_VALUE_TYPE}}));
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &list_ident(),
        "record",
        json!({"supertypes": [], "fields": [
            list_field("head", &ident("Level"), "required"),
            list_field("tail", &list_ident(), "optional"),
        ]}),
    ));
    types
        .iter_mut()
        .find(|ty| ty["identity"] == ident("Counter"))
        .expect("Counter")["fields"]
        .as_array_mut()
        .expect("fields")
        .push(wire_field("list", &list_ident(), "optional"));
    serde_json::to_vec(&document).expect("serializes")
}

/// The snapshot's `list` value: `DEPTH` records of head 1, built iteratively.
fn snapshot_list() -> String {
    let open = format!(
        r#"{{"record":{{"type":"{}","fields":{{"head":{{"integer":"1"}},"tail":"#,
        list_ident()
    );
    let mut text = String::with_capacity(DEPTH * (open.len() + 16));
    for _ in 0..DEPTH - 1 {
        text.push_str(&open);
        text.push_str(r#"{"present":"#);
    }
    text.push_str(&open);
    text.push_str(r#"{"absent":{}}"#);
    text.push_str("}}}");
    for _ in 0..DEPTH - 1 {
        text.push_str("}}}}");
    }
    text
}

/// The RFC 8785 text of the same list as a key value.
fn expected_list() -> String {
    let name = serde_json::to_string(&format!(
        r#"{{"node":"{}","package":"{PACKAGE}"}}"#,
        list_ident()
    ))
    .expect("a string");
    let open = r#"{"fields":[{"name":"head","value":{"type":"integer","value":"1"}},{"name":"tail","value":"#
        .to_owned();
    let close = format!(r#"}}],"name":{name},"type":"record"}}"#);
    let mut text = String::with_capacity(DEPTH * (open.len() + close.len() + 8));
    for _ in 0..DEPTH - 1 {
        text.push_str(&open);
    }
    text.push_str(&open);
    text.push_str("null");
    text.push_str(&close);
    for _ in 0..DEPTH - 1 {
        text.push_str(&close);
    }
    text
}

fn on_stack<T: Send + 'static>(stack_size: usize, run: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(run)
        .expect("spawns")
        .join()
        .expect("no overflow or panic on the requested stack")
}

/// TC-735 step 2 (FR-262-AC-2): the 100,000-long list, held in a field of an
/// admitted initial snapshot, keys as `filament.simulation.state-key/v1`'s
/// RFC 8785 text, byte for byte, on a 512 KiB stack. A list one record
/// shorter keys differently, so the comparison is over the whole depth.
#[trace("TC-735", "FR-262-AC-2")]
#[trace("TC-729", "FR-259-AC-3")]
#[test]
fn a_100_000_long_list_keys_as_its_rfc_8785_text() {
    keys_as_its_rfc_8785_text(false);
}

/// TC-735 step 2 over the caller-bounded `ModelSystem::state_key`, the strict
/// form of the same requirement. The explicit 1 GiB identity bound admits
/// these canonical key bytes.
#[trace("TC-735", "FR-262-AC-2")]
#[trace("TC-729", "FR-259-AC-3")]
#[test]
fn a_100_000_long_list_keys_as_its_rfc_8785_text_through_the_caller_bounded_state_key() {
    keys_as_its_rfc_8785_text(true);
}

fn keys_as_its_rfc_8785_text(through_public_api: bool) {
    let small = on_stack(512 * 1024, move || state_key_bytes(through_public_api));
    if !through_public_api {
        let large = on_stack(8 * 1024 * 1024, || state_key_bytes(false));
        assert!(small == large, "the key differs across stack sizes");
    }
}

fn state_key_bytes(through_public_api: bool) -> Vec<u8> {
    with_deep_state(|system, state, world| {
        let raw = if through_public_api {
            system
                .state_key(
                    state,
                    fsl_foundation::IdentityLimits::default().with_input_bytes(1 << 30),
                )
                .expect("the key encodes")
                .0
        } else {
            encode_state(system, state, 1 << 30).expect("the key encodes")
        };
        assert_expected_state_key(&raw, world);
        raw
    })
}

fn with_deep_state<R>(
    run: impl FnOnce(&ModelSystem<'_>, &fsl_eval::simulation::model::ModelSimState, &World) -> R,
) -> R {
    let mut world =
        build_world_document(list_document(), "", false, false).expect("the unit checks");
    let counter = format!(
        r#"{{"key":"c1","type":"{}","fields":{{"value":{{"integer":"0"}},"label":{{"integer":"0"}},"next":{{"absent":{{}}}},"list":{{"present":{}}}}}}}"#,
        ident("Counter"),
        snapshot_list()
    );
    let snapshot = format!(
        r#"{{"format":"filament.state.snapshot/v1","identity":{{"authority":"test","identity":"deep"}},"observation":"current","anchor":{{"kind":"initialization","name":"start"}},"model":{{"identity":"{PACKAGE}"}},"populations":[{{"population":"{}","complete":true,"objects":[{counter}]}}]}}"#,
        ident("counters")
    );
    let bytes = snapshot.into_bytes();
    world.snapshots.insert(
        fsl_semantics::model::observation::document_reference(&bytes)
            .expect("a document names itself"),
        bytes,
    );
    let reference = DocumentRef {
        authority: "test".to_owned(),
        identity: "deep".to_owned(),
    };
    let mut request = request(&world, vec![reference], universe(&["c1"]), UNLIMITED, 1_000);
    request.observation = ObservationLimits::default()
        .with_document_bytes(1 << 30)
        .with_values_per_document(10_000_000);
    let system = ModelSystem::new(request).expect("the system admits the deep snapshot");
    let initial = system.initial();
    assert_eq!(initial.len(), 1);
    run(&system, &initial[0], &world)
}

fn encode_state(
    system: &ModelSystem<'_>,
    state: &fsl_eval::simulation::model::ModelSimState,
    bound: u64,
) -> Result<Vec<u8>, fsl_eval::simulation::model::ModelKeyError> {
    system.state_key_bytes_with_budget(
        state,
        fsl_foundation::IdentityLimits::default().with_input_bytes(bound),
        &mut RawKeyBudget::unlimited(),
        &Cancel::new(),
    )
}

fn expected_state_key(world: &World) -> Vec<u8> {
    // The envelope around the list, canonical by construction: members
    // sorted, no whitespace.
    let empty = json!({"type": "map", "entries": []});
    let reference = world.reference("c1");
    let envelope = json!({"type": "simulation-state",
            "semantic": {"type": "map", "entries": [{
                "key": {"type": "text", "value": population_text()},
                "value": {"type": "map", "entries": [{
                    "key": {"type": "reference",
                        "universe": crate::support::ids::node_json(reference.universe().root_type()),
                        "object_type": crate::support::ids::effective_json(reference.object_type()),
                        "identity": "c1"},
                    "value": {"type": "record", "name": counter_text(), "fields": [
                        {"name": "value", "value": {"type": "integer", "value": "0"}},
                        {"name": "label", "value": {"type": "integer", "value": "0"}},
                        {"name": "next", "value": Json::Null},
                        {"name": "list", "value": "@@LIST@@"}]}}]}}]},
            "control": empty, "queues": empty, "roles": empty, "observations": empty,
            "bounds": empty, "memory": empty});
    let template = envelope.to_string();
    let marker = "\"@@LIST@@\"";
    assert_eq!(template.matches(marker).count(), 1);
    template.replace(marker, &expected_list()).into_bytes()
}

fn assert_expected_state_key(raw: &[u8], world: &World) {
    let expected = expected_state_key(world);
    assert!(raw.len() > DEPTH * 80, "the key holds every record");
    assert!(
        raw == expected.as_slice(),
        "the key bytes differ from the RFC 8785 text at byte {:?}",
        raw.iter()
            .zip(expected.iter())
            .position(|(actual, expected)| actual != expected)
    );
}

/// FR-259-AC-6: zero is an exact byte bound, `u64::MAX` accepts a key over
/// the former 16 MiB default, and the independently authored byte count is
/// the exact successful bound.
#[trace("TC-729", "FR-259-AC-6")]
#[test]
fn identity_input_bytes_accepts_the_full_u64_range_and_uses_the_exact_bound() {
    on_stack(512 * 1024, || {
        with_deep_state(|system, state, world| {
            let expected = expected_state_key(world);
            assert!(expected.len() > fsl_foundation::DEFAULT_IDENTITY_INPUT_BYTES as usize);
            let key = encode_state(system, state, u64::MAX).expect("u64::MAX admits the key");
            assert_eq!(key, expected);

            let exact_bound =
                u64::try_from(expected.len()).expect("the expected key length fits u64");
            let exact = encode_state(system, state, exact_bound)
                .expect("the bound equal to the canonical byte count admits the key");
            assert_eq!(exact, expected);

            let zero = encode_state(system, state, 0).expect_err("zero refuses nonempty key bytes");
            let fsl_eval::simulation::model::ModelKeyError::Encoding(
                fsl_eval::simulation::EncodingRefusal::Limit(limit),
            ) = zero
            else {
                panic!("zero-byte bound has the exact byte-limit refusal: {zero:?}");
            };
            assert_eq!(limit.setting(), fsl_foundation::Setting::IdentityInputBytes);
            assert_eq!(limit.configured_bound(), 0);
            assert_eq!(limit.actual(), 2);
        })
    });
}

/// TC-729 step 4 candidate for FR-259-AC-7: the same key fails one byte
/// below its independently authored canonical byte count and succeeds at
/// that exact count. This test is not tagged with AC-7 until the malformed
/// input and malformed-value distinction clauses are asserted as well.
#[trace("TC-729")]
#[test]
fn identity_input_bytes_refusal_reports_the_reached_bound_and_exact_bound_succeeds() {
    on_stack(512 * 1024, || {
        with_deep_state(|system, state, world| {
            let expected = expected_state_key(world);
            let exact_bound =
                u64::try_from(expected.len()).expect("the expected key length fits u64");
            let too_small = encode_state(system, state, exact_bound - 1)
                .expect_err("one byte below the key length refuses");
            let fsl_eval::simulation::model::ModelKeyError::Encoding(
                fsl_eval::simulation::EncodingRefusal::Limit(limit),
            ) = too_small
            else {
                panic!("one-byte-short bound has the exact byte-limit refusal: {too_small:?}");
            };
            assert_eq!(limit.setting(), fsl_foundation::Setting::IdentityInputBytes);
            assert_eq!(limit.configured_bound(), exact_bound - 1);
            assert_eq!(limit.actual(), u128::from(exact_bound));

            let exact = encode_state(system, state, exact_bound)
                .expect("the exact canonical byte count admits the key");
            assert_eq!(exact, expected);
        })
    });
}
