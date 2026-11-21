---
id: SR-20261121003296004
title: integrity review of QSL-2271 replay closure byte budgets
type: SpecReview
analysis: integrity
scope: agent-ix/filament-spec-language; QSL-2271; spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md;
  spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
review_set: subset
---

## Summary

Ticket: QSL-2271. Frozen PR filament-spec-language#3296, head f2f7950e1aa9f65497cb9cec1b9a0e590be01788; comparison base 79c0157721a1caed45d12f9976661e8aa076de13. Direct, bounded specification review; no applicable AssuranceProfile was found by type scan under spec/. Owner AGENTS.md explicitly keeps unmet implementation assurance separate. No enforced human-acceptance run was required or claimed.

Reviewed consistency and testability of FR-850 description, accounting rule, parity exception and AC-15 against TC-2410 and FR-071-AC-7. All use the same unchanged P_r plus D. The exception is singular and names exactly the route-specific numeric fields; successful records retain semantic parity. Trace chain: US-029 -> FR-850 -> TC-2410; FR-071 supplies the unchanged request bound. These edits change no stakeholder allocation. Refusal and exact-success oracles are observable and nonvacuous.

## Examined scope

```yaml
scope:
- id: FR-850-description
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'A `native-analyze/1` request MAY select an existing

    `filament.checked-package/v2` artifact through its `package` member instead

    of a `program` source (FSpec FR-520). The selection names the FR-517 handoff

    directory that holds the package. When it does, FSL SHALL admit the

    directory''s `checked-package.json` with the strict checked-package reader,

    attach the identity rows of its `identities.json` and, when present, the

    equality rows of its `equality.json`, admit the dependency artifact closure

    of `dependencies.json` with each package''s own witness files, and build from

    those files alone the anal'
- id: FR-850-replay-accounting
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: "Evaluation of a dependency-owned function or premise SHALL use that owner's\n\
    \  admitted package and witnesses. Replay SHALL consume the same retained\n  closure\
    \ without reopening source. Its existing `replay.input_bytes` bound\n  SHALL apply\
    \ to the unchanged reader-measured encoded request size of\n  [FR-071-AC-7](FR-071-implement-typed-replay-request.md),\
    \ plus the bytes of\n  each unique external dependency checked-package artifact\
    \ the replay consumes,\n  counted once by selected library identity. The encoded\
    \ request measurement\n  already includes the root package bytes and its target,\
    \ profile, sta"
- id: FR-850-route-budget-exception
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: "- **Named exception: route-specific encoded replay byte budgets.** The\
    \ source\n  and directory routes retain FR-071's actual encoded replay-request\n\
    \  measurements. The source route legitimately carries dependency-source\n  references\
    \ and source byte provisions that the directory route does not;\n  the directory\
    \ route SHALL NOT recover source to make those measurements\n  equal, and the\
    \ source route SHALL NOT discard its existing input charges or\n  change to a\
    \ different replay path to make them equal. Boundary comparisons\n  use each route's\
    \ own baseline `P_r` and the same consumed external artifact"
- id: FR-850-AC-15
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: For AC-12's handoff, compute `B` as the bytes of exactly the root, manifest,
    unique dependency package/witness files and selected model/runtime files read.
    At `dependent_bytes = B` intake completes; at `B - 1` it settles `incomplete`,
    `resource_exhausted`, exit `22`, naming `dependent_bytes` and its configured bound.
    A handoff whose manifest expands the selected-file total from 64 to 65 stops at
    the same phase naming the shared file limit before opening file 65. A diamond
    charges its shared triple once. For strict-read/native-join work and storage,
    an allowance equal to the completed closure p
- id: FR-850-AC-15-baseline
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'For route `r` (`source` or `directory`), measure `P_r` as FR-071-AC-7''s
    actual

    reader-measured encoded size of the replay request before adding external

    dependency artifacts. This includes the root checked-package bytes and every

    member already counted by that measurement: target, profiles, stage/source

    metadata and byte-provision labels and raw payloads, when present. `P_r` is

    not the root file length or a caller-declared size. In this fixture the source

    request carries dependency-source references and source provisions, so

    `P_source > P_directory`; those legitimate source charges remain inta'
- id: FR-850-AC-15-dependency-charge
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'Let `D` be the byte lengths of the unique external checked-package artifacts

    for `test/z` and `test/a` that the replay actually consumes, summed once per

    selected library identity. The same artifacts give the same `D` on both

    routes. The already-counted root is excluded, as are unconsumed artifacts;

    neither a second import path nor a repeated function call adds another copy.

    The total charged replay input is exactly `R_r = P_r + D`, with `D > 0`.

    No existing metadata or source-provision charge is subtracted from `P_r`.'
- id: FR-850-AC-15-refusal
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'Run each route''s same two-item request with `replay.input_bytes = P_r`.

    The encoded request itself fits; adding the consumed external closure gives

    attempted size exactly `R_r`, over bound `P_r`. The library-dependent item

    settles `inconclusive` with cause `replay_refused`, carrying the original

    `stage_limit_exceeded` / `input-bytes-exceeded` refusal, setting

    `replay.input_bytes`, bound `P_r` and actual `R_r`. It never settles `refuted`

    with an unverified counterexample, `failed`, or a whole-request admission

    refusal. `Bounded` still settles `proved` with its unlimited-run record, and

    the outc'
- id: FR-850-AC-15-success
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'At `replay.input_bytes = R_r`, each route''s library-dependent item settles

    `refuted` with the original counterexample and `replay.reproduced` `true`, and

    `Bounded` remains unchanged. Unlimited and exact-success semantic records agree

    across routes, with only the effective replay-byte-budget echo excepted.

    A reader that adds no dependency bytes incorrectly succeeds at `P_r`; a reader

    that adds the root twice incorrectly refuses at `R_r`. These cases therefore

    distinguish omitted dependency charges, preserved encoded-request charges and

    a duplicate root charge without moving an existing decode p'
- id: TC-2410-step-9
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 9. Measure the total files/bytes of the selected root and closure once,
    then run at the exact byte allowance and one below; test totals 64 and 65 files
    and exact/one-short strict-read/native-join work/storage allowances. For the transitive
    function counterexample of supplemental FR-850-AC-15, also request independent
    `Bounded`. For each route `r`, measure its unchanged FR-071-AC-7 encoded-request
    baseline `P_r`, including root bytes and all existing target/profile/stage/source/byte-provision
    metadata and payloads. The source fixture legitimately has `P_source > P_directory`.
    Measure the same u
- id: TC-2410-expected-step-9
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 9: exact intake/read/join allowances complete; one-short bytes/file-count
    stops are intake `incomplete`/`resource_exhausted`, exit `22`; read/join stops
    retain the original `i2.*` setting and typed refusal, with no per-dependency budget
    reset. Shared diamond files count once. At `replay.input_bytes = P_r`, the encoded
    request fits but the consumed closure produces actual attempted total exactly
    `R_r = P_r + D`: the library-dependent item is item-local `inconclusive`/`replay_refused`
    carrying `stage_limit_exceeded`/`input-bytes-exceeded`, setting `replay.input_bytes`,
    bound `P_r` and act'
- id: FR-071-AC-7
  path: spec/functional/FR-071-implement-typed-replay-request.md
  role: context_only
  excerpt: A request whose encoded size exceeds the configured reader bound refuses,
    and no truncated or partially-populated request is returned.
- id: FR-850-AC-12
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: context_only
  excerpt: A compiled root calls `test/z::answer`, which calls `test/a::answer` returning
    Integer `42`. Running the root function from source and from its FR-517 directory
    returns `42` on both routes. Its Counter temporal claim `always holds(c.value
    <= z::answer() - 40)` is analyzed with the same models/documents/items on both
    routes and gives equal item records, counterexamples and reproduced replays except
    the request selection. Deleting all unit/library source files and the handoff's
    `sources/` before the package run and analyze/replay changes neither result. Removing
    `dependencies.json` refuses `inva
