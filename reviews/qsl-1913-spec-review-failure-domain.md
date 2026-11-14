---
id: SR-2026111414035504
title: "failure-domain review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. This review checks soundness, identity and
purity in the three result-preserving claims.

The hash-indexed seen set is sound. A bucket hit must be confirmed by
comparing the full bytes, the hash never reaches an output or an order, and
AC-20's constant-hash test would catch coalescing. Unequal states cannot be
coalesced.

The packed layout is sound. It is a bijection on the values the checked
types admit, equality of packed forms is equality of key bytes, and values
outside the layout fall back to the canonical bytes. Canonical bytes stay the
identity in every output.

The worker rules are consistent across FR-101, FR-126 and FR-191. They
charge in merge order, discard any expansion the merge never reaches, cancel
to a prefix, and refuse `workers` 0.

The solved-assignment rule has two defects: FND-001 and FND-002.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-120 says "The generator MAY solve" an eligible assignment, then says the count "is a function of the model, the state and the request alone, never of the generator's strategy". Under MAY, a generator that solves and one that enumerates count different numbers for the same application, and stop at different `max_candidates` points. The engines of FR-120, FR-126 and FR-141 may also disagree. AC-29 to AC-31 and AC-33 require solving ("never generates the 10^4"), so a generator that enumerates fails them while meeting the FR text. Eligibility is syntactic and the value is deterministic, so the fix is SHALL. | FR-120:687-723, FR-120-AC-29, FR-120-AC-30, ADR-016 G-4 | wrong-requirement |
| FND-002 | medium | The Invariant assumes every skipped candidate's equation evaluates `Completed(false)`. Eligibility does not exclude frames that also `deletes` or `creates`. On a candidate whose receiver was deleted, `self.f` has no post value, so enumeration records an `Undefined`/`Refused` clause there (a `ContractUndetermined` finding) that the solver never generates. Eligibility also does not require that "`f` equals `v`" is the same test as QSL `=` on `f`'s type. Restrict eligibility to frames with only `modifies` and to types whose equality is decided by canonical bytes, or state why findings stay equal. | FR-120:694-712, FR-120:724-733 | wrong-requirement |
| FND-003 | medium | Workers weaken a guarantee without saying so: budgets bound the work done. Nothing limits how far a worker may expand ahead of the merge, so `max_states` 5 can spend unbounded time and memory on expansions it later discards. FR-191 Budgets still says a test stops "before it draws the next sample", and FR-101 still says the engine "calls `successors` once per expanded state". Both are false for physical calls at `workers` > 1, and neither sentence is amended. Bound the look-ahead (for example at most `workers` states or samples past the merge point) and amend both sentences. | FR-101:300-315, FR-101:361, FR-191:74, FR-191:95-100 | missing-requirement |
| FND-004 | medium | `explore_request<S: TransitionSystem>(system: &S, ..., workers: usize, ...)` adds threads but no `S: Sync` or `Send` bounds on `S::State` and `S::Finding`, and `TransitionSystem` declares none. As specified, `&S` cannot be shared across threads. QSL-1835 notes that the liveness and partial-order-reduction systems hold `RefCell` state. | FR-101:104-170 | missing-requirement |
| FND-005 | low | The seen-set paragraph says buckets use "a hash of their full bytes", confirmed "by comparing the full bytes". The packed-state paragraph lets the engine store states packed. A packed seen set that hashes and compares packed forms keeps identity (the bijection) but breaks the first paragraph's wording. Allow any injective encoding of the key bytes. | FR-101:266-289 | wrong-requirement |

## Verdict

Not mergeable until FND-001 is fixed (change MAY to SHALL). FND-002 to
FND-004 are real gaps that two implementers would trip on. The identity
guarantees asked about hold: hash coalescing (no), packed identity
(canonical bytes), and worker-independent stop points.
