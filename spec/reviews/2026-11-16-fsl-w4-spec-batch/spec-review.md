---
id: SR-2026111603433200000001
title: "Spec review of the FSL wave 4 spec readiness batch"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language#2734 (fsl-spec-w4, 448e79d0) and agent-ix/filament-specification#661 (fsl-spec-w4, 36061142); git diff origin/main...HEAD in each tree"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: reviews
---

## Summary

Tickets: QSL-2002, QSL-2274, QSL-2275, QSL-2269. One combined review of both
diffs: the base checklist, with EARS, integrity and failure-domain passes folded
in. No cargo or make run; stubs were judged by reading them against the crate APIs.

Verdict: REQUEST CHANGES. One medium finding (refusal order in FSpec FR-323) and four low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new clock refusal sentence sits just before "It refuses with the first of these, each rule applied over all entries ... before the next rule". "These" now reads as the clock refusals, and nothing places them against rules 1 to 3. `unused-clock-selection` can only be decided after every recompile. Fix: move the paragraph after the list. Put `missing-clock-selection` and `clock-kind-mismatch` in rule 2 (recompile refusals) and `unused-clock-selection` in a new rule 4 after rule 3. Mirror the order in language FR-640. | filament-specification spec/objects/interfaces/FR-323-native-runtime-envelope.md:97-112; spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md:178-189 |
| FND-002 | low | Replay serves libraries from the union of `temporal_bindings` rows and `dependency_clock_selections`. So a package that leaves out the entry the emitter must write (library clock shared with a program row) still replays, and nothing enforces the completeness rule. Fix: serve library units from `dependency_clock_selections` alone (FR-322 makes shared entries equal the rows), and refuse `missing-clock-selection` when an entry is absent. | spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md:124-135,166-175; filament-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:227-252 |
| FND-003 | low | FR-640 says `missing-clock-selection` names "the clock and the library", but FSpec FR-323 and its AC-38, and FR-640-AC-11, name only the clock. Fix: name the library in all three, or drop it from FR-640. | spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md:178-180 |
| FND-004 | low | The FR-495-AC-20 stub never checks that the work-limit refusal is located at the `mod`, which AC-20 and TC-2211 require. Fix: assert that `refusals[0].location` (or `limit.region`) is the `mod` node's span. | fsl-semantics/tests/it/division_law_selection.rs:322-333 |
| FND-005 | low | FR-091-AC-76 says S2 builds `absent` and `none` as the same `Absent`, but the non-ignored test never writes `absent`. Fix: add `function a using v(): Option<Integer> pure { absent }` to the admitted cases. | fsl-semantics/tests/it/option_values.rs:67-104 |

## Scope and evidence

- QSL-2002 (FR-495, NFR-011, FSpec FR-147-AC-14). The no-closed-form claim is
  hedged correctly ("no known"): deciding whether n has a factor of at most k is
  the decision form of factoring. Closed form 1 (`B-A+1 >= U` gives `[0, U-1]`)
  and closed form 2 (`0 <= A`, `B < L` gives `[A, B]`) hold. On a run with
  `qa = qb` the residues are linear in d. On a run with `qa < qb`, `(qa+1)d`
  lies in `(A, B]` and `(qa+1)d2 - 1` reaches `d2 - 1`. `floor(n/d)` takes at
  most 2√n+1 values, so the run bound of about `2√|A|+2√|B|+2` is right. I
  recomputed every AC-19 value: 91 gives [0,7], 97 gives [1,9], -7 mod [3,5]
  gives [1,3], truncating gives [-3,-1], floor over [-5,-3] gives [-3,-1], and
  the guarded cases hold. For AC-20, a prime singleton 1000000007 gives
  [1, 500000003] at about 63k runs, under the 16777216 default and over 10000.
  The limit is typed `stage_limit_exceeded`/`work-budget-exceeded`
  (`CheckCause::ResourceExhausted`, `CheckingLimitKind::WorkBudget`,
  `with_work_budget` exists), with no fallback to a wider interval. NFR-011
  states the per-run charge.
- QSL-2274. `dependency_clock_selections` agrees across FR-640, the FR-322
  prose, schema.json (`DependencyClockSelection`, minItems 1, defs exist, the
  JSON parses), AC-452 and FR-323 AC-38. The FR-322 reader refuses
  `conflicting-definition` when an entry disagrees with the package's own
  `temporal_bindings` row.
- QSL-2275. fsl-forms/src/value.rs:1914-1917 builds `absent`/`none` as Absent
  and `result` as Result. Check gives `ambiguous-literal`/`type-mismatch`
  (check.rs:3377) and `UnanchoredResult` (`wrong-anchor`, check.rs:4656). So
  the code is right. The FR-763 table has no none/result rows left, and AC-5
  and TC-2159 agree with it. The non-ignored option_values test should pass on
  main.
- QSL-2269. The standard has no `hashed_bytes` anywhere. It was removed in
  filament-specification #517 along with hash identities, and FR-150 says
  "Nothing is hashed". So refusing it narrows no specified feature.
- IDs: language TC-2210..2215 and standard TC-1660..1662, with no new FRs.
  The new ACs (FR-091-AC-76, FR-495-AC-19/20, FR-620-AC-11, FR-640-AC-10/11,
  FR-322-AC-452, FR-323-AC-38, FR-147-AC-14) each follow their FR's previous
  maximum. No id collides with origin/main, qsl-2121-code or qsl-1964-code.
- Stubs: the trace tags and FSpec- prefixes are correct, the ignore keys
  (QSL-2002/2274/2269) are correct, and the assertions are real.

## Verdict

REQUEST CHANGES for FND-001. FND-002 to FND-005 are small. Coordination note,
not a defect of this diff: branch qsl-2121-analyze-format still accepts
`_hashed_bytes` (fsl-command/src/command/wire.rs:406-410), which contradicts
the new FR-620. qsl-1964 also edits tests/it/clock_derivation.rs and
tests/it/main.rs, so expect textual conflicts.

## Dispositions

| ID | Disposition | Resolution |
| --- | --- | --- |
| FND-001 | fixed | FSpec FR-323: the clock paragraph now follows the ordered list; `missing-clock-selection` and `clock-kind-mismatch` are rule 2 recompile refusals, and a new rule 4, decided after every recompile, refuses `unused-clock-selection`. FR-640 states the same order. |
| FND-002 | fixed | FR-640 and FSpec FR-322/FR-323 now serve library units only from `dependency_clock_selections`, never from a program row. The emitter writes the entry even for a shared clock name. FR-640-AC-11, FR-323-AC-38, TC-2213, TC-1662 and the replay stub add the shared-clock case, which refuses `missing-clock-selection`. |
| FND-003 | fixed | `missing-clock-selection` names the clock and the library in FR-640, FR-640-AC-11, FSpec FR-323 and FR-323-AC-38. The stub asserts the library name. |
| FND-004 | fixed | The FR-495-AC-20 stub asserts the refusal's origin is `f`'s body and its path is the root, the `mod`. |
| FND-005 | fixed | The option_values test admits `function a using v(): Option<Integer> pure { absent }`; TC-2214 lists it. |
