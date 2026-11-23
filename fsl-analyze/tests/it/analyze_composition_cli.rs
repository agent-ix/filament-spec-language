// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044 (and FSpec FR-348-AC-3, FR-348-AC-4): a `native-analyze/1` item
//! naming a checked `compose` declaration selects its composition, and the
//! composition's component-guarantee premises settle in the same request,
//! each over its own component's checked package.
//!
//! The components are the checked packages `test/part-a` and `test/part-b`,
//! supplied as request libraries, and the unit `test/system` imports them
//! over the Counter domain package. The registry is `Engines::fsl()`, which
//! registers no composition backend, so a composition settles `unsupported`
//! (FSpec FR-348-AC-4); a registered provider that advertises `composition`
//! is sent the item with its derived premises (FSpec FR-348-AC-3). FSpec
//! FR-348-AC-1 and AC-2 are not delivered here.
//!
//! The outcome document names a stage for the document only (`last_stage`),
//! so a per-item stage is asserted only through a request whose sole item
//! settles at it. Text mode is the external driver's rendering and is not
//! exercised here (FR-1044 "Current system behavior").

use std::path::PathBuf;

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, analyze_with_providers, assert_no_nested_node_ids, counter_snapshot, label,
    package_selection, request_members, Workspace,
};
use crate::counter::{clause_text, document};

/// The clause `test/part-a` and, with `name` `Capped`, `test/part-b` declare.
const BOUNDED: &str = "always holds(c.value <= 3)";

/// The `rely` of `SysRely` and `SysAGRely`: `Capped` relies on `Bounded`.
const RELY: &str = "rely PB::Capped on PA::Bounded;";

const COMPONENTS: &str = "component PA from A using M; component PB from B using M; \
     guarantee PA::Bounded; guarantee PB::Capped; ";

/// `Fenced` and `Pings`, the extra clauses of `test/part-a`: a safety clause
/// conditional on fairness the compositions never supply, and a liveness
/// clause no composition declares a guarantee for.
const PART_A_EXTRA: &str = "temporal Fenced using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { fair weak each M::Counter::inc; always holds(c.value <= 3) }\n\
     temporal Pings using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { always eventually holds(c.value = 3) }\n";

/// `Done`, the extra liveness clause of `test/part-b`.
const PART_B_EXTRA: &str = "temporal Done using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { fair weak each M::Counter::inc; eventually holds(c.value = 3) }\n";

fn library(formula: &str, name: &str) -> String {
    let extra = if name == "Bounded" {
        PART_A_EXTRA
    } else {
        PART_B_EXTRA
    };
    crate::counter::unit_text(&format!("{}{extra}", clause_text(name, "", formula)))
}

fn program(body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile m0 = \"filament.model.complete/v1\";\n\
         profile t0 = \"filament.temporal.infinite-trace/v1\";\n\
         import \"test/part-a\" as A;\n\
         import \"test/part-b\" as B;\n\
         model M = \"test/counter\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{body}"
    )
}

fn compose(name: &str, rule: &str, members: &str) -> String {
    format!("compose {name} rule {rule} {{ {COMPONENTS}{members}}}\n")
}

/// The peers of a cycle: `RELY` and the reverse `rely`.
const MUTUAL: &str = "rely PB::Capped on PA::Bounded; rely PA::Bounded on PB::Capped;";

/// The declarations of `test/system`.
fn system() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", ""),
        compose("SysPR", "protocol-refinement", ""),
        compose("SysRT", "refinement-transfer", ""),
        compose("SysAG", "assume-guarantee", ""),
        compose("SysRely", "interface", RELY),
        compose("SysAGRely", "assume-guarantee", RELY),
        compose("SysCycle", "interface", MUTUAL),
        compose("SysCyclePR", "protocol-refinement", MUTUAL),
        compose("SysCycleRT", "refinement-transfer", MUTUAL),
        compose("SysCycleAG", "assume-guarantee", MUTUAL),
        // `PA::Bounded` is no declared guarantee, so the peer interface is
        // absent.
        "compose SysAbsent rule assume-guarantee { component PA from A using M; \
         component PB from B using M; guarantee PB::Capped; rely PB::Capped on PA::Bounded; }\n",
        // `Fenced` (slot 3) needs fairness nothing supplies, so it is `weaker`;
        // `Pings` (slot 4) is no declared guarantee, so it is `absent`, and
        // the reason-major order names it though it is the higher slot.
        "compose SysMixed rule assume-guarantee { component PA from A using M; \
         component PB from B using M; guarantee PB::Done; guarantee PA::Bounded; \
         guarantee PA::Fenced; rely PB::Done on PA::Fenced; depends PB::Done on PA::Pings; }\n",
        // The peer interfaces of `SysMixed`, as premises whose keys name them.
        "compose SysPeers rule interface { component PA from A using M; \
         component PB from B using M; guarantee PA::Fenced; guarantee PA::Pings; }\n",
        // No component guarantee: a composition with no premise to name.
        "compose SysNone rule interface { component PA from A using M; \
         component PB from B using M; }\n",
    )
}

/// A request over `body` with the component `Capped` clause `capped`, and
/// `items` (none for the default items).
fn request_for(ws: &Workspace, body: &str, capped: &str, items: Option<Value>) -> PathBuf {
    ws.write("part-a.fsl", library(BOUNDED, "Bounded").as_bytes());
    ws.write("part-b.fsl", library(capped, "Capped").as_bytes());
    ws.write("program.fsl", program(body).as_bytes());
    ws.write("model.json", &document(false));
    ws.write("start.json", &counter_snapshot(&[("a", 0)]));
    let mut request = request_members(items);
    let member = |identity: &str, file: &str| {
        json!({"identity": identity, "source": {"file": file, "authority": "test",
            "identity": file, "document": "Component", "formal_revision": 1}})
    };
    request["libraries"] = json!([
        member("test/part-a", "part-a.fsl"),
        member("test/part-b", "part-b.fsl")
    ]);
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

fn request(ws: &Workspace, body: &str, items: Value) -> PathBuf {
    request_for(ws, body, BOUNDED, Some(items))
}

fn composition(name: &str) -> Value {
    json!({"claim": name, "kind": "composition"})
}

/// The item's `record` diagnostic `cause`.
fn cause(item: &Value) -> &str {
    item["record"]["diagnostic"]["cause"]
        .as_str()
        .unwrap_or_else(|| panic!("a diagnostic cause in {item}"))
}

/// Asserts `item` is a composition settled `unsupported` by disposition,
/// naming kind `composition`, and returns its `request` member, when it has
/// one.
fn assert_unsupported_composition(item: &Value) {
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "unsupported_projection",
        "{item}"
    );
    assert_eq!(cause(item), "unsupported-requested-capability", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["kind"], "composition",
        "{item}"
    );
    assert!(item["record"]["backend"].is_null(), "{item}");
}

/// The roles of an item's `request.premises`.
fn roles(item: &Value) -> Vec<&str> {
    item["request"]["premises"]
        .as_array()
        .unwrap_or_else(|| panic!("premises in {item}"))
        .iter()
        .map(|premise| premise["role"].as_str().expect("a role"))
        .collect()
}

/// A routed composition item: the derived slot list, no candidates, and an
/// `unsupported` disposition.
fn assert_routed(item: &Value) {
    assert_unsupported_composition(item);
    assert_eq!(roles(item), ["component-guarantee"; 2], "{item}");
    assert_eq!(
        item["request"]["candidates"],
        json!({"type": "set", "backends": []}),
        "{item}"
    );
}

