---
id: SR-1795276384098317274
title: "Code review — shared protocol counterexample assembly"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2501; fsl-analyze/src/analyze.rs, analyze/document.rs, protocol_analysis/item.rs; fsl-replay/src/protocol_analysis.rs, protocol_analysis/counterexample.rs; three changed integration test modules and four hidden-choice fixtures"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

## Summary

The shared protocol-counterexample serialization and approved Package/Choice adapters now satisfy the bounded reviewed contract. All recorded findings are fixed; exact closing service qualification passes.

## Verdict

**PASS** — all original and same-disposition findings are fixed at `71e59b572727dc3e5bff8bc071ccfd818fb774d3`, with all six exact closing service filters green and the unchanged native projection pass retained. This is bounded PR acceptance; no whole-repository assurance or unrelated package-control completion is claimed.

Ticket: QSL-2501. Implementation draft, no PR number assigned. Frozen review head: `a65f49f8ccb103e8b3d2f2c948546fbdfce084fb`; base: `f5977125981288ea1bd36fb67f71780182055aac`. Review is read-only; no Cargo, runtime or aggregate gates were run. Initial author check exit 0 is reported by the lead, not independently rerun. Runtime acceptance is pending Draco service receipts. The FR-286 amendment already has an independent base review; this implementation review does not replace it. No CI files, manifests, pins or FR-352 record format changed in the initial draft.

No applicable AssuranceProfile was identified. Optional gap-analysis semantic review was not requested and is not claimed. Code-review behavior checks remain applicable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new protocol body serializes its structural key through the existing JSON-number u64 codec; a valid capacity or bound above 2^53-1 then fails the mandatory outcome canonical encoder instead of preserving the completed refutation. The new body needs its ruled lossless key spelling while existing key encodings remain unchanged. | fsl-replay/src/protocol_analysis/counterexample.rs:57; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:665 |
| FND-002 | high | The shared item producer chooses protocol evidence only by request_index and trusts its public wire Value, accepting another request's valid body or a body altered independently of its typed native evidence without checking backend, original analysis key or typed-body equality. | fsl-analyze/src/analyze.rs:1260 |
| FND-003 | medium | The new ProtocolAnalysis counterexample inherits item_stage's S8 catch-all, but an acting-instance projection is built directly as a typed witness without replay; FR-621 requires S7 for that path, so the document overstates its last stage. | fsl-analyze/src/analyze.rs:1226; fsl-analyze/src/protocol_analysis/project.rs:163 |

## Evidence and limits

The new typed enum preserves both realizability alternatives and both projection decisions. Named Serialize types cover each closed variant; send_record is omitted for non-send transitions, and existing native transition/send/witness encoders are reused. Play ordinal conversions are checked; no narrowing cast, new unsafe code, parser, compatibility layer or fake source graph was introduced. Source regression fixtures were introduced as complete retained files; exact external-byte custody is a lead-owned verification, not inferred from names. Native test assertions inspect complete ordered play lists, both action alternatives, original key and witness, nullable projection actions, role and branch order. The tests do not yet validate full emitted schema or exercise both request intake routes; the separate gap artifact records that deficiency.

Realizability runs replay_realizability_counterexample before refuting, and Choice projection runs replay_projection_counterexample. ActingInstance returns directly from project.rs:163 with a typed witness, demonstrating FND-003's distinct stage path. Existing nonprotocol handling is outside the proposed repairs.

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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The new public counterexample enum and key accessor, and the artifact counterexample accessor, promise replayed evidence for every variant; ActingInstance is a direct typed S7 witness, so these comments misstate the public guarantee even though the revised stage branch is correct. | fsl-replay/src/protocol_analysis/counterexample.rs:20; fsl-replay/src/protocol_analysis/counterexample.rs:31; fsl-analyze/src/protocol_analysis/item.rs:39 |

## Static disposition checkpoint

