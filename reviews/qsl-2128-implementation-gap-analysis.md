---
id: SR-202611212119190339300000005
title: "QSL-2128 implementation gap-analysis of source-free HP1 analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2128 PR3351; new package HP1 route, shared engine/checker/replay, FR531/940AC23-27/163/184"
review_set: subset
---

## Summary

FAIL for the three bounded new acceptance gaps; runtime acceptance pending. Frozen implementation reviewed at00faf94d27db4124a273692c68a711599016cf96 against normal main cf8d85826db560569cd8806cce07007d4b493e0c. Earlier implementation head a37647400f was initially inspected against21edb2ead; git range-diff shows every own commit unchanged after the rebase. The upstream #3395 delta concerns the existing temporal hyper slot/span contract, not the new HP1 relation semantics. The existing specification-only artifacts are separate; this is the implementation review requested by the parent.

No source changes, pin changes, Cargo commands, runtime runs, broad gates, matrices, measurements or Linear operations were performed by this reviewer. The user/parent explicitly prohibited those operations and retained runtime/merge ownership. Focused artifact validation and diff inspection are the only checks run here. No whole-repository audit or engine-backed matrix PASS is claimed. Plan completion: not assessed

## Verdict

FAIL for the three bounded new acceptance gaps; runtime acceptance pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new pre/post ownership oracle lacks AC23's required swapped-observation adverse check. Its ordered relation compares each pre/post field only with itself, so a consistently swapped evaluator can still prove the same tautologies. | fsl-analyze/tests/it/package_model_relation.rs:289-294,334-444; FR-940-AC-23 |
| FND-002 | medium | The package/source test compares terminal records, replay and certificates but never compares relation-tuple charge logs required by AC24. Every tuple meter is discarded, so equal final outcomes do not establish equal evaluator charging. | fsl-analyze/tests/it/package_model_relation.rs:50-125; FR-940-AC-24 |
| FND-003 | medium | AC26 is tagged only on the ambiguous-read item-local test. No new FSL test mutates duplicate/missing execution parameter levels, a wrong argument type or the step-relation body reference and asserts strict whole-request refusal before analysis. | fsl-analyze/tests/it/package_model_relation.rs:550-594; FR-940-AC-26 |

### FND-001

Use a real reachable transition whose pre/post field values differ and compare source/package tuple evaluation, then deliberately swap one execution's pre/post observation in a test projection and require the comparison to differ. Preserve the declared-alias/execution order and shared-root positive checks. The current set of (execution, observation) metadata entries contains both Pre and Post, so swapping their meanings does not establish the missing value oracle.

### FND-002

Evaluate actual labelled tuples through both route evaluators with fresh meters and compare the outcome and complete charge logs, including a holding body, a false body and a runtime-undefined body. The production engine/checker already invokes fresh meters; this finding is missing new acceptance evidence, not a claim that their counters are shared.

### FND-003

