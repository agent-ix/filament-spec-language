---
id: TC-3209
title: "An on-each reachability document is checked part by part"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: verifies
---
# TC-3209: An on-each reachability document is checked part by part

## Description

Each part of the `on-each-reachability/v1` `Ratio` document is an `Exact` document checked by its own conditions, ranks and coverage; forms outside the profile stay unsupported.

Scope: FR-201-AC-78, FR-201-AC-79, FR-201-AC-80.

## Test Procedure

Over AC-76's snapshot, change `numerator` to `1/2` at `m`'s initial key; change `denominator` to `1/2` there; set the `numerator` rank there to `0`; omit, repeat and add a row in each part; offer the document under every scheduler; offer a cyclic, a fair and a timed on-each claim. Test: `tools/fsl-probability-check/tests/it/on_each.rs`.

Tag the tests `#[trace("TC-3209", "FR-201-AC-78")]`, `#[trace("TC-3209", "FR-201-AC-79")]` and `#[trace("TC-3209", "FR-201-AC-80")]`.

## Expected Results

- `InequalityFails` at `m`'s initial key with `part` `numerator`; `InequalityFails` there with `part` `denominator`; `RankBroken` there with `part` `numerator`.
- Each row fault: `Malformed` at `CertificateMember` `numerator` or `denominator`, naming the faulty part.
- Under every scheduler: `Malformed` at `mode`; the out-of-profile forms unsupported with no accepted certificate.
