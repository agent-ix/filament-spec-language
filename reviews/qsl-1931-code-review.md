---
id: SR-2026111414542500000001
title: "Code and Rust review of QSL-1931 CheckMode on package checking (PR #2706)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1931; PR #2706 head 8478f1759e98aaa0e60d7779f59d0dd43099377f vs origin/main; qsl-semantics/src/check/{mod.rs,facts.rs,family.rs,lowering.rs}; qsl-semantics/tests/it/check_mode.rs; 237 mechanical PackageDeclarations::check call-site files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
# Code and Rust review: QSL-1931 CheckMode on package checking

## Summary

Ticket: QSL-1931. PR #2706, eight commits on origin/main. Methods: code-review
with the rust-review lane (rust-style baseline loaded). Scope is
`git diff origin/main...HEAD` only. The reviewer ran
`cargo test --locked -p qsl-semantics --test it check_mode -- --test-threads=2`
in the ticket worktree: 8 passed. The aggregate gate was not run; the lead runs
it at merge.

Call sites: 710 added lines pass `CheckMode::Linked`, and no added line passes
`Kernel` to a package-level check. The production callers in
qsl-package (`Resolution`, both cancel variants), qsl-bench, qsl-eval
sampled world, qsl-inspect and qsl-replay are purely mechanical.
`Definedness::new` keeps `kernel: false`, so Linked behaviour is unchanged.
The remaining removed lines in test files are rustfmt reflow around the new
argument.

Kernel semantics: `discharge` swallows only `CheckCause::Unproved(_)` and only
when `kernel` is set. The walk is post-order and discharges each node on its
own, so a dropped obligation never skips an operand. Call and dispatch edges
come from arms that never return `Unproved`, so termination sees the same call
graph. Only `check_declaration_body`'s body walk and
`check_clause_expression_inner` take the mode. The termination measure walk
has no `with_check_mode` (SR-61009 FND-002), and state-clause, protocol,
temporal, hyper and probabilistic walks stay Linked. The package path and the
expression path now share one Kernel rule (SR-61009 FND-001). The malformed
Rational operand pre-check runs before the swallow (SR-61009 FND-011). The
doc comments name all five dropped obligations (SR-977 FND-005).

## Verdict

Not mergeable as is, because of FND-001. The CheckMode half is correct and
behaviour-preserving under Linked. FND-001 is fixed by dropping the replayed
`operation_key` commit; FND-002 to FND-004 are small.

Non-vacuity: the Kernel-accepts assertions in
`an_unproved_result_bound_checks_only_under_kernel`,
`an_unproved_nonzero_divisor_checks_only_under_kernel`,
`an_unproved_presence_checks_only_under_kernel` and the `f(x) + 1` half of
`kernel_mode_still_refuses_a_nonterminating_recursion` would fail on the old
behaviour, which refused all of them. The tests that assert a refusal in both
modes (the state clause, the measure and the plain recursion) pass on the old
code by design. They guard the scope: giving the measure walk or a state-clause
walk the mode would make each one fail.

