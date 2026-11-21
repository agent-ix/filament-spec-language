---
id: SR-202611210022442313144108533
title: ears-conformance review of hybrid process exchange stop contract
type: SpecReview
analysis: ears-conformance
scope: agent-ix/filament-spec-language;
  spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md;
  QSL-2234 PR3310
review_set: subset
---

## Summary

Contract-only review of frozen candidate 173f960904042caadd1a2a3e48405549a1b76439 against 7d9dd0ee. Ticket: QSL-2234.

## Verdict

PASS for the scoped contract. Targeted quire validation reports 1/1 documents grammar-clean, zero grammar findings. Semantic inspection finds named actors and concrete responses for provider deadline, observed cancellation, returned errors, admitted bytes and late responses. Multiple actor obligations in explanatory bullets do not leave an ambiguous outcome; separate ACs expose the individual behaviors.

## Assurance context

No applicable AssuranceProfile document exists under this repository's spec tree. No enforced review policy was found. This direct review does not represent human acceptance of an enforced run.

## Scope examined

```yaml
scope:
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: |-
      `fsl_analyze::analyze` SHALL publicly export `HybridProcessExchange` and
        `HybridExchangeError` together, with this interface; the request and
        cancellation arguments and their borrowing lifetimes are unchanged:
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: >-
      The driver's plugin host SHALL enforce the provider's FR-462 transport
        bounds, `deadline_ms`, cancellation grace period and child-process cleanup.
        FSL owns the terminal record: the host reports a stop through
        `HybridExchangeError`, without manufacturing an FR-331 result frame or
        solver answer for it.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: >-
      When the provider's `deadline_ms` expires while its item is open, the
        host SHALL end and reap that process and return `Err(TimedOut)`; it SHALL
        leave the run's caller-owned `Cancel` unchanged. FSL SHALL map that stop
        through its existing `Refusal::Incomplete(IncompleteCause::TimedOut)` to
        the item's `incomplete` record with cause `timed-out`, naming the routed
        backend. Other providers' open items remain eligible to run under the
        same live cancellation handle.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: >-
      When the host observes cancellation of the supplied handle while its
        item is open, the host SHALL apply FR-462's cancel/grace/cleanup rules and
        return `Err(Cancelled { source })`, preserving the handle's actual
        `CancelCause`. FSL SHALL map it through the existing
        `Refusal::Incomplete(IncompleteCause::Cancelled { source })`. A provider
        deadline is `TimedOut`; a caller deadline that cancels the handle is
        `Cancelled { source: Deadline }`.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: >-
      FSL SHALL retain [FR-276](FR-276-cancel-a-lifecycle-operation.md)'s
        cancellation precedence: a caller cancellation observed while the item
        was open determines its `cancelled` record even if the exchange returns
        `TimedOut`, another error or result bytes. A cancellation after the item
        completed does not rewrite its record. The host SHALL return the observed
        cancellation source when cancellation and its provider deadline are both
        observed before it returns.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: >-
      When the exchange returns `Err(ToolFailure(failure))`, FSL SHALL retain
        the existing `failed`, `tool-failure` settlement with that exact defect
        and its existing payload, subject to the same cancellation precedence.
        A stop adds no `ToolFailure` defect, solver outcome, wire cause or catalog
        pair.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: |-
      When the exchange returns `Ok(bytes)`, FSL SHALL read those bytes through
        the existing strict FR-331 stage and hybrid-entry readers and settle the
        admitted answer by the existing dispatch rules. A malformed response
        retains its existing refusal; a genuine provider `incomplete` entry
        retains its existing settlement.
  - id: FR-750
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: |-
      Once a host stop has determined an open item's outcome, the host SHALL
        discard any subsequent response for it. FSL SHALL write no proof,
        counterexample, replay, witness or certificate from a stopped exchange;
        a late result cannot replace its `incomplete` record.
  - id: FR-750-AC-17
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: 'A consumer can import `HybridProcessExchange` and
      `HybridExchangeError` from `fsl_analyze::analyze` and implement the exact
      signature in "Process exchange stops", returning each of
      `ToolFailure(ToolFailure)`, `TimedOut` and `Cancelled { source:
      CancelCause }` without changing the request or cancellation argument
      lifetimes.'
  - id: FR-750-AC-18
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: With two process hybrid backends A and B registered and one item
      explicitly routed to each, A's exchange returns `Err(TimedOut)` while the
      shared caller `Cancel` remains live. A settles `incomplete` with cause
      `timed-out` and backend A; B is called and its valid horizon-covering
      proof settles by the existing dispatch rules. The handle remains live
      after both records are written.
  - id: FR-750-AC-19
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: A process exchange that observes cancellation of its supplied handle
      and returns `Err(Cancelled { source })` gives an `incomplete`, `cancelled`
      item carrying exactly that source, for both `Requested` and `Deadline`.
      Neither becomes `timed-out` or `tool-failure`; caller-deadline
      cancellation is distinct from AC-18's provider deadline.
  - id: FR-750-AC-20
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: An exchange that observes `Requested` cancellation while its item is
      open, then returns `Err(TimedOut)` or valid proof bytes, gives
      `incomplete`, `cancelled` with source `requested` in both cases.
      Cancelling the handle after a completed item's record has been returned
      does not change that record.
  - id: FR-750-AC-21
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: "With a live caller handle, `Err(ToolFailure(Crash))` settles `failed`
      with `tool-failure` defect `crash`; `Err(ToolFailure(FrameLength { limit:
      1048576, value: 1048577 }))` preserves that defect and both byte counts.
      Neither is converted to an incomplete stop."
  - id: FR-750-AC-22
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: With a live caller handle, `Ok(bytes)` containing a valid FR-331 hybrid
      proof or a genuine provider `incomplete`/`timed-out` entry settles by the
      existing readers and dispatch. `Ok(bytes)` containing non-JSON bytes still
      settles `failed`, `tool-failure` defect `malformed-frame`; a structurally
      admitted hybrid entry with a malformed `horizon` retains AC-6's typed
      entry refusal rather than becoming a host stop.
  - id: FR-750-AC-23
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: A fixture process that supplies a valid proof, refuting trace or
      certificate only after the host has determined a deadline or cancellation
      stop never contributes that response to settlement. The affected item has
      exactly one `incomplete` record with its stop cause and no counterexample,
      replay, witness or certificate from that exchange.
  - id: FR-750-AC-24
    path: spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md
    role: examined
    excerpt: A host fixture whose process stays open past the provider deadline
      returns `TimedOut` after ending and reaping it. On caller cancellation,
      both a process that acknowledges cancel and one that ignores it produce
      `Cancelled` with the actual source; the latter is ended and reaped when
      the configured grace period expires. No host stop requires a manufactured
      FR-331 result frame.
  - id: FR-276
    path: spec/functional/FR-276-cancel-a-lifecycle-operation.md
    role: context_only
    excerpt: |-
      When the handle is cancelled during `analyze`, the operation shall settle
        each open item `incomplete` with the cancellation cause, and shall keep
        each item already settled.
  - id: FSpec-FR-462
    path: ix://agent-ix/filament-specification/FR-462
    role: context_only
    excerpt: >-
      A deadline overrun ends the process

      and settles each open item `incomplete` with cause `timed-out`. Items
      routed

      to other providers are unaffected.
  - id: FSpec-FR-331
    path: ix://agent-ix/filament-specification/FR-331
    role: context_only
    excerpt: |-
      A solver that exhausts
      `max_solver_time`, and a provider that overruns `deadline_ms` (FR-462),
      record `incomplete` with `timed-out`, never `limit-reached`; a run the
      caller's frontend ends at its own deadline records `cancelled{source:
      deadline}`.
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Limits

No Cargo command or broad gate was run. No implementation, host cleanup runtime behavior, downstream integration, certificate checker or combined assurance debt is certified by this contract pass. QSL-2234 remains open. Structural validation was limited to the changed FR and these explicit review artifact paths.

Computed matrix limitation: `quire matrix --scope . --format json` produced no matrix output after approximately 60 seconds and was terminated to keep the contract checkpoint bounded. No computed coverage PASS is claimed. The changed FR passed targeted quire structural/grammar validation.
