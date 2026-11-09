//! The memory budget of `al-runner test` (ADR-066 amendment 9; `al_testrun::runtime::memory` uses it): the
//! smallest of the machine's free memory (`MemAvailable`), the `memory.max` of the process's cgroup and its parents
//! (cgroup v2: a container or a `systemd-run` scope), and `--max-memory`. Reading them is the front end's job
//! (ADR-006): the run gets the number and a way to read the resident memory.

use std::sync::Arc;

use al_testrun::runtime::memory::MemoryBudget;

/// `--max-memory SIZE`: a whole number with an optional unit `K`, `M`, `G` or `T` (powers of 1024, `B`, `iB` and
/// lower case also read), bytes without one.
pub fn parse_size(text: &str) -> Result<u64, String> {
    let t = text.trim();
    let digits = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    let (number, unit) = t.split_at(digits);
    let unit = unit.trim().to_ascii_uppercase();
    let unit = unit.trim_end_matches('B').trim_end_matches('I');
    let shift = match unit {
        "" => 0,
        "K" => 10,
        "M" => 20,
        "G" => 30,
        "T" => 40,
        _ => return Err(size_error(text)),
    };
    let n: u64 = number.parse().map_err(|_| size_error(text))?;
    n.checked_mul(1 << shift)
        .filter(|&b| b > 0)
        .ok_or_else(|| size_error(text))
}

fn size_error(text: &str) -> String {
    format!("--max-memory takes a size such as 6G or 800M, not `{text}`")
}

/// The budget of a run: the smallest limit that applies, with where it comes from. `hard`: the limit is a hard one
/// (ADR-066 amendment 10, decision 6; a plain test run): a run that cannot fit stops, its tests not reached are not
/// run, and the command exits with code 2. `serve` and `debug` go on with one thread.
pub fn budget(max_memory: Option<u64>, hard: bool) -> Option<MemoryBudget> {
    let (limit, source) = smallest(&[
        (max_memory, "--max-memory"),
        (cgroup_limit(), "cgroup memory.max"),
        (mem_available(), "free memory"),
    ])?;
    Some(MemoryBudget {
        limit,
        source: source.to_string(),
        rss: Arc::new(resident),
        hard,
    })
}

/// An explicit limit of the process (`--max-memory` or the cgroup's `memory.max`, the smaller) for the work after the
/// tests (the profile write, al-runner issue 6924), with the resident memory to compare it with. Free memory is not
/// one: the work after the tests holds what the run already holds.
pub struct HardLimit {
    pub limit: u64,
    pub source: &'static str,
    rss: fn() -> Option<u64>,
}

impl HardLimit {
    /// The resident memory is within a thirty-second of the limit (at least 32 MB): the streamed write needs a few MB.
    pub fn reached(&self) -> bool {
        (self.rss)().is_some_and(|now| now > self.limit.saturating_sub((self.limit / 32).max(32 << 20)))
    }
}

/// The explicit limit of the process, if any ([`HardLimit`]).
pub fn hard_limit(max_memory: Option<u64>) -> Option<HardLimit> {
    hard_limit_of(max_memory, cgroup_limit(), resident)
}

fn hard_limit_of(max_memory: Option<u64>, cgroup: Option<u64>, rss: fn() -> Option<u64>) -> Option<HardLimit> {
    let (limit, source) = smallest(&[(max_memory, "--max-memory"), (cgroup, "cgroup memory.max")])?;
    Some(HardLimit { limit, source, rss })
}

/// The exit code of a run that ended with `code`: 2 when it stopped at its hard memory limit (`halted`, ADR-066
/// amendment 10, decision 6), after the message that says how much memory it needed and what the limit allowed. The
/// tests it did not reach are already reported as not run.
pub fn exit_code(halted: Option<&str>, code: u8) -> u8 {
    match halted {
        Some(message) => {
            eprintln!("al-runner: {message}");
            2
        }
        None => code,
    }
}

/// The smallest of the limits that are known, with its source; the first wins a tie.
fn smallest<'a>(limits: &[(Option<u64>, &'a str)]) -> Option<(u64, &'a str)> {
    limits
        .iter()
        .filter_map(|&(l, s)| l.map(|l| (l, s)))
        .fold(None, |best, (l, s)| match best {
            Some((b, _)) if b <= l => best,
            _ => Some((l, s)),
        })
}

