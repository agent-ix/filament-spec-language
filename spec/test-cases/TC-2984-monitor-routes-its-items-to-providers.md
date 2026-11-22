---
id: TC-2984
title: "Monitor routes its analysis items to providers and reports them in the summary"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: verifies
---
# TC-2984: Monitor routes its analysis items to providers and reports them in the summary

## Description

Verify that `command::monitor` settles the request's `items` through the
registry of `Engines::fsl()` and the started providers, as
`analyze_with_providers` does, before the first record, reports them in the
summary, and leaves the monitored clauses to the session. Scope:
FR-644-AC-8 to FR-644-AC-11, FR-635-AC-5.

## Test Procedure

1. Run FR-644-AC-8's request (`Even` over `n: Integer`, item kind
   `value-validity`) with the recording provider answering `proved`, counting `settle` calls and the events handed before the first.
2. Run FR-644-AC-1's stream with the provider registered and no `items`.
3. Run AC-8's request naming `fsl.absent`; with a provider answering
   `unknown`; with two providers of one `BackendId`.
4. Run AC-8's request with `cancel` fired first; run it through the driver's
   `fsl monitor` with `--provider-config`.

## Expected Results

- Step 1: one `settle` call before any event, holding only `Even`'s items;
  summary `items` and `registry` equal those of the equivalent analyze
  request, the `package` request over the handoff of the monitor's own
  clocked compile,
  each item `proved` `trusted`; events and clause entries equal the run
  without `items`; exit 10.
- Step 2: no negotiation and no provider call; events and summary
  byte-equal to the run with no providers.
- Step 3: `unknown-backend` with no provider call and exit 20;
  `inconclusive` `solver-inconclusive` and, beside the violation, exit 22
  (FR-285 ranking); `invalid-request` naming
  `providers` before any record, with no event.
- Step 4: the item `incomplete` `cancelled{requested}` with no provider
  request; the driver writes step 1's events and a summary with `items`.
