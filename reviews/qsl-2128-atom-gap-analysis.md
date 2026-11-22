---
id: SR-21283002
title: QSL-2128 package hyper atom bounded gap analysis
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: agent-ix/filament-spec-language; PR3406 changed production and tests; bounded repository matrix attempt
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
---

## Summary

Frozen head 0a633aef7e54ed0f15d542cf5d1be785eaac8a4a; tested base 2f01e55faee9c79ee5b59b99ce85e22011f8e3b2. Ticket: QSL-2128; PR3406. Read-only review; no source, target, branch or pin mutation. No applicable AssuranceProfile exists. No specification/plan path changes, so spec-review and its sub-analyses are inapplicable. Code-review includes the Rust lane and mandatory rust-style API, implementation, safety, wire and testing references.

Gap-analysis skill procedures were loaded and applied. Reverse-gap, stub and coverage-inflation inspection completed for the nine-file diff. Repository-wide reconciliation did not complete.

## Verdict

FAIL for audit completeness only. No full-repository or full-AC-18 completion is asserted. The bounded dispatch explicitly permits honest incomplete assurance; the lead owns acceptance/deferment of this limitation. Implementation and trace defects are recorded once in SR-21283001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Repository-wide criterion reconciliation remains incomplete: the single 15-second bounded quire matrix attempt exited 124 with empty JSON. Static and evidence status totals are unavailable; no engine-backed repository assurance PASS can be established. | review/matrix.json; review/matrix.stderr | correct-requirement-no-evidence |

## Coverage

Plan completion: not assessed

Reconciliation attempted: `timeout 15s quire matrix --scope /home/peter/dev/worktrees/QSL-2128-hp1-certificate --format json`; exit 124, empty matrix.json; stderr module warnings only. No repeated extraction, grep fallback, fabricated totals or coverage percentage. Prior matrix timeout is not relabelled as this run. Engine coverage and evidence statuses are unavailable.

Reverse inventory: four production behaviors (ordered-slot projection, ownership propagation, explicit ownership refusal, Bare atom execution) all belong to FR-940 package evaluators and FR-487 atom execution; zero observed unowned behaviors, zero observed source stubs and zero observed test stubs in the changed slice. No new ignored tests, coverage suppression, copied upstream semantic rule, gate weakening, mock-only oracle, or source fallback found. Five explicit trace bindings were inspected; the two wrong AC-6 bindings are SR-21283001 FND-002, not duplicated. Broader audit outside the changed paths is incomplete. Optional separate semantic gap analysis was not invoked; code-test correctness was examined under the mandatory code-review lane.

Existing implementation logs inspected: package-tests-final.log has 5 passed, 0 failed, 0 ignored; atom-tests-final.log has 3 passed, 0 failed, 0 ignored. The initial OS error 28 build is unmeasured resource failure, not a test/code failure or benchmark. Formatting receipt passes; git diff --check passes. Clippy receipts fail only at unchanged semantics typing.rs:2193/2197, synthesis.rs:148 and eval frames.rs:214/302/328/342/345, model.rs:846. These source files and CI workflows are absent from this PR diff; no warnings are suppressed. No fresh Cargo, whole-crate, workspace, benchmark or deny run was authorized during disk recovery; final current-main gates remain the lead's responsibility.

## Examined scope

