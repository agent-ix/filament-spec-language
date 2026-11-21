---
id: SR-20261121230215000
title: "Code review (spec language) of PR filament-spec-language#3410"
type: SpecReview
analysis: code-review
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

Ticket: QSL-2751. PR filament-spec-language#3410, frozen head b12cb87d5accc22e26e0f3c944c4d23b455c6820, base 069d7ee184861cdda0f3fe776c3a0310a850dd6f; the review covers the actual diff only (FR-338, FR-126, FR-337, FR-1030, TC-525, TC-2920). The slice is authorized by the 2026-11-21 architecture ruling: TP-3 `on each` for ordinary transfer premises, plus real source and package protocol claim certification on existing paths, with literal protocol transfer still open. Method: dev-tools `/code-review` (0.3.2) applied to a spec-language diff: soundness of the normative statements and executability of the AC vectors against FR-338's own rules. No Rust or production diff, so no rust-review and no gap-analysis. The author's report and the ticket text were read as data. The PR is spec-first, so the absence of an implementation is not a finding; only contradictions and non-executable vectors are. Tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0; FSpec context read at filament-specification origin/main 2ccc41ea6c9b3a6639e46f069c768688efadca94. Model claude-opus-5-5, run ac5fa794-ab77-458a-8bab-334cb897d48a.

## Verdict

