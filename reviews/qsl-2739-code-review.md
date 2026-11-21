---
id: SR-202611210022442313144108506
title: "code-review of QSL-2739 PR3287 composition item through analyze (FR-1044)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/document/composition_item.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/claim_family.rs, fsl-analyze/src/provider.rs, fsl-analyze/tests/it/analyze_composition_cli.rs, fsl-analyze/tests/it/main.rs, fsl-command/src/command/analyze.rs, fsl-replay/src/planner.rs, fsl-replay/src/planner/from_package.rs; FR-1044, FR-507"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This reviews frozen PR #3287 at ebdaca7eecb4d062bc69fce9faa4bb80d1f3fefa over base f87b2a5c43. The code under review is implementation commit ebdaca7, written against the reviewed contract and SR baseline 773e661. The FR-1044/FR-507 contract was cleared by the independent spec reviewer (disposition round 4) and is not re-litigated here. The Rust lane is folded into this artifact. Its sources are the repository's `agent-skills/rust-review/SKILL.md` checkout (agent-skills c55c4a6), the installed dev-tools 0.2.0 `rust-review` and the `rust-style` baseline. The repository's AGENTS.md conventions take precedence.

The production design is sound and reuses shared code:

- **Selection.** A `compose` name is intercepted in `resolve_items` through `composition_item::resolve`. Kind admission, absent and unknown kind, FR-507 `inapplicable-kind`, the cycle refusal and caller `premises` equality (parsed `Value` equality, after the cycle refusal) follow the Behavior order.
- **Slots.** The derived slot list comes from the one existing planner derivation, `fsl_replay::planner::composition_premises` over `CompositionContext::slot`. Nothing duplicates it.
- **Main item.** It holds FSpec FR-331's five roles and no `fixed-point` slot (`list` filters it out).
- **Premise keys.** Premise items are deduplicated on `ItemKey`, whose `TemporalKey.subject` is `subject_text(subject, assumptions)`. That makes the assumption set part of the typed key, so AC-8's sixth item is a distinct key with an equal `request`.
- **Component execution.** Component guarantees resolve through the shared `resolve_one` over the component's own `CheckedPackage`. They run through the shared `work` path over `RunInput::source(component)`, keyed by request index in `analyze_bound`. Family, owning model and certificate identity are read from the component package. `check_provision` already walks dependencies, and cancellation and limits are the run's.
- **Planner-local and assumed premises.** An `assume-guarantee` item is planner-local, with no `request`. A premise with a non-empty assumption set settles `unsupported` before negotiation, keeping its echo and with cleared candidates. No engine runs on it.

No caller proof or result is read, and no provider, engine or AG refutation is added. The defects are in test strength and in small Rust idiom and robustness points.

## Verdict

**CONDITIONAL**: the findings are medium and low only, with no production correctness defect found. Merge also waits on runtime evidence. `/tmp/qsl-2739-test-results.md` records the two focused Draco commands as queued with no result, so no test here is claimed green.

Clean and examined:

- FR-507's new `correspondence`/`component_composition` arm (claim_family.rs:153). AC-1's routed `unsupported` exercises it: without the arm, FR-507-AC-10 would turn the result into `inapplicable-kind`.
- The request-stage `premises` shape refusal (fsl-command analyze.rs:121), scoped to `kind` `composition`, which leaves FR-571 realizability `premises` untouched.
- Request-index numbering of premise items after the caller's items (document.rs:1656) and after FR-368's derived items on the default path (document.rs:3082).
- The `Composition` arms in `resolve_derived`, `derived_items` and `protocol_derived_items`.
- The `fsl-replay` `composition_premises` export.

