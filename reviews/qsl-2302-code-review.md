---
id: SR-3880001
title: "Code review of QSL-2302 source-enum wire helpers (FSL PR #3880)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@52645e4479a2890db431b75592254521d0746992; fsl-eval/src/simulation/model/{encode,domain}.rs, fsl-replay/src/outcome/counterexample.rs, fsl-replay/src/contract_check.rs, fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/exact{,/measure,/undefined}.rs, fsl-command/src/command/plan_synthesis.rs, fsl-analyze/tests/it/{value_validity,operation_contract,probabilistic_replay,package_probabilistic_refutation}.rs"
review_set: subset
---
# SR-3880001: Code review of QSL-2302 source-enum wire helpers (FSL PR #3880)

## Summary

Ticket: QSL-2302. PR agent-ix/filament-spec-language#3880, head
52645e4479a2890db431b75592254521d0746992. This review covers code-review with the rust-review lane.

The change threads `enums: &[EnumBinding]` into `typed_value_json` and the counterexample wire
helpers, so a source `enum` value resolves through `Typing::with_enums`. That is the same typing
`ModelSystem::value_typing()` already uses (fsl-eval/src/simulation/model.rs:1200-1204). The
change is correct and leaves every existing byte unchanged. In `enum_member`, variant types are
checked before source enums, and source enums before the member index, so a model-variant value
is spelled as it was before.

Author claims, as verified:

1. **The value-check claim in the ticket is out of date: confirmed.** `scope.enum_member_index()`
   records every `EnumBinding` member (fsl-semantics/src/check/check.rs:1432-1436). With
   `.with_enums` removed from `typed_value_json_with_enum_members` only (mutation M2), all 6
   source-enum tests pass, and the value-check item bytes are identical. In production the
   `enums` argument never adds information:
   - `assignments_wire`: its only production caller passes `&[]` (analyze.rs:2599-2605).
   - `assignments_wire_with_enum_members`: `scope.enums()` duplicates the member index
     (analyze.rs:2616-2617).
   - `probabilistic_wire_with_enum_members`: it gets `input.enums`, which is `&[]` whenever
     `enum_members` is `Some` (fsl-eval/src/simulation/model/input.rs:1018). Only the source
     route reaches the plain `probabilistic_wire`, with `enum_members: None` (input.rs:221-222).

   See FND-002.
2. **No reachable input: confirmed as far as reviewed.** The real every-scheduler package test
   carries `Value::Enum` scheduler arguments
   (package_probabilistic_refutation.rs:441-444). Those are model variant values, and
   `probabilistic_wire` already spelled them on main without `enums`. Contract operation
   parameters, results and random draws are typed by the model package. On the contract and
   probabilistic routes the production change is defensive parity with `value_typing()`, not a
   reachable fix. The tests prove the helper contract only (FND-003).
3. **Callers in exact.rs, measure.rs, undefined.rs and plan_synthesis.rs are not exercised with
   a source enum: confirmed** (recorded as FND-001 in SR-3880002).
4. **`sampled_witness_wire` has no callers: confirmed** (grep over the workspace and ~/dev). The
   PR only adds the parameter to an already-dead public function; no finding.
5. **FR-120 vs FSpec FR-181-AC-19: a real conflict, older than this PR**, tracked on STD-493 (see
   SR-3880002).
6. **Clippy:** `cargo clippy -p fsl-replay --lib --no-deps -- -D warnings` at head exits 101
   with 18 errors, the same count as the author's run at db40154 before the allow. The allow
   removes the `probabilistic_wire` lint (line 2014). `probabilistic_wire_with_enum_members`
   (2043) and `probabilistic_undefined_wire` (2244) still fail at 9/7. Each already had 8
   arguments on main, so it was already failing there. The other errors are in files this PR
   does not touch. CI clippy runs only on `workflow_dispatch`. Clippy was not re-run on base;
   that it predates rests on the argument counts and the file locations (FND-004).

Mutation check (the literal-value tests must fail without the production change). M1 removes
`.with_enums` from both `typed_value_json` and `typed_value_json_with_enum_members`, which
reverts the production change. Result: 3 of the 6 tests fail:
- operation_contract.rs:971
- probabilistic_replay.rs:1031
- value_validity.rs:2402

The value-check test fails only at its direct `assignments_wire` call. Its production item
assertion (value_validity.rs:2390) passes without the change (FND-001).

## Verdict

**CONDITIONAL**: no high finding. The production change is correct and keeps existing bytes. It
is mergeable once the lead has decided on FND-002. The reviewer recommends taking the
redundant `enums` parameter off `assignments_wire`, `assignments_wire_with_enum_members`,
`probabilistic_wire_with_enum_members` and `typed_value_json_with_enum_members` in this PR, and
keeping it on `typed_value_json`, `probabilistic_wire` and `probabilistic_undefined_wire`. That
removal also takes `probabilistic_wire_with_enum_members` back to 8 arguments.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The value-check test's production assertion does not depend on the fix. With the production change reverted (M1), `item["counterexample"]` still equals the expected source-enum bytes, through `scope.enum_member_index()`. The test fails only at the direct `assignments_wire(..., scope.enums())` call, a path production never takes with non-empty enums. The test reads as proof that the source route was fixed, but the source route was never broken. | fsl-analyze/tests/it/value_validity.rs:2390, fsl-analyze/tests/it/value_validity.rs:2402, fsl-analyze/src/analyze.rs:2599-2617 |
| FND-002 | low | The new `enums` parameter is dead or redundant on four helpers. `assignments_wire` is only ever called with `&[]` in production. `assignments_wire_with_enum_members` gets `scope.enums()`, which repeats `scope.enum_member_index()` (M2 shows identical bytes). `probabilistic_wire_with_enum_members` always gets `&[]`, because the package input sets `enums: &[]`. `typed_value_json_with_enum_members` serves those two. The result is two sources of truth and a 9-argument signature. | fsl-replay/src/outcome/counterexample.rs:1944-1978, fsl-replay/src/outcome/counterexample.rs:2043-2067, fsl-eval/src/simulation/model/encode.rs:1200-1213, fsl-analyze/src/analyze.rs:2599-2617, fsl-analyze/src/analyze/exact.rs:3966-3975, fsl-eval/src/simulation/model/input.rs:1016-1019 |
| FND-003 | low | The contract and probabilistic tests hand-substitute source `Color` values into real witnesses. No production path is known to put a source enum into contract executions or probabilistic draws and arguments. The tests prove only the helper's spelling, but their doc comments say a counterexample that "holds source enum values" is spelled. | fsl-analyze/tests/it/operation_contract.rs:946-981, fsl-analyze/tests/it/probabilistic_replay.rs:989-1083 |
| FND-004 | low | The `too_many_arguments` allow covers only `probabilistic_wire`. `probabilistic_wire_with_enum_members` and `probabilistic_undefined_wire` go from 8 to 9 arguments and still fail clippy at head, which already failed on main at 8. The PR adds no allow and no reduction for them, so its handling is inconsistent. | fsl-replay/src/outcome/counterexample.rs:2014-2018, fsl-replay/src/outcome/counterexample.rs:2043, fsl-replay/src/outcome/counterexample.rs:2244 |
