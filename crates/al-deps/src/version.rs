//! App versions: four numeric parts, compared part by part.
//!
//! App version numbers.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Version(pub u32, pub u32, pub u32, pub u32);

impl FromStr for Version {
    type Err = String;

    /// `27`, `27.5` and `27.5.46862.48827` are all accepted; missing parts are 0.
    fn from_str(s: &str) -> Result<Version, String> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() > 4 {
            return Err(format!("version `{s}` has more than four parts"));
        }
        let mut n = [0u32; 4];
        for (slot, part) in n.iter_mut().zip(&parts) {
            *slot = part
                .parse()
                .map_err(|_| format!("version `{s}`: `{part}` is not a number"))?;
        }
        Ok(Version(n[0], n[1], n[2], n[3]))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0, self.1, self.2, self.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_four_parts_and_pads_short_versions() {
        assert_eq!("27.5.46862.48827".parse(), Ok(Version(27, 5, 46862, 48827)));
        assert_eq!("27".parse(), Ok(Version(27, 0, 0, 0)));
        assert_eq!("1.0".parse(), Ok(Version(1, 0, 0, 0)));
    }

    #[test]
    fn rejects_garbage_and_too_many_parts() {
        assert!("".parse::<Version>().is_err());
        assert!("a.b".parse::<Version>().is_err());
        assert!("1.2.3.4.5".parse::<Version>().is_err());
        assert!("1..2".parse::<Version>().is_err());
    }

    #[test]
    fn compares_part_by_part_not_as_text() {
        let v = |s: &str| s.parse::<Version>().unwrap();
        assert!(v("27.10.0.0") > v("27.9.99999.0"));
        assert!(v("27.5.46862.48827") < v("27.5.46862.53931"));
        assert_eq!(v("27.0.0.0"), v("27"));
        assert_eq!(v("27.5.1.2").to_string(), "27.5.1.2");
    }
}