FIX NEEDED. Three high findings: an AC-30 rejection vector a conforming checker cannot produce, an AC-31 key form the clause cannot have, and an unsound non-progress phase rule for unguarded `on each` protocol clauses. One medium finding: AC-31's Echo fairness vector is not executable. Clean: the model-subject `on each` past semantics (memory from the authoritative origin, so `PrevSafeEach`, `PrevSafeWhen` and AC-30's guarded vector are true and fail under activation-local past), the FR-337 activation keying correction, AC-29's removal of the no-certificate sentence, the `InitialNotAbstract` versus `transfer-not-preserving` separation (FSpec FR-820 rule 4 names slot 0 first even though slot 1 is also refuted from the value-1 initial state), and the scope boundary: no protocol transfer is claimed, the ticket stays open, and no new API, certificate kind or request member is added.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | AC-30 expects the accepted clause's certificate, offered for the violated `holds(c.value = 2)` and `holds(c.value = 3)` bodies, to be rejected `BadState` at model state 2 (and, per TC-525 step 17, at the terminal stutter) under `["safety",[]]`. Those are different clause nodes, so every key of that certificate carries a foreign context root; FR-338 Behavior says a key whose context root differs from the request's is a foreign context and follows the refused-key rule: `ShapeMismatch` at the least refused ProductStateRef, and a refused key is never used for membership or expansion. A conforming checker can never return the stated `BadState`. Offer a certificate keyed under the violated clause's own context (as AC-29's 'a certificate offered for a clause violated later' does), or expect `ShapeMismatch`. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1186 | wrong-requirement |
| FND-002 | high | AC-31's `Fill` clause `always (holds(k.v = 0) implies eventually[0,2] holds(k.v >= 2))` has no past operator, yet the AC expects its certificate keyed by `["safety", branches]` windows in the `distance-history` form and a tamper that replaces 'a window's `distance-history` bucket'. In FR-338 `distance-history` is the protocol form of the `past` member for general finite past windows and shifted open past windows; future interval debts live in `windows` as `deadline`/`extent`/`offsets`/`shift`. This clause yields a `deadline` window and no `distance-history` entry, so the key claim and the bucket tamper are unrealizable. Either add a general past window (for example a `once[1,2]` operand) to exercise `distance-history`, or key and tamper the `deadline` window. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1187 | wrong-requirement |
| FND-003 | high | The new 'TP-3 clause over a protocol subject' section keys an unguarded `on each` clause under activation `"infinite"` and sends a clause with a future interval through 'Protocol non-progress acceptance', whose auxiliary construction states 'For origin or infinite activation phase is "active" throughout' (only `each` gets waiting/active selection). Read literally, the non-progress search for an unguarded `on each` protocol clause starts the negated root only at the origin, so a fair zero-distance cycle that keeps open the debt of a later activation is never searched and the certificate is accepted: an unsound `proved`, `Certified`. The section must state that an unguarded `on each` clause selects its negated-root activation at any activation position (the `waiting`/`active` phase rule, as for `each` and `each-when`), or otherwise bind the activation-closed form, and AC-31 needs a vector whose open debt starts after the origin. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:684 | wrong-requirement |
| FND-004 | medium | AC-31's `Echo` non-progress vector is not executable as written. FR-205-AC-10's `Echo` is the default package, which carries the root's scheduler constraint: FR-329-AC-10 states the `duplicate`/`lose` loop leaves the root resting at `Rcv`, enabled and never taken, so the loop is unfair by default and fair only in the `scheduling adversarial` variant. The AC's 'when no fairness constraint is resolved' case therefore needs the `scheduling adversarial` package, which it never names, and its 'resolved constraint that the cycle never satisfies' names no constraint (the root scheduler constraint is the natural one). It also leaves 'the certificate `check_protocol_closure` is offered' unidentified, and states the locus as 'the least product state of the cycle', whereas FR-338 places `WitnessFails` at the canonical first product-state locus of the first passing auxiliary component, ordered by complete product-state then auxiliary bytes. Name both packages, the constraint, the offered certificate and the normative locus. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1187 | wrong-requirement |

## Assurance Context

No `type: AssuranceProfile` document applies under `spec/`. This is a direct delegated review, with no enforced workflow or human gate inferred.

## Examined scope

```yaml
scope:
- id: FR-1030
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: "unproved slot 1, never `proved`. A past or interval (TP-3) clause over an\n  ordinary model subject, `on origin` or `on each (q: T)` with or without a\n  `when` condition, settles its premise `proved`, `Certified` only when the\n  core checker accepts its FR-338 certificate (a clause the engine decides\n  `Holds` is not `proved` by that alone), and\n  `inconclusive`, `CertificateRejected` when it rejects it. A check that stops\n  on a limit or a cancel settles `incomplete`, and each of these makes the\n  transfer `inconclusive`, `premise-unproved` naming slot 1 with that result."
- id: FR-1030-AC-8
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: '`PrevSafeEach`, declared in `TRANSFER_DECLARATIONS` with `temporal PrevSafeEach using inf over (c: Spec::Counter) clock "model-steps" on each (q: Spec::Counter) { holds(c.value = 1) implies once[1,1] holds(c.value = 0) }`, `SoonTwoEach`, the same with the body `holds(c.value = 1) implies eventually[0,2] holds(c.value = 2)`, and `PrevSafeWhen`, `PrevSafeEach` with `when (c.value >= 1)` after the parameter, whose activation at position 1 reads position 0 as its previous position, transferred through the step-for-step `ThirdRefinesSpec` with no `premises`, settle slot 1 `proved`, `Certified` by t'
- id: FR-1030-AC-9
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: '`PrevJumpEach`, the template of `PrevSafeEach` with the body `holds(c.value = 2) implies once[1,1] holds(c.value = 0)`, is violated at position 2, whose previous position holds 1, so slot 1 settles `refuted` through its replay, and `PrevStutterEach`, with the body `holds(c.value = 2) implies once[1,1] holds(c.value = 1)`, is violated at the repeated terminal state as AC-5 reads it. Through `ThirdRefinesSpec` each transfer settles `inconclusive`, `premise-unproved` naming slot 1 with `result` `refuted`; exit `22`; neither is `proved`.'
- id: FR-1030-AC-10
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: 'The broken mapping: `PrevSafeEach` through `ThirdRefinesSpec` with `abstract.initial` naming a snapshot whose `value` is 1 settles the refinement premise `refuted` with `InitialNotAbstract` (FR-144-AC-5), not `transfer-not-preserving`, which is the outcome of a refinement that is not step-for-step (AC-2), so the transfer settles `inconclusive`, `premise-unproved` naming slot 0 with `result` `refuted`, `step_for_step.holds` `true`; exit `22`.'
- id: FR-1030-AC-11
  path: spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md
  role: examined
  excerpt: The checked package S4 emits for the unit carries `PrevSafeEach`'s `temporal_clause` node with its `on each` activation (FR-337). Its abstract claim item, settled from that package and not from source, has the value, `Certified` certification and accepted `ClosureCertificate` that AC-8 reads from source, and `planner::settle` over that package with both premises settled in-process (FR-529) writes the same `proved` transfer; a clause root the recompiled package does not hold settles `failed` (FR-126). FR-1031's exclusivity of `package` and `refinement_transfer` is unchanged.
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
- id: FR-205-AC-10
  path: spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md
  role: context_only
  excerpt: 'Protocol `Echo` over the `Fill` model of AC-1 (`c.v = 0`, which no step of `Echo` writes): channel `ch` `fifo`, capacity 2, `delivery unknown`, carrying `S`; `run sequence { send Snd via ch; receive Rcv via ch of Snd }`; the claim `always holds(c.v = 0)`, which reads no delivery index. After `send`, the queue entry carries the message (instance 0, `Snd`, path `(0)`, ordinal 0) and delivery 0, and the message''s `next-delivery` bound is 1 (state `s1`). From `s1`, `duplicate` then `lose` of delivery 0 reaches `s2`, whose queue holds delivery 1 and whose bound is 2;'
- id: FR-329-AC-10
  path: spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md
  role: context_only
  excerpt: The root thread rests at `Rcv`, enabled at every loop position and taken at none, so over the default package (the payload carrying its scheduler constraints) the lasso is unfair and refuses `invalid_runtime_input`/`invalid-value` naming the root's scheduler constraint, with no truth value. Against the package that declares `scheduling adversarial` the same lasso is fair and `eventually[1,1] holds(p)` is `Completed(false)` at position 0.
- id: FR-213-AC-1
  path: spec/functional/FR-213-measure-protocol-intervals-in-counted-steps.md
  role: context_only
  excerpt: 'Over ADR-027 §7''s repaired `Fill` (`TwoPre: self.v <= 1`), on the behaviour that takes `B` before `A`, positions 1 (`fork`), 4 (`join`) and 5 (`finish`) carry `protocol` anchors and positions 2 and 3 the anchors of `setTwo` and `inc`; the counted steps are `attempt(B)` and `attempt(A)`, and `eventually[0,2] holds(k.v = 3)` holds at position 0. `eventually[0,1] holds(k.v = 3)` does not hold at position 0.'
- id: FR-144-AC-5
  path: spec/functional/FR-144-request-and-settle-a-refinement-item.md
  role: context_only
  excerpt: Two `CasRefinesCounter` requests that differ only in the abstract subject's initial snapshots (`value` 0 and `value` 1) have different obligation identities; the second settles `refuted` with `InitialNotAbstract`.
- id: FSpec-FR-820
  path: agent-ix/filament-specification@2ccc41ea6c9b3a6639e46f069c768688efadca94:spec/objects/interfaces/FR-820-transfer-item-settlement-request-and-transfer-record.md
  role: context_only
  excerpt: "4. **Premises.** The first slot, in slot order, whose result is not `proved`\n   settles `inconclusive`, `unsettled`, `premise-unproved{premise, result}`,\n   naming the slot index and its FR-331 result value; `unsupported`,\n   `incomplete`, `refuted`, `inconclusive` and `failed` premises all settle\n   this way (FR-380-AC-5)."
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
| FND-002 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
| FND-003 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
| FND-004 | fixed | Verified at fix head 40c49fb70a21cb4ea2a904c9b206f6e9d37df15a: the fixed text is quoted in the dispositions comment's after_excerpt. |
