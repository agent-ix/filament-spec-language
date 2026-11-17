// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-630 to FR-633 and FR-635 (TC-1590 to TC-1593, TC-1595): one accepting
//! and one refuted `fsl analyze` item for the refinement, hyperproperty,
//! timed zone and schedulability families, and an item settled on an
//! external provider, each from source plus a `native-analyze/1` request
//! through the driver binary (`$FSL_BIN`).

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, counter_request_with, label, request_members, request_over, Workspace,
};

/// The two items' results, in request order.
fn results(run: &crate::analyze_cli_support::Run, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| label(&run.item(index)).to_owned())
        .collect()
}

/// A refinement of the Counter by an abstract Counter selected as `Spec`,
/// with the extra rows `rows` and the abstract value given by `mapped`.
fn refinement(name: &str, mapped: &str, rows: &str) -> String {
    format!(
        "refinement {name} using v of M as Spec {{\n\
         object Spec::Counter from M::Counter {{ value = {mapped}; }}\n\
         step M::Counter::inc as Spec::Counter::inc;\n{rows}}}\n"
    )
}

/// The Counter request with the abstract `Spec` model (the same domain
/// package under alias `Spec`) declared in the unit.
fn refinement_request(ws: &Workspace, body: &str, items: Value) -> std::path::PathBuf {
    counter_request(
        ws,
        &format!("model Spec = \"test/counter\";\n{body}"),
        items,
    )
}