- id: TC-2410
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: context_only
  excerpt: 'Verify that a `native-analyze/1` request selecting a v2 checked package
    is

    analyzed from its handoff directory''s package, manifest and root/dependency

    witness files alone, gives the source route''s outcome, and

    refuses malformed or mismatched package input before any engine runs.

    Scope: FR-850-AC-1 to FR-850-AC-5 and FR-850-AC-12 to AC-16.'
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for the bounded specification change. Reviewed consistency and testability of FR-850 description, accounting rule, parity exception and AC-15 against TC-2410 and FR-071-AC-7. All use the same unchanged P_r plus D. The exception is singular and names exactly the route-specific numeric fields; successful records retain semantic parity. Trace chain: US-029 -> FR-850 -> TC-2410; FR-071 supplies the unchanged request bound. These edits change no stakeholder allocation. Refusal and exact-success oracles are observable and nonvacuous.

## Verification and limits

Both edited artifacts validate with exit 0, 0 grammar findings; git diff --check passes. Source was inspected only as existing policy evidence, not reviewed or edited for implementation completeness. Runtime/Cargo tests, gap-analysis and aggregate gates were outside scope. Rust review is inapplicable (no Rust diff); object/app/security/architecture/criterion-strength analyses are inapplicable (no corresponding changed contract). Risk-complexity was not selected: the delta adds no novel technology, external contract, concurrency, policy volatility or performance guarantee. Matrix output includes pre-existing repository symbol/trace diagnostics and is not evidence of runtime conformance. Tool versions resolved to quoin 0.36.8 and quire 0.52.0 (engine 0.62.2), newer than the supplied release notice; no fallback was used.
