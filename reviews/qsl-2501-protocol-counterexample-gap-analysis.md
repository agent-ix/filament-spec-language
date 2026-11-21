---
id: SR-1795276384098317775
title: "Gap analysis — protocol counterexample serialization"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2501; repository computed matrix; changed shared counterexample assembly and FR-286-AC-33 through FR-286-AC-37 tests"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

## Summary

The changed criteria have executable trace bindings, actual Source/Package intake parity and complete emitted schema/adverse qualification. The bounded implementation gaps recorded by this review are closed; unrelated repository matrix gaps remain disclosed.

## Verdict

**PASS** — all original and same-disposition findings are fixed at `71e59b572727dc3e5bff8bc071ccfd818fb774d3`, with all six exact closing service filters green and the unchanged native projection pass retained. This is bounded PR acceptance; no whole-repository assurance or unrelated package-control completion is claimed.

Ticket: QSL-2501. Implementation draft, no PR number assigned. Frozen review head: `a65f49f8ccb103e8b3d2f2c948546fbdfce084fb`; base: `f5977125981288ea1bd36fb67f71780182055aac`. Review is read-only; no Cargo, runtime or aggregate gates were run. Initial author check exit 0 is reported by the lead, not independently rerun. Runtime acceptance is pending Draco service receipts. The FR-286 amendment already has an independent base review; this implementation review does not replace it. No CI files, manifests, pins or FR-352 record format changed in the initial draft.

No applicable AssuranceProfile was identified. Optional gap-analysis semantic review was not requested and is not claimed. Code-review behavior checks remain applicable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-286-AC-37 is untagged: no test validates the emitted full outcomes and protocol bodies against the official schemas, exercises required/unknown/mixed-kind/send_record adverses, or checks absent/substituted body refusal through the shared producer. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:873 |
| FND-002 | medium | The two native analysis tests exercise admitted packages directly and the retained hidden-choice regression exercises Source requests only; no test drives actual checked-Package request intake for the four required counterexample cases and compares it with Source intake. | fsl-analyze/tests/it/protocol_realizability.rs:1609; fsl-analyze/tests/it/protocol_projection.rs:1580; fsl-analyze/tests/it/hidden_choice_source.rs:12 |

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix (quoin 0.36.8, quire 0.52.0 / engine 0.62.2), explicit repository root. Raw `/home/peter/.cache/qsl2501-review-matrix.json`; command exit 0. The engine reports 632 requirements and 6170 criteria: 4833 tagged, 1137 untagged, 82 tagged-by-ignored-test, 105 method-without-symbol and 13 retired. All 6170 have no run evidence in that store. These are repository status facts, not a new runtime or whole-repository assurance claim. Existing unrelated gaps are not retired by this bounded change.

Changed criteria AC-33 and AC-34 bind protocol_counterexample_outcomes_keep_both_realizability_bodies, AC-35 binds protocol_counterexample_outcomes_keep_both_projection_decisions, AC-36 binds hidden_choice_source_refutes_with_a_counterexample, and AC-37 is untagged. No hand-written matrix was authored. Coverage extraction also completed (quire coverage exit 0), raw `/home/peter/.cache/qsl2501-review-coverage.json`. Its binding census reports Rust candidates 13552, tagged 12982, bound 12464. Repository diagnostics are 22236 trace-id-outside-marker, 453 untracked-id-has-minted-children, 22 uncatalogued-verification-method, 6 section-matches-nothing, 1 archetype-matches-nothing and 1 catch-all-universal. No local criterion id on the three newly added tests was unbound; one new FSpec-prefixed tag names an external criterion absent from this repository's local mint set, so it is not treated as a typo or local coverage. Existing repository diagnostics prevent an exhaustive assurance PASS and are not silently counted as covered.

Reverse inspection inventories four body alternatives, three shared assembly hooks (spell, take, item), and one document enum adapter. Each is owned by FR-286's amended serialization contract and FSpec FR-331, with no new unowned behavior. Source scans of the changed production files found no TODO, todo!, unimplemented!, test-only production bypass, empty stub or mock replacement of the unit under test. Native controls and the public command adapter are real production paths; the FR-290 negotiation double is an existing documented boundary seam. This reverse inspection is bounded to the changed surface; the rest of the very large source tree is not asserted exhaustively audited.

