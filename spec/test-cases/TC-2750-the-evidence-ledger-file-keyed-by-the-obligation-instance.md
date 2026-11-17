---
id: TC-2750
title: "The evidence ledger file is keyed by the whole obligation instance and tells a superseded predecessor from an unheld one"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-975
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: verifies
---
# TC-2750: The evidence ledger file is keyed by the whole obligation instance and tells a superseded predecessor from an unheld one

## Description

Verify that a run writes the `filament.finite-window-evidence-ledger/1` file
keyed by package, clause, occurrence and trigger, that a correction extends
the entry's `superseded` list, that a clause of the same name in another
package is another obligation, and that `predecessor-not-latest` and
`predecessor-unknown` are told apart in a run and in process.

Scope: FR-975-AC-1, FR-975-AC-2, FR-975-AC-3, FR-526-AC-8, FR-526-AC-24.

## Test Procedure

1. Run FR-526-AC-31's `Original` request; read its `evidence_ledger` member;
   run the repair over that file (FR-975-AC-1).
2. Run the `Original` request in two packages declaring `Seen`, carrying one
   ledger file, then a `Superseding` in the second package naming the first
   package's result (FR-975-AC-2).
3. Run `r1`, `r2` superseding `r1`, then `Superseding(r1)`, a `Superseding`
   naming an identity never issued, and `Superseding(r2)` with changed inputs,
   each over `r2`'s ledger (FR-975-AC-3).
4. In process, repeat step 3's first two corrections against `r2`'s
   `EvidenceLedger`, and run FR-526-AC-24's requests (FR-526-AC-8,
   FR-526-AC-24).

Tests: `tests/it/evidence_ledger_file.rs`
`the_ledger_file_is_keyed_by_the_whole_obligation_instance`,
`a_clause_of_the_same_name_in_another_package_is_another_obligation`,
`a_superseded_predecessor_is_not_latest_and_an_unheld_one_is_unknown`;
`fsl-eval/tests/it/temporal_relation.rs`
`a_predecessor_the_ledger_does_not_hold_is_unknown_not_stale`,
`only_the_ledger_entry_the_evaluator_wrote_for_the_predecessor_grants_authority`.

## Expected Results

- Step 1: one entry with the full key, `trigger` `null`, `lineage` 0 and an
  empty `superseded` list; after the repair, `lineage` 1 and `superseded`
  `[r1.result_identity]`.
- Step 2: two entries differing in `key.package`; the cross-package
  correction refuses `predecessor-unknown` at `evaluate`, member `relation`.
- Step 3: `predecessor-not-latest`, `predecessor-unknown`, then accepted at
  lineage 2.
- Step 4: the same causes in process; FR-526-AC-24's correction refuses
  `predecessor-unknown`.
