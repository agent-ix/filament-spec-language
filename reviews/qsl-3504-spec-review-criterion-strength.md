---
id: SR-96742
title: "spec-review/criterion-strength of FR-1051 certificate checker trusted base"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md; PR4035 47b83d02c1717277360e3b7794ea6851d200e3d1"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3504. Independent review at head 47b83d02c1717277360e3b7794ea6851d200e3d1 (merge base 9842eb939a). Read-only; no cargo was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-5 cannot be met: semantic type items in X-27, X-28 and X-33 produce no finding because AC-3 and the Behavior bullet let producer code name any checker type item | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:375 |
| FND-002 | high | AC-5 fails on main: X-40 lists check_product_closure, which takes no producer-built value | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:255 |
| FND-003 | high | AC-11 cannot be met: the definitional reference refuses everything outside a fragment that has no construct for many routes | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:381 |
| FND-004 | medium | Zone and task-product certificates have a canonical byte form but no byte or source check entry for the closure to call | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:285 |
| FND-005 | medium | The Routes table omits check entries that accept proofs or refutations, so they need no mutation fixture and no false claim | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:258 |
| FND-006 | medium | The checker half of the false-claim criteria passes vacuously when the producer writes no certificate | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:381 |
| FND-007 | medium | Nothing fails when the arch-lint configuration differs from the requirement tables | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:371 |
| FND-008 | medium | Enumerating memoryless deterministic schedulers does not refute a bounded claim over some scheduler | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:343 |

### FND-001 detail

Terms exclude AbstractProtocolSystem, AbstractClass, HybridModelReader, HybridBox, HybridInterval and HybridTri from the document codec. The Behavior bullet allows producer code to name "a type item of checker code", and AC-3 says naming a checker type item passes and reports functions only. X-27, X-28 and X-33 list those types as items, and AC-5 fails a named item that matches no finding. No implementation satisfies AC-3 and AC-5 together; an implementer who follows AC-3 also never reports producer use of the checker interval and reader types.

### FND-002 detail

ProductClosureRequest has package, clause: ProductClosureClause (CheckedHyperClause or CheckedModelRelation), subjects: &[ModelSubject] and meter (certificate/product.rs:169-174, 479-496). None is a type of "No producer objects"; the evaluator is made inside the entry from request.package. AC-4 yields no finding, so AC-5 reports the X-40 item check_product_closure as stale. Remove it from X-40.

### FND-003 detail

AC-11 demands a corpus entry for every route with the truth computed by the reference. The fragment has no protocols or memory models (protocol closure, components, trace and assumption witness routes), no refinement or simulation relation (refinement, simulation, refinement SMT, refinement replay), no model relation, no fairness, no long-run average (dtmc-long-run, weighted), no deadlock or time-lock freedom, no pre/post/simulation contract claim (operation contract), no on-each instances. The reference SHALL refuse them, so those routes can never have an entry. Extend the fragment or list the routes excluded and what covers them.

### FND-004 detail

check_zone_certificate and check_free_delay_zone_certificate take a typed ZoneCertificate (certificate/zone_certificate.rs:901); the bytes are read by the codec from_wire. The bullet requires a closure that calls "the route byte or source check entry", which does not exist for any zone route, including two of the first registrations named in AC-9. Say the closure may decode with the codec and call the typed entry.

### FND-005 detail

Missing: check_components_over_stutter_closure (settles proved, model_check/settle.rs:425-428), check_match_witness (gates an SMT hyper proved), replay_timed_refinement_trace, replay_free_delay_refinement_trace, replay_model_relation_tuple, static_replay, check_claim_exact_reachability, check_probability_certificate_package. A skipped rule on one of these sibling paths (design mechanism C) passes AC-7 to AC-12. Require every Check entries row to belong to a route and fail otherwise.

### FND-006 detail

A producer that correctly refutes or declines a false claim writes no certificate, so "the check entry does not accept the certificate a producer writes for it" is never exercised and a checker such as the one in QSL-3414 passes. The design class j is a self-consistent certificate for a false claim. AC-12 "no check entry accepts a certificate for any of them" quantifies over all certificates and no test decides it. Require, per entry, a committed or forged self-consistent certificate that the check entry rejects.

### FND-007 detail

"reads a declared configuration whose sets are exactly this requirement" is asserted by no fixture: AC-1 to AC-6 test the check against whatever the configuration holds. An exception row or codec function added only to the configuration silences a new sharing and passes AC-5 and AC-6. Require a test that reads these tables and compares them with the configuration, or make the configuration the one statement and have this requirement refer to it.

### FND-008 detail

For a horizon-bounded reachability objective the optimum can need a scheduler that depends on the steps left. The bullet limits enumeration to "reachability and expected-reward objectives" without saying unbounded, so a bounded some-scheduler claim that is true can be computed false and seed the corpus with a wrong oracle value. Say unbounded, and decide bounded ones over all history-dependent schedulers or refuse them.

## Verdict

Changes requested. AC-5 and AC-11 cannot be met as written. AC-2, AC-4, AC-6, AC-7, AC-8, AC-10, AC-13 and AC-14 can fail and can be met. The Jev client was not used; this judgement is manual.

## Scope and evidence

- FR-1051-AC-1: examined
- FR-1051-AC-2: examined
- FR-1051-AC-3: examined
- FR-1051-AC-4: examined
- FR-1051-AC-5: examined
- FR-1051-AC-6: examined
- FR-1051-AC-7: examined
- FR-1051-AC-8: examined
- FR-1051-AC-9: examined
- FR-1051-AC-10: examined
- FR-1051-AC-11: examined
- FR-1051-AC-12: examined
- FR-1051-AC-13: examined
- FR-1051-AC-14: examined
