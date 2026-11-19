---
id: TC-2601
title: "A tl_lowering request selects its correspondence"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-2601: A tl_lowering request selects its correspondence

## Description

Verify the `tl_lowering` member's default and named correspondences, and the
unsupported record when the clause does not have the named reading. Scope:
FR-931-AC-1, FR-931-AC-2, FR-931-AC-11, FR-931-AC-12.

## Test Procedure

1. Run `Low` with `{"clause": "Low"}` and with `"correspondence": "mltl.closed-trace/v1"`.
2. Run `Low` with `mltl.online-prefix/v1` and with `mltl.origin-complete-history/v1`.
3. Run the past-only clause with no `correspondence`.
4. Run the timed form of `Low`: over the ConfigVersion model timed by `time Config dense unit ms`, under `filament.temporal.timed/v1` with no `clock` member, body `always [0 ms, 3 ms] holds(c.versionNumber <= 2)`.
5. Run `Low` written with extra spaces and grouping, and FR-931-AC-11's operator and text leaves, from source and from the unit's checked package with the source removed; read each `atoms` entry.
6. Run FR-931-AC-12's enumeration and `allInstances` leaves under the alias `Config`, under `K`, and from each checked package with the source removed.

Test: `tests/it/request_surface_s1_16.rs` `a_tl_lowering_request_selects_its_correspondence`.

## Expected Results

- Step 1: `supported`, `mltl.closed-trace/v1`, `G[0,3] p0`, exit 0; byte-equal records.
- Step 2: `supported` under `mltl.online-prefix/v1`; `unsupported`, `classified` `mltl.closed-trace/v1`, `dimensions` `["direction"]`, exit 21.
- Step 3: `supported` under `mltl.origin-complete-history/v1`.
- Step 4: `unsupported`, no `classified`, no `formula`, exit 21.
- Step 5: each atom text exactly as FR-931-AC-11 states; the source and package routes give byte-equal `atoms`.
- Step 6: the two atom texts FR-931-AC-12 states, byte-equal across both aliases and both routes.