/// FR-630-AC-1: a correct mapping proves and a wrong one refutes with a
/// replayed counterexample carrying per-step records.
#[trace("TC-1590", "FR-630-AC-1")]
#[test]
#[ignore = "QSL-2127: refinement items are not reachable through fsl analyze yet"]
fn a_refinement_proves_and_a_wrong_mapping_refutes() {
    let body = refinement("Same", "self.value", "") + &refinement("Off", "3 - self.value", "");
    let ws = Workspace::new("family-refinement");
    let run = analyze(&refinement_request(
        &ws,
        &body,
        json!([{"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement"}},
            {"claim": "Off", "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
    ));
    assert_eq!(results(&run, 2), ["proved", "refuted"]);
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    let refuted = run.item(1);
    assert_eq!(refuted["replay"]["reproduced"], true);
    assert!(refuted["counterexample"]["steps"].is_array(), "{refuted}");
}

/// FR-630-AC-2: `refinement_internals` reaches compilation.
#[trace("TC-1590", "FR-630-AC-2")]
#[test]
#[ignore = "QSL-2245: refinement_internals is not an analyze request member yet"]
fn refinement_internals_are_a_request_member() {
    let ws = Workspace::new("family-refinement-internals");
    let mut request = request_members(Some(json!([
        {"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement"}}
    ])));
    request["refinement_internals"] = json!({"Same": []});
    let run = analyze(&counter_request_with(
        &ws,
        &format!(
            "model Spec = \"test/counter\";\n{}",
            refinement("Same", "self.value", "")
        ),
        request,
    ));
    assert_eq!(run.exit, 0, "{}", String::from_utf8_lossy(&run.stdout));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-630-AC-3, FR-138-AC-9: decimal and enum history rows check and the
/// refinement item over each proves.
#[trace("TC-1590", "FR-630-AC-3")]
#[trace("TC-1590", "FR-138-AC-9")]
#[test]
#[ignore = "QSL-2245: decimal and enum history rows are refused not-yet-implemented"]
fn decimal_and_enum_history_rows_check_and_prove() {
    for (name, rows) in [
        (
            "DecimalHistory",
            "history M::Counter last : Decimal[0, 100; 0, 2; nearest-even] = decimal(0, 0) \
             on M::Counter::inc {{ decimal(1, 0) }};\n",
        ),
        (
            "EnumHistory",
            "history M::Counter last : Phase = Phase::idle on M::Counter::inc {{ Phase::busy }};\n",
        ),
    ] {
        let rows = rows.replace("{{", "{").replace("}}", "}");
        let body =
            "enum Phase { idle, busy }\n".to_owned() + &refinement(name, "self.value", &rows);
        let ws = Workspace::new(&format!("family-history-{name}"));
        let run = analyze(&refinement_request(
            &ws,
            &body,
            json!([{"claim": name, "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
        ));
        assert_eq!(
            run.document()["last_stage"],
            Value::Null,
            "{name}: no compile refusal"
        );
        assert_eq!(label(&run.item(0)), "proved", "{name}");
    }
}

/// FR-630-AC-4, FR-135-AC-6: a nested-reference row checks and proves.
#[trace("TC-1590", "FR-630-AC-4")]
#[trace("TC-1590", "FR-135-AC-6")]
#[test]
#[ignore = "QSL-2245: nested-reference rows are refused not-yet-implemented"]
fn a_nested_reference_row_checks_and_proves() {
    let body = "record Slots { held: Option<Reference<M::Counter>>; }\n".to_owned()
        + &refinement(
            "Nested",
            "self.value",
            "history M::Counter slot : Slots = Slots { held: none } on M::Counter::inc \
             { Slots { held: some(self) } };\n",
        );
    let ws = Workspace::new("family-nested-reference");
    let run = analyze(&refinement_request(
        &ws,
        &body,
        json!([{"claim": "Nested", "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-631-AC-1: non-interference proves for the secure vault and refutes
/// for the leaky one with a two-trace counterexample.
#[trace("TC-1591", "FR-631-AC-1")]
#[test]
#[ignore = "QSL-2128: hyperproperty items are not reachable through fsl analyze yet"]
fn non_interference_proves_and_refutes() {
    use crate::hyper_world::{non_interference, request_files, Vault};
    for (vault, expected) in [(Vault::Secure, "proved"), (Vault::Leaky, "refuted")] {
        let (unit, model, snapshot) = request_files(vault, false, (1, 0), &non_interference(""));
        let ws = Workspace::new(&format!("family-hyper-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &[snapshot],
            json!({"items": [{"claim": "NonInterference"}]}),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), expected);
        if expected == "refuted" {
            assert_eq!(
                item["counterexample"]["traces"].as_array().map(Vec::len),
                Some(2)
            );
            assert_eq!(item["replay"]["reproduced"], true);
        }
    }
}

/// FR-631-AC-4: a hyper clause with no object parameter is analyzable.
#[trace("TC-1591", "FR-631-AC-4")]
#[test]
#[ignore = "QSL-2248: a hyper clause with no object parameter is checked but not emitted"]
fn an_object_free_hyper_clause_is_analyzed() {
    use crate::hyper_world::{request_files, Vault};
    let declaration = "hyper Whole using inf over behaviours () \
                       forall trace a of V forall trace b of V { always holds(true) }";
    let (unit, model, snapshot) = request_files(Vault::Secure, false, (1, 0), declaration);
    let ws = Workspace::new("family-hyper-object-free");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"items": [{"claim": "Whole"}]}),
    ));
    assert!(matches!(label(&run.item(0)), "proved" | "refuted"));
}

/// The timed `Rpc` subject of FR-239 with timeout bound `t` and `claim`.
pub(crate) fn rpc_unit(t: u32, claim: &str) -> String {
    rpc_unit_with(t, ">= 1", ">=", claim)
}

/// [`rpc_unit`] with `reply` guarded by `self.x {reply} ms` and `timeout` by
/// `self.x {timeout} {t} ms`: ADR-026 section 11's strict-guard variant is
/// `("> 3", ">")`.
pub(crate) fn rpc_unit_with(t: u32, reply: &str, timeout: &str, claim: &str) -> String {
    use crate::timed_world::{unit_text, DENSE, PACKAGE};
    let body = format!(
        "{DENSE}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ \
             self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x {reply} ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and \
             self.late = (pre(self.phase) = 3) and \
             (pre(self.phase) = 1 implies self.phase = 2) and \
             (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x {timeout} {t} ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ \
             not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
         {claim}"
    );
    unit_text(PACKAGE, crate::zone_search::TIMED, &body)
}

pub(crate) fn rpc_snapshot() -> Vec<u8> {
    let package = crate::timed_world::PACKAGE;
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/calls"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                    "late": {"boolean": false}, "retried": {"boolean": false}}}]}],
    })
    .to_string()
    .into_bytes()
}

/// FR-632-AC-1: the no-late-reply claim proves with a zone certificate at
/// timeout 4 and refutes with exact delays at timeout 3.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn zone_items_prove_with_certificates_and_refute_with_exact_delays() {
    let claim = "temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
                 { always holds(not c.late) }\n";
    for (t, expected) in [(4, "proved"), (3, "refuted")] {
        let ws = Workspace::new(&format!("family-zone-{t}"));
        let run = analyze(&request_over(
            &ws,
            &rpc_unit(t, claim),
            &crate::zone_search::rpc_document(),
            &[rpc_snapshot()],
            json!({"items": [{"claim": "NoLateReply"}]}),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), expected, "T = {t}");
        if expected == "proved" {
            assert_eq!(item["record"]["basis"]["type"], "zone-certified");
            assert_eq!(item["record"]["certification"], "certified");
        } else {
            assert_eq!(item["replay"]["reproduced"], true);
        }
    }
}

/// ADR-026 §12's `Ctl` with third WCET `c` and the claims after it.
fn ctl_body(c: u32, claims: &str) -> String {
    format!(
        "dimension Time;\nunit ms : Time = rational(1, 1);\n\
         taskset Ctl {{\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
         task c periodic 12 ms deadline 12 ms wcet {c} ms priority 3 lo;\n\
         }}\n{claims}"
    )
}

/// FR-633-AC-1, AC-2: `Ctl` proves on EN-7 and a response bound one below
/// the fixpoint refutes with the miss evidence.
#[trace("TC-1593", "FR-633-AC-1")]
#[trace("TC-1593", "FR-633-AC-2")]
#[test]
#[ignore = "QSL-2131: schedulability items are not reachable through fsl analyze yet"]
fn schedulability_proves_on_en7_and_a_tight_response_refutes() {
    let claims = "schedulable CtlFp on Ctl under fixed_priority;\n\
                  response Twelve on Ctl.c <= 12 ms under fixed_priority;\n\
                  response Eleven on Ctl.c <= 11 ms under fixed_priority;\n";
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n{}",
        ctl_body(5, claims)
    );
    let ws = Workspace::new("family-schedulability");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": [{"claim": "CtlFp"}, {"claim": "Twelve"},
            {"claim": "Eleven"}]}),
    ));
    assert_eq!(results(&run, 3), ["proved", "proved", "refuted"]);
    let proved = run.item(0);
    assert_eq!(proved["record"]["basis"]["type"], "closed-form");
    assert_eq!(proved["record"]["basis"]["analysis"], "fixed-priority");
    assert_eq!(proved["record"]["backend"], "en-7");
    assert!(proved.get("axes").is_none());
    let miss = &run.item(2)["counterexample"];
    assert_eq!(miss["task"], "c");
    assert_eq!(miss["response"], "12");
    assert_eq!(miss["deadline"], "11");
}

