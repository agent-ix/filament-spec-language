# QSL-2749 focused receipts

Implementation reviewed: `52bc8d9b58734bd9d7fdf72263eb7f2629d0796c`.

Four distinct focused cases passed using `cargo test --locked -p fsl-replay --lib <filter> -- --test-threads=1`:

| Filter | Result |
| --- | --- |
| `a_finite_window_` | Closed false-guard inactivity and open-scope pending passed; the new guard case initially failed fixture admission because its profile was event-position. |
| `a_finite_window_undefined_on_each_guard_retains_its_cause_and_locus` | After correcting the fixture profile, exit 0; 1 passed, 0 failed. Tests open and closed scopes. |
| `timestamped_incomplete_guard_scope_remains_unknown` | Final exit 0; 1 passed, 0 failed. An earlier additional activation-axis assertion exposed an unrelated gap and was removed; the test retains pending/no-truth assertions. |

The unchanged passing cases were not repeated. Initial failures remain in the coder's local receipts. Debug information and incremental compilation were disabled; one ticket target directory was retained. No full gates were run or claimed.

Independent code/Rust review SR-1542 and scoped gap analysis SR-1543 passed with no findings at the reviewed implementation head. The reviewer reported both artifact validations and readbacks exiting 0. Review evidence was accepted without repeating tests, builds or reviews. This evidence-only commit changes no implementation or tests.