/// FR-1044-AC-1: a routed composition settles `unsupported`, its premise
/// items `proved` by EN-1, each over its own component, and the composition
/// is not `proved`.
#[trace("FR-1044-AC-1")]
#[trace("FSpec-FR-348-AC-4")]
#[test]
fn a_routed_composition_is_unsupported_beside_its_proved_premises() {
    let ws = Workspace::new("compose-ac1");
    let run = analyze(&request(&ws, &system(), json!([composition("Sys")])));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "{document}");
    assert_routed(&items[0]);
    assert_eq!(items[0]["request"]["kind"], "composition", "{document}");
    assert_eq!(
        items[0]["request"]["assumptions"],
        json!([[], []]),
        "{document}"
    );
    for (slot, (premise, name)) in [(1, "Bounded"), (2, "Capped")].into_iter().enumerate() {
        let item = &items[premise];
        assert_eq!(item["claim_name"], name, "{document}");
        assert_eq!(label(item), "proved", "{document}");
        assert_eq!(item["record"]["backend"], "en-1", "{document}");
        // The composition names the premise by the slot key of the clause
        // the premise item checks.
        let key = &items[0]["request"]["premises"][slot]["key"];
        assert_eq!(key["kind"], "temporal-satisfaction", "{document}");
        // FSpec FR-322 "Scope": the composition writes the key's ids in the
        // component's graph, the graph the premise item's own claim indexes, so
        // the two name one claim in one spelling.
        assert_eq!(key["claim"], item["request"]["claim"], "{document}");
    }
    assert_eq!(run.exit, 21, "{document}");
}

/// FSpec FR-348-AC-4: the production registry advertises no composition
/// backend, so the item's candidates are empty before any negotiation.
#[trace("FSpec-FR-348-AC-4")]
#[test]
fn the_production_registry_has_no_composition_candidate() {
    let mut registry = fsl_route::Registry::new();
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the FSL engines register");
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a descriptor registers");
    }
    assert!(matches!(
        registry.candidates(fsl_semantics::check::Capability::Composition, None),
        fsl_route::CandidateOutcome::Candidates(set) if set.candidates().is_empty()
    ));
}

const PROVIDER: &str = "fsl.test-composition-provider";

type Seen = std::sync::Arc<std::sync::Mutex<Vec<Value>>>;

use fsl_analyze::provider::{ProviderRequest, ProviderTarget, ProviderVerdict};
use fsl_replay::planner::{InterfaceMismatch, MismatchReason};

/// How a recording provider answers the items of one request.
struct Answer(Box<dyn Fn(&ProviderRequest<'_>) -> Vec<ProviderVerdict> + Send + Sync>);

impl std::fmt::Debug for Answer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Answer")
    }
}

/// Every item answered with `verdict`.
fn fixed(verdict: ProviderVerdict) -> Answer {
    Answer(Box::new(move |request| {
        vec![verdict.clone(); request.items().len()]
    }))
}

/// A registered composition provider outside FSL that records every request
/// it is sent as serialized and answers it as `Answer` says. It proves
/// nothing of its own: the verdict is the external provider's claim.
#[derive(Debug)]
struct Recording(Seen, Answer);

impl fsl_analyze::provider::ExternalProvider for Recording {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        self.0
            .lock()
            .expect("the record lock")
            .push(serde_json::to_value(request).expect("the request serializes"));
        (self.1 .0)(request)
    }
}

/// The provider process that advertises `composition` and publishes the
/// option `profile`, which an item sends only to a backend it names
/// (FR-967).
fn composition_provider(seen: &Seen, answer: Answer) -> fsl_analyze::analyze::Engine {
    composition_provider_of(seen, answer, fsl_route::ProviderOrigin::Process)
}

/// [`composition_provider`] registered with `origin`.
fn composition_provider_of(
    seen: &Seen,
    answer: Answer,
    origin: fsl_route::ProviderOrigin,
) -> fsl_analyze::analyze::Engine {
    use fsl_semantics::check::Capability;
    let descriptor = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(PROVIDER)),
        origin,
        vec![(
            Some(Capability::Composition.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: None,
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises composition")
    .with_options([("profile".to_owned(), "p0".to_owned())]);
    fsl_analyze::analyze::Engine::external(
        descriptor,
        fsl_analyze::analyze::BackendTool::new("test-composition", "1").expect("a tool"),
        Box::new(Recording(std::sync::Arc::clone(seen), answer)),
    )
    .expect("a composition process provider registers (FR-635)")
}

/// The composition `name` selected by the provider backend, with the
/// provider's option.
fn provided(name: &str) -> Value {
    json!({"claim": name, "kind": "composition", "backend": PROVIDER,
        "options": {"profile": "p1"}})
}

/// The document of `items` over `Capped` clause `capped`, the provider
/// answering `verdict` to each composition, and every request it was sent.
fn through_provider(
    dir: &str,
    capped: &str,
    items: Value,
    verdict: fsl_analyze::provider::ProviderVerdict,
) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    through_answer(dir, capped, items, fixed(verdict))
}

/// [`through_provider`] with the provider answering as `answer` says.
fn through_answer(
    dir: &str,
    capped: &str,
    items: Value,
    answer: Answer,
) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    let ws = Workspace::new(dir);
    let path = request_for(&ws, &system(), capped, Some(items));
    let seen = Seen::default();
    let run = analyze_with_providers(&path, vec![composition_provider(&seen, answer)]);
    let sent = seen.lock().expect("the record lock").clone();
    (run, sent)
}

/// FSpec FR-348-AC-3: a registered composition-capable provider is
/// negotiated the routed item and receives it with its derived premise slot
/// keys and its options; its `unknown` verdict settles `inconclusive`
/// `solver-inconclusive`, never `proved`, and the components still settle by
/// their own engine.
#[trace("FSpec-FR-348-AC-3")]
#[test]
fn a_registered_provider_receives_the_composition_with_its_premises() {
    let (run, sent) = through_provider(
        "compose-provider",
        BOUNDED,
        json!([provided("Sys")]),
        fsl_analyze::provider::ProviderVerdict::Unknown,
    );
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(label(&items[0]), "inconclusive", "{document}");
    assert_eq!(
        items[0]["record"]["cause"]["kind"], "solver-inconclusive",
        "{document}"
    );
    assert_eq!(items[0]["record"]["backend"], PROVIDER, "{document}");
    assert_eq!(
        items[0]["request"]["candidates"],
        json!({"type": "set", "backends": [PROVIDER]}),
        "{document}"
    );
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_eq!(label(&items[2]), "proved", "{document}");
    assert_eq!(sent.len(), 1, "one request to the one provider");
    let sent_items = sent[0]["items"].as_array().expect("sent items");
    assert_eq!(sent_items.len(), 1, "only the composition is sent");
    assert_eq!(sent_items[0]["kind"], "composition");
    assert_eq!(sent_items[0]["request_index"], 0);
    assert_eq!(
        sent_items[0]["premises"], items[0]["request"]["premises"],
        "the derived slot keys reach the provider"
    );
    assert_eq!(sent_items[0]["options"], json!({"profile": "p1"}));
    assert_eq!(items[0]["request"]["options"], json!({"profile": "p1"}));
    assert_eq!(sent_items[0]["candidates"]["backends"], json!([PROVIDER]));
}

/// FR-1044 "Behavior": a provider's `proved` stands, with FR-635's
/// certification of a plugin, when every component guarantee of the
/// composition settled `proved` in the same request.
#[trace("FSpec-FR-348-AC-3")]
#[test]
fn a_provider_proof_stands_when_every_component_guarantee_is_proved() {
    let (run, _) = through_provider(
        "compose-provider-proved",
        BOUNDED,
        json!([provided("Sys")]),
        fsl_analyze::provider::ProviderVerdict::Proved,
    );
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_eq!(items[0]["record"]["backend"], PROVIDER, "{document}");
    assert_eq!(items[0]["record"]["certification"], "trusted", "{document}");
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_eq!(label(&items[2]), "proved", "{document}");
    assert_eq!(run.exit, 0, "{document}");
}

/// Asserts `item` is a composition the provider proved, which FSL did not
/// confirm: `inconclusive` `provider-answer-unconfirmed` naming `premise`,
/// and the provider as the backend, with no certification.
fn assert_proof_not_accepted(item: &Value, premise: u64) {
    assert_eq!(label(item), "inconclusive", "{item}");
    assert_eq!(
        item["record"]["cause"]["kind"], "provider-answer-unconfirmed",
        "{item}"
    );
    assert_eq!(item["record"]["cause"]["premise"], premise, "{item}");
    assert_eq!(item["record"]["backend"], PROVIDER, "{item}");
    assert!(item["record"]["certification"].is_null(), "{item}");
}