/// FR-635-AC-1, AC-2: an item naming a registered external provider settles
/// on it, the registry lists it, and another item is unchanged by it.
#[trace("TC-1595", "FR-635-AC-1")]
#[trace("TC-1595", "FR-635-AC-2")]
#[test]
#[ignore = "QSL-2296 (from QSL-2132): FR-635 says the driver loads providers from their manifests but no requirement states where manifests are found or how a process provider is run; FR-489 forbids test-only registrations in the production registry (spec question open)"]
fn an_item_naming_a_registered_provider_settles_on_it() {
    let provider =
        std::env::var("FSL_TEST_PROVIDER").unwrap_or_else(|_| "test-value-provider".to_owned());
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-provider");
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Even", "kind": "value-validity", "backend": provider},
            {"claim": "Bounded"}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved");
    assert_eq!(item["record"]["backend"], provider.as_str());
    // FR-635: the test provider is a process plugin, so its proof is trusted.
    assert_eq!(item["record"]["certification"], "trusted");
    let registry: Vec<Value> = run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .map(|entry| entry["backend"].clone())
        .collect();
    assert!(registry.contains(&json!(provider)));
    assert!(registry.contains(&json!("en-1")) && registry.contains(&json!("en-7")));
    // FR-635 "Outputs": each entry carries its backend's FSpec FR-331 tool.
    assert!(run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .all(|entry| entry["tool"]["name"]
            .as_str()
            .is_some_and(|name| !name.is_empty())));
    assert_eq!(label(&run.item(1)), "proved");
}

