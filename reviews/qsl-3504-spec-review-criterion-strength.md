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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | high | Any new sharing can be excused by adding a data entry under an existing row; AC-7 to AC-9 never compare an entry with its row's code | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:390 |
| FND-010 | high | Base rows B-9 to B-12 let the data bless any function, and no criterion tests "derives no member value" or "data items only" | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:136 |
| FND-011 | medium | AC-2 and AC-5 force a semantic model, and an unresolved finding can never be excused, so AC-7 depends on a resolution rate nobody has measured | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:108 |
| FND-012 | medium | Nothing checks that the certificate of a proof-route entry is self-consistent, so a broken certificate satisfies AC-16 and AC-17 | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:399 |
| FND-013 | medium | Replay-route entries need a true claim, but the reference has no stated way to decide an unbounded universal claim true | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:352 |
| FND-014 | low | A check entry missing from the data goes unnoticed when only settling code calls it, and the producer-object type list is pinned by no criterion | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:393 |

### FND-009 detail

AC-8 checks only that the item exists and that the row, route or codec class exists. AC-9 compares class paths, base rows, row IDs and route IDs. Nothing requires the entry's item to be in the row's "Code it concerns", and nothing stops a row gaining entries. A new producer call labelled P-1, or a new shared function labelled S-1, passes every criterion, so "No other exception is allowed" cannot fail. Require AC-8 to fail an entry whose item or site is outside its row's code and whose rule differs from the row's, and give each row a committed entry count in this table that AC-9 compares and that only falls.

### FND-010 detail

B-11 and B-12 cover 27 modules, including producer files such as model_check::counterexample; a function recorded under either row becomes trusted base, outside every sharing rule, permanently and without a ticket. B-10 says no condition-testing function and B-9 is whatever the data records. No AC fails when the data adds a deriving function to any of them. The earlier text failed a function under X-41 (old AC-6); that check is gone. Restore it for B-10, and for B-9, B-11 and B-12 put the function names, or a per-row count that only falls, in this table where AC-9 compares them.

### FND-011 detail

tools/arch-lint is syn only. Typed resolution needs rust-analyzer or compiler internals; in-process ra_ap crates or rustc_driver are heavy and the toolchain is pinned stable 1.98. The lightest route is an external rust-analyzer SCIP index read by the lint, with syn listing call sites and a missing occurrence treated as unresolved. An unresolved finding matches no exception row, so one call rust-analyzer cannot infer blocks AC-7. State that an unresolved call is treated as naming every workspace function of that name outside the base (still fails closed, and can be excused by a normal entry), allow the SCIP route, and measure the unresolved count before building.

### FND-012 detail

The Behavior asks that the certificate meet the checker's structural rules, but the criteria only ask that it be rejected. An empty or malformed certificate is rejected by any checker, including one with the defect the entry exists to catch. Require the criterion to name the rejection each entry expects (a semantic rule, not decoding, identity or shape) and fail on any other.

### FND-013 detail

The decision bullets cover bounded horizons, unbounded probability and reward over a chain, a FALSE every-scheduler claim by a given scheduler, some-scheduler claims and timed continuations. An unbounded always, until, eventually, hyper, deadlock-freedom or every-scheduler claim that is true on a finite model has none, and covered replay routes (R-19, R-20, R-46 to R-55) must take their truth from the reference. Say how (exhaustive reachable-graph and cycle search), or require those entries to use horizon-bounded claims.

### FND-014 detail

AC-10 detects an omitted check entry only through a producer call or an empty route; a second entry on a route called by settling code alone drops out of the data, and of the mutation obligation, silently. AC-4 is fixtures only and AC-9 does not compare the "No producer objects" list, so the data may omit a type from it. Extend AC-10 to settling-code calls and AC-9 to that list.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at 2483104d1e |
| FND-002 | fixed | Checked at 2483104d1e |
| FND-003 | fixed | Checked at 2483104d1e |
| FND-004 | fixed | Checked at 2483104d1e |
| FND-005 | fixed | Checked at 2483104d1e |
| FND-006 | fixed | Checked at 2483104d1e |
| FND-007 | fixed | Checked at 2483104d1e |
| FND-008 | fixed | Checked at 2483104d1e |