/// FR-1044 "Behavior" (the provider proof gate): a provider's `proved` for a composition whose component
/// guarantee `refuted` is not accepted.
#[test]
fn a_provider_proof_is_refused_over_a_refuted_component_guarantee() {
    let (run, sent) = through_provider(
        "compose-provider-refuted",
        "always holds(c.value <= 2)",
        json!([provided("Sys")]),
        fsl_analyze::provider::ProviderVerdict::Proved,
    );
    let document = run.document();
    assert_eq!(sent.len(), 1, "the provider answered: {document}");
    assert_proof_not_accepted(&document["items"][0], 1);
    assert_eq!(label(&document["items"][1]), "proved", "{document}");
    assert_eq!(label(&document["items"][2]), "refuted", "{document}");
}

/// FR-1044 "Behavior" (the provider proof gate): a provider's `proved` for a composition whose component
/// guarantee is checked under peer guarantees, which settles `unsupported` by
/// disposition before any engine runs, is not accepted.
#[test]
fn a_provider_proof_is_refused_over_an_unsupported_component_guarantee() {
    let (run, _) = through_provider(
        "compose-provider-unsupported",
        BOUNDED,
        json!([provided("SysAGRely")]),
        fsl_analyze::provider::ProviderVerdict::Proved,
    );
    let document = run.document();
    assert_proof_not_accepted(&document["items"][0], 1);
    assert_eq!(label(&document["items"][1]), "proved", "{document}");
    assert_eq!(label(&document["items"][2]), "unsupported", "{document}");
}

/// FR-1044 "Behavior" (the provider proof gate): two compositions of one request whose `Capped` guarantees
/// are equal on the wire and differ in assumptions each read their own item.
/// `Sys`'s proved `Capped` never discharges `SysAGRely`'s assumed one.
#[trace("TC-3154", "FR-621-AC-17")]
#[test]
fn an_unassumed_proof_does_not_discharge_an_assumed_guarantee() {
    let (run, sent) = through_provider(
        "compose-provider-peers",
        BOUNDED,
        json!([provided("Sys"), provided("SysAGRely")]),
        fsl_analyze::provider::ProviderVerdict::Proved,
    );
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 5, "{document}");
    let sent_items: usize = sent
        .iter()
        .map(|request| request["items"].as_array().map_or(0, Vec::len))
        .sum();
    assert_eq!(sent_items, 2, "both compositions reach the provider");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_proof_not_accepted(&items[1], 1);
    assert_eq!(items[3]["claim_name"], "Capped", "{document}");
    // FR-621-AC-17: a premise item over a dependency names it in `package`
    // and holds its node ids against that dependency's graph; the composition
    // items over the request's own package carry no such member.
    assert!(items[2]["package"].is_string(), "{document}");
    assert!(items[3]["package"].is_string(), "{document}");
    assert_no_nested_node_ids(&items[2]);
    assert_no_nested_node_ids(&items[3]);
    assert!(items[0].get("package").is_none(), "{document}");
    assert_eq!(label(&items[3]), "proved", "{document}");
    assert_eq!(items[4]["claim_name"], "Capped", "{document}");
    assert_eq!(label(&items[4]), "unsupported", "{document}");
    assert_eq!(
        items[3]["request"]["claim"], items[4]["request"]["claim"],
        "the two guarantees are equal on the wire"
    );
}

/// FSpec FR-290 "Replay context": a provider's `refuted` for a composition,
/// which has no replay, settles `inconclusive` `replay-refused`.
#[test]
fn a_provider_refutation_of_a_composition_is_not_replayable() {
    let (run, _) = through_provider(
        "compose-provider-refutes",
        BOUNDED,
        json!([provided("Sys")]),
        fsl_analyze::provider::ProviderVerdict::Refuted {
            counterexample: Vec::new(),
        },
    );
    let item = &run.document()["items"][0];
    assert_eq!(label(item), "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "replay-refused", "{item}");
    assert_eq!(
        item["record"]["cause"]["code"], "unknown_required_feature",
        "{item}"
    );
}

/// FR-1044-AC-2: the other routed rules, `assume-guarantee` included, write
/// AC-1's items: each has a `request` member and is not settled by a
/// planner-local disposition.
#[trace("FR-1044-AC-2")]
#[test]
fn every_rule_settles_as_one_routed_item() {
    let ws = Workspace::new("compose-ac2");
    for name in ["SysPR", "SysRT", "SysAG"] {
        let run = analyze(&request(&ws, &system(), json!([composition(name)])));
        let document = run.document();
        assert_routed(&document["items"][0]);
        assert_eq!(document["items"][0]["claim_name"], name, "{document}");
        assert_eq!(document["items"][0]["request"]["kind"], "composition");
        assert_eq!(
            document["items"][0]["request"]["assumptions"],
            json!([[], []]),
            "{document}"
        );
        let items = document["items"].as_array().expect("items");
        assert_eq!(items.len(), 3, "{document}");
        for (index, premise) in [(1, "Bounded"), (2, "Capped")] {
            assert_eq!(items[index]["claim_name"], premise, "{document}");
            assert_eq!(label(&items[index]), "proved", "{document}");
            assert_eq!(items[index]["record"]["backend"], "en-1", "{document}");
        }
        assert_eq!(run.exit, 21, "{document}");
    }
}

/// FR-1044-AC-3: each selector settles by its own rule, and the request's
/// other items still run.
#[trace("FR-1044-AC-3")]
#[test]
fn each_selector_settles_by_its_own_rule() {
    let ws = Workspace::new("compose-ac3");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            composition("Sys"),
            composition("Missing"),
            composition("Local"),
            {"claim": "Sys", "kind": "temporal-satisfaction"},
            {"claim": "Sys"},
            {"claim": "SysAG", "kind": "temporal-satisfaction"},
        ]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_unsupported_composition(&items[0]);
    assert_eq!(cause(&items[1]), "unknown-claim", "{document}");
    assert_eq!(
        items[1]["record"]["diagnostic"]["detail"]["name"],
        "Missing"
    );
    let inapplicable = |item: &Value, index: u64, capability: &str, family: Option<&str>| {
        assert_eq!(cause(item), "inapplicable-kind", "{item}");
        let detail = &item["record"]["diagnostic"]["detail"];
        assert_eq!(detail["request_index"], index, "{item}");
        assert_eq!(detail["capability"], capability, "{item}");
        match family {
            Some(family) => assert_eq!(detail["family"], family, "{item}"),
            None => assert!(detail["family"].is_null(), "{item}"),
        }
    };
    inapplicable(&items[2], 2, "composition", Some("temporal-trace"));
    inapplicable(&items[3], 3, "temporal-satisfaction", Some("protocol"));
    assert_eq!(cause(&items[4]), "absent-kind", "{document}");
    inapplicable(&items[5], 5, "temporal-satisfaction", Some("protocol"));
    assert_eq!(run.exit, 20, "{document}");
}

/// The members of every object of `value`, written in reverse of the order
/// serde_json writes them: the raw text of a caller that permutes members.
fn permuted(value: &Value) -> String {
    match value {
        Value::Object(members) => {
            let members: Vec<String> = members
                .iter()
                .rev()
                .map(|(name, member)| {
                    format!("{}:{}", Value::from(name.as_str()), permuted(member))
                })
                .collect();
            format!("{{{}}}", members.join(","))
        }
        Value::Array(entries) => {
            let entries: Vec<String> = entries.iter().map(permuted).collect();
            format!("[{}]", entries.join(","))
        }
        other => other.to_string(),
    }
}

/// The request of `items` where each item's `premises` member holds the raw
/// text `premises` in place of the placeholder string.
fn request_with_raw_premises(ws: &Workspace, name: &str, premises: &str) -> PathBuf {
    let items = json!([{"claim": name, "kind": "composition", "premises": "@premises@"}]);
    let path = request(ws, &system(), items);
    let text = std::fs::read_to_string(&path).expect("the request reads");
    assert!(
        text.contains("\"@premises@\""),
        "the placeholder is written"
    );
    std::fs::write(&path, text.replace("\"@premises@\"", premises))
        .expect("the request is rewritten");
    path
}

fn with_premises(name: &str, premises: &Value) -> Value {
    json!({"claim": name, "kind": "composition", "premises": premises})
}