Updated frozen head `2b545df1a346c4f1aeb3f7f033095a7e826e9573` on base `b1a310b605208560a361022877997a1d80740027` was read in full. The same four retained Source fixture files are byte-identical to the initial review head. FR-286 contract is now independently merged, and the qualification pin selects merged FSpec `2bec0061eaaa91f095eb1cafe6de11519302d337`. The body-only KeyWire exhaustive view converts max_depth, max_history, literal/symbolic capacity values and delivery lower/upper to decimal strings; other native key representations remain unchanged. Full-u64 test assertions cover all six member positions and prove the stored key remains numeric. Original native key is now retained privately in TerminalRecord settlement; the new ProtocolAnalysis branch compares backend and key and compares supplied wire against native.to_wire. ActingInstance is expressly S7 while the actually replayed realizability and Choice forms stay S8. No other counterexample branch changes these contracts.

The four requested runtime filters are statically ready. Production fixes and their test bodies are present; no runtime PASS is asserted until the Draco service returns the exact checks. FND-004 requires the three documentation comments to stop promising replay for direct witnesses.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: Body-only KeyWire converts all six potentially full-u64 key leaves to decimal strings; the native/stored key codec is unchanged. Focused runtime qualification remains pending. |
| FND-002 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: Settlement privately retains its original key; the new variant compares backend/key and recomputes native body before accepting its supplied wire. Focused runtime qualification remains pending. |
| FND-003 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: ActingInstance is an explicit direct S7 witness; native realizability and Choice counterexamples remain S8 after their actual replay. Focused runtime qualification remains pending. |
| FND-004 | fixed | Pass 1, `6a9ed4dc4d6971a6936c301cab47096e4b127505`: Docs-only successor 6a9ed4dc4d removes all three inaccurate replay promises. Focused runtime qualification remains pending. |

## Successor validation boundary

PR: agent-ix/filament-spec-language#3338. Reviewed frozen successor: `6a9ed4dc4d6971a6936c301cab47096e4b127505`. The lead reports scoped analyzer integration-target CHECK exit 0 (2m10s), `/tmp/protocol-counterexample-check-final.log`. Four exact runtime requests are queued to Draco, stop on first failure; no runtime result or final PASS is asserted. This reviewer ran no Cargo/runtime. Original findings remain unedited above. New docs-only delta was independently inspected.

## Association re-check during disposition pass 1

The lead asked whether the key is unique to an assessed root. It is not: artifact.rs explicitly documents that AssessedRootDraft is not a key member. project.rs:93 constructs the key before constructing the root separately at :111; realize.rs:522 likewise constructs the key before root admission at :559. AssessedRoot::record writes its observation/object root into the separating witness. Two genuine analyses differing only in assessed_root can therefore have equal keys/backend and distinct witness roots. Relabeling the second public ItemCounterexample.request_index to the first item and supplying both its native evidence and matching wire passes the current checks. This is the original substituted-body association finding, not a new generic validation scope. The repair needs original settled witness-root authority retained independently of the key and compared against the native counterexample's witness root, plus a same-key/different-root genuine negative.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | Pass 1 re-check supersedes the provisional fixed row: the structural key excludes assessed_root, leaving the genuine same-key/different-root substitution accepted. |

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
| FND-002 | fixed | Pass 2: the native root-retention comparison is implemented and the genuine equal-key/different-root substitution negative ran in the passing realizability filter. Other original static findings retain their fixed outcomes. |

## Tracker custody (disposition pass 2)

Actual three-green/one-red service evidence and the remaining real Package-family gap were posted to private Linear QSL-2501 once after marker deduplication. Code disposition2: 25be54fd-f003-45b9-a052-697ca3985b16. Gap disposition2: 70dec42d-97c8-4bb5-8b46-3be8ebb6c3a6. Both bodies read back byte-exact and their machine data parsed. The code association finding is fixed; the gap artifact still blocks acceptance. No final PASS or ticket closure is claimed.

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The first package adapter built native protocol-analysis work but its expected-backend guard still treated every Temporal form except GlobalConformance as model-check work, so a correctly negotiated local-projection or realizability backend overwrote that work with UnsupportedRequestedCapability before it could execute. | fsl-analyze/src/analyze/document/package_run.rs:1992 |

## Package adapter static inspection

Arch authorized the bounded request-family adapter and thin Capacities::admit_definition, without symbolic execution, schema/pin, compiler, graph or engine expansion. Initial adapter `cae82c749c1c4fbce3a5debb8bb8e87deb01bae1` was inspected across all six files. Successor `d1e00e757368fb10f9ed2fc17a15f65ec770ec6a` corrects FND-005 by expressly selecting the protocol backend for GlobalConformance, LocalProjection and Realizability before the temporal model-check fallback, and corrects the admitted package name accessor.

