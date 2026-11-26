---
id: SR-967182
title: "code-review of PR 4013 measure value-bound claim rules"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/probability_certificate/document/measure.rs, tools/fsl-probability-check/tests/it/value_bounds.rs; PR 4013 head 3bd1330ab159a085d94863acc33b32ad240485e0"
review_set: subset
---

## Summary

Ticket: QSL-3414. Code review with the rust-review lane folded in, of the
measure checker change that moves the initial-state refusal and the
unreachable-activation rejection ahead of the bound/exact split and applies
one tight-action rule (`tight_zero_rejection`) to the exact route and to a
claim-side bound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The interval producer offers `lower` as the claim side for an AtMost certified bound (`mean of F <= θ`, `>= c` quantile; the interval route serves only `fraction` and `shifted` payoffs), which the checker rejects `Malformed` at `kind`, so a true claim settles CertificateRejected; deferred, not this PR | fsl-analyze/src/analyze/exact/measure.rs:656-667 |
| FND-002 | low | Bound documents over a measure base are not range-checked and decided states are checked by containment, not equality, while FR-1042 says decided states carry their payoff and FR-1043 says values lie in range; no false claim; pre-existing, deferred | fsl-replay/src/probability_certificate/document/measure.rs:2439-2443 |

## Verdict

Soundness: `check_value_bounds` now applies every claim-level rule the exact
route applies. The payoff-kind and misplaced-kind rules already ran before
the split. The one-initial-key refusal and the unreachable-activation
rejection now run before it, for both roles. The threshold and the
tight-zero rule run for the claim role. The rule fires only for `strict`
(a `>= c` quantile), whose payoff must be `shifted`. The tight-action search
over an `upper` vector U is sound. With U a pre-fixed point and
U(init) = 0 >= V*(init) = 0, every action tight under V* is tight under U,
and U = V* on its successors. So the V*-tight set reachable from the
initial key is inside the U-tight set: an activation the exact route finds,
the bound route finds too. A counterpart has no threshold and no tight
rule, as FR-1042 "Pairing" states. The partial claim-side `lower` path is
unchanged. No valid document is newly refused except: (a) over several
initial states, where reading one initial key was the unsound part and the
exact route already refuses; (b) an `upper` of initial value exactly 0 over
a claim whose maximum is below 0. Case (b) is what FR-1042 "Pairing" text
prescribes; it is recorded as a spec finding in the spec-review artifact.
The producer runs its vacuity route before any numeric route
(fsl-analyze measure.rs:110), so the moved unreachable-activation rejection
refuses no producer output. Rust: no new panic, unsafe, conversion or
locking surface. The helper reuses existing metering and cancellation.
Tags `#[implements("FR-1042-AC-28")]` on `tight_zero_rejection` and
`FR-1043-AC-10` on `check` resolve under `quire trace` as `implements`.
Mergeable for this ticket; FND-001 and FND-002 are deferred to their own
tickets.

## Scope and evidence

- check (fsl-replay measure.rs:2394-2784): examined, claim-level rule order.
- check_value_bounds (measure.rs:2786-2955): examined.
- tight_zero_rejection / tight_activation (measure.rs:2324-2373): examined.
- finish_measure_interval (fsl-analyze measure.rs:585-726): examined, for
  the claim side the producer offers.
- Four new tests in value_bounds.rs, run green on the reviewed head.
