---
id: SR-202611212119190339300000004
title: "QSL-2128 implementation code-review of source-free HP1 analysis"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2128 PR3351; new package HP1 route, shared engine/checker/replay, FR531/940AC23-27/163/184"
review_set: subset
---

## Summary

PASS for static implementation inspection; runtime acceptance pending and separate gap findings block completion. Frozen implementation reviewed at00faf94d27db4124a273692c68a711599016cf96 against normal main cf8d85826db560569cd8806cce07007d4b493e0c. Earlier implementation head a37647400f was initially inspected against21edb2ead; git range-diff shows every own commit unchanged after the rebase. The upstream #3395 delta concerns the existing temporal hyper slot/span contract, not the new HP1 relation semantics. The existing specification-only artifacts are separate; this is the implementation review requested by the parent.

No source changes, pin changes, Cargo commands, runtime runs, broad gates, matrices, measurements or Linear operations were performed by this reviewer. The user/parent explicitly prohibited those operations and retained runtime/merge ownership. Focused artifact validation and diff inspection are the only checks run here. No whole-repository audit or engine-backed matrix PASS is claimed. Plan completion: not assessed

## Verdict

PASS for static implementation inspection; runtime acceptance pending and separate gap findings block completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No new production implementation defect established in this static pass. Separate gap findings remain. | - |

## Inspected implementation


The new package form reads an admitted claim occurrence, Boolean body, capability record, operation/parameter facts and the strict reader's declaration-order witness. Legacy absence is a distinct typed form-selection refusal mapped to item-local unknown_required_feature/unsupported-feature; malformed-present order stays a reader admission failure. The package route constructs no source CheckedPackage, CheckedGraph, source arena or CheckedModelRelation. Its descriptor contains execution, alias and parameter facts and dispatches tuple evaluation through the sealed ModelEvaluator.

The source adapter and source-free route converge on check_model_relation_input; tuple binding is shared, including actual typed results. Package expressions use the existing reader projection and machine. Execution reads route to the matching pre/post environments. Source graph handling remains confined to the source implementation/adapter. New public items have deliberate consumers and documentation; checked conversions and admitted typed body invariants protect package boundaries. Existing SPDX headers, integration-test registration, workspace unsafe policy and repository Rust conventions remain in place. No new unsafe, Tokio, external service, Python semantic implementation or CI/gate weakening appears.

Certificate handling remains one HP1 independent reconstruction algorithm under ModelRelationClosureRequest. It reconstructs original model subjects, states and complete labelled execution edges, validates alias and execution order, checks original typed results and evaluates every tuple with a fresh checker allowance. Engine proof results are not substituted for checker reconstruction. Counterexample replay reexecutes original model systems and uses the same evaluator port. Wrong/missing edges remain certificate-request input refusals; reachable-state tampering yields an independent rejection. Source wrapper compatibility is retained through descriptor adapters. No separate checker or copied semantic implementation was introduced.

The five new package tests cover source deletion, holding/refuting/undefined/budget/depth terminal parity, source-free independent checking and tampering, declaration/execution ordering with distinct aliases sharing one root, malformed-present order/legacy absence, and item-local ambiguous reads with a sibling. Two added source-port tests compare the shared engine and checker inputs/results; inherited HP1 tests remain. These are real production seams, not fake success paths. The three missing adverse/charging boundaries are recorded separately in the gap artifact; static code inspection did not establish an actual new production defect.

## Scoped coverage and reverse gaps

The package form, legacy refusal, evaluator port, route selection, shared check/replay and source-free checker are owned by FR531/940 and the existing FR163/184 semantics. No unowned behavior or source/test stub was identified in the changed surface. The new explicit trace attributes name FR940AC23–27, but tagging alone does not discharge the missing assertions above. This is a manual scoped criterion/source/test reconciliation authorized by the parent; no computed matrix or whole-source reverse audit was run, and no baseline repository debt is classified as a new PR defect.

## Validation and limits

Parent/author reports focused fsl-analyze --test it compilation passed at a37647400f before the spec-only normal rebase. Rebased focused compile and package five plus source twelve runtime tests are pending the parent/Titan lane. Earlier PR-body source-only prerequisite runs do not establish this final package implementation's acceptance. git diff --check over the rebased own delta passes. No applicable AssuranceProfile was found under spec. Skills applied: reviewer, code-review with rust-review/rust-style API/implementation/safety/wire/testing lanes, and scoped semantic gap analysis. The parent explicitly excluded fresh specification review, full gates, matrices and measurement; those were not silently claimed complete.

## Review data