The extracted protocol_analysis_selection preserves Source member validation order and exact named refusals. Both routes use the same native item constructor with admitted CheckedPackageV2, original subject/root settings and native engine adapters. Package request resolution retains the protocol declaration occurrence and native PackageModelInput, attaches its specific analysis settings and candidates, and uses the existing negotiation/echo/settlement path. Realizability reads ProtocolDefinition through the existing route-neutral input accessor. Source capacity admission delegates its old CheckedProtocol implementation; package admission reads the admitted channel queues and symbolic names and factors the same unknown/duplicate/unbound binding rules. The existing package symbolic-execution guard and nonempty-capacity-binding guard remain unchanged. No value from bytes is made into CheckedGraph or CheckedPackage.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | Pass 3 static inspection: d1e00e7573 includes LocalProjection and Realizability in the protocol-backend expected arm, preserving actual native work instead of overwriting it. Fresh focused runtime remains pending. |

All six proposed exact filters are statically ready at d1e00e757368fb10f9ed2fc17a15f65ec770ec6a, subject to the running focused compile check. The unchanged native projection filter's prior PASS is retained; no new runtime PASS is asserted here.

## Final static candidate before the adapter runtime batch

Frozen clean head `ab0416d2ea6f91b89cec6f7ebc67d6c7142671cb` was inspected. The last delta from d1e00e7573 changes only two fixture files: package_selection retains the existing analyze-unit default through a wrapper, one shared implementation accepts an explicit handoff namespace, and only the new parity test passes the exact original single-unit program.source.identity. Production Source derives exactly that identity; no key normalization or original fixture alteration occurs. The four retained Source fixture files and protocol.rs/package_program.rs symbolic/nonempty execution guards have no diff from 26d. The lead reports final focused IT CHECK exit 0 in 22.17s, `/tmp/protocol-counterexample-package-final-check.log`. Six agreed exact filters are statically ready, with the unchanged native projection PASS retained. Actual adapter Package parity and emitted schema qualification remain pending; no final PASS is asserted.

## New findings (disposition pass 3, runtime follow-up)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | Package realizability admission now reads a route-neutral protocol definition but still constructs the transition system through source-only ProtocolSystem::new; that constructor unconditionally asks the Bare evaluator for a source CheckedProtocol and returns UnknownProtocol, which the owned item adapter collapses to runtime_invariant instead of producing the required Package refutation. | fsl-analyze/src/protocol_analysis/realize.rs:582; fsl-eval/src/simulation/protocol.rs:1454; fsl-analyze/src/protocol_analysis/item.rs:243 |

## Actual adapter runtime follow-up

At `ab0416d2ea6f91b89cec6f7ebc67d6c7142671cb`, the first exact parity/schema test selected 1, passed 0, failed 1, ignored 0, filtered 2635, exit 101. The hidden Package case now negotiates protocol-analysis and reaches an item at S6c but returns exit 30, runtime_invariant/established-invariant-broken, aggregate incomplete. This confirms the backend dispatch fix reaches the engine; it does not qualify Package parity. The remaining five requested tests did not run and stay held. The prior unchanged native projection pass is retained. The original three 26d native/Source passes remain historical evidence of that head, not new adapter qualification. Raw `/home/peter/.cache/handoff/protocol-batch-2-review2501-impl/raw-ab0/qsl-3338-ab0416d-test01.log` was read and independently hashed SHA256 `b7d4d4411e23619d572999d2c0e11ffe466c049e891383ca1d4aa4a2284034eb`.

The typed cause is deterministically reachable without a local run: realize.rs:582 calls ProtocolSystem::new, which delegates to new_with_loop_proof at protocol.rs:1442; that function's :1454 asks evaluator.protocol and Bare returns None. UnknownProtocol propagates as RealizabilityRefusal::Protocol and the owned item adapter maps it to fault. ProtocolSystem::new_package at :1603 is a separate existing admitted-package constructor. The contained repair selects the existing constructor using the already admitted ProtocolDefinition rather than adding an engine/compiler/model port or source-shaped graph. This static chain establishes a cause before expansion, not a claim that no later package guard can fail. No Cargo/runtime was run here, no parity oracle was weakened, and unrelated SSH activity inspection failure does not block relay/service execution.

