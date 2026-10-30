//! The run seed (ADR-061): every `al-runner` run has one, and it derives, per test method, the seed of the session's
//! `Random`, the key of its Guids and (opt in) the seed of `Library - Random`. The caller picks the run seed (the CLI
//! from the OS, `al-test-cli` the fixed 1); this module only derives from it, so it does no OS access (ADR-006).

/// The run seed of `al-test-cli`, the corpus ratchet and a host nobody gave one (ADR-061 section 5).
pub const FIXED_RUN_SEED: i32 = 1;

/// The highest run seed (v2's range: 0 to 2147483647).
pub const MAX_RUN_SEED: i32 = i32::MAX;

fn fnv1a32(text: &str) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for b in text.bytes() {
        hash ^= u32::from(b);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn fnv1a64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The seed of the session's `Random` when the test method `method` of codeunit `codeunit` starts: AL Runner v2's
/// `RunSeed.Derive`, FNV-1a (32 bit) over the UTF-8 bytes of `"{run_seed}|{codeunit}|{method}"` (ADR-061 owner
/// decision 3), so a seed of a v2 run gives the same `Random` numbers here.
pub fn derive(run_seed: i32, codeunit: u32, method: &str) -> i32 {
    fnv1a32(&format!("{run_seed}|{codeunit}|{method}")) as i32
}

/// The seed the runner hands `Library - Random`.SetSeed before the test method (`--seed-library`, ADR-061 section 2):
/// FNV-1a (32 bit) over `"{run_seed}|{codeunit}|{method}|library"`.
pub fn derive_library(run_seed: i32, codeunit: u32, method: &str) -> i32 {
    fnv1a32(&format!("{run_seed}|{codeunit}|{method}|library")) as i32
}

/// The 64-bit key of the Guids and SystemIds the test method makes (ADR-061 section 2): FNV-1a (64 bit) over
/// `"{run_seed}|{codeunit}|{method}|guid"`. 64 bits keep two methods from getting the same Guids.
pub fn guid_key(run_seed: i32, codeunit: u32, method: &str) -> u64 {
    fnv1a64(&format!("{run_seed}|{codeunit}|{method}|guid"))
}

/// The key of the Guids of a session that runs no derivation (install code, a host with a fixed `guid_seed`): the
/// form the Guid generator had before ADR-061, so those Guids do not change.
pub fn fixed_guid_key(seed: i32) -> u64 {
    u64::from(seed as u32) << 32
}

/// A run seed as written on the command line or in the environment: an integer from 0 to 2147483647, surrounding
/// blanks allowed. `None` is a bad invocation (ADR-061 section 4).
pub fn parse(text: &str) -> Option<i32> {
    text.trim().parse::<i32>().ok().filter(|n| *n >= 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AL Runner v2's golden values (`AlRunner.Tests/RunSeedTests.cs`, `Derive_IsFnv1aOverRunSeedCodeunitAndMethod`).
    #[test]
    fn derive_is_v2s_fnv1a_over_seed_codeunit_and_method() {
        assert_eq!(derive(4271833, 50100, "PostingRoundsAmountCorrectly"), -1379714443);
        assert_eq!(derive(4271833, 50100, "PostingRoundsAmountCorrectlY"), -842830635);
        assert_eq!(derive(4271834, 50100, "PostingRoundsAmountCorrectly"), -1936696938);
        assert_eq!(derive(4271833, 50101, "PostingRoundsAmountCorrectly"), 1186558372);
        assert_eq!(derive(0, 0, ""), -1742010779);
    }

    #[test]
    fn each_value_has_its_own_derivation() {
        let (s, c, m) = (4271833, 50100, "PostingRoundsAmountCorrectly");
        assert_ne!(derive_library(s, c, m), derive(s, c, m));
        assert_ne!(guid_key(s, c, m), guid_key(s, c, "Other"));
        assert_ne!(guid_key(s, c, m), guid_key(s + 1, c, m));
        assert_eq!(guid_key(s, c, m), guid_key(s, c, m));
    }

    #[test]
    fn a_run_seed_is_an_integer_from_0_to_int_max() {
        assert_eq!(parse("4271833"), Some(4271833));
        assert_eq!(parse(" 7 "), Some(7));
        assert_eq!(parse("0"), Some(0));
        assert_eq!(parse("2147483647"), Some(MAX_RUN_SEED));
        assert_eq!(parse("2147483648"), None);
        assert_eq!(parse("-1"), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse(""), None);
    }
}
