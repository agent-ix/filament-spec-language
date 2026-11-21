---
id: SR-1795279163610073935
title: "gap-analysis — QSL-2271 Parallel and Send"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2271 PR 3342; 16 changed Rust files and selected criteria"
review_set: subset
---

## Summary

Ticket QSL-2271; agent-ix/filament-spec-language PR #3342. Frozen head f35f8fac7a030b73888a72d4226309f0c6bf2785; base dd993b018aad1a47469a7f2ccb3b99e176a2165d. All 16 changed Rust files examined. The initial static pass found no concrete defect; subsequent root-owned runtime disproved route parity (FND-001 below).

## Verdict

**PASS** — all five bounded runtime filters pass and FND-001/FND-002 are fixed. Exact-final-head all-target compile and timed matched-head merge remain root-owned merge conditions; full-repository assurance is unassessed.

## Coverage

Plan completion: not assessed

`quire matrix --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-parallel-send --format json` exited 0. Actual installed tools: /home/peter/.npm-global/bin/quoin 0.36.8; /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2); machine terra; spec-artifacts-process@v0.36.0. The generated matrix contains 632 requirement documents, but only four rows and their changed-slice bindings were interpreted. FR-940-AC-6, FR-940-AC-22, FR-206-AC-4 and FR-220-AC-15 compute tagged. The new identity/weak-read functions bind to FR-940-AC-6 and replay binds to FR-940-AC-22 / FR-206-AC-4. These are partial regression contributions, not full satisfaction of the multi-part criteria. Broad FR/TC tags on the original combination test alone do not establish criterion coverage.

The command emitted symbol-id-collision and unmarked-trace-id diagnostics on unrelated fixtures/historical comments, including alternate cfg imports in fsl-semantics/src/check/mod.rs and xtask legacy trace comments. Exit 0 does not mean those diagnostics are resolved. No baseline comparison or full diagnostics audit was performed; no new-tool regression or whole clean matrix is claimed.

Changed-slice behavior inventory: four groups examined: semantic message identity/cycle lookup; native constraint projection/read derivation; analyzer typed refusal adaptation; source-absent mixed-program acceptance/replay. Owning contracts are FR-205/FR-894/FSpec FR-425, FR-220/ADR-025 MA-1/FR-155, and FR-940. Newly untraced groups: 0. Source stubs within slice: 0. Test stubs within slice: 0. No copied schema/fixture or duplicate footprint engine introduced. Typed default evaluator refusal is deliberate; BarePackage has the concrete implementation.

The four changed test bodies use genuine handoff admission, public analyzer paths, explicit expected statuses/counts, queue identities and full graph comparison. They do not mock behavior or rely only on existence checks. New projection-refusal branches and reference-literal membership preservation were traced statically; there is no new dedicated forced-projection-refusal or created-literal runtime fixture here.

Whole-source public-surface inventory, unrelated untagged rows, parent matrix completion, ignored-test inventory, qualification/metrics evidence and full-workspace assurance are unassessed. The optional separate semantic-review mode was not invoked. Ordinary changed-code/test faithfulness was checked as part of code review. No plan was selected or inferred.

## Runtime Receipt — First Execution

Root-owned exact filter `analyze_package_protocol::package_channel_send_parallel_combinations_match_both_routes` at frozen f35f8fac7a030b73888a72d4226309f0c6bf2785 exited 101: 0 passed, 1 failed, 0 ignored, 2639 filtered out. The four remaining filters were held and NOT RUN; no runtime passes are established. This supersedes the initial pending state recorded above.

The failing assertion is at analyze_package_protocol.rs:1826, route equality. Loop ordering identifies the first input as SC / inside. Source settles proved/exhaustive/certified, certificate accepted true, 5 states and 4 transitions. The source-absent Package settles failed at S6c, internal-failure, backend en-1, runtime_invariant / established-invariant-broken. The Source result is proved, not refuted.

Raw log: /home/peter/.cache/handoff/protocol-batch-2-review-parallel-send/raw-f35/qsl-3342-f35f8fa-test01.log. SHA-256 independently checked: 0edcdbc59c6e760f8d1d2c542ee043a89d7ebc4e077818c633d5fe4a1d994919.

Independent static diagnosis: access.rs chain_of filters the SC parallel out before the new package weak-read derivation. The immediate failure therefore does not establish a defect in that derivation. The newly unguarded mixed path reaches native construction, fork/settle, record supply, normalization and closure certification. The public outcome erases the precise internal invariant via the existing refusal adapter; the raw log does not identify its originating typed variant. No exact hidden cause is invented here. Preserve the route-equality oracle and source-absent execution; diagnose and correct the narrow native-path invariant before replacing the unsupported guard. Do not weaken the oracle, suppress the error or add Source reconstruction. No reviewer runtime or code edit was performed.