/// The refusal document of a whole-request refusal: its category, its code
/// and the member its message names, as `member: reason`.
fn assert_request_refusal(run: &crate::analyze_cli_support::Run, pointer: &str) {
    let document = run.document();
    assert_eq!(run.exit, 20, "{document}");
    assert_eq!(document["category"], "refusal", "{document}");
    assert_eq!(document["items"], json!([]), "{document}");
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    let message = document["diagnostics"][0]["message"]
        .as_str()
        .expect("a diagnostic message");
    assert!(
        message.starts_with(&format!("invalid analyze request member {pointer}:")),
        "the member is exactly {pointer}: {message}"
    );
}

/// FR-1044-AC-4: a caller may only restate the derived slot list; a
/// mistyped member refuses the request at its exact pointer, a different
/// list refuses the item, and another kind's `premises` is untouched.
#[trace("FR-1044-AC-4")]
#[test]
fn a_caller_may_only_restate_the_derived_premise_keys() {
    let ws = Workspace::new("compose-ac4");
    let derived = analyze(&request(&ws, &system(), json!([composition("Sys")]))).item(0);
    let listed = derived["request"]["premises"].clone();
    // (a) the derived list, every object's members written in reverse order
    // in the raw request, is the item without it.
    let raw = permuted(&listed);
    assert!(
        raw.starts_with("[{\"role\":"),
        "the members are permuted in the raw text: {raw}"
    );
    for name in ["Sys", "SysAG"] {
        let plain = analyze(&request(&ws, &system(), json!([composition(name)])));
        let path = request_with_raw_premises(&ws, name, &raw);
        assert!(std::fs::read_to_string(&path)
            .expect("reads")
            .contains(&raw));
        let run = analyze(&path);
        let (plain, run) = (plain.document(), run.document());
        assert_eq!(plain["items"], run["items"], "{name}");
    }
    // (b) a slot removed, two swapped or one relabelled refuses that item,
    // before any premise item, and the sibling still runs.
    let slots = listed.as_array().expect("the list").clone();
    let mut relabelled = slots.clone();
    relabelled[1]["role"] = json!("rely");
    // A swapped list puts a key of another component's package at slot 0, which
    // FSpec FR-322 "Scope" refuses at that key.
    let variants = [
        (json!([slots[0]]), "/items/0/premises"),
        (json!([slots[1], slots[0]]), "/items/0/premises/0/key"),
        (Value::Array(relabelled), "/items/0/premises"),
    ];
    for name in ["Sys", "SysAG"] {
        for (premises, pointer) in &variants {
            let run = analyze(&request(
                &ws,
                &system(),
                json!([with_premises(name, premises), {"claim": "Local"}]),
            ));
            let document = run.document();
            let items = document["items"].as_array().expect("items");
            assert_eq!(items.len(), 2, "no premise item: {document}");
            assert_eq!(items[0]["category"], "refusal", "{document}");
            assert_eq!(
                items[0]["record"]["diagnostic"]["code"], "invalid_runtime_input",
                "{document}"
            );
            assert_eq!(cause(&items[0]), "invalid-value", "{document}");
            assert_eq!(
                items[0]["record"]["diagnostic"]["detail"]["pointer"], *pointer,
                "{document}"
            );
            assert_eq!(
                items[0]["record"]["settled_by"], "disposition",
                "{document}"
            );
            assert_eq!(label(&items[1]), "proved", "{document}");
            assert_eq!(run.exit, 20, "{document}");
            // Alone, the refused item is the only one settled, at `S4`.
            let alone = analyze(&request(
                &ws,
                &system(),
                json!([with_premises(name, premises)]),
            ));
            assert_eq!(alone.document()["last_stage"], "S4", "{name}: {premises}");
        }
    }
    // (c) a mistyped member refuses the whole request at its exact pointer.
    for (premises, pointer) in [
        (json!({}), "/items/0/premises"),
        (json!([1]), "/items/0/premises/0"),
        (
            json!([{"role": "nope", "key": {}}]),
            "/items/0/premises/0/role",
        ),
    ] {
        let run = analyze(&request(
            &ws,
            &system(),
            json!([with_premises("Sys", &premises)]),
        ));
        assert_request_refusal(&run, pointer);
    }
    // (f) a caller's `assumptions` is an unknown member of the closed item.
    for name in ["Sys", "SysAG"] {
        let run = analyze(&request(
            &ws,
            &system(),
            json!([{"claim": name, "kind": "composition", "assumptions": [[], []]}]),
        ));
        let document = run.document();
        assert_eq!(run.exit, 20, "{document}");
        assert_eq!(document["items"], json!([]), "{document}");
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid-request",
            "{document}"
        );
        assert!(
            document["diagnostics"][0]["message"]
                .as_str()
                .is_some_and(|message| message.contains("assumptions")),
            "{document}"
        );
    }
    // (d) a name that is no composition keeps its own refusal, with no
    // premise item.
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            with_premises("Missing", &json!([])),
            with_premises("Local", &json!([]))
        ]),
    ));
    let document = run.document();
    assert_eq!(cause(&document["items"][0]), "unknown-claim", "{document}");
    assert_eq!(
        cause(&document["items"][1]),
        "inapplicable-kind",
        "{document}"
    );
    assert_eq!(document["items"].as_array().map(Vec::len), Some(2));
}

/// FR-1044-AC-4(e): an FR-571 `realizability` item's `premises` is not
/// refused for being a member of an item that is no composition.
#[trace("FR-1044-AC-4")]
#[test]
fn a_realizability_item_keeps_its_premises() {
    let item = crate::protocol_item_bounds::realizability(
        json!({"profile": "bounded-static", "max_depth": 16, "max_history": 16}),
    );
    assert!(item["premises"].is_object(), "{item}");
    let run = crate::protocol_item_bounds::run_items("compose-ac4e", vec![item]);
    assert_eq!(run.exit, 0, "{}", run.document());
    assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
}

/// FR-1044-AC-5: `interface` derives no assumption set, so its `rely` slot
/// writes no item and both guarantees run as the clauses they are.
#[trace("FR-1044-AC-5", "FR-530-AC-10")]
#[test]
fn an_interface_composition_derives_no_assumption_set() {
    let ws = Workspace::new("compose-ac5-interface");
    let run = analyze(&request(&ws, &system(), json!([composition("SysRely")])));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "no item for the rely slot: {document}");
    assert_eq!(roles(&items[0]).len(), 3, "{document}");
    assert_eq!(
        items[0]["request"]["assumptions"],
        json!([[], []]),
        "{document}"
    );
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_eq!(label(&items[2]), "proved", "{document}");
    assert_eq!(items[2]["record"]["backend"], "en-1", "{document}");
    assert_unsupported_composition(&items[0]);
    assert_eq!(run.exit, 21, "{document}");
}

/// FR-1044-AC-5: under `assume-guarantee`, a guarantee checked under peer
/// guarantees is not run as the clause alone.
#[trace("FR-1044-AC-5")]
#[test]
fn a_premise_under_assumptions_is_not_run_as_the_clause_alone() {
    let ws = Workspace::new("compose-ac5");
    let run = analyze(&request(&ws, &system(), json!([composition("SysAGRely")])));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "no item for the rely slot: {document}");
    assert_eq!(roles(&items[0]).len(), 3, "{document}");
    assert_eq!(
        items[0]["request"]["assumptions"],
        json!([[], [0]]),
        "{document}"
    );
    assert_eq!(label(&items[1]), "proved", "{document}");
    let assumed = &items[2];
    assert_eq!(assumed["claim_name"], "Capped", "{document}");
    assert_eq!(label(assumed), "unsupported", "{document}");
    assert_eq!(assumed["record"]["settled_by"], "disposition", "{document}");
    assert_eq!(
        assumed["record"]["diagnostic"]["code"], "unsupported_projection",
        "{document}"
    );
    assert_eq!(
        cause(assumed),
        "unsupported-requested-capability",
        "{document}"
    );
    assert_eq!(
        assumed["record"]["diagnostic"]["detail"]["kind"], "temporal-satisfaction",
        "{document}"
    );
    // No engine ran: no backend produced the record, and the item has no
    // candidate an engine could have been negotiated from.
    assert!(assumed["record"]["backend"].is_null(), "{document}");
    assert_eq!(
        assumed["request"]["candidates"],
        json!({"type": "set", "backends": []}),
        "{document}"
    );
    assert_unsupported_composition(&items[0]);
    assert_eq!(run.exit, 21, "{document}");
    // Its request is the clause as `test/part-b` declares it: the item the
    // clause settles to alone, but for the candidates.
    let alone = analyze(&request(&ws, &system(), json!([composition("Sys")])));
    let mut clause = alone.item(2)["request"].clone();
    clause["candidates"] = assumed["request"]["candidates"].clone();
    assert_eq!(assumed["request"], clause, "{document}");
    assert_eq!(label(&alone.item(2)), "proved");
}

