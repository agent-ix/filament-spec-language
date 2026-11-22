---
id: SR-20261121230215003
title: "Spec failure-domain analysis of PR filament-spec-language#3410"
type: SpecReview
analysis: failure-domain
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

Ticket: QSL-2751. PR filament-spec-language#3410, frozen head b12cb87d5accc22e26e0f3c944c4d23b455c6820, base 069d7ee184861cdda0f3fe776c3a0310a850dd6f; the review covers the actual diff only (FR-338, FR-126, FR-337, FR-1030, TC-525, TC-2920). The slice is authorized by the 2026-11-21 architecture ruling: TP-3 `on each` for ordinary transfer premises, plus real source and package protocol claim certification on existing paths, with literal protocol transfer still open. Method: quoin `spec-failure-domain-analysis` (0.31.1): limit, cancel, undefined, tamper, identity, fairness and bound failure modes of the newly certified modes. The author's report and the ticket text were read as data. The PR is spec-first, so the absence of an implementation is not a finding; only contradictions and non-executable vectors are. Tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0; FSpec context read at filament-specification origin/main 2ccc41ea6c9b3a6639e46f069c768688efadca94. Model claude-opus-5-5, run ac5fa794-ab77-458a-8bab-334cb897d48a.

## Verdict

FIX NEEDED. One medium finding: the newly certified `on each` mode has no concrete stop, cancel or undefined-atom vector. The protocol section's stop, undefined and tamper bullet and its memory and instance bounds are consistent with FR-338's existing payloads.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | For the newly certified unguarded `on each` mode, AC-30's failure-domain clause names no limit, setting, value or cancel source for its `Stopped` outcome, so it pins none of FR-338's typed `LimitReached{limit,value,setting}` and `Cancelled{source}` payloads. It also gives no undefined-letter vector, although the new section makes every model position and every terminal stutter an activation position that reads the atoms. AC-29 states the undefined-atom loci (`BadState` at the origin or a later model state under `["safety",[]]`) only for `on origin`. Add one concrete stop (setting and value), one cancel, and one undefined-atom vector under the `on each` context. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1186 | missing-requirement |

## Assurance Context

No `type: AssuranceProfile` document applies under `spec/`. This is a direct delegated review, with no enforced workflow or human gate inferred.

## Examined scope