## First Fix — Targeted Static Recheck

Frozen fix candidate: 1c9d783869305280f9ec68e1bf3483d4d532afab. Delta from f35f8fac7a030b73888a72d4226309f0c6bf2785 is three Rust files, +161/-4. FND-001 wording and ids above are retained unchanged. FND-001 remains open until runtime evidence confirms the fix; this static recheck grants readiness to rerun the same five exact filters, not runtime acceptance.

The originating mismatch is statically grounded: source protocol slot origins carry complete control.path including branch labels; canonical IR declaration paths omit Branch/Case holders. The previous BareSlot origin built from declaration.skip(2) therefore omitted `left`, and OriginRows::pair could report MissingLocationProjection before evaluate_package_control collapsed the failure to broken(). The original raw receipt alone did not identify that hidden variant.

The fix reconstructs only the slot-site path from the admitted protocol's unique run ancestry, using each actual branch/case label and body edge. It refuses missing nodes, malformed members and repeated ancestry through existing typed native view/slot refusal variants. It scans the complete tree rather than accepting the first matching location; it reads no source text and changes no canonical key, descriptor, schema or source emission. Existing children/records helpers remain the topology owners.

The original combination fixture now asserts the precise native origin (`protocol Flow run Main Both left Left Only body` inside, `protocol Flow run Main Only body` after) and calls real project_protocol_slot before returning the same authentic package request to the existing parity helper. The expected proved result, complete item/exit equality and source removal remain unchanged. package_request is called once per workspace; the probe does not write a second handoff.

No new static defect found in the targeted three-file fix. Coder-reported focused analyzer IT compile check exit 0 is compilation-only evidence. No reviewer Cargo/runtime/code edits performed. The same five exact runtime filters are pending against this new head; the original f35 failure remains recorded.

## First Fix — Runtime Disposition

At frozen 1c9d783869305280f9ec68e1bf3483d4d532afab, the root reports actual exact c1 original-combinations PASS (1 test) and c2 identity/concrete-and-cycle PASS (1 test). These receipts support FND-001 fixed: the original SC/inside failing input and the direct real-origin projection now pass, and identity graph parity passes. Raw receipts for this head have been requested by root; this reviewer has not independently read those raw files yet.

The clean actual rerun of c3 package_parallel_send_preserves_weak_model_reads fails: 0 passed / 1 failed / 0 ignored. label() panics on a null Source item at analyze_cli_support.rs:287 while evaluating the assertion at analyze_package_protocol.rs:2253. This occurs before package_protocol_graph / Package construction and before any weak-read counts/shared-set comparison. The expected Source result is proved, but its actual diagnostic is not printed because label() panics before the outer assertion message is evaluated. Therefore the underlying Source refusal/diagnostic is undetermined. A preceding stale-shared-target compile failure is not the c3 test result. c4/c5 were held at this receipt; root requested only those remaining two, preserving c1/c2 greens. No c3 Package runtime acceptance is claimed.

Static fixture sanity check rejects one unsupported hypothesis: Fill's Attempt.n uses Tally, and bounded("Tally", "0") supplies min 0/max 0; it is not an unbounded Integer payload. Cell.v is Level0..3. Any fixture repair must follow the actual generated Source diagnostic, preserve finite domains and the intended TSO/RA over-versus-capture assertions, and leave production footprint semantics unchanged unless separately justified. The immediate diagnostic repair is to assert a terminal Source record with the complete outcome document before calling label(), retaining the expected proved check.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Root-owned actual c1 and c2 PASS at 1c9d783869305280f9ec68e1bf3483d4d532afab; original mixed SC inside route parity, direct native origin/projection, and concrete/cycle identity graph parity pass. |

## FND-002 — Actual Source Admission Diagnostic

The reviewer independently read all five local raw receipts at `/home/peter/.cache/handoff/protocol-batch-2-review-parallel-send/raw-1c/pr3342-1c9d78-c{1..5}.log`: c1/c2/c4/c5 each report 1 passed, 0 failed, 0 ignored, 2639 filtered; c3 reports 0 passed, 1 failed. Thus four of five selected filters are green at 1c9d783869305280f9ec68e1bf3483d4d532afab. The retained c4 replay/refusal test takes 0.75s and c5 Source consuming-codec regression 0.11s. These are inherited exact receipts, not reruns by this reviewer.