```yaml
scope:
  - id: FR-940-AC-17
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: A `program` request and a `package` request over the handoff directory of AC-16's unit, written by its compile request's `handoff` member, with the items `Equal`, `Partner`, `CanReachThree` and the projection-aligned clause, and the same items over the unit with `Equal`'s bound lowered to `c.value @ b <= 2` so it refutes, give equal item records, counterexamples (one trace per copy; for `Partner` the exhausted witness set), copy-by-copy replays with `reproduced` `true` and product-closure certificates accepted by FR-163's checker; `Equal` settles `proved` with certification `certified` on both
  - id: FR-940-AC-18
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: context_only
    excerpt: AC-17's `package` request run after the unit's source file is overwritten with bytes that are not a native unit and the handoff directory's `sources/` is deleted settles `Equal` `proved` with the record of AC-17's source request; FR-163's checker run on that route accepts the certificate with no source present.
  - id: FR-940-AC-19
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: 'Three test-only variants of AC-16''s unit pass the strict reader and change `Equal` or the projection-aligned clause: `Equal`''s slot node `slot_sc_sb` is renamed `slot_sc_sz` (`z` is declared by no Group); `Equal`''s atom''s `c.value` read is rewritten so its operand roots in both `slot_sc_sa` and `slot_sc_sb`; or a conjunct slot of the projection-aligned clause''s `match` is renamed `opx_sa`. In each admitted variant, `Equal` (or the projection-aligned clause in the last) settles `failed` with `MalformedNode` naming its `hyperproperty` node key and no verdict, while a sibling `Partner` item settl'
  - id: FR-940-AC-6
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-
  - id: FR-488-AC-16
    path: spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md
    role: examined
    excerpt: '`Linked` with `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a` emits.'
  - id: FR-487-AC-3
    path: spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md
    role: examined
    excerpt: Over ADR-023 §8's secure vault at instance `v`, `evaluate_hyper_holds` for `NonInterference`'s atom gives `Completed(true)` with components `(0, 0)` and `(1, 0)`, and `Completed(false)` with `(0, 1)` and `(0, 0)`. For FR-176-AC-5's atom `1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b)` with components `(1, 0)` and `(0, 0)`, it gives `Undefined` with cause `division-by-zero`. A meter with a zero budget gives `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. A `slots` vector of length 1 gives `CallFailure::Fault` and charges nothing. Two equal calls give equal evaluations and equal ch
  - id: fsl-analyze/Cargo.toml
    path: fsl-analyze/Cargo.toml
    role: examined
    excerpt: Public re-export or development dependency wiring.
  - id: fsl-analyze/tests/it/package_forms.rs
    path: fsl-analyze/tests/it/package_forms.rs
    role: examined
    excerpt: '//! FR-940 "Package forms of the source-checked evaluators" (TC-2958): each'
  - id: fsl-analyze/tests/it/package_forms/hyper.rs
    path: fsl-analyze/tests/it/package_forms/hyper.rs
    role: examined
    excerpt: //! Genuine joint-state evaluation of a package atom through the shared machine.
  - id: fsl-eval/src/value/expression/bare.rs
    path: fsl-eval/src/value/expression/bare.rs
    role: examined
    excerpt: '//! FR-531, FR-533, FR-945: evaluation of calls from a package read from its'
  - id: fsl-package/src/checked_views.rs
    path: fsl-package/src/checked_views.rs
    role: examined
    excerpt: '//! FR-515 (TC-1310): the checked predicate and the temporal subject, read as'
  - id: fsl-package/src/checked_views/hyper.rs
    path: fsl-package/src/checked_views/hyper.rs
    role: examined
    excerpt: '//! FR-940: source-independent facts of a universal hyper clause.'
  - id: fsl-package/src/checked_views/hyper/projection.rs
    path: fsl-package/src/checked_views/hyper/projection.rs
    role: examined
    excerpt: '//! FR-940: an admitted hyper atom projected over its ordered atom slots.'
  - id: fsl-package/src/emit/tests/hyper_clause.rs
    path: fsl-package/src/emit/tests/hyper_clause.rs
    role: examined
    excerpt: '//! FR-488 (TC-1266): a checked hyper clause over `behaviours` emits as FSpec'
  - id: fsl-package/src/lib.rs
    path: fsl-package/src/lib.rs
    role: examined
    excerpt: '//! ADR-011 §6.1 layer 4, `package` (ADR-011 §7.3 X-7): the S4'
bindings:
  - test_id: package_hyper_atom_has_source_values_and_complete_charge_logs
    ac_id: FR-940-AC-6
    trace: wrong
  - test_id: package_hyper_atom_preserves_absent_object_outcomes_and_charges
    ac_id: FR-940-AC-6
    trace: wrong
  - test_id: package_hyper_atom_has_source_values_and_complete_charge_logs
    ac_id: FR-487-AC-3
    trace: correct
  - test_id: package_hyper_atom_refuses_wrong_binding_shapes_without_charges
    ac_id: FR-487-AC-3
    trace: correct
  - test_id: a_package_hyper_atom_refuses_an_admitted_mixed_component_receiver
    ac_id: FR-940-AC-19
    trace: correct
```
