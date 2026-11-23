---
id: SR-967175026302197768900474181941935084782
title: "ears review of PR #3550 every-scheduler long-run proof profile"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3190-check-every-scheduler-long-run-proofs.md, spec/test-cases/TC-3191-refute-a-long-run-fraction-over-every-scheduler.md (git diff origin/main...900ddd14)"
review_set: subset
---
# ears review of PR #3550

## Summary

Ticket: QSL-2924 (item 3). PR: filament-spec-language#3550 at 900ddd1477c116b0c5ad487671b073327f60e92d. EARS conformance of the new SHALL statements in FR-201's 'Long-run claim profile admission' and 'Closed MDP long-run document profile'.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Admission SHALL names a document profile as the actor that 'decides and certifies', and leaves the admitted profile set open ('any profile FSpec FR-407 admits') | spec/functional/FR-201-check-a-probability-certificate.md:989-1007 |

## Detail

**FND-001** (low, ambiguous, confidence medium): The admission section says 'every long-run document profile below SHALL therefore admit the claim ... and SHALL decide and certify it identically'. A document profile neither decides nor certifies: EN-5 decides, the codec admits and the checker certifies. The EARS subject should be the system element that does each. The section also admits 'any temporal profile FSpec FR-407 admits for it, other than the timed profile', but FR-407 does not list profiles for the long-run form. The profiles admitted over a model subject are ADR-018 section 1's event-position false-extension and infinite-trace (ADR-026:62-64). Fix: name those two profiles so the admitted set is closed, and give each SHALL its actor (codec admission, checker, EN-5 settlement).

## Verdict

The MDP profile's SHALL statements are well formed (codec SHALL admit, checker SHALL derive/require/check/reject). Only the admission section's actor and its open profile set need tightening.
