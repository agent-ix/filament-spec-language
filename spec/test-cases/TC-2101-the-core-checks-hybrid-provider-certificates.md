---
id: TC-2101
title: "The core checks hybrid provider certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-751
    type: verifies
---
# TC-2101: The core checks hybrid provider certificates

## Description

Verify the exact interval check of hybrid certificates, rule by rule with the rule name and pointer of every rejection, the structural refusals, and the check's effect on certification and refutation. Scope: FR-751-AC-1 to FR-751-AC-8. Rule-level tests live in `fsl-replay/src/hybrid_replay/certificate/tests.rs` and end-to-end tests in `fsl-analyze/tests/it/hybrid_requests.rs`, tagged `#[trace("TC-2101", "FR-751-AC-n")]`.

## Test Procedure

1. Check FSpec FR-652-AC-1's two `inductive-enclosure` certificates.
2. Check a `flowpipe` with a Picard violation; read FSpec FR-652-AC-3's trace with its accepted witness and its two straddling witnesses.
3. Check a certificate under a one-evaluation work limit.
4. Check the `inductive-enclosure` certificates of FSpec FR-652-AC-7 and FR-652-AC-8.
5. Check the `flowpipe` certificates of FSpec FR-652-AC-5 and FR-652-AC-9 to FR-652-AC-14.
6. Check the `validated-witness` certificates of FSpec FR-652-AC-15 and FR-652-AC-16.
7. Read each structural mismatch of FSpec FR-652-AC-17 through `read_entry` and through a replay request.
8. Check FSpec FR-652-AC-1's accepted certificate and FR-652-AC-3's nine-step witness under work limits that stop them.

## Expected Results

- Step 1: `Accepted` and `Certified`; `Rejected` at rule (a) and the `Moving` box pointer, `Uncertified`.
- Step 2: `Rejected` at the step's pointer; `refuted` and recorded; `inconclusive` twice, each rejected `witness-violation`.
- Step 3: `Rejected { rule: work-limit }`.
- Step 4: each outcome, rule and `at` the FSpec criteria state.
- Step 5: each outcome, rule and `at` the FSpec criteria state; the `[0, 20]` jump step accepted.
- Step 6: each outcome, rule and `at` the FSpec criteria state; `GrowJump`'s decisive reading at position 1, time 4.
- Step 7: a certificate refusal with the stated code and pointer, `invalid_result` from `read_entry` and `invalid_runtime_input` at `/replay/certificate/...`; never a rejection or `internal_fault`.
- Step 8: `Rejected { rule: work-limit, at: /certificate/boxes/Moving }`; the witness `Rejected { rule: work-limit }` and its item `incomplete`.
