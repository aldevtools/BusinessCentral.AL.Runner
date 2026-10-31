//! Text order for `<`, `>`, `<=` and `>=`: the culture order of the en-US service tier, not code
//! points (types/TestFormatEvaluateTextContracts.al:Operators_TextComparison_IsCultureOrderAndCaseSensitiveEquality).
//!
//! Three levels, as in a UCA collation: letters compare by base letter first (`a` before `b`
//! whatever the case, punctuation before digits before letters, a space before a letter); an accent
//! comes next (`a` before `ä`); the case comes last (`a` before `A`). Equality is not decided here:
//! two texts are equal only when they are the same characters.
//!
//! Only ASCII and the accented Latin-1 letters have weights. Anything else is refused, never ordered
//! by a guess. The order of two different accents on the same letter is not pinned either.

use std::cmp::Ordering;

/// ASCII punctuation and symbols in the order of the root collation, lowest first.
const SYMBOLS: &str = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// Accented Latin-1 letters and the base letter they fold to.
const ACCENTED: &[(&str, char)] = &[
    ("àáâãäå", 'a'),
    ("ç", 'c'),
    ("èéêë", 'e'),
    ("ìíîï", 'i'),
    ("ñ", 'n'),
    ("òóôõö", 'o'),
    ("ùúûü", 'u'),
    ("ýÿ", 'y'),
];

struct Weight {
    primary: u32,
    /// The accented letter itself, if it is one.
    accent: Option<char>,
    upper: bool,
}

fn weight(c: char, total: bool) -> Result<Weight, String> {
    let lower: char = c.to_lowercase().next().unwrap_or(c);
    let upper = c != lower;
    if let Some((_, base)) = ACCENTED.iter().find(|(set, _)| set.contains(lower)) {
        return Ok(Weight {
            primary: 2000 + (*base as u32 - 'a' as u32),
            accent: Some(lower),
            upper,
        });
    }
    if lower.is_ascii_lowercase() {
        return Ok(Weight {
            primary: 2000 + (lower as u32 - 'a' as u32),
            accent: None,
            upper,
        });
    }
    if c.is_ascii_digit() {
        return Ok(Weight {
            primary: 1000 + (c as u32 - '0' as u32),
            accent: None,
            upper: false,
        });
    }
    // In the order of keys a double quote comes before a comma: a Text primary key `a"b` is read before `a,b`
    // (misc/lt-y14-triage/Y14Small.al:173 `format [a"b]` before `format [a,b]`, corpus master). The weights are spaced by two
    // to make room for it.
    if total && c == '"' {
        let comma = SYMBOLS.find(',').expect("a comma has a weight") as u32;
        return Ok(Weight {
            primary: 100 + 2 * comma - 1,
            accent: None,
            upper: false,
        });
    }
    if let Some(i) = SYMBOLS.find(c) {
        return Ok(Weight {
            primary: 100 + 2 * i as u32,
            accent: None,
            upper: false,
        });
    }
    if total {
        // Only for the order of keys, which must be total: a character without a weight follows every weighted one, by code point.
        return Ok(Weight {
            primary: 100_000 + c as u32,
            accent: None,
            upper: false,
        });
    }
    Err(format!(
        "ordering of the character U+{:04X} (no collation weight)",
        c as u32
    ))
}

/// The order of two texts; `Err` names a character that has no weight.
pub fn compare(a: &str, b: &str) -> Result<Ordering, String> {
    compare_with(a, b, false)
}

/// [`compare`] as a total order, for sorting keys: a character without a weight follows the weighted ones (by code point), and
/// two different accents on one letter are ordered by the accent. Never an `Err`. Unpinned beyond what [`compare`] pins.
pub fn compare_total(a: &str, b: &str) -> Ordering {
    compare_with(a, b, true).unwrap_or_else(|_| a.cmp(b))
}

fn compare_with(a: &str, b: &str, total: bool) -> Result<Ordering, String> {
    if a == b {
        return Ok(Ordering::Equal);
    }
    let wa: Vec<Weight> = a.chars().map(|c| weight(c, total)).collect::<Result<_, _>>()?;
    let wb: Vec<Weight> = b.chars().map(|c| weight(c, total)).collect::<Result<_, _>>()?;
    let primary = wa.iter().map(|w| w.primary).cmp(wb.iter().map(|w| w.primary));
    if primary != Ordering::Equal {
        return Ok(primary);
    }
    // Same base letters in the same places: an accent decides, then the case.
    for (x, y) in wa.iter().zip(&wb) {
        match (x.accent, y.accent) {
            (None, Some(_)) => return Ok(Ordering::Less),
            (Some(_), None) => return Ok(Ordering::Greater),
            (Some(p), Some(q)) if p != q && total => return Ok(p.cmp(&q)),
            (Some(p), Some(q)) if p != q => {
                return Err(format!(
                    "ordering of the accents {p} and {q} on one letter (not pinned)"
                ));
            }
            _ => {}
        }
    }
    for (x, y) in wa.iter().zip(&wb) {
        if x.upper != y.upper {
            return Ok(if x.upper { Ordering::Greater } else { Ordering::Less });
        }
    }
    Ok(a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::compare;
    use std::cmp::Ordering::{Greater, Less};

    #[test]
    fn the_pinned_order() {
        // types/TestFormatEvaluateTextContracts.al:Operators_TextComparison_IsCultureOrderAndCaseSensitiveEquality.
        for (a, b) in [
            ("a", "A"),
            ("a", "B"),
            ("A", "b"),
            ("a", "Z"),
            ("_", "a"),
            ("1", "A"),
            ("a", "ä"),
            ("ä", "b"),
            ("a b", "ab"),
            ("", "a"),
        ] {
            assert_eq!(compare(a, b), Ok(Less), "{a} < {b}");
            assert_eq!(compare(b, a), Ok(Greater), "{b} > {a}");
        }
    }

    #[test]
    fn characters_without_a_weight_are_refused() {
        assert!(compare("a", "Ж").is_err());
    }
}