/// A process provider double: a `Process`-origin descriptor advertising
/// (`value-validity`, `unbounded`) over `Integer`, whose handle always
/// answers `verdict`. It stands where the driver passes a started plugin
/// session; the real plugin process is the driver's test (QSL-2283).
#[derive(Debug)]
struct AnsweringProvider(fsl_analyze::provider::ProviderVerdict);

impl fsl_analyze::provider::ExternalProvider for AnsweringProvider {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        assert_eq!(request.backend().as_str(), PROCESS_PROVIDER);
        for (item, target) in request.items().iter().zip(request.targets()) {
            assert_eq!(item.occurrence(), target.claim());
        }
        vec![self.0.clone(); request.items().len()]
    }
}

const PROCESS_PROVIDER: &str = "fsl.test-process-provider";

fn process_descriptor(backend: &str) -> fsl_route::BackendDescriptor {
    use fsl_semantics::check::Capability;
    fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(backend)),
        fsl_route::ProviderOrigin::Process,
        vec![(
            Some(Capability::ValueValidity.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises the external value arm")
}

fn process_provider(
    backend: &str,
    verdict: fsl_analyze::provider::ProviderVerdict,
) -> fsl_analyze::analyze::Engine {
    fsl_analyze::analyze::Engine::external(
        process_descriptor(backend),
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(AnsweringProvider(verdict)),
    )
    .expect("the descriptor is unbounded")
}

/// FR-635 "One rule for every provider-routing entry": a process provider
/// handed to `analyze_with_providers` registers beside `Engines::fsl()`, an
/// item naming it settles on it with certification `trusted`, the registry
/// lists it, and an item that does not negotiate to it is unchanged.
#[trace("TC-1595", "FR-635-AC-2")]
#[test]
fn analyze_with_providers_registers_a_process_provider() {
    use fsl_analyze::provider::ProviderVerdict;
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-process-provider");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Even", "kind": "value-validity", "backend": PROCESS_PROVIDER},
            {"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![process_provider(PROCESS_PROVIDER, ProviderVerdict::Proved)],
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["certification"], "trusted", "{item}");
    // FR-635 "Outputs": the sorted `{backend, tool}` of every registered
    // backend, the provider's with the tool it was registered with.
    let entries = run.document()["registry"]
        .as_array()
        .expect("registry")
        .clone();
    let registry: Vec<Value> = entries
        .iter()
        .map(|entry| entry["backend"].clone())
        .collect();
    assert!(registry.contains(&json!(PROCESS_PROVIDER)), "{registry:?}");
    assert!(registry.contains(&json!("en-1")), "{registry:?}");
    let mut sorted: Vec<&str> = registry.iter().filter_map(Value::as_str).collect();
    sorted.sort_unstable();
    let listed: Vec<&str> = registry.iter().filter_map(Value::as_str).collect();
    assert_eq!(listed, sorted, "the registry is sorted by backend");
    let provider = entries
        .iter()
        .find(|entry| entry["backend"] == PROCESS_PROVIDER)
        .expect("the provider is listed");
    assert_eq!(
        provider["tool"],
        json!({"name": "test-provider", "version": "1"}),
        "{provider}"
    );
    let en1 = entries
        .iter()
        .find(|entry| entry["backend"] == "en-1")
        .expect("en-1 is listed");
    assert_eq!(en1["tool"]["name"], "fsl-analyze", "{en1}");
    let without = analyze(&request);
    assert_eq!(run.item(1), without.item(1));
}

/// A provider that records the FSpec FR-331 `request` member `analyze`
/// built for it, as the typed request serializes it, with the negotiated
/// manifest entry's backend, and answers `proved`.
#[derive(Debug, Default)]
struct RecordingProvider(std::sync::Arc<std::sync::Mutex<Vec<(Value, String)>>>);

impl fsl_analyze::provider::ExternalProvider for RecordingProvider {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        // The package member is the v2 document's own bytes.
        let document: Value =
            serde_json::from_slice(request.package().bytes()).expect("the package is JSON");
        assert!(document.is_object());
        // The manifest is the registry snapshot, and the negotiated entry is
        // one of its descriptors.
        let snapshot: Vec<&str> = request
            .manifest()
            .iter()
            .map(|descriptor| descriptor.id().as_str())
            .collect();
        assert!(
            snapshot.contains(&request.backend().as_str()),
            "{snapshot:?}"
        );
        assert!(snapshot.contains(&"en-1"), "{snapshot:?}");
        let wire = serde_json::to_value(request).expect("the request serializes");
        self.0
            .lock()
            .expect("the record")
            .push((wire, request.descriptor().id().to_string()));
        vec![fsl_analyze::provider::ProviderVerdict::Proved; request.items().len()]
    }
}