```yaml
clean: true
scope:
- id: FR-940-AC-23
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: 'For FR-179-AC-1''s secure-vault `Det`, `model_relation_form` returns equal
    facts over the `AdmittedPackageRead` and the `BarePackage` of its emission: claim
    occurrence, name, recorded span, model requirement, body, alias identities, execution
    order, operation anchors, receiver/result/argument slots and pre/post reads. A
    two-alias variant declares `A` before `B` but binds `B`''s execution first: both
    forms retain aliases `[A, B]` and execution order `[B, A]`; two aliases selecting
    the same domain document remain separate. A test-only projection that swaps an
    execution''s pre/post read di'
- id: FR-940-AC-24
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: Source and package requests for FR-179-AC-1's secure `Det` both settle
    `proved`, certification `certified`, with equal terminal records and HP-1 certificates
    accepted by FR-163's independent checker. FR-179-AC-2's nondeterministic variant
    gives equal violated tuple paths and reproduced replay; FR-179-AC-4's undefined
    body gives equal undefined cause and replay. Evaluating their tuples on both routes
    gives equal charge logs. The requests with `max_relation_tuples` 63 stop at count
    64 with the same first unpaid tuple; those with 64 complete; the depth-1 variant
    has equal bounded outcom
- id: FR-940-AC-25
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: AC-24's package requests still produce the same proved, refuted, undefined
    and budget-stopped records after the source unit is overwritten with invalid bytes
    and the handoff's `sources/` is deleted. With no source present, FR-163's checker
    accepts the secure certificate, rejects a variant that omits a reachable state
    with a certificate rejection, refuses variants that omit an applicable edge or
    alter its typed result as certificate-request input mismatches without a certificate
    verdict or proof, and under tuple allowance 63 stops at count 64 identifying the
    first unpaid tuple. Checki
- id: FR-940-AC-26
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: Variants of AC-23's package with a duplicate or missing execution parameter
    level, an argument parameter with the wrong type, or an invalid step-relation
    body reference refuse the whole request before an engine runs, retaining the strict
    reader's code, cause and pointer. A test-only projection with a body model read
    that cannot be assigned to exactly one declared execution returns `MalformedNode`
    naming the relation node; its item has no verdict and a sibling valid item is
    unaffected. Neither case loads source or becomes a proved relation.
- id: FR-940-AC-27
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: AC-23's two-alias package carries `subject_declaration_order` entries `[A,
    B]` joining the same alias/root pairs as its first-use `subjects` entries `[B,
    A]`. The strict reader admits that witness and preserves `[A, B]`. Repeated aliases,
    unknown aliases, omitted aliases and an alias joined to the other subject's root
    each refuse with the reader's code, cause and member pointer. Distinct aliases
    sharing one root remain admitted. Removing only the optional member leaves the
    legacy package admitted globally, but its HP-1 model item settles `unsupported`
    with `unknown_required_feature`/
findings: []
```

## Fix round 1 implementation disposition

Narrow diff00faf94d27db4124a273692c68a711599016cf96..7fb07cd31064fb12e94ae36c038af9e7434b4c15 is confined to package acceptance tests and the dev-only existing filament-exact test-support feature. No production, lockfile or dependency revision changed. The original three gap findings are fixed statically in the companion gap artifact: actual tuple pre/post swap, complete nonempty/untruncated tuple charge parity, and four malformed package whole-request refusals after source deletion. Current sibling invariant syntax was corrected without changing its positive proof assertion. No new code/Rust concern identified in this narrow fix; no additional full review was performed.

Parent/author reports focused --locked analyze it compilation PASS in2m15. Runtime remains CONDITIONAL: parent reports an actual source-emitted package InvalidModelBinding at execution0 parameters before evaluator dispatch, under separate architecture triage. Nothing in the new tests repairs or bypasses that product blocker. No runtime acceptance or broad gate PASS is claimed. No Cargo/runtime/matrix/measurement/Linear operation ran in this reviewer lane. Original review text and the separate gap findings/dispositions remain preserved.

## Final narrow fix round 2

The exact delta a62a7bed66d73f072603332d0f6581b323f96abc..19ddb91ed77454a4f28734e1d05ddf19282e4f4e changes only package_model_relation.rs. No production source, dependency feature, pin or lockfile changes in this round. This is a refinement of the original AC23/26 test findings, not a new full review.

For FND-003, the malformed-input fixture no longer assumes raw operand-bearing node_id JSON equals a reference target JSON. It asserts the second execution is x with exactly two binders, selects the unique value/parameter nodes whose literal name/level bodies are self/2 and i/3, and fails if either selection is absent or ambiguous. Those levels are explicit facts of this two-execution fixture. The existing four malformed variants and all strict-reader/command code, cause, path, exit20 and zero-item assertions are unchanged. The selection correction makes those intended mutations reachable without altering production ID semantics or manufacturing a rekeyed package.

For FND-001, a secure `post OtherStep using v on N::Vault::step { self.l = i }` is added to the fixture, matching the existing V-side secure postcondition. Both subjects now justify the strengthened Ordered positive conjunction, while the deliberate one-execution pre/post swap still must change a real tuple evaluation. Declaration/execution order, distinct-alias/shared-root cases, real source/package outcome/charge comparisons, complete logs and expected result assertions remain intact. FND-002's charge-log parity changes are unchanged.

Static disposition: CLEAR for the two narrow corrections; the original three findings remain fixed statically. Parent/author reports focused compile PASS in32.69s at19ddb91ed77454a4f28734e1d05ddf19282e4f4e. git diff --check passes. Final runtime acceptance remains CONDITIONAL on the separately reported positive emitted-package/readback InvalidModelBinding owner triage and the parent's exact-head package/source test runs. No reviewer Cargo, runtime, matrix, measurement, Linear operation, source edit or third full review occurred.