## Scope data

```yaml
{
  "scope": [
    {
      "id": "FR-286-AC-33",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "A genuine `IndistinguishableHistories` refutation assembled through both Source and checked-Package intake gives an `analyze` item with its original request index, claim, admitted request, `refuted` record and category violation. Its `counterexample` has `type` `protocol-analysis-counterexample`, `kind` `indistinguishable-histories`, the exact analysis key and role, both histories and required actions, obligation and separating witness of the typed result. Each history retains its initial state and ordered successor/transition steps in FR-331's encodings; each send step carries its send record"
    },
    {
      "id": "FR-286-AC-34",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "A genuine `EnvironmentDefeat` refutation assembled through both intake routes gives the same body type with `kind` `environment-defeat`, preserving its key, every `defeats_per_branch` play in branch order, obligation and separating witness; each play retains the data AC-33 requires. An empty or shortened replacement for a nonempty branch list, reordered branches or a substituted play fails the typed-result-to-body comparison even when the enclosing outcome validates structurally."
    },
    {
      "id": "FR-286-AC-35",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "Genuine projection refutations assembled through both intake routes give the same body type with `kind` `projection` and preserve the typed key, role, decision and separating witness. A choice decision retains both indexed cases, paths and nullable actions with its choice and owner; an acting-instance decision retains event, owner, both instances and maximum. The result remains `refuted`, category violation, with a separate canonical body and no fabricated realizability strategy."
    },
    {
      "id": "FR-286-AC-36",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "Running the exact Source request `fsl-analyze/tests/fixtures/hidden-choice-source/requests/protocols/protocol-unrealizable.json` with its referenced `handover-hidden-choice.fsl`, `lock.semantic-ir.json` and `lock-free.json` gives one `Handover` item at request index 0, `record.result` `refuted`, category violation and a non-null `protocol-analysis-counterexample` body equal to the genuine realizability counterexample. The document is `filament-outcome/1`, operation `analyze`, category violation, with the non-null last stage FR-621 assigns to the completed analysis, and exits 10. Exit 30 with `"
    },
    {
      "id": "FR-286-AC-37",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "For each of AC-33 to AC-35, the full emitted document validates against the published outcome schema and its body satisfies FR-331's positive/adverse protocol-counterexample contract vectors. Removing a required body member, adding an unknown member, using an unknown kind, mixing members of different kinds, omitting a send step's `send_record`, adding it to a non-send step or setting it to `null` fails that body contract. Removing the body makes the shared producer return `MissingCounterexample`; attaching a valid body from a different request item fails association checks. A record alone, a g"
    }
  ]
}
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: Official merged FSpec body/full-outcome schemas qualify four genuine emitted cases; required/unknown/mixed/send-record/numeric mutations plus shared missing/association/body-mismatch negatives are implemented and AC37 is now engine-bound. Focused runtime qualification remains pending. |
| FND-002 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: The new four-case test drives actual Source and handoff-selected Package command paths, deletes program.fsl before Package analysis, and compares request/claim/record/category and canonical body bytes. Focused runtime qualification remains pending. |

## Successor validation boundary

PR: agent-ix/filament-spec-language#3338. Reviewed frozen successor: `6a9ed4dc4d6971a6936c301cab47096e4b127505`. The lead reports scoped analyzer integration-target CHECK exit 0 (2m10s), `/tmp/protocol-counterexample-check-final.log`. Four exact runtime requests are queued to Draco, stop on first failure; no runtime result or final PASS is asserted. This reviewer ran no Cargo/runtime. Original findings remain unedited above. New docs-only delta was independently inspected.

