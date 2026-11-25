// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared bounded-protocol fixture helper.

use crate::fill::{contracts, protocol};

pub(crate) const ECHO_RUN: &str = "sequence Main { send Snd via ch as (s: Boolean) { s }; \
                       receive Rcv via ch of Snd as (r: Boolean) { r }; }";

pub(crate) fn echo_body(scheduling: &str) -> String {
    let members = format!(
        "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   {{ true }} delivery unknown capacity 2 overflow block; {scheduling}"
    );
    contracts("self.v <= 1") + &protocol("Echo", &members, ECHO_RUN, "d")
}
