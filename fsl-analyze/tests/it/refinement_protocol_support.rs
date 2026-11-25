// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared protocol refinement fixture source.

pub(crate) fn cas_twice(peek: bool, lost_update: bool) -> String {
    let retries = if lost_update {
        ""
    } else {
        "step Impl::Counter::retryA -> stutter;\n\
         step Impl::Counter::retryB -> stutter;\n"
    };
    let peek_row = if peek {
        "step Impl::Counter::peek -> stutter;\n"
    } else {
        ""
    };
    format!(
        "profile p = \"filament.protocol.finite-global/v1\";\n\
         protocol Twice using p over (c: Spec::Counter) on origin {{\n\
         role w on Spec::Counter; scheduling adversarial;\n\
         run sequence Main {{\n\
         attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) {{ true }};\n\
         attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) {{ true }};\n\
         }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n\
         refinement CasTwice using inf abstract Spec::Twice concrete Impl {{\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::Counter {{ value = self.value; }}\n\
         step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
         step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
         step Impl::Counter::beginA -> stutter;\n\
         step Impl::Counter::beginB -> stutter;\n\
         {retries}{peek_row}}}"
    )
}
