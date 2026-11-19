---
id: TC-2724
title: "An unpublished option settles unknown-option"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-967
    type: verifies
---
# TC-2724: An unpublished option settles unknown-option

## Description

Verify that an item naming an option its named backend does not publish settles `invalid-request` `invalid_capability`/`unknown-option` on analyze, that an unknown backend wins over it, and that a published option reaches the provider.

Scope: FR-967-AC-1, FR-967-AC-2.

## Test Procedure

1. Analyze FR-967-AC-1's two items.
2. Analyze an item naming `fsl.absent` with options, then items naming the test plugin with `depth` and with `depth` and `width`.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs` `an_unpublished_option_settles_unknown_option`, `an_unknown_backend_wins_and_a_published_option_reaches_the_provider`.

## Expected Results

- Step 1: the first item `unknown-option`, `request_index` 0, `backend` `en-1`, `option` `alpha`, exit 20; the second byte-equal to the request without the first.
- Step 2: `unknown-backend`; the provider receives `depth`; `unknown-option` naming `width` and no request sent.
