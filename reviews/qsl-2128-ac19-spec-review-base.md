---
id: SR-21281902
title: "spec-review/base review of the AC-19 reader boundary"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/filament-spec-language; spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md; QSL-2128; TC-3127 context"
---

## Summary

Ticket: QSL-2128. Review of PR 3395's narrow reader-boundary correction. AC-19 moves the duplicate-level mutation to whole-request refusal, but its named TC-3127 still requires strict admission, item MalformedNode and Partner settlement for that same mutation.

## Verdict

**FAIL** — The corrected AC and refusal prose match the pinned contract. Its own referenced test case still specifies the old impossible admission and outcome; reconcile that test procedure and expected result before merging.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-19 moves the duplicate-level mutation to whole-request refusal, but its named TC-3127 still requires strict admission, item MalformedNode and Partner settlement for that same mutation. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:738; spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md:28; spec/test-cases/TC-3127-a-package-hyper-item-settles-as-its-source-item.md:44 |

## Required correction

Move TC-3127's duplicate-level mutation out of its admitted variants, and require the whole-request invalid_semantic_graph refusal with no cause at the slot level pointer, empty items, last_stage null, exit 20, no compiled source and no Partner settlement. Keep the three admitted malformed forms and the independent atom/conjunct level control.

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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: TC-3127 procedure step 4 and Expected Results step 4 now require the duplicate-level whole-request reader refusal, with no cause, exact slot-level pointer, empty items, null last_stage, exit 20, no source compiled and no Partner settlement; the three admitted malformed variants remain in step 3. |

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

## Disposition pass 2

**Latest verdict: PASS.** Frozen head 4dddd9a97a5f0cbd6d2a0800139dd8cf1641e217 against base 514d2f5596859fbd449f27f77e08488c73072d30. Independently inspected the new span-policy delta against the prior custody tree 0b92916 (direct tree diff because the branch was rebased), the complete current FR-940 outputs/input restriction, AC-16 and TC-3126, with FR-488 declaration-region ownership as context. This is the final authorized author fix round. No new findings. The prior FND-001 remains fixed: AC-17, AC-18, AC-19 and TC-3127 were compared byte-for-byte with the prior reviewed tree and remain unchanged. All prior canonical SRs were also compared with their committed copies and match verbatim.

The new rule derives every PackageHyperHolds and rebuilt Holds diagnostic span from only the selected claim row's declaration_region on BOTH AdmittedPackageRead and BarePackage. The source-checked S3 clause retains expression diagnostic spans; only the equality oracle canonicalizes S3 Holds spans after atom renumbering. The existing ban on expression source_map reads remains unchanged. Changing only an expression provenance region therefore cannot change the package hyper form under this contract. Changing a semantic node kind, operator, binder level, operand structure or conjunct partition still changes the comparison; the TC-3126 conjunct-partition negative control remains present. Copy-group classes, generators, aliases, parameters, prefix/fairness, normal-form rules and occurrence equality have no new exception. The claim-level span remains its existing declaration-region value. FR-488's whole-declaration region definition agrees. Source evaluator/checker behavior is not changed by this metadata policy.

This resolves the SPEC incompatibility exposed by external code review SR-21282001/FND-001; it does not disposition that separate code finding or claim its implementation fixed. Implementation verification stays with the code reviewer/lead. No waiver or full AC-18 claim is made: the full source-deletion, settlement, replay and certificate journey remains required in the later slice after PR3351. No Cargo, benchmarks, source edits, push or merge were performed. Scoped Quire validation and diff whitespace checks passed; no Rust embedding was found for the changed documents. Existing method selection remains sufficient: code-review, base specification review and scope-boundary. No applicable AssuranceProfile or new production language/gap-analysis lane applies.

### Prior finding outcome

FND-001 remains **fixed**, as recorded in round 1. No unresolved finding requires a new disposition row, and no new finding was added. Original finding text and prior disposition history remain intact.

### Scope examined in disposition pass 2

```yaml
scope:
  - id: FR-940/hyper-form-span-policy
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: >-
      - Every `PackageHyperHolds.span`, and the corresponding rebuilt formula
        `Holds` span, SHALL be the span of the selected `claim_names` row's
        `declaration_region` over both package views, `AdmittedPackageRead` and
        `BarePackage`. It is declaration-level diagnostic metadata, derived from
        no expression `source_map` region. S3's source-checked clause retains its
        expression diagnostic spans; only the comparison below canonicalizes them.
  - id: FR-940/hyper-form-input-restriction
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: >-
      The record charges nothing. It reads no source text, no `source_map`
      region

      other than the `declaration_region` of its row, no occurrence site label
      and

      no `NodeRef` path.
  - id: FR-940/hyper-form-comparison
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: |-
      atom index and S3's formula `Holds` spans are canonicalized to that row's
        `declaration_region` span. This canonicalization changes only diagnostic
        spans: every semantic node kind, operator, level and operand structure
        remains in the equality comparison.
  - id: FR-940-AC-16
    path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
    role: examined
    excerpt: only S3's formula `Holds` spans are canonicalized to the selected
      `claim_names` row's `declaration_region` span, preserving comparison of
      every semantic node kind, operator, level and operand structure; every
      `PackageHyperHolds.span` and rebuilt formula `Holds` span equals that
      declaration span
  - id: TC-3126/step-3
    path: spec/test-cases/TC-3126-a-package-hyper-form-rebuilds-the-checked-hyper-clause.md
    role: examined
    excerpt: >-
      3. Compare each rebuilt clause with S3's checked clause, renumbering S3's
         atoms by FR-532's atom index and canonicalizing only S3's formula `Holds`
         spans to the selected `claim_names` row's `declaration_region` span.
  - id: TC-3126/expected-step-2
    path: spec/test-cases/TC-3126-a-package-hyper-form-rebuilds-the-checked-hyper-clause.md
    role: examined
    excerpt: |-
      - Step 2: the two forms of each clause are equal, and every
        `PackageHyperHolds.span` and rebuilt formula `Holds` span equals the
        selected `claim_names` row's `declaration_region` span.
  - id: TC-3126/expected-step-3-equality
    path: spec/test-cases/TC-3126-a-package-hyper-form-rebuilds-the-checked-hyper-clause.md
    role: examined
    excerpt: >-
      - Step 3: name, span, profile, aliases, read object parameters, quantifier
        prefix with fairness sets, atom slots, conjunct count per part,
        `align skip`, form, copy-group classes, occurrence, formula arena, atom
        indices and copy generators are equal after step 3's atom renumbering and
        diagnostic-span canonicalization; every semantic node kind, operator,
        level and operand structure remains compared; `condition` of every
        rebuilt atom is `None`.
  - id: TC-3126/semantic-negative-control
    path: spec/test-cases/TC-3126-a-package-hyper-form-rebuilds-the-checked-hyper-clause.md
    role: examined
    excerpt: "- Step 4: the comparison reports the conjunct partition as different."
  - id: FR-488/declaration-region
    path: spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md
    role: context_only
    excerpt: "`declaration_region` is the whole hyper declaration's span."
```