/// FSpec FR-331's `Request` definition, read in place from `$FSPEC_DIR`
/// with the checked-package schema it refers to.
fn fr331_request_schema() -> jsonschema::JSONSchema {
    let fspec = std::env::var_os("FSPEC_DIR")
        .expect("FSPEC_DIR names the FSpec checkout whose schema this reads");
    let read = |relative: &str| -> Value {
        let path = std::path::PathBuf::from(&fspec).join(relative);
        serde_json::from_slice(&std::fs::read(&path).expect("the schema reads"))
            .expect("the schema is JSON")
    };
    let schema = read("proposals/backend-provider-v1/schema.json");
    let package = read("proposals/checked-package-v2/schema.json");
    let request = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": "#/$defs/Request",
    });
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:filament:checked-package:v2".to_owned(), package)
        .compile(&request)
        .expect("the schema compiles")
}

/// FR-635 "How a process provider runs an item", FSpec FR-331: for the
/// `Boolean` value clause `Even`, which has no requirement record, the
/// request `analyze` builds carries the package's emitted bytes, the
/// caller's `deadline_ms`, and one item at the analyze request index whose
/// claim is the clause occurrence, whose extent is unbounded over the
/// parameter's integer domain, and whose backend is the negotiated
/// provider, its one candidate; the request validates against FSpec's
/// `Request` schema, and the negotiated manifest entry is the provider's.
#[trace("TC-1595", "FR-635-AC-1")]
#[test]
fn analyze_builds_the_fr331_request_for_a_boolean_value_clause() {
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-provider-request");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"},
            {"claim": "Even", "kind": "value-validity", "backend": PROCESS_PROVIDER}]),
    );
    let recording = RecordingProvider::default();
    let sent = std::sync::Arc::clone(&recording.0);
    let limits = fsl_analyze::provider::ProviderLimits {
        deadline_ms: 250,
        ..fsl_analyze::provider::ProviderLimits::default()
    };
    let engine = fsl_analyze::analyze::Engine::external(
        process_descriptor(PROCESS_PROVIDER),
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(recording),
    )
    .expect("the descriptor is unbounded");
    let options = fsl_analyze::provider::ProviderOptions {
        provider_limits: limits,
    };
    let run =
        crate::analyze_cli_support::analyze_with_providers_under(&request, vec![engine], &options);
    assert_eq!(label(&run.item(1)), "proved", "{}", run.document());
    let sent = sent.lock().expect("the record").clone();
    let [(sent, manifest_entry)] = sent.as_slice() else {
        panic!("one request: {sent:?}");
    };
    assert_eq!(manifest_entry, PROCESS_PROVIDER);
    let schema = fr331_request_schema();
    if let Err(errors) = schema.validate(sent) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("the request violates FR-331: {errors:?}\n{sent}");
    }
    assert_eq!(sent["deadline_ms"], 250);
    let items = sent["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "{sent}");
    let item = &items[0];
    // The item's position in this one-item request, not its analyze index
    // (1), which the verdict is joined back to.
    assert_eq!(item["request_index"], 0);
    assert_eq!(item["kind"], "value-validity");
    assert_eq!(item["backend"], PROCESS_PROVIDER);
    assert_eq!(
        item["candidates"],
        json!({"type": "set", "backends": [PROCESS_PROVIDER]})
    );
    assert_eq!(item["extent"]["classification"], "unbounded", "{item}");
    let domains = item["extent"]["domains"].as_array().expect("domains");
    assert_eq!(domains.len(), 1, "{item}");
    assert_eq!(domains[0]["kind"], "integer", "{item}");
    // The claim is the clause occurrence: `Even`'s body at its declaration.
    assert!(
        matches!(
            item["claim"]["occurrence"]["role"].as_str(),
            Some("claim" | "declaration")
        ),
        "{item}"
    );
    // A request whose item has lost its domain is not an FR-331 request,
    // so the validation above is not vacuous.
    let mut defective = sent.clone();
    defective["items"][0]["extent"]["domains"] = json!([]);
    assert!(!schema.is_valid(&defective));
}