```yaml
scope:
- id: FR-1030
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: "unproved slot 1, never `proved`. A past or interval (TP-3) clause over an\n  ordinary model subject, `on origin` or `on each (q: T)` with or without a\n  `when` condition, settles its premise `proved`, `Certified` only when the\n  core checker accepts its FR-338 certificate (a clause the engine decides\n  `Holds` is not `proved` by that alone), and\n  `inconclusive`, `CertificateRejected` when it rejects it. A check that stops\n  on a limit or a cancel settles `incomplete`, and each of these makes the\n  transfer `inconclusive`, `premise-unproved` naming slot 1 with that result."
- id: FR-1030-AC-10
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: 'The broken mapping: `PrevSafeEach` through `ThirdRefinesSpec` with `abstract.initial` naming a snapshot whose `value` is 1 settles the refinement premise `refuted` with `InitialNotAbstract` (FR-144-AC-5), not `transfer-not-preserving`, which is the outcome of a refinement that is not step-for-step (AC-2), so the transfer settles `inconclusive`, `premise-unproved` naming slot 0 with `result` `refuted`, `step_for_step.holds` `true`; exit `22`.'
- id: FR-338
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: 'An infinite-trace TP-1 or TP-3 clause whose `on each (q: T)` activation has

    no `when` condition (`Activation::Each { guarded: false }`, FR-337), checked

    over a model subject, SHALL use the guarded algebra above ("Guarded `on each`

    activation") with the condition read true at every activation position.

    Its state is `["safety", branches]`, and its activation is `"infinite"`. The

    context''s root names the clause node, whose content identity covers the

    `on each` activation, so the root binds it and a key of an `on each` clause

    never equals a key of an `on origin` clause with the same body.'
- id: FR-338-AC-29
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: TP-3 safety branches. Over the Counter subject (values 0 to 3, then the terminal stutter), an infinite-trace `on origin` TP-3 clause returns `Holds` whose certificate keys each explored model state with the `["safety", branches]` body of the bad-prefix monitor state it was read to, and `check_closure` accepts it. For `always (holds(p) implies once[1,1] holds(q))` the state after a position at which `q` holds has body `["safety",[[[[[],"positive"]],[[["0","1"],"history",[[["0","1","0"],["1"]]]]],[],[]]]]`, and after a further position at which `q` is false the offsets are `[]`;
- id: FR-338-AC-30
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: 'Unguarded `on each` TP-3. Over the Counter subject (values 0 to 3, then the terminal stutter), the infinite-trace clause `on each (q: Counter)` with the body `holds(c.value = 1) implies once[1,1] holds(c.value = 0)` returns `Holds` whose certificate keys every product state with activation `"infinite"` and a `["safety", branches]` body, and `check_closure` accepts it, so FR-127 settles `proved`, `Certified`.'
- id: FR-338-AC-31
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: TP-3 over a protocol subject. Over ADR-027 §7's repaired `Fill`, whose counted steps are `attempt(B)` and `attempt(A)` and whose `fork`, `join` and `finish` are uncounted (FR-213-AC-1), the clause `always (holds(k.v = 0) implies eventually[0,2] holds(k.v >= 2))` returns `Holds` with a certificate keyed by `["safety", branches]` windows in the `distance-history` form, whose protocol components are cycle keys, and `check_protocol_closure` accepts it, so FR-127 settles `proved`, `Certified`.
- id: FR-126
  path: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md
  role: examined
  excerpt: "- An infinite-trace TP-1 or TP-3 clause with an unguarded `on each` activation\n  over a model subject SHALL carry the `ClosureCertificate` of every explored\n  product state in its `Holds`, keyed as FR-338 \"Unguarded `on each` TP-1 and\n  TP-3 clause\" states; FR-127 settles it `proved` when the checker accepts it.\n  A TP-3 clause over a protocol subject carries the certificate FR-338 \"TP-3\n  clause over a protocol subject\" states, and a clause with a future interval\n  is `Holds` only when no fair zero-distance cycle keeps its debt open."
- id: TC-525
  path: spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md
  role: examined
  excerpt: "17. FR-338-AC-30: over the Counter subject, check the unguarded `on each`\n    TP-3 clause with the body `holds(c.value = 1) implies once[1,1]\n    holds(c.value = 0)`; offer its certificate for the bodies `holds(c.value =\n    2) implies once[1,1] holds(c.value = 0)` and `holds(c.value = 3) implies\n    once[1,1] holds(c.value = 2)`; remove one reached product state; replace\n    every activation by `\"each\"`; check the guarded variant `when (c.value >=\n    2)`.\n18."
- id: FR-205-AC-10
  path: spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md
  role: context_only
  excerpt: 'Protocol `Echo` over the `Fill` model of AC-1 (`c.v = 0`, which no step of `Echo` writes): channel `ch` `fifo`, capacity 2, `delivery unknown`, carrying `S`; `run sequence { send Snd via ch; receive Rcv via ch of Snd }`; the claim `always holds(c.v = 0)`, which reads no delivery index. After `send`, the queue entry carries the message (instance 0, `Snd`, path `(0)`, ordinal 0) and delivery 0, and the message''s `next-delivery` bound is 1 (state `s1`). From `s1`, `duplicate` then `lose` of delivery 0 reaches `s2`, whose queue holds delivery 1 and whose bound is 2;'
- id: FR-329-AC-10
  path: spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md
  role: context_only
  excerpt: The root thread rests at `Rcv`, enabled at every loop position and taken at none, so over the default package (the payload carrying its scheduler constraints) the lasso is unfair and refuses `invalid_runtime_input`/`invalid-value` naming the root's scheduler constraint, with no truth value. Against the package that declares `scheduling adversarial` the same lasso is fair and `eventually[1,1] holds(p)` is `Completed(false)` at position 0.
- id: FSpec-FR-820
  path: agent-ix/filament-specification@2ccc41ea6c9b3a6639e46f069c768688efadca94:spec/objects/interfaces/FR-820-transfer-item-settlement-request-and-transfer-record.md
  role: context_only
  excerpt: "4. **Premises.** The first slot, in slot order, whose result is not `proved`\n   settles `inconclusive`, `unsettled`, `premise-unproved{premise, result}`,\n   naming the slot index and its FR-331 result value; `unsupported`,\n   `incomplete`, `refuted`, `inconclusive` and `failed` premises all settle\n   this way (FR-380-AC-5)."
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
