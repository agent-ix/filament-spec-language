---
id: SR-967175026302197768900474181941935084755
title: "QSL-2823 analyze code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3484; QSL-2823 analyze and X19; QSL-2817 provider parity"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen source: `30cd4a356dec52c8e781840b604c1ab55a98fb65`. Tickets: QSL-2823 (Z2/Z5/Z6/Z8/Z12/X19) and QSL-2817 provider parity. Exact-target compilation passed on titan merged tree `3def70481b`; eight of nine named analyze cases passed, with the new nested cause assertion the sole failure. Z12 printed `value_occurrences = 25` and its exact/one-short boundary passed. The three unchanged X19 qualification consumers passed earlier. No local build, test, repository executable or titan job ran in this review. `git diff --check` passed. No applicable AssuranceProfile exists under `spec/`; textual matches are older review prose.

The probability tests now require genuine independently checked exact evidence and real reproduced path-set evidence. The new comparison helper covers every ProbabilisticCounterexample member; horizon-zero paths contain exactly one empty cylinder and no subsystem, and the initial content comparison uses the model's canonical state key alongside its full initial product key. Other exact fields, item/binding/scheduler and mass are checked. The certificate mutation control changes value 1 to 0 and requires rejection. Existing supported bounded-always contracts permit these horizon-zero fixtures without claiming the separate Health-chain AC-23/AC-56 case. Search calibration observes the real finite four-outcome walk, asserts all four values, tests the exact highwater and one fewer, and refuses partial publication; it does not compute an independent guessed charge count. The typed limit/cause remains exact.

Package/source default selection compares full item records and exit status while retaining CtlFp identity. Both timed tests retain complete exact-delay/operation expectations, certification and replay. Service emission before/after provider calls and byte equality replace the superseded refusal; measured rejected parity and all three real witness replays remain. FR-280-AC-5 still carries its older Service-refusal premise; the explicitly assigned stale-test ruling and current FR-185-AC-7 supersede that premise, while the provider parity part is retained. This review changes no requirement.

The consuming manifest aliases and lockfile move together to the full FSpec SHA `9985576655bd1e176c6b112aae9d398b0ca9bbaa`, consuming merged #805. The lead verified upstream main reachability and retention of STD-712; upstream fixture/source review and both named upstream tests passed. No sibling-checkout reader, copied upstream fixture, new dependency artifact, workflow edit or production implementation is introduced. Helper search found no existing horizon-zero full witness comparison to duplicate.

## Verdict

CONDITIONAL — fix the one concrete stale-shape assertion; no other scoped source finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new nested assertion reads kind.cause.cause, but the current envelope writer emits kind.cause as the scalar returned by undefined_reason. This gives Null and fails the required division-by-zero check on titan. Restore the direct kind.cause scalar assertion, retaining the typed undefined cause/site replay, exact position/locus and refuted/exit-10 oracles. | fsl-analyze/tests/it/clause_item.rs:191 |

## Coverage

