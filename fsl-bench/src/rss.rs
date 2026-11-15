// SPDX-License-Identifier: AGPL-3.0-or-later
//! The process's peak memory, for the probe's one-shot runs.

/// This process's peak resident memory in KiB, or `None` where the host
/// offers no reader or the reader fails. The figure is process-wide and
/// monotone: a probe run that measures several inputs in one process reports
/// the peak of all of them, so the probe measures one input per process, or
/// sizes in ascending order when it reports a marginal figure.
///
/// Linux reads `VmHWM` from `/proc/self/status`. macOS reads `Physical
/// footprint (peak)` from `vmmap -summary <pid>`: `getrusage` needs `unsafe`
/// or a binding crate this lock does not carry, and this crate forbids
/// `unsafe`. The macOS figure counts compressed pages the resident set omits,
/// so compare macOS runs with macOS runs only.
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
        let output = std::process::Command::new("vmmap")
            .arg("-summary")
            .arg(std::process::id().to_string())
            .output()
            .ok()?;
        let text = String::from_utf8(output.stdout).ok()?;
        text.lines()
            .find_map(|line| footprint_kib(line.strip_prefix("Physical footprint (peak):")?))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// `vmmap`'s size text, a number with a `B`, `K`, `M` or `G` suffix, in KiB.
#[cfg(any(target_os = "macos", test))]
fn footprint_kib(text: &str) -> Option<u64> {
    let text = text.trim();
    let (unit_at, unit) = text.char_indices().next_back()?;
    let number: f64 = text[..unit_at].trim().parse().ok()?;
    let kib = match unit {
        'B' => number / 1024.0,
        'K' => number,
        'M' => number * 1024.0,
        'G' => number * 1024.0 * 1024.0,
        _ => return None,
    };
    // Rounded to a whole KiB; a footprint never reaches the `u64` bound.
    u64::try_from(kib.round() as i128).ok()
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

    #[test]
    fn footprint_text_reads_in_kib() {
        assert_eq!(footprint_kib("  2384K"), Some(2384));
        assert_eq!(footprint_kib("1.5M"), Some(1536));
        assert_eq!(footprint_kib("2G"), Some(2 * 1024 * 1024));
        assert_eq!(footprint_kib("12"), None);
        assert_eq!(footprint_kib("12\u{e9}"), None);
        assert_eq!(footprint_kib("\u{e9}"), None);
        assert_eq!(footprint_kib(""), None);
    }
}
