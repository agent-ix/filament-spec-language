// SPDX-License-Identifier: AGPL-3.0-or-later
//! The process's peak memory, for the probe's one-shot runs.

/// This process's peak resident memory in KiB, or `None` where the host
/// offers no reader or the reader fails. The figure is process-wide and
/// monotone: a probe run that measures several inputs in one process reports
/// the peak of all of them, so the probe measures one input per process, or
/// sizes in ascending order when it reports a marginal figure.
///
/// Linux reads `VmHWM` from `/proc/self/status`. macOS reads `ru_maxrss` from
/// `getrusage(RUSAGE_SELF)` through nix's safe API; Darwin reports that value in
/// bytes, which this function converts to KiB. Compare runs on the same OS.
///
/// `#[string_edge]`: reads the host tool's text report by its field label;
/// it selects no family semantics.
#[ix_trace_rs::string_edge]
pub fn peak_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        status.lines().find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse()
                .ok()
        })
    }
    #[cfg(target_os = "macos")]
    {
        use nix::sys::resource::{getrusage, UsageWho};

        u64::try_from(getrusage(UsageWho::RUSAGE_SELF).ok()?.max_rss() / 1024).ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_rss_is_a_number_on_linux_and_macos() {
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            assert!(peak_rss_kib().is_some_and(|kib| kib > 0));
        }
    }
}
