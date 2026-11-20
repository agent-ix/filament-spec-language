# QSL-2093 lifecycle boundary slice

This slice adds behavior tests for caller-owned compile limits and real lifecycle cancellation under current FR-255, FR-276 and FR-277. It changes tests and their registration only.

- A compiler setting changes only its own bound, including zero and its published default. The collection-maximum setting accepts its specified `u32::MAX` bound; an emitter refuses settings it does not own.
- S1 input bytes count UTF-8 bytes, with the exact inclusive bound admitted.
- The five-node `1 + 1 + 1` body reaches a typed S3 node limit at four, names its setting and authored locus, and checks at five.
- E4 denies one byte below its actual canonical package length, returns no partial package, and emits identical complete bytes at the exact bound.
- Parse, check and package cancellation starts at actual observed charges, retains Requested or Deadline, and produces no partial stage value.
- FR-276-AC-2 uses an actual 200,000-declaration source and a synchronized second-thread Deadline at charge 1,000. No clock or sleep determines the outcome.

The 200,000-declaration case reached a real defect: checking returns `Cancelled(Deadline)` but continues to 200,666 observed charges. QSL-2720 owns the implementation fix and blocks QSL-2093. The desired at-most-1,001-charge oracle remains strict and ignored; it is not passing coverage. Seven other new tests passed. The original actual failure receipt is retained without a repeated probe.

These tests do not claim that every historical family in QSL-2093 is newly covered. In particular this slice adds no fabricated SMT certificate, terminal record, or successful proof; the remaining certificate, result, plugin and lifecycle families remain outside this delivered slice. Existing peer-tested cases are neither rerun nor duplicated for trace tags. The matrix is not regenerated.

Each test must exercise production setters, parser, checker or emitter behavior and assert an independently specified bound, typed outcome, whole artifact or cancellation boundary. No source-grep claim or synthetic successful lifecycle outcome is used.
