---
id: TC-2660
title: "Analyze builds the complete provider request and the transport carries it unchanged"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-950
    type: verifies
---
# TC-2660: Analyze builds the complete provider request and the transport carries it unchanged

## Description

Verify that analyze builds one complete FSpec FR-331 request per provider
backend after routing and negotiation, and that the request's members come
from the admitted package, the routed items, the negotiated manifest entry
and the provider limits. Scope: FR-950-AC-1, FR-950-AC-2, FR-950-AC-3,
FR-950-AC-4.

## Test Procedure

1. Register a recording process provider and analyze FR-635-AC-1's request; capture the `ProviderRequest` passed to `settle`.
2. Repeat with `provider_limits` setting `deadline_ms=250` and `max_frame_bytes=1048576`; serialize the request and validate it against the FR-331 schema.
3. Route two items to the provider and one to `en-1`.
4. Analyze an item routed to the provider over a package `package` refuses to emit.

Test: `fsl-analyze/tests/it/provider_request_seam.rs`.

## Expected Results

- Step 1: one call, after negotiation; the package decodes to the admitted package; the items, manifest entry and `deadline_ms` 600000 are as FR-950-AC-1 states.
- Step 2: the two set limits and five defaults; schema-valid, and byte-equal to the frame body.
- Step 3: one request holding both items in request order; the `en-1` item is absent.
- Step 4: `unsupported` `unsupported-requested-capability`; `settle` is never called.
