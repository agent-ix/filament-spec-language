---
id: SR-2026111919340478491681405
title: "Code review of filament-spec-language PR 3085: reference hybrid provider class check, option domains, descriptor"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@fde13341c3fe4acf6d22d1ad4ec0ed607cb4276d; git diff origin/main...HEAD (main 5fae529b7): fsl-replay/src/hybrid_replay/class.rs, fsl-replay/src/hybrid/provider.rs, fsl-replay/src/lib.rs, fsl-replay/src/hybrid.rs, fsl-replay/src/hybrid_replay.rs, fsl-hybrid-reference/src/{lib,options,tests}.rs, fsl-hybrid-reference/Cargo.toml, fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/hybrid_item.rs, fsl-analyze/src/provider.rs, fsl-analyze/tests/it/hybrid_reference.rs, Cargo.lock"
review_set: subset
date: 2026-11-19
---
# Code review of filament-spec-language PR 3085

## Summary

Ticket: QSL-2534 (slice of QSL-2234). Rust lane folded in. The PR is an honestly
labelled partial delivery: the class check (`fsl_replay::polynomial_class_gap`),
`HybridProvider::check_options` with the reference option domains, the
`invalid-option-value` settlement in `fsl-analyze`, and the descriptor's option
names. The flowpipe solver is not built and every solver AC keeps an ignored
test citing QSL-2534.

Evidence, in a throwaway worktree with its own target dir under /home:

- `cargo-slot cargo test -p fsl-hybrid-reference --lib`: 3 passed.
- `cargo-slot cargo test -p fsl-analyze --test it -- hybrid_reference:: --test-threads=1`
  (house skips): 7 passed, 9 ignored. Item JSON dumped for AC-3, the guard
  division and AC-7: AC-3 locates `form: division`, mode `Moving`, variable `v`,
  at the division node inside the flow; `{step: "0"}` settles
  `invalid_capability`/`invalid-option-value`, `settled_by: disposition`,
  payload `request_index`, `backend`, `option`, `value`.
- `cargo-slot cargo clippy -p fsl-hybrid-reference -p fsl-analyze --all-targets`
  with `--cap-lints warn`: E0004 in the fsl-analyze lib-test target (FND-001)
  and a new dead_code warning (FND-007).

Spec read: FSL FR-1019, FR-750, FR-967, FR-635 at the PR head; the rulings on
QSL-2534 (comment e1f295bd, posted against this head) settle the class, source
order, `with_options` and FR-1021-AC-3 questions; findings below cite them.

## Verdict

Not mergeable at fde13341c. FND-001 breaks compilation of the fsl-analyze unit
tests. FND-002 to FND-004 are behaviour the ticket's rulings on this head
require and the PR does not yet do.

What is right: the error-path order matches FR-750 (FR-967 `unknown-option`
in routing before candidates, `check_options` after `tool-unavailable` and
before budget resolution, package emission and `solve`); the `ref-7e` case
proves the option refusal precedes the class check. The option domains match
FR-1019 "Options" exactly, including leading zeros, the empty string and u32
overflow. Division and numeric `if` outside the class match FR-1019:43 and
ruling 3 (`rational(n, d)` keeps rational coefficients expressible). The PR
body, the crate doc and the ignore reasons frame the slice honestly. No
`#[allow]`, no panics in production code; traces use `#[trace("TC-2869",
"FR-1019-AC-n")]`. Public surface added: `fsl_replay::{polynomial_class_gap,
OptionRefusal}`, `HybridProvider::check_options` (default method, additive),
`fsl_hybrid_reference::OPTION_DEFAULTS`, `fsl_analyze::Payload::OptionValue`
(an exhaustive pub enum gains a variant), and the `OptionRefusal` re-export;
nothing in fsl-route.

