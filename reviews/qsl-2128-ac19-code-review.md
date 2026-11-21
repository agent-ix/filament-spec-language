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

## Disposition pass 1

**Latest verdict: PASS.** Frozen fix head: 54b94d4003d19b04b9eccb1b4acdbb99041025ea; base ba280846beb6e8bf79c89b179c8841839c203090. Re-read the full original finding and inspected the fix diff independently. All three committed initial SRs match the original canonical files byte-for-byte. No new findings. FR-940 and AC-16 through AC-18 are unchanged by the fix. TC-3127 now separates the duplicate-level mutation into procedure step 4 with its exact whole-request refusal in Expected Results step 4; its admitted variants, Partner behavior, atom/conjunct positive control, and prior body/quantifier refusals remain intact (the latter are step 5). No reader, dependency, pin, API, algorithm, source code or executable test changed. This remains a spec-only review; no Cargo or aggregate gate applies. No full AC-18 implementation claim is made.

### Scope examined in disposition pass 1

```yaml
scope:
  - id: TC-3127/step-3
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: >-
      3. Analyze `Equal`, the projection-aligned clause and `Partner` over
         test-only packages the strict reader admits: `Equal`'s `slot_sc_sb`
         renamed `slot_sc_sz`, a read rooted in
         `slot_sc_sa` and `slot_sc_sb`, and a projection-aligned conjunct slot
         renamed `opx_sa`; also analyze the unmodified projection-aligned clause.
  - id: TC-3127/step-4
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: |-
      4. Analyze a handoff directory whose package has `Equal`'s `slot_sc_sb`
         written at level 0 in place of level 1, leaving two atom slot nodes at
         level 0 and none at level 1; include `Partner` in the request.
  - id: TC-3127/step-5
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: |-
      5. Analyze handoff directories whose package has `Equal`'s `body` binding
         referencing the atom's `expression` node, or its `quantifiers` Tuple
         removed.
  - id: TC-3127/expected-step-3
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: >-
      - Step 3: the modified clause settles `failed` with `MalformedNode` naming
        its `hyperproperty` node key and no verdict; `Partner` and the unmodified
        projection-aligned clause (an atom slot and a conjunct slot both at level
        0) settle as in step 2.
  - id: TC-3127/expected-step-4
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: |-
      - Step 4: a whole-request refusal at the request stage, before
        `hyper_clause_form` or any item runs, with the reader's
        `invalid_semantic_graph`, no cause, at
        `/semantic_graph/nodes/{s}/body/members/1`, where `s` is `slot_sc_sb`'s
        graph index, `items` `[]`, `last_stage` `null`, exit 20, no source
        compiled and no `Partner` item settled.
  - id: TC-3127/expected-step-5
    path: spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md
    role: examined
    excerpt: >-
      - Step 5: whole-request refusals at the request stage with the reader's
        code, cause and pointer (`ill_typed`/`operator-ineligible` at the
        `hyperproperty` node for the body case), `items` `[]`, `last_stage` `null`, exit 20, and no
        source compiled.
```