Updated static reconciliation: quire matrix --scope <this repository> --format json, exit 0; `/home/peter/.cache/qsl2501-review-final-matrix.json`. Current base plus implementation has 632 requirements and 6185 criteria: 4859 tagged, 1127 untagged, 81 tagged-by-ignored-test, 105 method-without-symbol, 13 retired. AC33/34 bind native realizability and actual intake parity; AC35 binds native projection and actual intake parity; AC36 retains the exact Source fault regression; AC37 now binds all three new native/intake tests. Baseline repository gaps remain outside this bounded disposition, with no whole-repository PASS.

## Association qualification re-check during disposition pass 1

The new association negatives cover missing index, wrong backend, altered key and typed/wire disagreement, but not a genuine second request with the same key and a different assessed root. That scenario passes the current producer: the key explicitly excludes the root. AC37's different-item association obligation therefore needs this retained-authority negative before FND-001 closes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Pass 1 re-check supersedes the provisional fixed row: schema/adverse/intake tests are implemented, but genuine equal-key/different-root association is neither refused nor tested. |

## Tracker custody (review and disposition pass 1)

Posted only to existing private Linear QSL-2501. Public markers describe the verified PUBLIC GitHub repository, not tracker visibility. Original findings and current still-open assessed-root association disposition were each posted once after marker deduplication; all four bodies read back byte-exact and their JSON-compatible YAML parsed. Original code: 72b6ab1a-c670-48aa-a49e-d93ac40af930; original gap: bbe4e003-54f0-4393-b33d-9e66ba20ebef; code disposition1: 688343c9-e043-42a0-9186-899c9bbcd4b9; gap disposition1: 53abcf8c-1eb5-4dce-a720-c0af82160c34. Existing has-review label retained. No GitHub review comment, new ticket, status change or worktree edit was made.

## Assessed-root fix static inspection

Frozen `26d12e38de94e6818d203d7199992555986d1f6d` was independently inspected in full. TerminalRecord now privately retains the original decisive witness RuntimeValuePath at native settlement. The new ProtocolAnalysis assembly branch compares native.separating_witness().value_path with this retained authority alongside backend/key checks, before comparing the supplied wire with native.to_wire. The stored key, body encodings and other counterexample assembly paths do not change. The same original realizability test now runs a second genuine analysis differing only in declared assessed-root observation identity; it asserts equal analysis keys, unequal retained witness roots, acceptance against its own terminal record, exact native/wire agreement, and rejection after relabeling its public request index onto the original terminal. This addresses the remaining association scenario statically.

All four exact filters are statically ready. Final disposition awaits focused compilation and runtime at the fixed head; the old 6a9 service attempt produced no compiler/test evidence because FSpec cache FETCH_HEAD permissions failed during setup (exit 101). No test ran and no old-head runtime pass is retained. The service owns the narrow setup repair, with no source/pin workaround or interrupted job. This reviewer still ran no Cargo/runtime. The tracker pass-1 still-open rows remain the latest published disposition until the corrected-head evidence is available.

## Runtime evidence and package-family assessment (disposition pass 2)

At frozen `26d12e38de94e6818d203d7199992555986d1f6d`, Draco reports three exact integration filters each 1 passed, 0 failed, 0 ignored, 2635 filtered, exit 0: original Source hidden-choice fault; both native realizability bodies plus backend/key/body and genuine same-key/different-root association negatives; both native projection decisions plus S7/S8 stages. These are lead-provided service receipts, not local reviewer runs. The fourth exact intake/schema filter selected 1 and failed (0 passed, 1 failed, 0 ignored, 2635 filtered, exit 101). Its first hidden Source result and full/body schema assertions completed before it reached the real Package command. The Package path then returned exit 21, unsupported_construct/not-yet-implemented, with message items[0] names the declaration family, no package form yet, items empty and last_stage null. No later Package cases or remaining loop cases ran, so neither all-four intake parity nor complete emitted schema qualification is established. Raw log `/home/peter/.cache/handoff/protocol-batch-2-review2501-impl/raw-26d/qsl-2501-26d12e3-intake-schema.log`, SHA256 `d7283f50a916d79bc97655791b7dd704af206f232aa4ca897b3091139a511690`, independently read and hashed.