Arch question (rewriting AC-6 to read the Rust descriptor): answered by ruling 1
on the ticket. The rewrite is correct, because AC-6 is about the registered
descriptor and the FR-621 `registry` wire is `{backend, tool}`. The test's
weakened assertions are FND-005.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Adding `Refusal::InvalidOptionValue` leaves the `code()` helper's match in the fsl-analyze lib tests non-exhaustive (E0004), so `cargo test -p fsl-analyze --lib` and clippy `--all-targets` do not compile | fsl-analyze/src/analyze/items.rs:1206-1218; fsl-analyze/src/analyze.rs:1398-1408 |
| FND-002 | high | The class walk refuses every `if` as `conditional`, including a Boolean-valued `if` in a guard, reset, invariant or initial condition, which ruling 3 on QSL-2534 places inside the class (a Boolean combination) | fsl-replay/src/hybrid_replay/class.rs:96; fsl-replay/src/hybrid_replay/class.rs:122 |
| FND-003 | medium | "First construct in source order" walks the modes before the initial entries, and rates in variable order instead of the flow equations as written; ruling 4 puts the initial entries first, then each mode's invariant, flow equations as written, and each guard followed by its reset | fsl-replay/src/hybrid_replay/class.rs:142-221 |
| FND-004 | medium | The descriptor publishes the option names only. The defaults sit in `OPTION_DEFAULTS`, and no production path reads them. FR-1019 "Options" and ruling 2 require `with_options` to carry name-to-default pairs, which means changing fsl-route's `with_options` and its callers | fsl-analyze/src/analyze.rs:461-463; fsl-route/src/lib.rs:852; fsl-hybrid-reference/src/options.rs:9-13 |
| FND-005 | medium | The rewritten AC-6 test checks `dynamics().len() == 1`, not that the one kind is `hybrid`, and compares `OPTION_DEFAULTS` with its own literal instead of the descriptor's published defaults. A descriptor with `dynamics ["discrete"]` or no defaults passes | fsl-analyze/tests/it/hybrid_reference.rs:283-306 |
| FND-006 | medium | `solve` matches only `Ok(Some(_))` from `polynomial_class_gap`. An `Err` (the unpublished `CLASS_WORK_UNITS` meter exhausted, or a missing function body) falls through to the in-class path, so once the solver lands it will run on a model whose class was never established | fsl-hybrid-reference/src/lib.rs:44-52 |
| FND-007 | low | The AC-6 rewrite leaves the `descriptor` helper unused, which adds a new dead_code warning in the `it` test target | fsl-analyze/tests/it/hybrid_reference.rs:46-57 |
| FND-008 | low | `Exp::Ranged`, the range check of a call argument or result, is walked as transparent. A model whose flow calls a range-typed function is admitted, but the range definedness check is not one of the class's constructs | fsl-replay/src/hybrid_replay/class.rs:109 |
| FND-009 | low | The AC-1, AC-9 and AC-11 ignore reasons do not name the QSL-2531 certified-checker dependency, which only the PR body states. AC-1's reason ("the first-party reference hybrid provider is not implemented yet") is stale now that the provider exists | fsl-analyze/tests/it/hybrid_reference.rs:63 |
| FND-010 | low | The FR-1021-AC-3 stub is still labelled QSL-2534, and the PR body lists it as remaining here. Ruling 5 assigns it to QSL-2531 | fsl-analyze/tests/it/hybrid_requests.rs:1114-1116 |

## New findings (disposition pass 1)

Found while checking round 1 at 269ecdaa44ebd141577a0396d68efcd6fe79509f, against FR-1019 "Class" as merged on main by #3088 (SW-1, 38f5f0d38).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | FR-1019-AC-14 (merged on main with SW-1) has no test on this PR, active or ignored. The fixtures C1 to C6 are untested as an AC: the outer `/` (C2), the `/` in `half`'s body at its own span (C4) and the `rational(1, 2)` coefficient (C5) have no case at all, and the initial-entry and `if` cases are tagged AC-3 | fsl-analyze/tests/it/hybrid_reference.rs:156-197 |
| FND-012 | medium | A numeric `if` or an activation reached through an expanded call is located at the call site. `polynomial_gap` returns no locus for `Exp::Conditional` and activation nodes. Merged FR-1019 "Class" requires such a construct to be reported at its own source span inside the called function's body | fsl-replay/src/hybrid_replay/class.rs:140-142 |
| FND-013 | low | The class check now meters call expansion against `hybrid.max_solver_steps` and reports `Stopped(Steps(budget))`. FR-1019 "Budgets" counts a step per Picard attempt, so a call-heavy model can stop `limit-reached` `max_solver_steps` before any Picard attempt | fsl-hybrid-reference/src/lib.rs:44-71 |

## Dispositions

Round 1, head 269ecdaa44ebd141577a0396d68efcd6fe79509f.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `Refusal::InvalidOptionValue { .. }` added to the test helper's `None` arm (items.rs:1215); `cargo test -p fsl-analyze --lib --no-run` builds |
| FND-002 | fixed | `HybridTerm::Conditional` and `Exp::Conditional` in Boolean position walk c, a and b; only numeric position is `conditional`. Tested by `a_boolean_if_is_in_the_class_and_a_numeric_if_is_not` |
| FND-003 | deferred | Initial entries now precede the modes (fixed part). Rates stay in variable order because `CheckedMode.rates` and the wire store them by variable, so the written order cannot be recovered. That diverges from merged FR-1019 "its flow equations as written" only when a mode writes its equations out of variable order. The limitation is documented on `polynomial_class_gap`. It is out of scope for this PR: either amend FR-1019 to "in variable declaration order" (recommended, since the order only breaks ties for the diagnostic locus) or file a contract ticket for an equation-order field (class.rs:147-156) |
| FND-004 | fixed | `BackendDescriptor::with_options` takes (name, default) pairs and stores `option_defaults`, exposed by `option_defaults()`. The reference descriptor is built from `OPTION_DEFAULTS`, and callers are updated | - |
| FND-005 | fixed | The test asserts `dynamics` == ["hybrid"] and reads the defaults from `reference.option_defaults()` | - |
| FND-006 | fixed | `ResourceExhausted` answers `Stopped(Steps)`, and any other `Err` answers `Unknown` (inconclusive, never in-class proof). `CLASS_WORK_UNITS` is removed. The budget semantics are FND-013 | - |
| FND-007 | fixed | The `descriptor` helper is removed | - |
| FND-008 | accepted-no-change | Kept transparent, with the rationale documented on `polynomial_gap`: a range check is a definedness condition on the wrapped term's value, as in FR-450 exact replay's polynomial flow. Merged FR-1019 "Class" is silent on range checks, and the finding was low confidence | - |
| FND-009 | fixed | The AC-1, AC-9 and AC-11 ignore reasons now cite QSL-2531 and the remaining solver work | - |
| FND-010 | fixed | The FR-1021-AC-3 stub is relabelled QSL-2531 (ignore and `todo!`) | - |