## Targeted approved Choice disposition (same disposition pass 3)

Arch authorized the bounded admitted Choice/case adapter inside the original PR, not a new engine or a broad review round. Constructor/private lifetime fix `499d35d673d61030be29c3eca2bd1dc316e84d68` now selects the existing Source or Package transition-system constructor from ProtocolDefinition. Admission borrows the owned request for the system's scope, and all three private admission callers retain that request until the result has settled; no public result borrows the request. This statically corrects FND-006 without changing native Source semantics.

Choice commit `e238a4a16275cec04fde9ae5d3a8ff4e2d3eaf11` was inspected only over its four changed fsl-eval files and the required native readers/stepper. Source guards retain the same CheckedBody references under a private enum and call the unchanged decide path. Package Choice reads owner and cases from admitted ControlBody, selects cases.guard slots by the admitted case order, and extends the existing branch-wrapper topology with Case controls. Predicate evaluation uses the existing package observation, typed binder supply and evaluator; the same settling loop retains exclusivity/exhaustiveness checks. No BareSlot origin helper, source-shaped graph, wire, pin or other control guard is changed by this adapter. The authoritative Parallel/Case ancestry helper from #3342 remains an external integration requirement; no copy or unmerged dependency was introduced here.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The new Package Choice guard slots were absent from Program::new_package's binder reader inventory, so frame(choice) removed the received binder before evaluating heard.granted and liveness could also discard it. Choice guards must contribute their existing slot_binders at the owning Choice control. | fsl-eval/src/simulation/protocol/program.rs:1299; fsl-eval/src/simulation/protocol.rs:2740 |

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | Same pass 3 static inspection: 499d35d selects ProtocolSystem::new_package for admitted Package definitions and scopes the borrowed request correctly. Focused constructor/Choice runtime is still pending. |
| FND-007 | fixed | Same pass 3 static inspection: 98f72a656a67fd2a297260304c19c65080afe9ac chains every admitted Choice guard into the same reader inventory at its owning Choice ID, preserving both frame values and binder liveness. |

### Concrete intentional-refusal audit

The suspected owned-item Unsupported fold is not reached by the intentional constructor guards. A symbolic package queue returns PackageProgramRefusal::Unsupported(Channel), maps to ProtocolRefusal::Unsupported, and realize::admit catches that into Settled::Unsupported(EvaluatorUnsupported) before the owned item adapter. Correlation, RepeatProof and other unsupported control constructor paths use the same typed catch. A nonempty package capacity-binding request is a distinct Capacity(UnknownName), which the item adapter already maps through invalid(refusal.cause()); it is not an unsupported case and retains that typed input refusal. No redundant generic item-adapter fallback was added. Other typed faults remain faults; no hypothetical taxonomy expansion is claimed.

Targeted static READY at 98f72a656a67fd2a297260304c19c65080afe9ac is conditional on the authoritative ancestry integration and actual six-filter runtime. No reviewer Cargo/runtime ran, and no final PASS is asserted.

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
| FND-001 | fixed | Body-only lossless key view retains all six full-u64 quantities; native realizability full-range assertions passed in c3. |
| FND-002 | fixed | Original backend/key/witness root and native-to-wire equality are enforced; genuine equal-key/different-root substitution refusal passed in c3. |
| FND-003 | fixed | ActingInstance retains direct S7 and actually replayed evidence S8; complete intake/stage parity passed c1 and the unchanged native projection result remains retained. |
| FND-004 | fixed | All three public comments accurately describe native decisive evidence without promising replay for direct witnesses. |
| FND-005 | fixed | Package expected-backend guard retains native protocol-analysis work for both new kinds; c1 reaches and completes all four Package analyses. |
| FND-006 | fixed | Admission selects the existing Package constructor with a scoped borrowed request; c1 completes both native Package realizability cases. |
| FND-007 | fixed | All Choice guard slot reads enter the existing inventory at their owning Choice node; c1 executes received-binder guards and preserves Source/Package parity. |