/// The record every cyclic composition settles with, under any rule: no
/// `request` member and no candidates, `unsupported` by disposition at `S4`
/// with no proof basis, naming kind `composition`, form `fixed-point` and the cycle's guarantees in
/// edge order.
fn assert_cycle_answer(item: &Value, name: &str) {
    assert_eq!(item["claim_name"], name, "{item}");
    assert!(item["request"].is_null(), "no request member: {item}");
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert!(item["record"]["backend"].is_null(), "{item}");
    // The record carries no proof basis; FSpec FR-243's `unavailable` is a
    // settlement basis no terminal record writes.
    assert!(item["record"]["basis"].is_null(), "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "unsupported_projection",
        "{item}"
    );
    assert_eq!(cause(item), "unsupported-requested-capability", "{item}");
    let detail = &item["record"]["diagnostic"]["detail"];
    assert_eq!(detail["kind"], "composition", "{item}");
    assert_eq!(detail["form"], "fixed-point", "{item}");
    assert_eq!(
        detail["cycles"],
        json!([["PB::Capped", "PA::Bounded"]]),
        "{item}"
    );
}

/// FR-1044-AC-6: a dependency cycle settles `unsupported` under every rule,
/// with no provider registered, and derives no premise item.
#[trace("FR-1044-AC-6")]
#[test]
fn a_cycle_settles_unsupported_under_every_rule_before_any_premise() {
    let ws = Workspace::new("compose-ac6");
    for name in ["SysCycle", "SysCyclePR", "SysCycleRT", "SysCycleAG"] {
        let run = analyze(&request(
            &ws,
            &system(),
            json!([composition(name), {"claim": "Local"}]),
        ));
        let document = run.document();
        let items = document["items"].as_array().expect("items");
        assert_eq!(items.len(), 2, "no premise item for {name}: {document}");
        assert_cycle_answer(&items[0], name);
        assert_eq!(label(&items[1]), "proved", "{document}");
        assert_eq!(run.exit, 21, "{document}");
        // Alone, the cycle answer is the only item settled, at `S4`.
        let alone = analyze(&request(&ws, &system(), json!([composition(name)])));
        assert_eq!(alone.document()["last_stage"], "S4", "{name}");
    }
}

/// FR-1044-AC-6: a `rely` of a component on itself is not this answer; it
/// refuses the unit `invalid_model_binding`/`malformed-declaration`, exit 20.
#[trace("FR-1044-AC-6")]
#[test]
fn a_self_rely_refuses_the_unit_as_a_malformed_declaration() {
    let ws = Workspace::new("compose-ac6-self");
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("SysSelf", "interface", "rely PA::Bounded on PA::Bounded;")
    );
    let run = analyze(&request(&ws, &body, json!([composition("SysSelf")])));
    let document = run.document();
    assert_eq!(run.exit, 20, "{document}");
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["code"], "invalid_model_binding", "{document}");
    assert_eq!(diagnostic["cause"], "malformed-declaration", "{document}");
}

/// FR-1044-AC-6: a registered provider that answers `proved` receives no
/// request for a cyclic composition and cannot make it `proved`, and no
/// component item runs for it.
#[trace("FR-1044-AC-6")]
#[test]
fn a_cycle_is_never_sent_to_a_registered_provider() {
    for name in ["SysCycle", "SysCyclePR", "SysCycleRT", "SysCycleAG"] {
        let (run, sent) = through_provider(
            &format!("compose-ac6-{name}"),
            BOUNDED,
            json!([provided(name)]),
            ProviderVerdict::Proved,
        );
        let document = run.document();
        assert!(
            sent.is_empty(),
            "no request reaches the provider: {document}"
        );
        let items = document["items"].as_array().expect("items");
        assert_eq!(items.len(), 1, "no component item runs: {document}");
        assert_cycle_answer(&items[0], name);
        assert_eq!(run.exit, 21, "{document}");
    }
}

/// FR-1044-AC-8: premise items are written once however many selections
/// derive them, and a guarantee under another assumption set is another
/// item with an equal request.
#[trace("FR-1044-AC-8")]
#[test]
fn a_shared_premise_is_written_once() {
    let ws = Workspace::new("compose-ac8");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([composition("Sys"), composition("Sys")]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 4, "{document}");
    assert_eq!(items[2]["claim_name"], "Bounded", "{document}");
    assert_eq!(items[3]["claim_name"], "Capped", "{document}");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            composition("Sys"),
            composition("Sys"),
            composition("SysAGRely")
        ]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 6, "{document}");
    for item in &items[..3] {
        assert_unsupported_composition(item);
    }
    assert_eq!(items[3]["claim_name"], "Bounded", "{document}");
    assert_eq!(label(&items[4]), "proved", "{document}");
    assert_eq!(label(&items[5]), "unsupported", "{document}");
    let mut equal = items[4]["request"].clone();
    equal["request_index"] = json!(5);
    equal["candidates"] = items[5]["request"]["candidates"].clone();
    assert_eq!(items[5]["request"], equal, "{document}");
}

/// FR-1044-AC-9: a refuted component guarantee keeps its own record, and the
/// composition is neither `proved` nor `refuted`.
#[trace("FR-1044-AC-9")]
#[test]
fn a_refuted_premise_keeps_its_own_record() {
    let ws = Workspace::new("compose-ac9");
    let run = analyze(&request_for(
        &ws,
        &system(),
        "always holds(c.value <= 2)",
        Some(json!([composition("Sys")])),
    ));
    let document = run.document();
    assert_unsupported_composition(&document["items"][0]);
    let (bounded, capped) = (&document["items"][1], &document["items"][2]);
    assert_eq!(label(bounded), "proved", "{document}");
    assert_eq!(
        bounded["record"]["certification"], "certified",
        "{document}"
    );
    assert_eq!(label(capped), "refuted", "{document}");
    assert_eq!(capped["record"]["backend"], "en-1", "{document}");
    assert!(capped["record"]["certification"].is_null(), "{document}");
    // Each premise keeps the extent of its own clause, which is the extent
    // its slot key in the composition's request names.
    for (slot, premise) in [bounded, capped].into_iter().enumerate() {
        let key = &document["items"][0]["request"]["premises"][slot]["key"];
        assert_eq!(key["extent"]["classification"], "unbounded", "{document}");
        assert_eq!(
            premise["request"]["extent"]["classification"], key["extent"]["classification"],
            "{document}"
        );
    }
    // The counterexample ends in a state with `value = 3`.
    let value =
        |state: &str| format!(r#""name":"value","value":{{"type":"integer","value":"{state}"}}"#);
    let counterexample = capped["counterexample"].to_string();
    let last = counterexample.rfind(r#""name":"value""#).expect("a value");
    assert!(
        counterexample[last..].starts_with(&value("3")),
        "the final state holds value = 3: {capped}"
    );
    assert_eq!(run.exit, 21, "{document}");
}

/// FR-1044-AC-10: with no `items`, the composition is a default item in
/// source order, then the subject's derived item, then the premise items.
#[test]
fn a_composition_is_a_default_item() {
    let ws = Workspace::new("compose-ac10");
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", "")
    );
    let run = analyze(&request_for(&ws, &body, BOUNDED, None));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 5, "{document}");
    assert_eq!(items[0]["claim_name"], "Local", "{document}");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_eq!(items[1]["claim_name"], "Sys", "{document}");
    assert_unsupported_composition(&items[1]);
    assert_eq!(
        items[2]["request"]["temporal"]["item_kind"], "deadlock-freedom",
        "{document}"
    );
    assert_eq!(items[3]["claim_name"], "Bounded", "{document}");
    assert_eq!(items[4]["claim_name"], "Capped", "{document}");
    assert_eq!(run.exit, 21, "{document}");
}