Diagnostic-only head 79bc1e3df6f5711064c5b3c282bdb3aa20a7f932 adds five assertion lines without changing production or the Source/Package behavioral oracle. Its actual exact c3 fails on the first `tso/false` fixture before Package execution: Source returns category refusal, invalid_package/operator-ineligible, details.path `/semantic_graph/nodes/56/body/result_type`, last_stage S4, items empty. It is not an unbounded or weak-footprint result. Raw receipt `/home/peter/.cache/handoff/protocol-batch-2-review-parallel-send/raw-79/recovery-pr3342-79bc1e-c3.log` was independently read and SHA-256 verified as `849599ea1d27c76ba8e3f958f1f413af2f7bc41d0ca831f2d45fab48ed61f2ec`. Compile time in the raw file is 4m35; actual test time is 0.08s; exit 101, 0 passed / 1 failed / 0 ignored / 2639 filtered.

FND-002 remains open. The exact source expression/type represented by node 56 has not yet been independently mapped. Do not infer a profile, domain or production-footprint change from the pointer alone. The read-count oracle remains 1 for over and 2 for capture, under both TSO and RA. No new full review or runtime was run by this reviewer.

## FND-002 — Targeted Fixture Repair Static Disposition

Candidate 27c6892950c0e42e01558626c02d53617bbfd388 reuses the existing finite_channel_model() in the weak-read fixture. Relative to the first origin fix, the net test delta is that helper substitution plus the five-line Source-document guard before label(). The helper retains Cell, Level, Idle and Attempt and all eight Cell operations; it changes Attempt.n to the existing finite Level0..3 and excludes unrelated unsupported model types/constructs. Fill contracts and the writer operation remain intact. This is a fixture dependency-closure repair, not a Variant implementation or a production guard change.

The four TSO/RA × over/capture cases, the original `observed_payload.n <= binder.v` predicate, Source expected proved, exact over=1/capture=2 read counts, shared-set comparison, complete graph comparison, source removal and analyzer item/exit parity remain unchanged. The temporary failure-only emission probe is absent. Static targeted inspection finds no oracle weakening or new production change; the candidate is ready for the one exact c3 runtime retry. No new full review was performed.