This is a real missing Package request family, not a fixture alias or wrong capability kind. FR-636 Inputs expressly declares local-projection and realizability as distinct protocol item kinds. FR-850 Description and Equality with the source route require matching items, records, counterexamples and exit code from selected admitted bytes. FR-940 Behavior's protocol-subject paragraph requires the admitted protocol-node subject and forbids refusing merely because a package is selected. The merged FR-286 AC33–35 explicitly require both intake routes. No contract currently permits replacing this required result with unsupported.

The implementation resolves only temporal protocol DeadlockFreedom and RaceFreedom specially in document/package_run.rs:750–772, and a protocol declaration otherwise reaches PendingFamily::Declaration at :807 or :825. Named and Route have no protocol-analysis variant (lines 220–269), and package options leave protocol_analysis None. Merely changing a request alias, temporal.item_kind or claimed capability cannot request the same realizability/projection semantics through another admitted arm. Rewriting the test to direct native package analysis or accepting exit 21 would weaken the mandated intake oracle.

A bounded implementation plan must also account for realize.rs:503–514: admission unconditionally asks evaluator.protocol for a CheckedProtocol and admits capacities from that source clause. Bare evaluator input.rs:2380 returns None from protocol, exposing its actual admitted ProtocolExecutionView separately at :2384. Thus an envelope arm alone still fails native realizability admission. Reuse the existing admitted package clause/capacity/subject machinery and shared native request validation; do not manufacture CheckedGraph or CheckedPackage, duplicate the engine, or invent a source fallback. Projection already consumes CheckedPackageV2 directly. Production extension is held for arch ruling; no route or oracle change is authorized merely by this assessment.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Pass 2: the first Source schema assertions ran, but actual Package admission fails before its body/schema checks and the remaining loop cases; full required qualification remains incomplete. |
| FND-002 | still-open | Pass 2: the new actual intake test exposes a real missing package realizability/projection envelope route, returning unsupported declaration rather than matching Source; arch scope ruling is pending. |

## Tracker custody (disposition pass 2)

Actual three-green/one-red service evidence and the remaining real Package-family gap were posted to private Linear QSL-2501 once after marker deduplication. Code disposition2: 25be54fd-f003-45b9-a052-697ca3985b16. Gap disposition2: 70dec42d-97c8-4bb5-8b46-3be8ebb6c3a6. Both bodies read back byte-exact and their machine data parsed. The code association finding is fixed; the gap artifact still blocks acceptance. No final PASS or ticket closure is claimed.

## Approved package admission adapter static checkpoint

The original Package intake gap now has an arch-approved contained implementation at cae82c749c1c4fbce3a5debb8bb8e87deb01bae1 plus the independently identified/fixed backend guard at d1e00e757368fb10f9ed2fc17a15f65ec770ec6a. The same source/Package four-case parity oracle is unchanged. Existing native symbolic-capacity qualification now additionally compares genuine Source and admitted Package ProtocolDefinition admission for empty/full-u64 bindings, UnknownName, Duplicate, unbound order and qualified channel names; it makes no symbolic execution-support claim. The two existing request-bound filters exercise the shared Source validator after extraction. Six exact filters are statically ready; the original native projection pass is unaffected and retained. The latest gap dispositions remain still-open until actual Package analysis completes and all four emitted body/schema cases run. No gate was weakened and no new runtime evidence is claimed.

## Final static candidate before the adapter runtime batch

Frozen clean head `ab0416d2ea6f91b89cec6f7ebc67d6c7142671cb` was inspected. The last delta from d1e00e7573 changes only two fixture files: package_selection retains the existing analyze-unit default through a wrapper, one shared implementation accepts an explicit handoff namespace, and only the new parity test passes the exact original single-unit program.source.identity. Production Source derives exactly that identity; no key normalization or original fixture alteration occurs. The four retained Source fixture files and protocol.rs/package_program.rs symbolic/nonempty execution guards have no diff from 26d. The lead reports final focused IT CHECK exit 0 in 22.17s, `/tmp/protocol-counterexample-package-final-check.log`. Six agreed exact filters are statically ready, with the unchanged native projection PASS retained. Actual adapter Package parity and emitted schema qualification remain pending; no final PASS is asserted.