/// The sum of the items the provider was sent over every request.
fn sent_items(sent: &[Value]) -> Vec<&Value> {
    sent.iter()
        .flat_map(|request| request["items"].as_array().into_iter().flatten())
        .collect()
}

/// FR-1044-AC-11: `Sys`, `SysAG` and `SysAGRely` each reach the provider as
/// one `composition` item with the derived `premises` and `assumptions`, no
/// premise result and no component proof; two compositions equal except in a
/// valid `assumptions` are different requests.
#[trace("FR-1044-AC-11")]
#[test]
fn a_provider_is_sent_the_derived_premises_and_assumptions() {
    let (run, sent) = through_provider(
        "compose-ac11",
        BOUNDED,
        json!([provided("Sys"), provided("SysAG"), provided("SysAGRely")]),
        ProviderVerdict::Unknown,
    );
    let document = run.document();
    let items = sent_items(&sent);
    assert_eq!(items.len(), 3, "{document}");
    for (item, assumptions) in
        items
            .iter()
            .zip([json!([[], []]), json!([[], []]), json!([[], [0]])])
    {
        assert_eq!(item["kind"], "composition", "{item}");
        assert_eq!(item["assumptions"], assumptions, "{item}");
        assert!(item["premises"].is_array(), "{item}");
        for premise in item["premises"].as_array().expect("premises") {
            assert!(premise.get("result").is_none(), "no premise result: {item}");
        }
        assert_eq!(item["options"], json!({"profile": "p1"}), "{item}");
    }
    // The echo carries the same derived members the provider read.
    for (index, item) in items.iter().enumerate() {
        let echoed = &document["items"][index]["request"];
        assert_eq!(echoed["assumptions"], item["assumptions"], "{document}");
        assert_eq!(echoed["premises"], item["premises"], "{document}");
    }
    // `SysAG` and `SysAGRely` differ in their premises (the `rely` slot) and
    // in their assumptions, so neither request stands for the other.
    assert_ne!(items[1]["assumptions"], items[2]["assumptions"]);
}

/// FR-1044-AC-12: a provider's `proved` for `SysAG` stands over proved
/// components with the certification of the provider's origin, and is
/// unconfirmed, naming premise 1, over a refuted `Capped`.
#[trace("FR-1044-AC-12")]
#[test]
fn an_assume_guarantee_proof_stands_only_over_proved_components() {
    let (run, _) = through_provider(
        "compose-ac12-stands",
        BOUNDED,
        json!([provided("SysAG")]),
        ProviderVerdict::Proved,
    );
    let document = run.document();
    assert_eq!(label(&document["items"][0]), "proved", "{document}");
    assert_eq!(
        document["items"][0]["record"]["certification"], "trusted",
        "{document}"
    );
    assert_eq!(run.exit, 0, "{document}");
    // A linked provider's proof stands `uncertified`, never `trusted`.
    let ws = Workspace::new("compose-ac12-linked");
    let path = request_for(&ws, &system(), BOUNDED, Some(json!([provided("SysAG")])));
    let seen = Seen::default();
    let run = analyze_with_providers(
        &path,
        vec![composition_provider_of(
            &seen,
            fixed(ProviderVerdict::Proved),
            fsl_route::ProviderOrigin::Linked,
        )],
    );
    let document = run.document();
    assert_eq!(label(&document["items"][0]), "proved", "{document}");
    assert_eq!(
        document["items"][0]["record"]["certification"], "uncertified",
        "{document}"
    );
    let (run, _) = through_provider(
        "compose-ac12-refuted",
        "always holds(c.value <= 2)",
        json!([provided("SysAG")]),
        ProviderVerdict::Proved,
    );
    let document = run.document();
    assert_proof_not_accepted(&document["items"][0], 1);
    assert_eq!(label(&document["items"][2]), "refuted", "{document}");
}

/// FR-1044-AC-12 (AC-8's request): the two `Sys` compositions stand, and
/// `SysAGRely` is unconfirmed with premise 1: its `Capped` (index 5) has a
/// `request` member equal to item 4's `proved` one but another typed context,
/// and is `unsupported`.
#[trace("FR-1044-AC-12")]
#[test]
fn an_equal_wire_guarantee_under_another_context_does_not_confirm_a_proof() {
    let (run, _) = through_provider(
        "compose-ac12-context",
        BOUNDED,
        json!([provided("Sys"), provided("Sys"), provided("SysAGRely")]),
        ProviderVerdict::Proved,
    );
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 6, "{document}");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_proof_not_accepted(&items[2], 1);
    assert_eq!(label(&items[4]), "proved", "{document}");
    assert_eq!(label(&items[5]), "unsupported", "{document}");
    let mut equal = items[4]["request"].clone();
    equal["request_index"] = json!(5);
    equal["candidates"] = items[5]["request"]["candidates"].clone();
    assert_eq!(items[5]["request"], equal, "{document}");
}

/// FR-1044-AC-12 (zero-guarantee seam): a composition with no
/// `component-guarantee` entry has no premise to name, so a provider's
/// `proved` stands.
#[trace("FR-1044-AC-12")]
#[test]
fn a_proof_with_no_component_guarantee_to_confirm_stands() {
    let (run, _) = through_provider(
        "compose-ac12-none",
        BOUNDED,
        json!([provided("SysNone")]),
        ProviderVerdict::Proved,
    );
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "no premise item: {document}");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_eq!(items[0]["record"]["certification"], "trusted", "{document}");
}

/// The mismatch `SysAbsent` declares, built from the node `Sys`'s first
/// premise names (`PA::Bounded`), which is the peer interface no guarantee of
/// `SysAbsent` declares; `change` rewrites it.
fn mismatch_answer(change: fn(InterfaceMismatch) -> InterfaceMismatch) -> Answer {
    Answer(Box::new(move |request| {
        let node = |item: usize| {
            let ProviderTarget::Composition(target) = &request.targets()[item] else {
                panic!("a composition target");
            };
            let wire = &target.premises()[0]["key"]["claim"]["node"];
            fsl_foundation::node_wire::node_key_from_json(wire).expect("a node key")
        };
        let found = InterfaceMismatch {
            component: "PA".to_owned(),
            interface: node(1),
            premise: 1,
            reason: MismatchReason::Absent,
        };
        vec![
            ProviderVerdict::Mismatch(change(found)),
            ProviderVerdict::Proved,
        ]
    }))
}

/// FR-1044-AC-14: a provider's `interface-mismatch` equal to the first
/// mismatch the declaration gives settles the composition `refuted` by a
/// decisive counterexample, built and not replayed, with no certification.
#[trace("FR-1044-AC-14")]
#[test]
fn a_declared_interface_mismatch_the_provider_names_refutes_the_composition() {
    let (run, _) = through_answer(
        "compose-ac14",
        BOUNDED,
        json!([provided("SysAbsent"), provided("Sys")]),
        mismatch_answer(|found| found),
    );
    let document = run.document();
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{document}");
    assert_eq!(item["record"]["backend"], PROVIDER, "{document}");
    assert!(item["record"]["certification"].is_null(), "{document}");
    assert!(item.get("replay").is_none(), "{document}");
    let counterexample = &item["counterexample"]["counterexample"];
    assert_eq!(counterexample["type"], "interface-mismatch", "{document}");
    assert_eq!(counterexample["component"], "PA", "{document}");
    assert_eq!(counterexample["premise"], 1, "{document}");
    assert_eq!(counterexample["reason"], "absent", "{document}");
    assert_eq!(item["category"], "violation", "{document}");
    // `SysAbsent`'s one premise is `Capped` with no assumption (the absent
    // `Bounded` has no index), so it runs as its clause and proves; `Sys`'s
    // premises prove beside it.
    let items = document["items"].as_array().expect("items");
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_eq!(label(&items[2]), "proved", "{document}");
    assert_eq!(label(&items[3]), "proved", "{document}");
    assert_eq!(run.exit, 10, "{document}");
}