The test harness's `Fr290` negotiator is the existing shared test stand-in for the driver's FR-290 adapter (analyze_cli_support.rs:97). It is not a forged result. Candidates come from the production `Engines::fsl()` registry inside `fsl_command::command::analyze_with_providers`. The AC-1 test asserts the real public settlement (`unsupported`, `settled_by` `disposition`, the `composition` diagnostic, empty candidates, exit 21). The separate `the_production_registry_has_no_composition_candidate` test supplements it and replaces nothing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-4(a)'s "members of each object written in reverse order" case is vacuous. The workspace's serde_json has no `preserve_order` (Cargo.lock lists no indexmap dependency), so `serde_json::Map` is a BTreeMap. `restated` inserting `key` before `role` serializes exactly as the derived echo does (`{"key":…,"role":…}`), so a byte or ordered-member comparison in place of parsed equality would still pass. | fsl-analyze/tests/it/analyze_composition_cli.rs:253 |
| FND-002 | medium | AC-4(c) asserts the refusal pointer with `document.to_string().contains(pointer)`. For the `"premises": {}` case the expected `/items/0/premises` is a prefix of `/items/0/premises/0` and `/items/0/premises/0/role`, so a refusal at the wrong pointer still passes. The `invalid-request` category is also never asserted (exit 20 alone does not distinguish it). | fsl-analyze/tests/it/analyze_composition_cli.rs:353 |
| FND-003 | low | AC-4(b) requires each mismatched-`premises` composition item to settle "at stage `S4`", but the loop asserts no stage. Only the lone `with_premises("Sys", [])` request checks the document `last_stage`, and that request is none of the six AC-4(b) variants. | fsl-analyze/tests/it/analyze_composition_cli.rs:307 |
| FND-004 | low | The AC-1 test asserts only the roles of `request.premises`, not that they equal the two component-guarantee slot keys. It does not assert that `Bounded` and `Capped` are proved by EN-1 (their `backend`). | fsl-analyze/tests/it/analyze_composition_cli.rs:152 |
| FND-005 | low | The AC-2 test does not assert the `SysAG` item's stage `S4`, the absence of `candidates`, or the labels of the premise items at indices 1 and 2. | fsl-analyze/tests/it/analyze_composition_cli.rs:197 |
| FND-006 | low | The AC-5 test does not assert, for the assumed `Capped` item, that it has no `backend`, its stage `S4`, or that its `request` equals the clause as `test/part-b` declares it. "No engine run" is not observed either. | fsl-analyze/tests/it/analyze_composition_cli.rs:391 |
| FND-007 | low | The AC-9 test does not check that each premise keeps its own extent and certification. The counterexample check `contains("\"3\"")` matches any string `"3"` anywhere in the counterexample, not a final state with `value = 3`. | fsl-analyze/tests/it/analyze_composition_cli.rs:503 |
| FND-008 | low | The AC-10 default-items test skips index 2. It asserts `names[..2]` and `names[3..]` but never that index 2 is the subject's derived deadlock-freedom item (FR-620-AC-5), or that exactly five items are written. | fsl-analyze/tests/it/analyze_composition_cli.rs:529 |
| FND-009 | low | The ignored package-route test has a dead `let _ = HANDOFF;`, which exists only to silence an unused import. | fsl-analyze/tests/it/analyze_composition_cli.rs:559 |
| FND-010 | low | The module doc claims coverage of "FSpec FR-348-AC-3", but no test carries that trace tag and no test negotiates with a registered composition-capable backend. | fsl-analyze/tests/it/analyze_composition_cli.rs:2 |
| FND-011 | low | The `inapplicable-kind` payload's `family` is hard-coded as `DeclarationFamily::Protocol` for a non-AG composition. It is not read through `claim_family::claim_family`, the reader FR-507 names, which this PR teaches the same row (claim_family.rs:153). That makes two sources of truth for one table row. | fsl-analyze/src/analyze/document/composition_item.rs:184 |
| FND-012 | low | `premises_of` swallows invariant failures through `filter_map` and `?`: the component lookup (`find(...)?`) and the fixed-point wire (`to_wire().ok()?`). A derived premise item is then silently omitted, where the slot-derivation failure path settles a typed `failed` record. The result is fewer premise items with no fault. | fsl-analyze/src/analyze/document/composition_item.rs:376 |
| FND-013 | low | On the default-items path, `if let Some(selected) = composition_item::resolve(...)` silently skips a composition node when `resolve` returns `None`, with no fault. `composition_at` has already found the declaration, so a `None` here is an invariant break that should be surfaced, not dropped. | fsl-analyze/src/analyze/document.rs:3016 |
| FND-014 | low | `negotiate_items` keeps two redundant nested blocks `{ { … } }` left from the refactor. They indent the whole negotiation body two extra levels and serve no scope purpose. | fsl-analyze/src/analyze/document.rs:3810 |
| FND-015 | low | `PREMISE_ROLES` restates as string literals the five FR-331 role spellings that `fsl_replay::planner::SlotRole::as_str`/`parse` already own. fsl-command already depends on fsl-replay. A role rename would drift between the request-stage check and the derivation. | fsl-command/src/command/analyze.rs:109 |
| FND-016 | low | `if let (Some(premises), true) = (&item.premises, item.kind… == Some(…))` matches a tuple with a `bool` literal. A plain `if` on the kind, then `if let Some(premises)`, is the direct form rust-review asks for ("`if let`/`let else` over nested match"). | fsl-command/src/command/analyze.rs:1105 |
| FND-017 | low | `PremiseKind::Guarantee` carries the component library as a bare `String`, re-parsed with `LibraryName::new(...).ok()` at use. A parse failure is folded into the same `failed` record as a missing dependency. rust-review asks for newtypes over primitives: carry the `LibraryName`. | fsl-analyze/src/analyze/document/composition_item.rs:77 |

