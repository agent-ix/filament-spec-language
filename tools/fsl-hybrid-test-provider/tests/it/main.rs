// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750's process providers: `Engine::hybrid_process` over a real child
//! process, the scripted `fsl-hybrid-test-provider` executable, through the
//! command adapter's analyze entry.

mod exchange_stop;
mod host;
mod process_provider;
mod support;