Root reports the preceding 04c5951b44937145f9e6fa72eded68a9da3852e9 c3 ran past Source but failed in PackageModelInput with the existing unsupported Form{Variant, node:test/fill/variant_type/ix://test/fill/Mode}; it did not qualify the weak-read assertions. The separate legitimate Tally0..0 declared-result-type identity defect remains named on existing QSL-2271 for later work, not fixed or silently broadened in this PR.

FND-002 remains open pending actual c3 success at this candidate. Four prior actual 1c c1/c2/c4/c5 passes are retained. No Cargo/runtime/worktree edit or tracker action was performed by this reviewer. Linear publishing is explicitly held by owner direction until 19:37 UTC; final comments/state writes must be prepared locally and written once after that hold.

## Final Targeted Disposition

The root received the actual Titan code-seat result: `simple3342-27c6-c3 rc=0: 1 passed, 0 failed, 0 ignored` for exact `analyze_package_protocol::package_parallel_send_preserves_weak_model_reads` at qualified fixture head 27c6892950c0e42e01558626c02d53617bbfd388. This completes all five bounded filters when combined with the retained c1/c2/c4/c5 actual passes at 1c9d783869305280f9ec68e1bf3483d4d532afab. This reviewer did not run runtime or request another raw collection.

FND-002 is fixed: the repaired fixture now reaches and passes the unchanged TSO/RA × over/capture cases, Source proved check, exact read counts 1 versus 2, shared sets, source-absent Package graph comparison and full analyzer parity. FND-001 was already fixed by direct native slot origin/projection and original combination/identity passes. All earlier pending/open statements above are historical and superseded by this final disposition; original finding text is unchanged.

Current local context-only rebased candidate reported by root: 6b02d8bcbec9f4d8d1c11cf73f34d6bcf76ae695 on main 19264. Root verified all twelve remaining edit streams preserved and the runtime test file byte-equal to the qualified fixture candidate; shared identity/origin work landed in #3338. No runtime repeat is claimed on the rebase. This is the original review's targeted closure, not a fresh full review.

**Review verdict: PASS for this bounded five-filter slice. Merge qualification remains conditional:** under the owner's 19:39 policy, root must run a successful exact-final-head `cargo check --locked --all-targets` across all touched crates plus fsl-analyze and filament-spec-language after committing these SR files, then matched-head merge within ten minutes; repeat if main moves. This reviewer has not run or received that final compile result. No whole-repository assurance or gate completion is implied.

### Final Findings Disposition

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Actual original combination and identity/concrete-cycle exact filters pass at 1c9d783; direct native origin and projection assertions pass. |
| FND-002 | fixed | Actual exact weak-read filter passes at 27c6892 with all four intended cases and unchanged exact read/parity assertions. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The first SC inside-branch Parallel+Send case returns Package runtime_invariant/established-invariant-broken while Source proves and certifies the same deadlock-freedom item. | fsl-analyze/tests/it/analyze_package_protocol.rs:2017; fsl-eval/src/simulation/protocol/package_program.rs:401 |
| FND-002 | medium | The weak-model-read regression panics on a null Source item label before Package construction, so its native footprint assertions are never reached. | fsl-analyze/tests/it/analyze_package_protocol.rs:2253; fsl-analyze/tests/it/analyze_cli_support.rs:287 |

## Examined Scope

```yaml
{
  "scope": [
    {
      "id": "fsl-analyze/src/analyze/items.rs",
      "path": "fsl-analyze/src/analyze/items.rs",
      "role": "examined",
      "excerpt": ") -> Refusal {"
    },
    {
      "id": "fsl-analyze/tests/it/analyze_package_protocol.rs",
      "path": "fsl-analyze/tests/it/analyze_package_protocol.rs",
      "role": "examined",
      "excerpt": "    package_protocol_graph("
    },
    {
      "id": "fsl-eval/src/simulation/model.rs",
      "path": "fsl-eval/src/simulation/model.rs",
      "role": "examined",
      "excerpt": "    StateClauseEntry, TerminalWhen,"
    },
    {
      "id": "fsl-eval/src/simulation/model/footprints.rs",
      "path": "fsl-eval/src/simulation/model/footprints.rs",
      "role": "examined",
      "excerpt": "        let mut symbolic ="
    },
    {
      "id": "fsl-eval/src/simulation/model/input.rs",
      "path": "fsl-eval/src/simulation/model/input.rs",
      "role": "examined",
      "excerpt": "/// Why the admitted constraint of a protocol node has no read footprint."
    },
    {
      "id": "fsl-eval/src/simulation/protocol.rs",
      "path": "fsl-eval/src/simulation/protocol.rs",
      "role": "examined",
      "excerpt": "    /// A package protocol constraint retains its original selection refusal."
    },
    {
      "id": "fsl-eval/src/simulation/protocol/access.rs",
      "path": "fsl-eval/src/simulation/protocol/access.rs",
      "role": "examined",
      "excerpt": "use super::ProtocolRefusal;"
    },
    {
      "id": "fsl-eval/src/simulation/protocol/channel.rs",
      "path": "fsl-eval/src/simulation/protocol/channel.rs",
      "role": "examined",
      "excerpt": "            node,"
    },
    {
      "id": "fsl-eval/src/simulation/protocol/cycle.rs",
      "path": "fsl-eval/src/simulation/protocol/cycle.rs",
      "role": "examined",
      "excerpt": "use fsl_semantics::check::ProtocolNodeId;"
    },
    {
      "id": "fsl-eval/src/simulation/protocol/identity.rs",
      "path": "fsl-eval/src/simulation/protocol/identity.rs",
      "role": "examined",
      "excerpt": "    node: ProtocolNodeId,"
    },
    {
      "id": "fsl-eval/src/simulation/protocol/package_program.rs",
      "path": "fsl-eval/src/simulation/protocol/package_program.rs",
      "role": "examined",
      "excerpt": "        // A branch's source-local numeric node indices are not admitted"
    },
    {
      "id": "fsl-eval/src/simulation/protocol/program.rs",
      "path": "fsl-eval/src/simulation/protocol/program.rs",
      "role": "examined",
      "excerpt": "    /// Execution lookup for a message's route-independent declaration path."
    },
    {
      "id": "fsl-eval/src/value/expression/evaluate/reader_source.rs",
      "path": "fsl-eval/src/value/expression/evaluate/reader_source.rs",
      "role": "examined",
      "excerpt": "/// MA-1 reads of an admitted protocol constraint. `receivers` contains only"
    },
    {
      "id": "fsl-eval/src/value/expression/mod.rs",
      "path": "fsl-eval/src/value/expression/mod.rs",
      "role": "examined",
      "excerpt": "    package_constraint_reads, package_guard_reads, package_operation_footprint, package_term_reads,"
    },
    {
      "id": "fsl-eval/src/value/mod.rs",
      "path": "fsl-eval/src/value/mod.rs",
      "role": "examined",
      "excerpt": "pub(crate) use expression::reference_occurrences;"
    },
    {
      "id": "fsl-semantics/src/check/footprint/read_node.rs",
      "path": "fsl-semantics/src/check/footprint/read_node.rs",
      "role": "examined",
      "excerpt": "/// [`super::derive_node_reads`] over a body of any [`ReadNode`] source."
    },
    {
      "id": "FR-206-AC-4",
      "path": "spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md",
      "role": "examined",
      "excerpt": "A `fifo` channel of capacity 1 under `block`: a second `send` is not enabled while the queue holds one entry. Under `reject` the second send is enabled, leaves the queue unchanged and its step records a send record with outcome `rejected`. Under delivery `at-least-once`, `duplicate` is enabled when capacity admits it and `lose` never is; under `at-most-once`, `lose` is enabled and `duplicate` never is. An `unordered` channel holding two distinct payloads gives two `receive` steps; a `fifo` channel gives one, for the head."
    },
    {
      "id": "FR-220-AC-15",
      "path": "spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md",
      "role": "examined",
      "excerpt": "In `SB`'s `parallel` under `tso`, an `event` reader whose constraint reads `x.v` beside the branch whose enabled attempt stores `x.v` explores, its read a `relaxed` load in the reader's step with one successor per value its thread can see; with that store carrying `acquire`, it explores as a `relaxed` store and the record lists the restriction. Under `sc` the reader reads memory, and a `finish` whose constraint reads `x.v` after the `join` explores under `tso`."
    },
    {
      "id": "FR-940-AC-6",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-"
    },
    {
      "id": "FR-940-AC-22",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "AC-4's Counter request over source with the items `Bounded` and `AtMostTwo`, run with `replay.input_bytes` set below the length of the package's emission (the up-front read-back is bounded by `i2.input_bytes` and admits the package), settles `AtMostTwo` `inconclusive` with cause `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded` naming `replay.input_bytes`, never `refuted`, never `failed` with `analyze`/`package-emission`, and settles `Bounded` `proved` as without the setting; the document's category and exit follow its items (category inconclusive, never `internal-failure"
    },
    {
      "id": "FSpec-FR-425",
      "path": "filament-specification@2bec0061eaaa91f095eb1cafe6de11519302d337/spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md",
      "role": "examined",
      "excerpt": "- `<node>` is the node's static declaration path ([FR-221](../../objects/protocol/FR-221-protocol-node-occurrence.md))."
    },
    {
      "id": "FR-894",
      "path": "spec/functional/FR-894-keep-message-delivery-and-business-effect-identities-apart.md",
      "role": "context_only",
      "excerpt": "FSL SHALL give a protocol's messages, deliveries and business effects three\nseparate identities, in the protocol system and in the assessment of a\nrecorded execution (FR-895). It SHALL never derive one identity from\nanother (FSpec FR-051, FR-226).\n\n- **Message identity.** A `send` step's send record (ADR-027 ST-3) is the\n  message identity: the protocol instance, the send node, the thread path and\n  the send's occurrence ordinal on that path.\n- **Delivery identity.** A message has one delivery for each copy in its\n  queue. The original copy is delivery 0, and each `duplicate` step (ST-5)\n  add"
    },
    {
      "id": "FR-155",
      "path": "spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md",
      "role": "context_only",
      "excerpt": "Partial-order reduction needs to know which locations each transition reads\nand writes. S3 SHALL derive each operation's read footprint from its checked\n`pre` and `post` clause bodies, with the receiver and parameters symbolic,\nand its write footprint from its frame (ADR-021 POR-1). `ModelSystem` SHALL\ninstantiate both per transition identity, and SHALL enforce them on every\nstep it takes for the model checker: writes through `decide_frame` at entry\nscope (FR-154), reads by evaluating each clause over an observation\nrestricted to the read footprint (POR-2). A footprint is therefore a checked\nf"
    },
    {
      "id": "FR-205",
      "path": "spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md",
      "role": "context_only",
      "excerpt": "FSL SHALL build a temporal claim's **protocol subject**, a model subject\n(FR-125) together with one checked protocol clause of its package and the\nclause's bindings, as FSpec FR-425 defines it, and SHALL key each protocol\nstate by FSpec FR-425's state key, so that two states with equal keys are\none state (ADR-027 PS-1 to PS-8, FO-3, AE-2)."
    },
    {
      "id": "fsl-package/src/bare_slot.rs",
      "path": "fsl-package/src/bare_slot.rs",
      "role": "examined",
      "excerpt": "control_site_path(package, p, &node.node_id).map_err(|refusal| match refusal {"
    },
    {
      "id": "fsl-package/src/protocol_view.rs",
      "path": "fsl-package/src/protocol_view.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn control_site_path("
    }
  ]
}
```