Reconciliation: static `quire matrix --scope /home/peter/dev/worktrees/qsl-2823-analyze-tests --format json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2823-analyze-tests --format json` completed and their JSON was parsed; the combined static command exited 0. No runtime evidence store was read. Installed tools: /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2), /home/peter/.npm-global/bin/quoin 0.36.8. `quoin matrix --repo /home/peter/dev/worktrees/qsl-2823-analyze-tests --json` exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`, reproducing the previously reported native-size limitation. The installed documented static fallback above succeeded; this is not a native or runtime-backed matrix PASS.

Matrix counts: 5020 tagged, 1081 untagged, 68 tagged-by-ignored-test, 107 method-without-symbol, 15 retired. Coverage: 6209 untracked symbols, 7084 unmatched tags, 23504 diagnostics (22866 trace-id-outside-marker, 608 untracked-id-has-minted-children, 22 uncatalogued-verification-method, 6 section-matches-nothing, 1 archetype-matches-nothing, 1 catch-all-universal). Rust census: 14444 candidates, 13869 tagged, 13271 bound. These are tool totals, not individually established new defects. FR-281-AC-7, FR-185-AC-7, FR-280-AC-5 and FR-1011-AC-2 have actual active bindings in the changed tests; FR-201/FR-202 broad artifact tags carry inherited minted-child diagnostics and do not establish Health-chain criterion coverage.

Plan completion: not assessed

Reverse/stub inspection followed all eight changed paths and seven behavior groups (undefined envelope, exact certificate admission, path-set replay, package defaults, timed model steps, Service provider parity, native retention budget), plus the pin consumer. Existing real production seams were read: outcome/counterexample::undefined and undefined_reason, probability witness/limits APIs, ModelSystem canonical state keys, retain_outcome's original meter charge, and provider settlement. No new unspecified production behavior, source/test stub, unconditional skip or coverage-inflating replacement exists in this test-only diff. A full repository reverse census is not established. No scoped gap is manufactured from that limitation. Optional separate semantic gap lens was not requested and is skipped; required Rust code/test/requirement checks ran. No spec/plan files changed, so formal spec-review and its sub-analyses are inapplicable.

Changed paths examined:

- `Cargo.toml`
- `Cargo.lock`
- `fsl-analyze/tests/it/analyze.rs`
- `fsl-analyze/tests/it/analyze_package_selection.rs`
- `fsl-analyze/tests/it/analyze_timed_cli.rs`
- `fsl-analyze/tests/it/clause_item.rs`
- `fsl-analyze/tests/it/exact_probabilistic_item.rs`
- `fsl-analyze/tests/it/plan_search.rs`

## Examined scope

- FR-281-AC-7 — spec/functional/FR-281-analyze-claims-with-in-process-engines.md; role: examined; excerpt: Over a finite model with one reachable state at which the invariant `100 / x > 0` divides by zero, an EN-1 item claiming that invariant settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names that state and whose `cause` is `division-by-zero`; its `ItemCounterexample` is of kind `undefined-evaluation` and replay reproduces the undefined value at that state, the record carries no `undefined` label, and FR-285 maps the outcome to exit 10.
- FR-286-AC-14 — spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; role: examined; excerpt: Over FR-281-AC-2's record and `ItemCounterexample`, `from_analyze` gives an item with `result` `refuted`, category violation, naming `en-1`, a `counterexample` of kind `formula` with a non-empty `prefix`, a `loop`, an `initial_state` and a `trace_position`, and no `position`, `decisive` or `witness`; the same record with no counterexample returns `Err(OutcomeItemError::MissingCounterexample)`, and FR-281-AC-1's `proved` record gives an item with no `counterexample`. For each other `Counterexample` variant, a `refuted` record with that variant gives an item whose `counterexample` has the varian
- FR-185-AC-7 — spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; role: examined; excerpt: `Service` (with `d` typed `Millis = Int[1, 12]`) checks at S3 and S4 emits it with its random parameters, reward and workload as FR-740 nodes; the package reader admits it and gives back the checked record.
- FR-246-AC-4 — spec/functional/FR-246-check-task-sets-and-schedulability-claims.md; role: examined; excerpt: `Ctl` with `schedulable CtlFp on Ctl under fixed_priority;`, `schedulable CtlEdf on Ctl under edf;` and `response R on Ctl.c <= 12 ms under fixed_priority;` writes three requirement records of kind `schedulability`, each keyed by its claim node's `claim` occurrence, extent `Bounded`, no owner, empty extent roots and path condition, and its policy; `items_from_requirements` over them and `Engines::fsl()`'s registry returns three items in occurrence-key order whose candidate set is `["en-7"]`; the same records read from the compiled package's `schedulability_claims` rows give equal items; a task
- FR-632-AC-1 — spec/functional/FR-632-request-timed-zone-items.md; role: examined; excerpt: A timer subject whose operation fires within 2 time units settles the MITL claim `eventually[0,2]` `proved` with basis `zone-certified`, certification `certified`, and `eventually[0,1]` `refuted` with a counterexample whose delays are exact rationals and replay.
- FR-760-AC-1 — spec/functional/FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md; role: examined; excerpt: `cargo metadata --locked` for the workspace lists `filament-specification-qualification` from `https://github.com/agent-ix/filament-specification` with a `tag` or full-sha `rev` source, and the published positive fixture `proposals/checked-package-v2/fixtures/positive-control-operations.json`, read from that pinned package, carries the `contract_version` FSL writes and reads (`fsl_package::CHECKED_PACKAGE_V2`).
- FR-760-AC-3 — spec/functional/FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md; role: examined; excerpt: `cargo xtask pin-check` passes over the workspace manifests, every `agent-ix` git dependency of which names a `tag` or a full 40-hex-character `rev`; over a scratch manifest with `filament-contract-model = { git = "https://github.com/agent-ix/filament-contract-ir", branch = "main" }` it exits 1 and names the manifest and `filament-contract-model`.
- FR-003-AC-12 — spec/functional/FR-003-format-native-source.md; role: examined; excerpt: Formatting `present (x)`, `forall (r in c : p)`, a parameter `(v : T)`, `Int [0,9]`, `always [0,30] holds (p)`, `convert < T > (e)`, `Option < Int[0,9] >` and `a<b` prints `present(x)`, `forall(r in c: p)`, `(v: T)`, `Int[0, 9]`, `always[0, 30] holds(p)`, `convert<T>(e)`, `Option<Int[0, 9]>` and `a < b`; `over (o: M::O)`, `when (w.l = 0)` and `as (r: Boolean)` keep one space before `(`; removing all whitespace from input and output gives equal bytes.
- FR-1011-AC-2 — spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md; role: examined; excerpt: A low `accounting.work_units` names `accounting.work_units`, not a `plan_search` setting.
- FR-280-AC-5 — spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md; role: context_only; excerpt: `Service` (FR-185-AC-7), whose `emit_checked` refuses `unsupported_construct`/`not-yet-implemented`, is passed to the provider entry as its `CheckedPackage`, with its subject binding, the `SourceProvision` of `Service`'s source and the item settled `Supported(en-4)` for the `2 ms` claim of ADR-024 §7.2 over `Service` (the claim of FR-193-AC-1 and FR-193-AC-5) with evidence kind `statistical`, the Okamoto method (FR-189-AC-1), seed 7 and `max_witnesses` 3. The entry returns one record, `measured` with decision `rejected`, equal to the record `analyze` returns for that item over the same checked
- FR-202-AC-14 — spec/functional/FR-202-replay-a-probabilistic-witness.md; role: context_only; excerpt: At horizon 0, initially true P gives an empty accepting path of mass 1 refuting AtMost 1/2, while initially false P gives an empty rejecting path refuting AtLeast 1/2; opposite-verdict empty paths yield parity. At horizon 2, a terminal initially false P closes by false extension with mass 1 and no fabricated stutter step, refuting AtLeast 1/2. A live undecided prefix after one failure does not refute; after two failures horizon rejection does, even though another model action is enabled. An initial true P path extended by an operation cannot reproduce.
- FR-202-AC-23 — spec/functional/FR-202-replay-a-probabilistic-witness.md; role: context_only; excerpt: Over FR-201-AC-56's `Health` chain (value `81/100`), `probability >= 0.9 [always[0,2] holds(ok)]` is refuted by the checked-subject bounded entry with the rejecting cylinders fault after one step (`1/10`) and fault after two steps (`9/100`), mass `19/100 > 1/10`; the cylinder that reaches the accepting decided key after position 2 (`81/100`) refutes `probability <= 0.8 [always[0,2] holds(ok)]`. The rejecting set without its second cylinder (mass `1/10`, equal to `1 − 0.9`) yields replay parity, and a cylinder naming a model-only key yields no reproduced token.
- FR-201/Closed reachability document profile — spec/functional/FR-201-check-a-probability-certificate.md; role: examined; excerpt: The bounded subset SHALL admit   origin-activated event-position `eventually[0,h] holds(P)` and   `always[0,h] holds(P)` with a finite nonnegative position horizon and a   checked state predicate P; its undecided monitor product SHALL terminate,   even when the underlying model graph is cyclic.
- FR-940/Package forms — spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md; role: examined; excerpt: The package form SHALL return the value the source   form returns for the package and the source it was compiled from.
- FR-1011/Behavior — spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md; role: examined; excerpt: A denied charge SHALL stop the search `incomplete` with `limit-reached{limit, value, setting}`.
- spec/spec.md — spec/spec.md; role: examined; excerpt: name: filament-spec-language
