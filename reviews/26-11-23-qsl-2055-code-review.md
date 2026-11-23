---
id: SR-96718
title: "QSL-2055 code and Rust review of the value-layer property suite (filament-spec-language PR 3578)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@c8b490ec61671f57a4f5fcbece68df8f4d297235; git diff origin/main...HEAD: Cargo.lock; fsl-eval/Cargo.toml; fsl-eval/tests/it/main.rs; fsl-eval/tests/it/value_properties.rs; tests/it/family_outcome_layering.rs"
review_set: subset
date: 2026-11-23
---

## Summary

Ticket: QSL-2055 (PR filament-spec-language#3578). `/code-review` with the
`/rust-review` lane over `git diff origin/main...HEAD` at `c8b490ec6`.

Every acceptance-criterion test exists by its exact name in
`fsl-eval/tests/it/value_properties.rs`, at 1,000 cases each (both near-miss
runs at 256; the ticket fixes no count for the negatives, and the shared
near-miss generator has only 48 distinct sources). All 7 tests pass. The
reference evaluator is independent of `fsl-eval`: it uses only `std` and
`i128` arithmetic, and `fsl-eval` is called only to evaluate. P4 is not
vacuous: each of the five refusal categories must occur more than 10 times.
The mutation test fails under a reference that turns `+` into `-`, and the
shrunk case is asserted to be a one-operator `+` source. `cargo clippy -p
fsl-eval --test it --no-deps` reports nothing in the new file. The layering
test passes.

Runtime on terra (14 cores, load 5 to 12): 8.67 s, 7.12 s and 9.08 s for the
file. P1 alone takes 6.8 s. The serial CPU is 33 to 42 s, and the file takes
13.55 s at `--test-threads=3`.

The branch is 23 commits behind `origin/main`. Main's #3557 moved the shared
policy into the leaf crate `fsl-proptest-policy` and changed
`fsl-eval/Cargo.toml`, `Cargo.lock` and `tests/it/family_outcome_layering.rs`,
the same lines this PR edits.

## Verdict

FAIL: one high finding. The dependency edge from `fsl-eval` to `fsl-bench`
conflicts with main and goes against main's layering design. The other
findings are medium or low. The reference, the property structure, the
shrinking and the exact assertions of P3, P4, P5 and the mutation test are
sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new dev edge from fsl-eval to fsl-bench brings the layers above 5 into fsl-eval's test build: fsl-replay, fsl-analyze, fsl-route and fsl-eval itself, a dev cycle. ADR-011 X-8 and TC-390 step 4 limit fsl-eval's dev table to fsl-forms, fsl-cst and serde_json. The PR widens the enforcing test instead. Main (#3557) already gives fsl-eval the leaf fsl-proptest-policy, and fsl-bench is a dev dependency only of fsl-replay, so this PR conflicts in all three files. Fix: rebase and take the policy from fsl-proptest-policy. Move `prop::value` (needs only proptest) and `prop::near_miss` (needs only fsl-foundation) into a leaf that fsl-eval may dev-depend on. Drop the fsl-bench edge and the edit to the layering test. | fsl-eval/Cargo.toml:63; tests/it/family_outcome_layering.rs:376 |
| FND-002 | medium | The 10 s runtime limit is met only narrowly and depends on the machine. The file took 7.1 to 9.1 s on 14 cores, P1 alone takes 6.8 s, and it takes 13.6 s at 3 test threads. It is not measured inside the full `it` binary (1,600 tests, plus main's new equality_matrix and temporal_laws property suites). The ticket fixes 1,000 cases for P1 to P5, so making each case cheaper is the only fix. P1 costs about 7 ms per case in debug; profile its parse, format and parse loop, or give fsl-cst a test opt-level. | fsl-eval/tests/it/value_properties.rs:1801 |
| FND-003 | medium | The near-miss assertion accepts any Err from try_compile, so a parse, forms or assemble failure would also pass. Each NearMiss carries its expected code and cause; the test should assert them. The Kernel evaluation half also never checks that a Kernel compile ran, so it could become vacuous without anyone noticing. | fsl-eval/tests/it/value_properties.rs:1992-2003 |
| FND-004 | medium | The Rational family is limited to one operation per node because of a header claim that is false: "a nested operation has no declared domain, so it is unproved-rational-range". `(a + b) * c` into `W`, with a, b and c of type `Rational[-20, 20; 1, 9]`, checks under Linked. Only an intermediate typed at the wide `W` (`let s: W = a + b in s * c`) is refused, and that refusal is correct interval arithmetic. The ticket forbids narrowing a family. | fsl-eval/tests/it/value_properties.rs:36-40 |
| FND-005 | low | The header says a `!= 0` guard on a zero-free divisor is refused unproved-range. The refusal is actually at the literal `0` in `c != 0`: it is typed into c's interval (proved [0,0], required [-9,-4]), and the same happens for `Int[4, 9]` and with `div`. It is about the comparison, not the division. | fsl-eval/tests/it/value_properties.rs:29-31 |
| FND-006 | low | The AC asks for Decimal "with each rounding mode", but the test only counts cases per family. Nothing requires each of the six modes to hit an inexact `*` or `/` that actually rounds. | fsl-eval/tests/it/value_properties.rs:1873-1885 |
| FND-007 | low | Two doc comments are stale. `agree` documents `Ok(true)`/`Ok(false)` but returns `Ref<()>`. The doc on `try_compile` ("or the evaluator's answer") describes something it does not do. | fsl-eval/tests/it/value_properties.rs:382; fsl-eval/tests/it/value_properties.rs:452-453 |

## New findings (disposition pass 1)

Round 1 reviewed head `0492fd71d`, with main merged in.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The rounding-mode counter counts a case once the reference's `place` saw a nonzero remainder, even when the case then ends in a domain refusal. It sits in P4, not in P3, which the AC names. Count only cases whose expected outcome is a rounded value, or a `DecimalInexact` refusal for `exact`. | fsl-eval/tests/it/value_properties.rs:1974-1989 |
| FND-009 | low | The manifest of `fsl-proptest-policy` still describes the crate as "the one property-test policy", and its comment says `fsl-bench` re-exports it "for its generators". The crate now holds the `value` and `near_miss` generators itself. | fsl-proptest-policy/Cargo.toml:8-12 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