## Gates

No Cargo, fmt, clippy, test, deny or aggregate gate was run by this reviewer, per the brief (no builds or runtime). The coder reports `cargo check --locked -p fsl-analyze --tests` and `-p fsl-command --tests` exit 0 (`/tmp/qsl-2739-check-analyze*.log`, `/tmp/qsl-2739-check-command.log`). They also report scoped rustfmt on the changed files. Runtime evidence for `analyze_composition_cli` and for `protocol_item_bounds claim_family analyze_request_wire` is queued on Draco and pending, so it is not green. No CI workflow file changed. Tool versions: quoin 0.36.8, module spec-artifacts-process@v0.36.0.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-018 | medium | The package route's new `PendingFamily::Composition` is chosen when any unnamed claim of the package is a `component_composition` node, whatever `name` the item gives. In a package that holds a composition, a misspelled name, or the name of another unnamed claim form, is refused naming the `composition` family. Before this change it got `unnamed-claim`, so the refusal now misattributes the family. | fsl-analyze/src/analyze/document/package_run.rs:778 |
| FND-019 | low | `&*node.semantic_form == "component_composition"` restates a form word that `claim_family::Form::from_wire` already decodes (behind `#[ix_trace_rs::string_edge]`). That leaves two unaudited spellings of one wire word. | fsl-analyze/src/analyze/document/package_run.rs:780 |
| FND-020 | low | `RequirementItem::composition_clause` is built through `value_clause` and then mutated. It sets `result_bound` (documented as a value clause's `Boolean` result type node) to the composition's own node, plus a `ClaimExtent::Bounded` that is overwritten at once. A composition item then carries a meaningless value-validity field, and the doc does not say so. | fsl-route/src/request.rs:892 |
| FND-021 | low | `the_package_route_refuses_a_composition_typed` asserts the refusal only with `document.to_string().contains("composition family")`. It never asserts the exit code, `items == []` or the refusal code, so any refusal whose text mentions the phrase passes. | fsl-analyze/tests/it/analyze_composition_cli.rs:746 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-022 | medium | `assert_package_refused` accepts either `missing_import` or `unsupported_construct`, and asserts cause and exit only for the second. Both package tests pass on whichever path runs, so neither can fail when the package route's own guard (`unnamed_composition`, or the `unnamed-claim` refusal for `Sys`/`Typo`) regresses or is removed, as long as admission refuses `missing_import` first. The `missing_import` branch asserts no exit code. Once Draco reports the actual code at 6ff10e95, the oracle should name that one boundary exactly. | fsl-analyze/tests/it/analyze_composition_cli.rs:750 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-023 | low | The proof gate passes vacuously for a composition with no component-guarantee index. `required.iter().all(...)` is true on an empty list, and the derived `fixed-point` item is built by `claim_item` with `required: Vec::new()` (composition_item.rs:366), even though its premises are the cycle's component-guarantee slot keys. A provider's `proved` for a fixed-point item would therefore stand on the provider's answer alone, against FR-1044's gate bullet and the arch proof-guard ruling. No public request reaches the fixed-point item today (AC-7 deferred), so this is latent. The fix is contained: fail closed on an empty `required`, or give the fixed-point item its cycle guarantees' indexes. | fsl-analyze/src/analyze/document.rs:3776 |
| FND-024 | low | When it refuses a `proved` record, `gate_compositions` names the provider through `record.backend().cloned().unwrap_or_else(\|\| BackendId::new("composition"))`. That fallback invents a backend id no registry holds. A `proved` record always carries its backend, so the branch is an unreachable invariant break, and it would write a fabricated `backend` into the outcome instead of being surfaced. | fsl-analyze/src/analyze/document.rs:3783 |

## Dispositions

Disposition round 1 inspected only the fix diff `ebdaca7..cec151ddc635c8bad1b3296c6deb378208470e13` (one commit, 11 files). Outcomes come from that diff alone. Runtime is pending and is not claimed. On ebdaca7, Draco's runs failed: 9 composition tests failed on a fixture header with no `filament.model.complete/v1`, and 3 `analyze_request_wire` default-item tests failed. All three are unclassified. The cec module run, those exact three on this head, and the same three on parent f87b2a5c43 are queued. Static check: the f87b2a5c43..HEAD hunks in `default_items` only add to it (a composition node arm and premise items). The non-composition path is unchanged in the diff, but only the queued parent run can classify the three failures.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `permuted` writes the raw request text with every object's members in reverse order. The test asserts the raw text starts `[{"role":` (serde_json writes `key` first), then compares the whole `items` with the plain request's, for both Sys and SysAG. |
| FND-002 | fixed | `assert_request_refusal` asserts exit 20, `category` `refusal`, `items == []`, code `invalid-request`, and a message starting with `invalid analyze request member {pointer}:`, the trailing colon making the pointer exact. |
| FND-003 | fixed | Each of the six AC-4(b) variants is also run alone and asserts `last_stage == "S4"`. The per-item record asserts `category` `refusal` and `settled_by` `disposition`. |
| FND-004 | fixed | For each slot, the test asserts the premise is `proved` with `backend` `en-1`, and that the composition's `premises[slot].key` has `kind` `temporal-satisfaction` and the premise item's own `claim`. |
| FND-005 | fixed | The test asserts SysAG has no `request` and no `candidates`, the item count is 3, and each premise is `proved` by `en-1` with its name. The per-item `S4` is not observable for SysAG, whose premises settle later: the document names only `last_stage`. The module doc records that limit, so it is accepted, not left open. |
| FND-006 | fixed | The assumed `Capped` item asserts `backend` null, the empty candidate set, its code, cause and kind, and that its `request` equals the clause item `Sys` writes alone, but for the candidates. Its per-item `S4` has the same document-level limit as FND-005. |
| FND-007 | fixed | `Bounded` is asserted `certified` and refuted `Capped` uncertified with `backend` `en-1`, each with a `bounded` extent. The last `"name":"value"` in the counterexample must start a `"value":"3"` integer. |
| FND-008 | fixed | The test asserts five items: Local `proved`, Sys unsupported, index 2's `temporal.item_kind` `deadlock-freedom`, then Bounded and Capped. |
| FND-009 | fixed | `HANDOFF` is removed from the import list and from the ignored test. |
| FND-010 | fixed | The module doc's FSpec-FR-348-AC-3 claim is now backed by `a_registered_provider_receives_the_composition_with_its_premises`, which carries that tag. |
| FND-011 | fixed | The `inapplicable-kind` family is now read by `crate::claim_family::claim_family(ctx.package, ItemKind::Formula, &occurrence(composition.node()))`, and stays `None` for AG. |
| FND-012 | fixed | `premise_of` maps an unresolved component or library, or a fixed-point wire failure, to `PremiseKind::Unresolved`, which `premise_item` settles as the typed `failed` record. |
| FND-013 | fixed | The default path now pushes `composition_item::failed(index, composition.name())` when `resolve` returns `None`. |
| FND-014 | fixed | The redundant nested blocks are gone. The `ResolvedClaim` destructure and the negotiation body sit at loop level, with the logic unchanged. |
| FND-015 | fixed | `PREMISE_ROLES` is replaced by `SlotRole::parse(role).is_some_and(is_premise_role)` over fsl-replay's `SlotRole`. |
| FND-016 | fixed | The check is now a plain `if item.kind… == Some(composition)` with an inner `if let Some(premises)`. |
| FND-017 | fixed | `PremiseKind::Guarantee` now carries `library: LibraryName`, parsed once in `premise_of` (a parse failure is `Unresolved`) and looked up directly in `dependencies()`. |

Disposition round 2 inspected the fix diff `cec151dd..6ff10e952a660f72c8452dd1294e286619ea4818`, which is two commits: 227382b0 (review fixes) and 6ff10e95 (runtime completion). The round-1 fixes FND-001..017 are intact; the diff does not touch them.

The late root cause is real. A `component_composition` node is minted only at emission (`CheckedComposition::semantic_node`, called from fsl-package emit.rs), so the source `CheckedGraph.semantic_graph` lacks it. `claim_family::node_form` now falls back to the checked `graph.compositions()`, the same way it does for refinements and task sets (claim_family.rs:211-217). It returns the existing `(Correspondence, ComponentComposition)` row, which maps to `Protocol` (FR-507). This is shared checked metadata, not a fake candidate or a hard-coded bypass. My round-0 verdict listed that row as clean and reached by AC-1; that was wrong, and only the runtime exposed it.

Runtime: the three `analyze_request_wire` default failures are measured as inherited. They fail identically on parent f87b2a5c43 (lines 113/321/477, raw logs in /tmp/qsl-3287-cec-and-parent). At cec the composition module had 4 passed, 9 failed and 1 ignored. The 6ff10e95 module run (expected 15 passed, 1 ignored) and the two package-helper consumer tests are queued, so nothing is green yet.

| FND | outcome | reason |
| --- | --- | --- |
| FND-018 | fixed | `resolve_names` no longer inspects composition nodes. Any unresolvable name over a package with an unnamed claim gets `PendingFamily::UnnamedClaim`, so no family is guessed from package contents. |
| FND-019 | fixed | `claim_family::is_component_composition` decodes the word through `Form::from_wire` (string_edge), and package_run.rs no longer spells the word. |
| FND-020 | fixed | `composition_clause` is now a direct struct literal with `kind` `Composition` and an unbounded extent. It makes no `value_clause` call and no mutation. `result_bound` is a required `NodeKey` on every `RequirementItem`, so filling it with the declaration's own node is the documented constraint. No framework rewrite is asked for. |
| FND-021 | fixed | The substring check is replaced. The test now asserts `category` `refusal`, `items == []` and the diagnostic code, plus cause and exit 21 on the route path. The remaining disjunctive oracle is new finding FND-022. |

Disposition round 3 is the arch-authorized extra completion round. It inspected `6ff10e95..cf48c388fc3fe694ec6a367771f0ad8a2982a325` (one commit) and reread `/tmp/qsl-2739-arch-ruling.txt` (mtime 02:58, extra-round proof-guard ruling).

Proof guard, verified in source:
- **Typed derivation.** `premise_items` assigns each premise its request index keyed by the typed `ItemKey`, whose `TemporalKey.subject` carries the FR-491 assumption set. It records each composition's component-guarantee indexes in `CompositionClaim.required`. Wire equality is never consulted, so `Sys`'s `Capped` and `SysRely`'s assumed `Capped` get different indexes.
- **Where the gate runs.** `negotiate_items` collects `gates` for every negotiated composition. `gate_compositions` runs in `settle_run` straight after `analyze_bound` and before `write_run`. `outcome.records` is the only place a provider `proved` exists, and `SettledParts` carries no aggregate, so no raw `proved` escapes.
- **Fail-closed cases.** Disposition-settled premises (S4 assumed `unsupported`, `Unresolved` → `failed`) and refuted, inconclusive, incomplete or cancelled premises are not `proved` in `records`, so each fails closed to `inconclusive` `ReplayRefused(UnsupportedProjection)` with no certification. A fault or refused `analyze_bound` returns `Err` with no records.
- **What is unchanged.** No proof is inferred from premises alone: the gate only demotes a provider's `proved`. With no provider the item stays `unsupported`. A provider's `unknown` gives `solver-inconclusive`. A provider's `refuted` on a composition gives `ReplayRefused(UnknownRequiredFeature)`, for `ProviderSelection::Composition` only, so the contract path is unchanged. A `proved` that passes the gate keeps FR-635's `certification(origin)`. There is no AG refutation and no built-in algorithm.
- **Tests.** The five provider tests use a real `ExternalProvider` registered through `analyze_with_providers`. Each names `backend` and publishes `profile` through `with_options`, per FR-967.

New FND-023 and FND-024 are low and contained. The spec addendum is not yet clear: SR-202611211049161 round 1 has FND-005..007 open. Runtime: cf48 module (expected 19 passed, 1 ignored) is queued and not green. The two helper consumers passed at 6ff. The three default failures are measured inherited.

| FND | outcome | reason |
| --- | --- | --- |
| FND-022 | fixed | The disjunctive oracle is gone. The test asserts exit 20, `category` `refusal`, `items == []`, `code` `invalid_package`, a null `cause`, and a message starting `invalid_package at /semantic_graph/operand_rows/`. That matches Draco's measured public diagnostic at 6ff (`invalid_package at /semantic_graph/operand_rows/9/0`). The already-admitted view guard is deferred to package-consumer-api by arch ruling. |

Disposition round 4 is the narrow final round and covers FND-023 and FND-024 only. The frozen source is fe674b00c1e8a8dae308ad9fcb5194973aedaa9e. Inspected:

- **19ff988c**: gate follow-through.
- **92726f02**: the test fixture, giving each named package case its own workspace. This fixes the only cf48 failure, `handoff.directory already exists`, which was fixture reuse and not a product defect.
- **fe674b00**: the merge of main cd7d1a7b once. The one package_run.rs conflict is resolved mechanically: main's removal of `PendingFamily::ProtocolSubject` is kept, and this branch's `Composition` variant, doc comment and `name()` arm are kept. FR-1044 is byte-identical to 92726f02.

Runtime at fe674b00 is queued and not green: the composition module (20 tests, 1 ignored) plus 4 `composition_gate_tests`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-023 | fixed | `gate_compositions` keeps a provider's `proved` only when `!required.is_empty()` and every required index is `proved`, so an empty list (the latent fixed-point item) fails closed to `inconclusive` `replay-refused`. The unit test `a_proof_with_no_guarantee_to_check_is_refused` covers it, beside tests for the stands, unproved/missing and ungated cases. |
| FND-024 | fixed | The invented `BackendId::new("composition")` fallback is gone. A `proved` record with no backend now settles `TerminalRecord::disposition_failed` with the existing typed `InternalFault::new("composition-gate", "proved-without-backend")`. |