## Actual adapter runtime follow-up

At `ab0416d2ea6f91b89cec6f7ebc67d6c7142671cb`, the first exact parity/schema test selected 1, passed 0, failed 1, ignored 0, filtered 2635, exit 101. The hidden Package case now negotiates protocol-analysis and reaches an item at S6c but returns exit 30, runtime_invariant/established-invariant-broken, aggregate incomplete. This confirms the backend dispatch fix reaches the engine; it does not qualify Package parity. The remaining five requested tests did not run and stay held. The prior unchanged native projection pass is retained. The original three 26d native/Source passes remain historical evidence of that head, not new adapter qualification. Raw `/home/peter/.cache/handoff/protocol-batch-2-review2501-impl/raw-ab0/qsl-3338-ab0416d-test01.log` was read and independently hashed SHA256 `b7d4d4411e23619d572999d2c0e11ffe466c049e891383ca1d4aa4a2284034eb`.

The typed cause is deterministically reachable without a local run: realize.rs:582 calls ProtocolSystem::new, which delegates to new_with_loop_proof at protocol.rs:1442; that function's :1454 asks evaluator.protocol and Bare returns None. UnknownProtocol propagates as RealizabilityRefusal::Protocol and the owned item adapter maps it to fault. ProtocolSystem::new_package at :1603 is a separate existing admitted-package constructor. The contained repair selects the existing constructor using the already admitted ProtocolDefinition rather than adding an engine/compiler/model port or source-shaped graph. This static chain establishes a cause before expansion, not a claim that no later package guard can fail. No Cargo/runtime was run here, no parity oracle was weakened, and unrelated SSH activity inspection failure does not block relay/service execution.

## Same-disposition Choice and constructor checkpoint

The authorized Package reachability and Choice extension has targeted static coverage: the original four-case intake/schema test already contains receive-binder guards and both realizability variants, so no replacement fixture or weaker oracle is needed. Constructor routing is corrected in 499d35d; admitted Choice/case execution is added in e238; its omitted binder-reader entry was found and corrected in 98f72a656a67fd2a297260304c19c65080afe9ac. The same exact six filters remain required after #3342's independently owned Parallel/Case origin ancestry fix lands and the branch integrates it. Existing symbolic/control refusals remain typed; symbolic execution support is not expanded. Original gap findings remain pending actual Package parity and complete emitted schema/adverse execution. No fresh full gap method, new SR or broader test run was started.

## Targeted shared-dependency transfer inspection (same disposition)

Clean candidate `71e59b572727dc3e5bff8bc071ccfd818fb774d3`, based on fresh main `07b3c42ba071662867ebc9312301c761527cac0a`, was inspected only for the arch-approved dependency split and coherent consumer integration; no fresh full review method was started. The two BareSlot/ProtocolView files are byte-equal to reviewed source commit `1c9d783869305280f9ec68e1bf3483d4d532afab`. The channel/cycle declaration-path message identity changes match selected source commit `f35f8fac7a030b73888a72d4226309f0c6bf2785`; identity.rs preserves fresh main's additional recommendation instance/actor fields instead of reverting them. Both Source and Package Program builds create the path-to-native-ID message index; cycle consumers translate through that index and retain their internal native-ID counters. The native slot origin helper derives exact labelled Parallel/Choice ancestry from the admitted tree, without source text, and is reused rather than copied.

The six transferred files exclude #3342's weak footprint/read APIs, extra fixtures and mixed Parallel+Send guard removal. The mixed guard at package_program.rs:418 remains, and other intentional control/correlation/repeat-proof/symbolic queue guards remain relative to fresh main, apart from the expressly authorized Choice admission. Fresh main's NotMarkov diagnostics and replicated-role acting inventory survive the rebase; native counterexample key/root/backend/body association checks, Choice guard readers and constructor routing remain intact. The lead reports the shared source slice was already reviewed with four focused passes; this reviewer reuses that evidence and checks the integration, without claiming a new execution. The unchanged native projection result remains retained.