Out of scope, not flagged: the FR-495-AC-3 integer `d1` Linked refusal
(QSL-1962).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR adds a second public FR-093 accessor, `CheckedGraph::operation_key(&Location)`, with its own `operation_keys` map, a `draft_operation_keys` resolution path in `settle` and a Debug field. Main already ships the accessor as `CheckedGraph::node_key(CheckedNode)` (#2624, mod.rs:8418). Its doc says "No identity is derived from a location", and `lowering/tests/node_keys.rs` already covers FR-093-AC-62 and recursive call drafts. The replayed commits a8755b5b4 and 2a494068b come from the stale stack, from before #2624. They are outside QSL-1931's scope (CheckMode only), and they key identity by location, the design main rejected. Fix: drop `operation_keys`, `draft_operation_keys`, `record_operation_key`, `operation_key`, its Debug field, and the tests `a_checked_operation_exposes_its_node_key` and `a_call_exposes_the_key_of_its_application_node`. | qsl-semantics/src/check/mod.rs:828-829,1176-1181,8197-8210; qsl-semantics/src/check/lowering.rs:973,1103-1105,3964-3967,4271-4288; qsl-semantics/tests/it/check_mode.rs:183-250 |
| FND-002 | medium | Kernel suppression is tested for range, decimal nonzero and presence only. The ticket and the doc comments name five dropped obligations, but no test shows Kernel dropping a nonempty-reduction or rational-range obligation, or a rational `/` nonzero obligation. SR-61009 disposed FND-003 as fixed with exactly those cases, but they were not carried into this regeneration. Fix: add Linked-refuses / Kernel-checks pairs for a `reduce` over a possibly empty collection, a rational result-domain overflow and a rational division by an unguarded divisor. | qsl-semantics/tests/it/check_mode.rs:60-108; qsl-semantics/src/check/facts.rs:1198-1211 |
| FND-003 | low | `Definedness::kernel` is a `bool` set by `mode == CheckMode::Kernel`, not by an exhaustive match. If `CheckMode` (an external crate) gains a variant, that variant silently gets Linked behaviour. SR-977 FND-006 was disposed as fixed ("store the CheckMode ... match it exhaustively"), but this branch reverts it. Fix: store `CheckMode` and match it in `discharge`, or use `match mode { Linked => false, Kernel => true }`. | qsl-semantics/src/check/facts.rs:536-541,563-566 |
| FND-004 | low | The call sites spell `quire_semantic_value::checking::CheckMode::Linked` inline 710 times instead of importing `CheckMode` once per file, which adds about three reflowed lines per call. SR-61009 FND-013 was disposed as fixed ("CheckMode is imported once per file"), but this regeneration does not carry that fix. Fix: import `CheckMode` per file and pass `CheckMode::Linked`. | qsl-semantics/src/check/region.rs:816-822; qsl-package/src/checked.rs:3703-3716 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The tests tagged FR-495-AC-10 and FR-495-AC-11 assert only the refusal half of each criterion. AC-10 also requires that `function g using v(x: Option<Integer>): Integer pure { if present(x) then value(x) else 0 }` checks under both modes. AC-11 also requires that the guarded `ParentOrder` invariant checks under both modes. TC-1419 steps 1 and 2 name both as expected results. `admit_and_assemble_with_body` assembles only the given clause, so `ParentOrder` is not exercised implicitly. A Kernel or Linked change that over-refused the guarded forms would pass both tests. Fix: add the `g` and `ParentOrder` checks under each mode to the two tagged tests. | qsl-semantics/tests/it/check_mode.rs:97-140 |
| FND-006 | low | The AC-13 dispatch test asserts the refusal kind in each mode but never compares the Linked refusal with the Kernel refusal, although AC-13 says "Each outcome is the same refusal under both modes". Its second case accepts any matching refusal among many (`iter().any`). Fix: collect the refusals per mode and `assert_eq!` the Linked and Kernel lists. | qsl-eval/tests/it/dispatch_calls.rs:3251-3312 |
| FND-007 | low | The doc comment on `an_unproved_nonempty_reduction_checks_only_under_kernel` says the reduction "is `unproved-range` under `Linked`", but the test asserts `Obligation::NonemptyReduction`. Fix: name the nonempty-reduction obligation. | qsl-semantics/tests/it/check_mode.rs:212-221 |

## Dispositions

Round 1 verified at head 6297dca4c111619607591b408f56ef5c41a59fb5 (fix commit 6297dca4c, rebased on origin/main including spec PR #2707). Focused runs, all with --test-threads=2: qsl-semantics `it check_mode` 9 passed; qsl-semantics lib `kernel_mode_tests node_keys` 7 passed; qsl-eval `it dispatch_declaration_refusals` 1 passed; qsl-package lib `source_map_debug` 5 passed. Each of the three new kernel_drops pairs asserts the Linked refusal with its typed obligation and expects Kernel to check, so each would fail on the old Linked-only behaviour. rust-style re-check: no new allow, unwrap, expect or panic on production paths, and no Cargo or pin changes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 2 verified at head 6a3caa977b91ac9784872beff784a4b2e8774479 (fix commit 6a3caa977, fast-forward on 6297dca4c, two test files only). Focused runs with --test-threads=2: qsl-semantics `it check_mode` 9 passed; qsl-eval `it dispatch_declaration_refusals` 1 passed; `cargo fmt --all -- --check` clean. FND-005: `ParentOrder` is now the only clause handed to `admit_and_assemble_with_body`, and the test asserts that it checks under both modes and that `state_clause("ParentOrder")` is present. `g` checks under both modes. A Kernel or Linked change that refused either guarded form now fails.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