/// `MemAvailable` of `/proc/meminfo` in bytes (Linux).
fn mem_available() -> Option<u64> {
    meminfo_available(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

fn meminfo_available(text: &str) -> Option<u64> {
    kb_field(text, "MemAvailable:")
}

/// A `Name:   123 kB` line of a `/proc` file, in bytes.
fn kb_field(text: &str, name: &str) -> Option<u64> {
    let kb: u64 = text
        .lines()
        .find_map(|l| l.strip_prefix(name))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kb * 1024)
}

/// The resident memory of this process in bytes (`VmRSS`, Linux).
fn resident() -> Option<u64> {
    kb_field(&std::fs::read_to_string("/proc/self/status").ok()?, "VmRSS:")
}

/// The smallest `memory.max` of this process's cgroup v2 and its parents, in bytes.
fn cgroup_limit() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let mut dir = std::path::Path::new("/sys/fs/cgroup").join(cgroup_path(&text)?.trim_start_matches('/'));
    let mut least: Option<u64> = None;
    loop {
        if let Some(max) = std::fs::read_to_string(dir.join("memory.max"))
            .ok()
            .and_then(|t| memory_max(&t))
        {
            least = Some(least.map_or(max, |l| l.min(max)));
        }
        if dir == std::path::Path::new("/sys/fs/cgroup") || !dir.pop() {
            return least;
        }
    }
}

/// The cgroup v2 path of `/proc/self/cgroup` (the `0::` line).
fn cgroup_path(text: &str) -> Option<&str> {
    text.lines().find_map(|l| l.strip_prefix("0::"))
}

/// A `memory.max` file: bytes, or `max` for no limit.
fn memory_max(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The profile write after the tests (al-runner issue 6924) stops near the explicit limit, not at free memory.
    #[test]
    fn the_work_after_the_tests_stops_near_the_explicit_limit() {
        assert!(hard_limit_of(None, None, || Some(1)).is_none());
        let l = hard_limit_of(Some(18 << 30), Some(20 << 30), || Some(17 << 30)).unwrap();
        assert_eq!((l.limit, l.source), (18 << 30, "--max-memory"));
        assert!(!l.reached(), "17 GB of 18 GB leaves room for a streamed write");
        let l = hard_limit_of(Some(18 << 30), Some(16 << 30), || Some((16 << 30) - (100 << 20))).unwrap();
        assert_eq!(l.source, "cgroup memory.max");
        assert!(l.reached(), "within a thirty-second of the limit");
        assert!(!hard_limit_of(Some(1 << 30), None, || None).unwrap().reached());
    }

    #[test]
    fn sizes_read_with_units() {
        assert_eq!(parse_size("6G"), Ok(6 << 30));
        assert_eq!(parse_size("800M"), Ok(800 << 20));
        assert_eq!(parse_size("800mb"), Ok(800 << 20));
        assert_eq!(parse_size("2GiB"), Ok(2 << 30));
        assert_eq!(parse_size("512k"), Ok(512 << 10));
        assert_eq!(parse_size("1048576"), Ok(1 << 20));
        for bad in ["", "G", "lots", "6X", "-1G", "0", "1.5G"] {
            assert!(parse_size(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_smallest_known_limit_wins() {
        assert_eq!(smallest(&[(Some(6), "a"), (None, "b"), (Some(4), "c")]), Some((4, "c")));
        assert_eq!(smallest(&[(Some(4), "a"), (Some(4), "b")]), Some((4, "a")));
        assert_eq!(smallest(&[(None, "a")]), None);
    }

    #[test]
    fn proc_and_cgroup_files_read() {
        assert_eq!(
            meminfo_available("MemTotal: 32000 kB\nMemAvailable:   4000 kB\n"),
            Some(4000 * 1024)
        );
        assert_eq!(
            cgroup_path("0::/user.slice/run-1.scope\n"),
            Some("/user.slice/run-1.scope")
        );
        assert_eq!(memory_max("15032385536\n"), Some(15_032_385_536));
        assert_eq!(memory_max("max\n"), None);
    }
}