/// FR-1044-AC-14, the recomputation seam: `SysMixed` has `weaker` at slot 3
/// and `absent` at slot 4, and FSpec FR-492's reason-major order names the
/// `absent` one. A provider's `weaker` witness at slot 3 is refused, and its
/// `absent` witness at slot 4 refutes the composition.
#[trace("FR-1044-AC-14")]
#[test]
fn the_provider_witness_is_admitted_in_reason_major_order() {
    // The peer interfaces `SysPeers` names in its premises: `Fenced`, then
    // `Pings`.
    let witness = |premise: usize, reason: MismatchReason| {
        Answer(Box::new(move |request| {
            let ProviderTarget::Composition(target) = &request.targets()[1] else {
                panic!("a composition target");
            };
            let wire = &target.premises()[premise - 3]["key"]["claim"]["node"];
            let interface =
                fsl_foundation::node_wire::node_key_from_json(wire).expect("a node key");
            vec![
                ProviderVerdict::Mismatch(InterfaceMismatch {
                    component: "PA".to_owned(),
                    interface,
                    premise,
                    reason,
                }),
                ProviderVerdict::Proved,
            ]
        }))
    };
    let items = || json!([provided("SysMixed"), provided("SysPeers")]);
    let (run, _) = through_answer(
        "compose-ac14-weaker",
        BOUNDED,
        items(),
        witness(3, MismatchReason::Weaker),
    );
    let document = run.document();
    let item = &document["items"][0];
    assert_eq!(label(item), "failed", "{document}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_result",
        "{document}"
    );
    assert_eq!(cause(item), "invalid-value", "{document}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["pointer"], "/counterexample",
        "{document}"
    );
    let (run, _) = through_answer(
        "compose-ac14-absent",
        BOUNDED,
        items(),
        witness(4, MismatchReason::Absent),
    );
    let document = run.document();
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{document}");
    let counterexample = &item["counterexample"]["counterexample"];
    assert_eq!(counterexample["premise"], 4, "{document}");
    assert_eq!(counterexample["reason"], "absent", "{document}");
    assert_eq!(counterexample["component"], "PA", "{document}");
}

/// FR-1044-AC-14: a well-formed witness that differs from the recomputation
/// in `component`, `interface`, `premise` or `reason` is the reader's
/// `invalid_result`/`invalid-value` at `/counterexample`, never `refuted`.
#[trace("FR-1044-AC-14")]
#[test]
fn a_well_formed_mismatch_that_differs_from_the_declaration_is_refused() {
    // The peer interface is the node `Sys`'s first premise names; the
    // `interface` variant names its second premise's node instead, a clause
    // that is no peer interface of `SysAbsent`.
    let wrong_interface = |slot: usize| {
        Answer(Box::new(move |request| {
            let ProviderTarget::Composition(target) = &request.targets()[1] else {
                panic!("a composition target");
            };
            let wire = &target.premises()[slot]["key"]["claim"]["node"];
            let interface =
                fsl_foundation::node_wire::node_key_from_json(wire).expect("a node key");
            vec![
                ProviderVerdict::Mismatch(InterfaceMismatch {
                    component: "PA".to_owned(),
                    interface,
                    premise: 1,
                    reason: MismatchReason::Absent,
                }),
                ProviderVerdict::Proved,
            ]
        }))
    };
    let answers: [(&str, Answer); 4] = [
        (
            "component",
            mismatch_answer(|found| InterfaceMismatch {
                component: "PB".to_owned(),
                ..found
            }),
        ),
        ("interface", wrong_interface(1)),
        (
            "premise",
            mismatch_answer(|found| InterfaceMismatch {
                premise: 0,
                ..found
            }),
        ),
        (
            "reason",
            mismatch_answer(|found| InterfaceMismatch {
                reason: MismatchReason::Weaker,
                ..found
            }),
        ),
    ];
    for (name, answer) in answers {
        let (run, _) = through_answer(
            &format!("compose-ac14-{name}"),
            BOUNDED,
            json!([provided("SysAbsent"), provided("Sys")]),
            answer,
        );
        let document = run.document();
        let item = &document["items"][0];
        assert_ne!(label(item), "refuted", "{name}: {document}");
        assert_eq!(label(item), "failed", "{name}: {document}");
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_result",
            "{name}: {document}"
        );
        assert_eq!(cause(item), "invalid-value", "{name}: {document}");
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["pointer"], "/counterexample",
            "{name}: {document}"
        );
    }
}

/// FR-1044-AC-14: a witness answered for a declaration that holds no mismatch
/// (`Sys`), and a `proved` answer over a refuted `Capped`, are never
/// `refuted`; with no provider registered `SysAbsent` is `unsupported`.
#[trace("FR-1044-AC-14")]
#[test]
fn a_mismatch_without_a_declared_mismatch_is_never_a_refutation() {
    let (run, _) = through_answer(
        "compose-ac14-none",
        BOUNDED,
        json!([provided("Sys"), provided("SysAbsent")]),
        Answer(Box::new(|request| {
            let ProviderTarget::Composition(target) = &request.targets()[0] else {
                panic!("a composition target");
            };
            let wire = &target.premises()[0]["key"]["claim"]["node"];
            let interface =
                fsl_foundation::node_wire::node_key_from_json(wire).expect("a node key");
            vec![
                ProviderVerdict::Mismatch(InterfaceMismatch {
                    component: "PA".to_owned(),
                    interface,
                    premise: 0,
                    reason: MismatchReason::Absent,
                }),
                ProviderVerdict::Proved,
            ]
        })),
    );
    let document = run.document();
    assert_eq!(label(&document["items"][0]), "failed", "{document}");
    assert_eq!(
        document["items"][0]["record"]["diagnostic"]["detail"]["pointer"], "/counterexample",
        "{document}"
    );
    let (run, _) = through_provider(
        "compose-ac14-unproved",
        "always holds(c.value <= 2)",
        json!([provided("Sys")]),
        ProviderVerdict::Proved,
    );
    let document = run.document();
    assert_proof_not_accepted(&document["items"][0], 1);
    let ws = Workspace::new("compose-ac14-unregistered");
    let run = analyze(&request(&ws, &system(), json!([composition("SysAbsent")])));
    let document = run.document();
    assert_unsupported_composition(&document["items"][0]);
}

/// The package handoff of the source request at `path`.
fn package_handoff(ws: &Workspace, path: &std::path::Path) -> PathBuf {
    let source: Value =
        serde_json::from_slice(&std::fs::read(path).expect("the request reads")).expect("JSON");
    let (selected, _) = package_selection(ws, &source["request"]);
    ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": selected})
            .to_string()
            .as_bytes(),
    )
}

/// Asserts the public `package` handoff of a composition unit is settled
/// whole as unsupported, exit 21, with no item: the well-formed request meets
/// a family the package route cannot read yet, typed
/// `unsupported_construct`/`not-yet-implemented`, whose message names the
/// family at `detail`. Nothing settles, so the package route's own
/// resolution of a composition is not exercised (QSL-2739: package-consumer
/// API).
fn assert_package_pending(run: &crate::analyze_cli_support::Run, detail: &str) {
    let document = run.document();
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(run.exit, 21, "{document}");
    assert_eq!(document["category"], "unsupported", "{document}");
    assert_eq!(document["items"], json!([]), "{document}");
    assert!(document["last_stage"].is_null(), "{document}");
    assert_eq!(diagnostics.len(), 1, "{document}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{diagnostic}");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains(detail)),
        "{diagnostic}"
    );
}

/// FR-1044-AC-10, FR-940: a `package` handoff of a unit with composition
/// declarations, requested with no `items`, writes the source request's
/// items, records and exit: every composition not under `assume-guarantee`
/// (the cyclic ones settled by their cycle), each derived premise item once
/// and the subject's derived item.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_items_settle_every_composition_as_the_source_does() {
    let ws = Workspace::new("compose-package-default");
    let path = request_for(&ws, &system(), BOUNDED, None);
    let by_source = analyze(&path);
    let by_package = analyze(&package_handoff(&ws, &path));
    let items = by_source.document()["items"].clone();
    assert!(
        items.as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["request"]["kind"] == "composition")),
        "the source request selects compositions: {items}"
    );
    assert_eq!(by_package.document()["items"], items);
    assert_eq!(by_package.exit, by_source.exit);
}

