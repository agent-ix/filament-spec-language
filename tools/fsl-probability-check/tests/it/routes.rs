// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 AC-13: the harness's route check, run against this crate's real
//! registrations, which use Routes-table ids.

use fsl_certificate_mutation::{registered_routes, require_routes, HarnessError};
use ix_trace_rs::trace;

/// The three registrations here are R-11, R-12 and R-13. R-14 has none, and
/// `require_routes` names it. (The check that every Routes-table route is
/// registered belongs with the corpus, not here.)
#[trace("FR-1051-AC-13")]
#[test]
fn require_routes_reads_the_real_registrations_by_routes_table_id() {
    crate::fair_mdp::mutation::cyclic_mdp_fair_certificate_rejects_each_single_mutation();
    crate::bounded::mutation::finite_horizon_measure_certificate_rejects_each_single_mutation();
    crate::value_bounds::mutation::value_bounds_certificate_rejects_each_single_mutation();
    let registered = registered_routes();
    let registered = || registered.iter().map(String::as_str);
    require_routes(&["R-11", "R-12", "R-13"], registered())
        .expect("each registration is a Routes-table id");
    assert_eq!(
        require_routes(&["R-11", "R-13", "R-14"], registered()),
        Err(HarnessError::MissingRoute {
            route: "R-14".into()
        })
    );
}
