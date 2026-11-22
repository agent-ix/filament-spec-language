---
id: SR-20261121230215004
title: "Spec dependency analysis of PR filament-spec-language#3410"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-2751; PR3410 head b12cb87d5accc22e26e0f3c944c4d23b455c6820; spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-525
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2920
    type: reviews
---

## Summary

Ticket: QSL-2751. PR filament-spec-language#3410, frozen head b12cb87d5accc22e26e0f3c944c4d23b455c6820, base 069d7ee184861cdda0f3fe776c3a0310a850dd6f; the review covers the actual diff only (FR-338, FR-126, FR-337, FR-1030, TC-525, TC-2920). The slice is authorized by the 2026-11-21 architecture ruling: TP-3 `on each` for ordinary transfer premises, plus real source and package protocol claim certification on existing paths, with literal protocol transfer still open. Method: quoin `spec-dependency-analysis` (0.31.1): relationship edges that the diff adds or implies. The author's report and the ticket text were read as data. The PR is spec-first, so the absence of an implementation is not a finding; only contradictions and non-executable vectors are. Tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0; FSpec context read at filament-specification origin/main 2ccc41ea6c9b3a6639e46f069c768688efadca94. Model claude-opus-5-5, run ac5fa794-ab77-458a-8bab-334cb897d48a.

## Verdict

FIX NEEDED (low). One low finding: the FR-1030 → FR-338 edge is typed `references` where the dependency is normative. The TC `verifies` edges are unchanged and correct. No relationship cycle or missing target was introduced.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The new FR-1030 → FR-338 edge is typed `references`, but the edited Behavior makes the TP-3 premise `proved`, `Certified` only when FR-338's checker accepts the certificate. That is a normative dependency, like the existing FR-128 and FR-144 edges, which are typed `depends_on`. Type it `depends_on`. | spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md:24 | wrong-requirement |

## Assurance Context

No `type: AssuranceProfile` document applies under `spec/`. This is a direct delegated review, with no enforced workflow or human gate inferred.

## Examined scope

```yaml
scope:
- id: FR-1030
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: "unproved slot 1, never `proved`. A past or interval (TP-3) clause over an\n  ordinary model subject, `on origin` or `on each (q: T)` with or without a\n  `when` condition, settles its premise `proved`, `Certified` only when the\n  core checker accepts its FR-338 certificate (a clause the engine decides\n  `Holds` is not `proved` by that alone), and\n  `inconclusive`, `CertificateRejected` when it rejects it. A check that stops\n  on a limit or a cancel settles `incomplete`, and each of these makes the\n  transfer `inconclusive`, `premise-unproved` naming slot 1 with that result."
- id: FR-126
  path: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md
  role: examined
  excerpt: "- An infinite-trace TP-1 or TP-3 clause with an unguarded `on each` activation\n  over a model subject SHALL carry the `ClosureCertificate` of every explored\n  product state in its `Holds`, keyed as FR-338 \"Unguarded `on each` TP-1 and\n  TP-3 clause\" states; FR-127 settles it `proved` when the checker accepts it.\n  A TP-3 clause over a protocol subject carries the certificate FR-338 \"TP-3\n  clause over a protocol subject\" states, and a clause with a future interval\n  is `Holds` only when no fair zero-distance cycle keeps its debt open."
- id: FR-337
  path: spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md
  role: examined
  excerpt: "clauses, as the protocol activation forms are (FR-381 \"Activation\"):\n  FR-338 keys the first with activation `\"each\"` (a bounded or TP-4 clause)\n  or `\"infinite\"` (an infinite-trace TP-1 or TP-3 clause) and the second with\n  `\"each-when\"`."
- id: TC-525
  path: spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md
  role: examined
  excerpt: "17. FR-338-AC-30: over the Counter subject, check the unguarded `on each`\n    TP-3 clause with the body `holds(c.value = 1) implies once[1,1]\n    holds(c.value = 0)`; offer its certificate for the bodies `holds(c.value =\n    2) implies once[1,1] holds(c.value = 0)` and `holds(c.value = 3) implies\n    once[1,1] holds(c.value = 2)`; remove one reached product state; replace\n    every activation by `\"each\"`; check the guarded variant `when (c.value >=\n    2)`.\n18."
- id: TC-2920
  path: spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: '8. Declare `PrevSafeEach`, `SoonTwoEach`, `PrevSafeWhen`, `PrevJumpEach` and `PrevStutterEach` as FR-1030-AC-8 and AC-9 give them, and run `PrevSafeEach`, `SoonTwoEach` and `PrevSafeWhen` through `ThirdRefinesSpec` with no `premises`.

    9. Run `PrevJumpEach` and `PrevStutterEach` through `ThirdRefinesSpec` with no `premises`.

    10. Run `PrevSafeEach` through `ThirdRefinesSpec` with `abstract.initial` naming a snapshot of `value` 1.

    11. Settle `PrevSafeEach`''s abstract claim from the checked package S4 emits for the unit, and settle its transfer through `planner::settle` over that package.'
- id: FR-338
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: context_only
  excerpt: 'An infinite-trace TP-1 or TP-3 clause whose `on each (q: T)` activation has

    no `when` condition (`Activation::Each { guarded: false }`, FR-337), checked

    over a model subject, SHALL use the guarded algebra above ("Guarded `on each`

    activation") with the condition read true at every activation position.

    Its state is `["safety", branches]`, and its activation is `"infinite"`. The

    context''s root names the clause node, whose content identity covers the

    `on each` activation, so the root binds it and a key of an `on each` clause

    never equals a key of an `on origin` clause with the same body.'
- id: FSpec-FR-820
  path: agent-ix/filament-specification@2ccc41ea6c9b3a6639e46f069c768688efadca94:spec/objects/interfaces/FR-820-transfer-item-settlement-request-and-transfer-record.md
  role: context_only
  excerpt: "4. **Premises.** The first slot, in slot order, whose result is not `proved`\n   settles `inconclusive`, `unsettled`, `premise-unproved{premise, result}`,\n   naming the slot index and its FR-331 result value; `unsupported`,\n   `incomplete`, `refuted`, `inconclusive` and `failed` premises all settle\n   this way (FR-380-AC-5)."
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