Targeted static READY for the same six closing filters is contingent on the running focused check and their actual service results. No final PASS, broad capability retirement or QSL-2501 closure is asserted. Linear access is embargoed until the lead releases it around 19:30Z: no tracker query, comment, label or state operation was performed for this checkpoint; only these existing cache artifacts were updated. No reviewer Cargo/runtime or worktree edit occurred.

## Final current disposition — all closing filters green

**PASS** at frozen `71e59b572727dc3e5bff8bc071ccfd818fb774d3`. TitanCode closing service receipts simple3338-71e5-c1 through c6 each report rc 0, selected 1, passed 1, failed 0 and ignored 0. These are accepted exact-head service results forwarded by the lead; no local reviewer execution or inferred counts are substituted. No additional evidence packet or raw archive is required for this disposition.

| Receipt | Exact filter | rc | Selected | Passed | Failed | Ignored |
| --- | --- | --- | --- | --- | --- | --- |
| simple3338-71e5-c1 | `hidden_choice_source::protocol_counterexample_intake_parity_and_schema` | 0 | 1 | 1 | 0 | 0 |
| simple3338-71e5-c2 | `hidden_choice_source::hidden_choice_source_refutes_with_a_counterexample` | 0 | 1 | 1 | 0 | 0 |
| simple3338-71e5-c3 | `protocol_realizability::protocol_counterexample_outcomes_keep_both_realizability_bodies` | 0 | 1 | 1 | 0 | 0 |
| simple3338-71e5-c4 | `protocol_realizability::a_symbolic_capacity_premise_carries_the_bound_value_in_the_key` | 0 | 1 | 1 | 0 | 0 |
| simple3338-71e5-c5 | `protocol_item_bounds::an_absent_profile_or_bound_settles_missing_member` | 0 | 1 | 1 | 0 | 0 |
| simple3338-71e5-c6 | `protocol_item_bounds::an_out_of_range_profile_or_bound_is_refused_naming_it` | 0 | 1 | 1 | 0 | 0 |

The four-case actual Source/Package intake, complete emitted schema/adverse qualification and exact canonical body equality now finish successfully, including original hidden-choice Source fixtures. Native key/backend/body/original assessed-root association negatives, full-u64 key preservation, constructor and case-guard reachability, symbolic capacity admission parity and shared profile/bound refusals are exercised by the exact closing filters. The earlier unchanged native projection filter at 26d remains retained: one pass, no failure or ignored case, including both native projection decisions and S7/S8 distinctions. It is not represented as rerun at 71.

This is the final targeted disposition of the original reviews, not a fresh full method or a whole-repository assurance claim. Existing unrelated matrix gaps and QSL-2271 package-control remainders are not retired. Mixed Parallel+Send and the other unimplemented execution guards remain typed, and no symbolic execution support is claimed. Original finding text and intermediate failed receipts remain unedited. The same SR IDs are retained.

The candidate stays frozen at 71e59b572727dc3e5bff8bc071ccfd818fb774d3. Current main has an independently owned compile repair pending; the lead must wait for its authoritative repair SHA before any rebase, and perform final context/diff verification and admin matched-head merge. This reviewer changed no worktree file or Git state and ran no Cargo/runtime.

### Final tracker blackout boundary

All Linear view/query/comment/label/state actions remain embargoed until 19:37 UTC. This final checkpoint performs no Linear action. Closing disposition comment bodies are prepared locally once for the lead to publish after the blackout, with marker deduplication and exact readback then. Publication is not required to return these merge-ready cache artifacts.

### Final finding outcomes

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC37 schema/adverse/association tests now execute successfully: c1 completes all four emitted full/body schema cases and c3 passes the genuine assessed-root association negative. |
| FND-002 | fixed | The actual four-case Source/Package command intake loop passes c1 after source deletion, with unchanged strong result/request/body oracles and exact original Source fixture identities. |