/// FR-940: the package route has no path yet for an explicit item naming a
/// composition (QSL-2917): an authenticated named composition names the
/// missing owning-work permission; an unresolved name
/// remains an unnamed claim. Neither selection settles `unknown-claim`.
#[test]
fn a_package_item_naming_a_composition_is_refused() {
    // Each compile writes its own handoff directory, so each name gets its
    // own workspace.
    for (name, detail) in [
        ("Sys", "items[0] composition owning execution is pending missing native before-work permission"),
        ("Typo", "items[0] names the unnamed-claim family, which has no package form yet"),
    ] {
        let ws = Workspace::new(&format!("compose-package-named-{name}"));
        let path = request_for(&ws, &system(), BOUNDED, Some(json!([composition(name)])));
        assert_package_pending(
            &analyze(&package_handoff(&ws, &path)),
            detail,
        );
    }
}

/// FR-940: a name the package carries a row for still resolves beside a composition; only the unresolved names are
/// refused.
#[test]
fn a_package_item_naming_a_local_clause_beside_a_composition_settles() {
    let ws = Workspace::new("compose-package-local");
    let path = request_for(&ws, &system(), BOUNDED, Some(json!([{"claim": "Local"}])));
    let run = analyze(&package_handoff(&ws, &path));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "{document}");
    assert_eq!(items[0]["claim_name"], "Local", "{document}");
    assert_eq!(label(&items[0]), "proved", "{document}");
    assert_eq!(run.exit, 0, "{document}");
}

/// FR-1044-AC-10, second half: over the unit's `package` handoff the same
/// items, records and exit code as the source request.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_route_settles_the_same_items() {
    let ws = Workspace::new("compose-ac10-package-equal");
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", "")
    );
    let path = request_for(&ws, &body, BOUNDED, None);
    let handoff = package_handoff(&ws, &path);
    let by_source = analyze(&path);
    let by_package = analyze(&handoff);
    assert_eq!(by_package.exit, by_source.exit);
    assert_eq!(
        by_package.document()["items"],
        by_source.document()["items"]
    );
}

/// The default request over `test/system-default` with `Capped`'s formula
/// `capped`, run from source and over its `package` handoff.
fn default_runs(
    name: &str,
    capped: &str,
) -> (
    crate::analyze_cli_support::Run,
    crate::analyze_cli_support::Run,
) {
    let ws = Workspace::new(name);
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", "")
    );
    let path = request_for(&ws, &body, capped, None);
    let handoff = package_handoff(&ws, &path);
    (analyze(&path), analyze(&handoff))
}

/// Asserts the item at `index` is written the same on both routes, request,
/// record and every other member, and that both runs exit the same.
fn assert_same_item(
    (by_source, by_package): &(
        crate::analyze_cli_support::Run,
        crate::analyze_cli_support::Run,
    ),
    index: usize,
) -> Value {
    let source = by_source.document()["items"][index].clone();
    assert!(source.is_object(), "{}", by_source.document());
    assert_eq!(by_package.document()["items"][index], source);
    assert_eq!(by_package.exit, by_source.exit);
    source
}

/// FR-1044-AC-10, FR-940: the authored clause `Local` (index 0) is written
/// the same over the `package` handoff.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_clause_item_matches_the_source() {
    let item = assert_same_item(&default_runs("compose-ac10-clause", BOUNDED), 0);
    assert_eq!(item["claim_name"], "Local", "{item}");
    assert_eq!(label(&item), "proved", "{item}");
}

/// FR-1044-AC-10: the composition `Sys` (index 1), with its derived
/// `premises` and `assumptions`, is written the same over the `package`
/// handoff: `unsupported` at negotiation, naming `composition`.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_composition_item_matches_the_source() {
    let item = assert_same_item(&default_runs("compose-ac10-composition", BOUNDED), 1);
    assert_eq!(item["claim_name"], "Sys", "{item}");
    assert_unsupported_composition(&item);
    assert_eq!(item["request"]["assumptions"], json!([[], []]), "{item}");
}

/// FR-1044-AC-10, FR-620-AC-5: the subject's derived deadlock-freedom item
/// (index 2) is written the same over the `package` handoff.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_derived_item_matches_the_source() {
    let item = assert_same_item(&default_runs("compose-ac10-derived", BOUNDED), 2);
    assert_eq!(
        item["request"]["temporal"]["item_kind"], "deadlock-freedom",
        "{item}"
    );
}

/// FR-1044-AC-10: the premise items `Bounded` and `Capped` (indices 3 and 4),
/// each run over its own component package, are written the same over the
/// `package` handoff, naming that package.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_premise_items_match_the_source() {
    let runs = default_runs("compose-ac10-premises", BOUNDED);
    for (index, name, package) in [(3, "Bounded", "test/part-a"), (4, "Capped", "test/part-b")] {
        let item = assert_same_item(&runs, index);
        assert_eq!(item["claim_name"], name, "{item}");
        assert_eq!(item["package"], package, "{item}");
        assert_eq!(label(&item), "proved", "{item}");
    }
}

/// FR-1044-AC-10 with AC-9's refuted `Capped`: the premise's counterexample,
/// replayed over its component package, and every other item are written
/// the same over the `package` handoff.
#[trace("FR-1044-AC-10")]
#[test]
fn the_package_default_refuted_premise_matches_the_source() {
    let runs = default_runs("compose-ac10-refuted", "always holds(c.value <= 2)");
    let capped = assert_same_item(&runs, 4);
    assert_eq!(label(&capped), "refuted", "{capped}");
    assert!(capped["counterexample"].is_object(), "{capped}");
    assert_eq!(runs.1.document()["items"], runs.0.document()["items"]);
}

/// FSpec FR-322-AC-520 (NI-25, NI-27, NI-28) through `analyze`: the held
/// premise keys a result wrote are restated as written and resolve in the
/// component's graph; an index the component graph does not hold is refused
/// `malformed_wire` at its `node` member, and a key of another package
/// `invalid_runtime_input`/`invalid-value` at the key, before the range
/// check.
#[trace("FSpec-FR-322-AC-520")]
#[test]
fn a_held_premise_key_resolves_in_its_components_graph() {
    let ws = Workspace::new("compose-held");
    let plain = analyze(&request(&ws, &system(), json!([composition("Sys")])));
    let listed = plain.item(0)["request"]["premises"].clone();
    let operand = |premises: &Value| {
        premises[0]["key"]["claim"]["node"]["path"]
            .as_array()
            .expect("a path")
            .iter()
            .position(Value::is_object)
            .expect("an operand segment")
    };
    let at = operand(&listed);
    // The written list, restated, is the item itself.
    let restated = analyze(&request(
        &ws,
        &system(),
        json!([with_premises("Sys", &listed)]),
    ));
    assert_eq!(plain.document()["items"], restated.document()["items"]);
    // An index outside the component graph names its `node` member.
    let mut beyond = listed.clone();
    beyond[0]["key"]["claim"]["node"]["path"][at] = json!({"node": 100_000});
    let run = analyze(&request(
        &ws,
        &system(),
        json!([with_premises("Sys", &beyond)]),
    ));
    let item = run.item(0);
    assert_eq!(
        item["record"]["diagnostic"]["code"], "malformed_wire",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["pointer"],
        format!("/items/0/premises/0/key/claim/node/path/{at}/node"),
        "{item}"
    );
    // FSpec FR-272: `malformed_wire` carries no cause.
    assert!(
        item["record"]["diagnostic"].get("cause").is_none(),
        "{item}"
    );
    // A subject identity of another package is refused at the key, even with
    // an index beyond the graph.
    let mut foreign = beyond.clone();
    foreign[0]["key"]["temporal"]["subject"]["identity"] = json!("other/package using m");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([with_premises("Sys", &foreign)]),
    ));
    let item = run.item(0);
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(cause(&item), "invalid-value", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["pointer"], "/items/0/premises/0/key",
        "{item}"
    );
    // The subject package is checked on every `component-guarantee` key, with
    // or without an index segment (NI-28).
    let mut bare = listed.clone();
    bare[0]["key"]["claim"]["node"]["path"][at] = json!("plain");
    bare[0]["key"]["temporal"]["subject"]["identity"] = json!("other/package using m");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([with_premises("Sys", &bare)]),
    ));
    let item = run.item(0);
    assert_eq!(cause(&item), "invalid-value", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["pointer"], "/items/0/premises/0/key",
        "{item}"
    );
}
