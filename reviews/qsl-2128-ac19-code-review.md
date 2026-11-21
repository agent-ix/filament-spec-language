---
id: SR-21281901
title: "code-review review of the AC-19 reader boundary"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/filament-spec-language; spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md; QSL-2128; TC-3127 context"
---

## Summary

Ticket: QSL-2128. Review of PR 3395's narrow reader-boundary correction. No defects found by this method.

## Verdict

**PASS** — The prose-only change preserves the stronger reader and changes no runtime code, dependency, pin, API, test binding, algorithm or gate. No duplication, vendored artifact, suppression or new tracking mechanism was introduced. Runtime acceptance coverage remains outside this spec-only review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and applicability

Frozen candidate: 190a304cc2c8e86748ad96bdd83320c0bc28f143 against ba280846beb6e8bf79c89b179c8841839c203090. Only FR-940 Refusals and AC-19 change; AC-16 through AC-18 are unchanged context. No executable code or embedded Rust test includes this document, so Cargo and the aggregate gate do not apply. No full AC-18 implementation claim is made.

Independent contract inspection: Cargo.toml pins Contract IR a03c86b3bcb639c0f029bfc5d983803ef66124c3 and FSpec 2bec0061eaaa91f095eb1cafe6de11519302d337. The pinned FSpec FR-402 Reader order Body step 3.5 and wrapper scope agree with IR checked_package/v2/temporal.rs:3071-3116: slot names are unique in scope, each level is its ordered position, a level defect is InvalidSemanticGraph with no cause at that slot's body/members/1, before hyper_clause_form. Renaming slot_sc_sb to slot_sc_sz retains the two ordered levels; the mixed-root read retains the scope; the opx_sa conjunct mutation concerns the separate match scope. The unchanged atom/conjunct level-0 control must remain admitted. Body-target and missing-quantifiers refusals remain earlier reader failures.

Scoped quire validation and git diff --check passed. No applicable AssuranceProfile was found in spec/. Base plus scope-boundary was selected by the dispatch and repository instruction to use applicable lenses. No new requirement statement, structural artifact change, domain object, relationship edge, verification method, security algorithm or performance change warrants EARS, integrity, object, dependency, evidence, risk or security analyses. Production code did not change, so gap-analysis and language lanes do not apply.

## Scope

```yaml
scope:
  - id: FR-940-AC-19
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: "Three test-only variants of AC-16's unit pass the strict reader and
      change `Equal` or the projection-aligned clause: `Equal`'s slot node
      `slot_sc_sb` is renamed `slot_sc_sz` (`z` is declared by no Group);
      `Equal`'s atom's `c.value` read is rewritten so its operand roots in both
      `slot_sc_sa` and `slot_sc_sb`; or a conjunct slot of the
      projection-aligned clause's `match` is renamed `opx_sa`. In each admitted
      variant, `Equal` (or the projection-aligned clause in the last) settles
      `failed` with `MalformedNode` naming its `hyperproperty` node key and no
      verdict, while a sibling `Partner` item settl"
  - id: FR-940/Refusals/admission
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: |-
      A duplicate or missing atom-slot level is refused
        `invalid_semantic_graph` at the offending slot's
        `/semantic_graph/nodes/{s}/body/members/1`.
  - id: FR-940/Refusals/admitted
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: >-
      The strict reader checks the `hyperproperty` aggregate's layout, its
        subjects and quantifier Groups, its body wrapper's kind, fairness and slot
        scope, but does not check every slot-name and model-read condition below.
  - id: FR-940-AC-16
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: context_only
    excerpt: Over AC-4's Counter unit with the hyper clauses `Equal` (`forall trace
      a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b
      <= 3) }`, universal, copy-swap symmetric), `Partner` (forall-exists
      equality), `CanReachThree` (single existential) and a projection-aligned
      clause with an `align skip` list and a `match` block holding one universal
      and one existential conjunct, `hyper_clause_form` of each clause's
      `claim_name` occurrence returns equal records over the
      `AdmittedPackageRead` and over the `BarePackage` of its handoff directory,
      and the clause `of_package_form` rebuilds
  - id: FR-940-AC-17
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: context_only
    excerpt: A `program` request and a `package` request over the handoff directory
      of AC-16's unit, written by its compile request's `handoff` member, with
      the items `Equal`, `Partner`, `CanReachThree` and the projection-aligned
      clause, and the same items over the unit with `Equal`'s bound lowered to
      `c.value @ b <= 2` so it refutes, give equal item records, counterexamples
      (one trace per copy; for `Partner` the exhausted witness set),
      copy-by-copy replays with `reproduced` `true` and product-closure
      certificates accepted by FR-163's checker; `Equal` settles `proved` with
      certification `certified` on both
  - id: FR-940-AC-18
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: context_only
    excerpt: AC-17's `package` request run after the unit's source file is
      overwritten with bytes that are not a native unit and the handoff
      directory's `sources/` is deleted settles `Equal` `proved` with the record
      of AC-17's source request; FR-163's checker run on that route accepts the
      certificate with no source present.
  - id: TC-3127/step-3
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: context_only
    excerpt: >-
      3. Analyze `Equal`, the projection-aligned clause and `Partner` over
         test-only packages the strict reader admits: `Equal`'s `slot_sc_sb`
         renamed `slot_sc_sz`, `slot_sc_sb` written at level 0, a read rooted in
         `slot_sc_sa` and `slot_sc_sb`, and a projection-aligned conjunct slot
         renamed `opx_sa`; also analyze the unmodified projection-aligned clause.
  - id: TC-3127/expected-step-3
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: context_only
    excerpt: >-
      - Step 3: the modified clause settles `failed` with `MalformedNode` naming
        its `hyperproperty` node key and no verdict; `Partner` and the unmodified
        projection-aligned clause (an atom slot and a conjunct slot both at level
        0) settle as in step 2.
```