Add the four specified malformed-present package variants and exercise the package analyze admission boundary, asserting its reader code/cause/path and whole-request refusal with no item verdict. The existing malformed declaration-order variants and ambiguous-read sibling test should remain. Reader-owner tests alone do not establish this consumer's whole-request routing.

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
clean: false
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
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-940-AC-23
  related_ids: []
  path: fsl-analyze/tests/it/package_model_relation.rs
  lines: 289-294,334-444
  excerpt: 'For FR-179-AC-1''s secure-vault `Det`, `model_relation_form` returns equal
    facts over the `AdmittedPackageRead` and the `BarePackage` of its emission: claim
    occurrence, name, recorded span, model requirement, body, alias identities, execution
    order, operation anchors, receiver/result/argument slots and pre/post reads. A
    two-alias variant declares `A` before `B` but binds `B`''s execution first: both
    forms retain aliases `[A, B]` and execution order `[B, A]`; two aliases selecting
    the same domain document remain separate. A test-only projection that swaps an
    execution''s pre/post read di'
  finding: The new pre/post ownership oracle lacks AC23's required swapped-observation
    adverse check. Its ordered relation compares each pre/post field only with itself,
    so a consistently swapped evaluator can still prove the same tautologies.
  details: Use a real reachable transition whose pre/post field values differ and
    compare source/package tuple evaluation, then deliberately swap one execution's
    pre/post observation in a test projection and require the comparison to differ.
    Preserve the declared-alias/execution order and shared-root positive checks. The
    current set of (execution, observation) metadata entries contains both Pre and
    Post, so swapping their meanings does not establish the missing value oracle.
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-940-AC-24
  related_ids: []
  path: fsl-analyze/tests/it/package_model_relation.rs
  lines: 50-125
  excerpt: Source and package requests for FR-179-AC-1's secure `Det` both settle
    `proved`, certification `certified`, with equal terminal records and HP-1 certificates
    accepted by FR-163's independent checker. FR-179-AC-2's nondeterministic variant
    gives equal violated tuple paths and reproduced replay; FR-179-AC-4's undefined
    body gives equal undefined cause and replay. Evaluating their tuples on both routes
    gives equal charge logs. The requests with `max_relation_tuples` 63 stop at count
    64 with the same first unpaid tuple; those with 64 complete; the depth-1 variant
    has equal bounded outcom
  finding: The package/source test compares terminal records, replay and certificates
    but never compares relation-tuple charge logs required by AC24. Every tuple meter
    is discarded, so equal final outcomes do not establish equal evaluator charging.
  details: Evaluate actual labelled tuples through both route evaluators with fresh
    meters and compare the outcome and complete charge logs, including a holding body,
    a false body and a runtime-undefined body. The production engine/checker already
    invokes fresh meters; this finding is missing new acceptance evidence, not a claim
    that their counters are shared.
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-940-AC-26
  related_ids: []
  path: fsl-analyze/tests/it/package_model_relation.rs
  lines: 550-594
  excerpt: Variants of AC-23's package with a duplicate or missing execution parameter
    level, an argument parameter with the wrong type, or an invalid step-relation
    body reference refuse the whole request before an engine runs, retaining the strict
    reader's code, cause and pointer. A test-only projection with a body model read
    that cannot be assigned to exactly one declared execution returns `MalformedNode`
    naming the relation node; its item has no verdict and a sibling valid item is
    unaffected. Neither case loads source or becomes a proved relation.
  finding: AC26 is tagged only on the ambiguous-read item-local test. No new FSL test
    mutates duplicate/missing execution parameter levels, a wrong argument type or
    the step-relation body reference and asserts strict whole-request refusal before
    analysis.
  details: Add the four specified malformed-present package variants and exercise
    the package analyze admission boundary, asserting its reader code/cause/path and
    whole-request refusal with no item verdict. The existing malformed declaration-order
    variants and ambiguous-read sibling test should remain. Reader-owner tests alone
    do not establish this consumer's whole-request routing.
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round1 at7fb07cd31064fb12e94ae36c038af9e7434b4c15: The real Ordered relation now requires post l=i, not only self-equalities. Actual initial reachable labelled tuples are evaluated through the two real evaluator ports; the package evaluator is then supplied one execution with pre/post observations exchanged and at least one outcome must differ. |
| FND-002 | fixed | Round1 at7fb07cd31064fb12e94ae36c038af9e7434b4c15: Fresh source/package meters retain complete nonempty admitted charge logs for actual reachable transition tuples. The test compares outcomes and complete charges, checks neither log truncated, and asserts that the intended holding, false and runtime-undefined tuple cases were actually encountered. |
| FND-003 | fixed | Round1 at7fb07cd31064fb12e94ae36c038af9e7434b4c15: Four malformed raw package variants exercise duplicate parameter level, missing level, wrong argument type and invalid relation body target. After source deletion, the real package analyze boundary must refuse the whole request with exit20, zero items and diagnostic code/cause/path equal to the real strict reader first refusal. No reader copy or fabricated deeper pointer is introduced. |
| FND-001 | fixed | Round2 at19ddb91ed7 supplies the matching secure N postcondition so the stronger Ordered positive and swapped-observation control are grounded on both subjects. |
| FND-003 | fixed | Round2 at19ddb91ed7 selects unique literal self/2 and i/3 parameter bodies after asserting execution x and exactly two binders; all four malformed-input boundary oracles remain. |

## Fix round 1 verdict

CLEAR for the three original static acceptance findings at7fb07cd31064fb12e94ae36c038af9e7434b4c15; each is fixed as inspected, with runtime confirmation still conditional. The original findings and initial FAIL remain above as the historical review, not an assertion that they remain open. No new static defect was identified in this narrow correction.

The exact diff00faf94d..7fb07cd310 changes only package_model_relation.rs and fsl-analyze/Cargo.toml. The dependency change enables filament-exact's existing test-support feature solely in dev-dependencies to expose the meter log; production dependency declaration, dependency revision and lockfile are unchanged. No semantic stub, runtime test bypass or aggregate gate wiring is added. The package fixtures now use the current explicit `at current` invariant syntax; the sibling is still asserted to prove. The existing five tagged tests remain selected; helpers are called from those tests rather than adding unregistered top-level integrations.