/// A process provider repeating a linked `BackendId` refuses the request
/// `invalid-request` naming `providers` before any item runs.
#[test]
fn analyze_with_providers_refuses_a_repeated_backend() {
    use fsl_analyze::provider::ProviderVerdict;
    let ws = Workspace::new("family-provider-repeat");
    let request = counter_request(
        &ws,
        &crate::analyze_cli_support::bounded(),
        json!([{"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![process_provider("en-1", ProviderVerdict::Proved)],
    );
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["items"], json!([]), "{document}");
    assert!(document.to_string().contains("providers"), "{document}");
}

/// The `Rpc` request at timeout `t` with `claim` and the request members
/// `extra`.
pub(crate) fn rpc_request(ws: &Workspace, t: u32, claim: &str, extra: Value) -> std::path::PathBuf {
    request_over(
        ws,
        &rpc_unit(t, claim),
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        extra,
    )
}

/// The `Ctl` request (third WCET 5) with `claims` and `items`.
pub(crate) fn ctl_request(ws: &Workspace, claims: &str, items: Value) -> std::path::PathBuf {
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n{}",
        ctl_body(5, claims)
    );
    request_over(
        ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": items}),
    )
}

/// FR-640-AC-12 (TC-2243), analyze and replay half: an event-position hyper
/// clause whose two traces range over two models carries one clock row per
/// trace, so it compiles, analyzes to a verdict and, when refuted, replays
/// under the two rows' configurations; it is never `not-yet-implemented`.
#[trace("TC-2243", "FR-640-AC-12")]
#[test]
#[ignore = "QSL-2277: a hyper clause over two models under event-position is refused not-yet-implemented"]
fn a_hyper_clause_over_two_models_analyzes_and_replays_per_trace() {
    let declaration = "hyper Saves using e over behaviours (v: V::Vault, w: N::Vault) \
                       forall trace a of V forall trace b of N \
                       { always[0,3] holds(v.l @ a = w.l @ b) }";
    let (unit, model, other, snapshot) =
        crate::hyper_world::two_model_request_files((1, 0), declaration);
    let unit = unit.replace(
        "profile p =",
        "profile e = \"filament.temporal.event-position.false-extension/v1\";\nprofile p =",
    );
    let ws = Workspace::new("family-hyper-two-models");
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    models.as_array_mut().expect("models").push(
        json!({"format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}}),
    );
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"models": models, "items": [{"claim": "Saves"}]}),
    ));
    let item = run.item(0);
    assert_ne!(
        item["record"]["diagnostic"]["cause"], "not-yet-implemented",
        "{item}"
    );
    let verdict = label(&item);
    assert!(matches!(verdict, "proved" | "refuted"), "{item}");
    if verdict == "refuted" {
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
    }
}