The new helper obtains the source checked handoff and an independently admitted bare package with real identity/equality supply. It constructs actual model systems and transition edges, observes their real pre/post environments and typed results, and invokes both ModelEvaluator implementations. It never copies a source result to stand in for a package result. Nonempty edge lists, observed outcome categories, nonempty/untruncated logs and at least one swapped-observation difference are all unconditional assertions, so the new evidence cannot pass by skipping its intended path. The deliberately swapped evaluator input changes one execution's observation meaning and preserves the other execution and transition label.

Malformed bytes are tested at both the strict reader and real command boundary after invalidating/removing source. Integrity validation may precede the deeper execution joins on those deliberately edited bytes; the command must preserve that actual first reader refusal. The test does not rekey the document or invent an expected deeper pointer. The original malformed order, legacy item-local refusal and ambiguous-read sibling cases remain.

Parent/author reports focused --locked fsl-analyze --test it compilation PASS in2m15 at7fb07cd310. Reviewer ran no Cargo, runtime test, matrix, measurement or fresh full review; git diff --check passes. The parent separately reports a real emitted-package/readback InvalidModelBinding at execution0 parameters before evaluator dispatch. That runtime source-emission blocker remains under architecture triage and was not repaired by this test-only round. Accordingly this review does not claim package runtime acceptance, successful source-free end-to-end behavior, final gate qualification or whole-repository assurance. The parent owns the pending package/source runtime runs and merge gate.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "                swapped_pre[0] = after[0];\n                swapped_post[0]\
    \ = before[0];"
  reason: The real Ordered relation now requires post l=i, not only self-equalities.
    Actual initial reachable labelled tuples are evaluated through the two real evaluator
    ports; the package evaluator is then supplied one execution with pre/post observations
    exchanged and at least one outcome must differ.
- fnd: FND-002
  outcome: fixed
  after_excerpt: "            assert_eq!(actual, expected);\n            assert!(!source_meter.charge_log_truncated());\n\
    \            assert!(!package_meter.charge_log_truncated());\n            assert!(!source_meter.admitted_charges().is_empty());\n\
    \            assert_eq!(\n                package_meter.admitted_charges(),\n\
    \                source_meter.admitted_charges()\n            );"
  reason: Fresh source/package meters retain complete nonempty admitted charge logs
    for actual reachable transition tuples. The test compares outcomes and complete
    charges, checks neither log truncated, and asserts that the intended holding,
    false and runtime-undefined tuple cases were actually encountered.
- fnd: FND-003
  outcome: fixed
  after_excerpt: "        assert_eq!(run.exit, 20, \"defect {defect}: {document}\"\
    );\n        assert_eq!(\n            document[\"items\"].as_array().map(Vec::len),\n\
    \            Some(0),\n            \"{document}\"\n        );"
  reason: Four malformed raw package variants exercise duplicate parameter level,
    missing level, wrong argument type and invalid relation body target. After source
    deletion, the real package analyze boundary must refuse the whole request with
    exit20, zero items and diagnostic code/cause/path equal to the real strict reader
    first refusal. No reader copy or fabricated deeper pointer is introduced.
```

## Final narrow fix round 2

The exact delta a62a7bed66d73f072603332d0f6581b323f96abc..19ddb91ed77454a4f28734e1d05ddf19282e4f4e changes only package_model_relation.rs. No production source, dependency feature, pin or lockfile changes in this round. This is a refinement of the original AC23/26 test findings, not a new full review.

For FND-003, the malformed-input fixture no longer assumes raw operand-bearing node_id JSON equals a reference target JSON. It asserts the second execution is x with exactly two binders, selects the unique value/parameter nodes whose literal name/level bodies are self/2 and i/3, and fails if either selection is absent or ambiguous. Those levels are explicit facts of this two-execution fixture. The existing four malformed variants and all strict-reader/command code, cause, path, exit20 and zero-item assertions are unchanged. The selection correction makes those intended mutations reachable without altering production ID semantics or manufacturing a rekeyed package.

For FND-001, a secure `post OtherStep using v on N::Vault::step { self.l = i }` is added to the fixture, matching the existing V-side secure postcondition. Both subjects now justify the strengthened Ordered positive conjunction, while the deliberate one-execution pre/post swap still must change a real tuple evaluation. Declaration/execution order, distinct-alias/shared-root cases, real source/package outcome/charge comparisons, complete logs and expected result assertions remain intact. FND-002's charge-log parity changes are unchanged.

Static disposition: CLEAR for the two narrow corrections; the original three findings remain fixed statically. Parent/author reports focused compile PASS in32.69s at19ddb91ed77454a4f28734e1d05ddf19282e4f4e. git diff --check passes. Final runtime acceptance remains CONDITIONAL on the separately reported positive emitted-package/readback InvalidModelBinding owner triage and the parent's exact-head package/source test runs. No reviewer Cargo, runtime, matrix, measurement, Linear operation, source edit or third full review occurred.
