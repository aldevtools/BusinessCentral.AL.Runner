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

/// ASCII punctuation and symbols in the order of the keys of a normal record (SQL Server), lowest first; the hyphen and the
/// apostrophe are not here, as they do not count at the first level ([`compare_key`]). Measured: record/keys-sorting/
/// KeyPunctuationOrder.al:194-236 (staging r6980), the same for a Code and a Text key and at the start, inside and at the end of a
/// value. A double quote before a comma agrees with format-evaluate/Y14Small.al:173.
const KEY_SYMBOLS: &str = " !\"#$%&()*,./:;?@[\\]^_`{|}~+<=>";

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
    if total {
        if let Some(i) = KEY_SYMBOLS.find(c) {
            return Ok(Weight {
                primary: 100 + i as u32,
                accent: None,
                upper: false,
            });
        }
    }
    if let Some(i) = SYMBOLS.find(c).filter(|_| !total || matches!(c, '-' | '\'')) {
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

/// [`compare`] as a total order, for sorting keys, with the symbols in the order of [`KEY_SYMBOLS`]: a character without a weight
/// follows the weighted ones (by code point), and two different accents on one letter are ordered by the accent. Never an `Err`.
pub fn compare_total(a: &str, b: &str) -> Ordering {
    compare_with(a, b, true).unwrap_or_else(|_| a.cmp(b))
}

/// The order of two keys of a normal record (record/keys-sorting/KeyPunctuationOrder.al:194-251, :277-310, staging r6980; al-runner issue
/// 6980): a hyphen and an apostrophe do not count first (`A-B` and `A'B` sort next to `AB`, `TTS.001` before `TTS-HD.001`);
/// of two texts that tie without them, the one without comes first and an apostrophe before a hyphen (`AB`, `A'B`, `A-B`; `A`,
/// `A'`, `A-`), and fewer hyphens before more (`-5` before `--5`, record/AsCodeKeySigns.al:56). Two texts that still tie (the
/// same characters in other places, not pinned) are ordered by code point, so the order stays total.
///
/// A temporary record uses this order too: KeyPunctuationOrder.al (codeunit 95572) expects every temporary value (ascending,
/// descending, ranges, compare and wildcard filters, :199-296) to equal the normal record's.
pub fn compare_key(a: &str, b: &str) -> Ordering {
    let ignorable = |c: char| matches!(c, '-' | '\'');
    if !a.contains(ignorable) && !b.contains(ignorable) {
        return compare_total(a, b);
    }
    let rest = |t: &str| t.chars().filter(|c| !ignorable(*c)).collect::<String>();
    let marks = |t: &str| {
        t.chars()
            .filter(|c| ignorable(*c))
            .map(|c| c == '-')
            .collect::<Vec<bool>>()
    };
    compare_total(&rest(a), &rest(b))
        .then_with(|| marks(a).cmp(&marks(b)))
        .then_with(|| a.cmp(b))
}

/// The primary weight and the upper-case flag of an ASCII character, `None` for one without a weight.
type AsciiWeight = Option<(u32, bool)>;

/// The weights of the 128 ASCII characters (index 0: `total` off, 1: on), made by [`weight`]. An ASCII character has no accent,
/// so a weight is the primary value and the case flag. Looked up in place of the work `weight` does for every character (a
/// lower-casing, a scan of the accent sets and of the symbols).
static ASCII_WEIGHTS: std::sync::LazyLock<[[AsciiWeight; 128]; 2]> = std::sync::LazyLock::new(|| {
    let mut t = [[None; 128]; 2];
    for (k, row) in t.iter_mut().enumerate() {
        for (c, slot) in row.iter_mut().enumerate() {
            *slot = weight(c as u8 as char, k == 1).ok().map(|w| (w.primary, w.upper));
        }
    }
    t
});

fn compare_with(a: &str, b: &str, total: bool) -> Result<Ordering, String> {
    if a == b {
        return Ok(Ordering::Equal);
    }
    if a.is_ascii() && b.is_ascii() {
        return compare_ascii(a, b, total);
    }
    compare_general(a, b, total)
}

/// [`compare_general`] for two ASCII texts, with no allocation: the same three levels (no accent exists in ASCII).
fn compare_ascii(a: &str, b: &str, total: bool) -> Result<Ordering, String> {
    let table = &ASCII_WEIGHTS[usize::from(total)];
    // The first character without a weight is refused, the ones of `a` before the ones of `b`.
    for c in a.bytes().chain(b.bytes()) {
        if table[usize::from(c)].is_none() {
            return Err(weight(c as char, total)
                .err()
                .expect("a character without a weight is refused"));
        }
    }
    let w = |c: u8| table[usize::from(c)].unwrap_or_default();
    let primary = a.bytes().map(|c| w(c).0).cmp(b.bytes().map(|c| w(c).0));
    if primary != Ordering::Equal {
        return Ok(primary);
    }
    for (x, y) in a.bytes().zip(b.bytes()) {
        let (x, y) = (w(x).1, w(y).1);
        if x != y {
            return Ok(if x { Ordering::Greater } else { Ordering::Less });
        }
    }
    Ok(a.cmp(b))
}

fn compare_general(a: &str, b: &str, total: bool) -> Result<Ordering, String> {
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

    /// The table-driven order of ASCII texts is the general order, for every pair of texts over characters that cover each
    /// kind of weight (letters of both cases, digits, symbols, the quote, a control character, a space), both ways of
    /// `total`, and the refusals with their texts.
    #[test]
    fn the_ascii_order_is_the_general_order() {
        let alphabet: Vec<char> = "aAbB09 _-,\"@~\u{1}\u{7f}Zz!".chars().collect();
        let mut texts = vec![String::new()];
        for &c in &alphabet {
            texts.push(c.to_string());
            for &d in &alphabet {
                texts.push(format!("{c}{d}"));
                texts.push(format!("a{c}{d}"));
            }
        }
        for a in &texts {
            for b in &texts {
                for total in [false, true] {
                    if a == b {
                        continue;
                    }
                    assert_eq!(
                        super::compare_ascii(a, b, total),
                        super::compare_general(a, b, total),
                        "{a:?} {b:?} total={total}"
                    );
                }
            }
        }
    }

    #[test]
    fn characters_without_a_weight_are_refused() {
        assert!(compare("a", "Ж").is_err());
    }

    /// The key order of a normal record, measured on SQL Server (record/keys-sorting/KeyPunctuationOrder.al:194-251, :277-310, staging
    /// r6980; al-runner issue 6980): sorting the values of each list gives the list.
    #[test]
    fn the_key_order_of_punctuation() {
        let lists = [
            // Order_Leading, Code (the Text key adds ` B` first).
            "!B¦\"B¦#B¦$B¦%B¦&B¦(B¦)B¦*B¦,B¦.B¦/B¦:B¦;B¦?B¦@B¦[B¦\\B¦]B¦^B¦_B¦`B¦{B¦|B¦}B¦~B¦+B¦<B¦=B¦>B¦0¦9¦A¦B¦'B¦-B¦BB¦C",
            " B¦!B¦\"B¦#B¦$B¦%B¦&B¦(B¦)B¦*B¦,B¦.B¦/B¦:B¦;B¦?B¦@B¦[B¦\\B¦]B¦^B¦_B¦`B¦{B¦|B¦}B¦~B¦+B¦<B¦=B¦>B¦0¦9¦A¦B¦'B¦-B¦BB¦C",
            // Order_Interior.
            "A¦A B¦A!B¦A\"B¦A#B¦A$B¦A%B¦A&B¦A(B¦A)B¦A*B¦A,B¦A.B¦A/B¦A:B¦A;B¦A?B¦A@B¦A[B¦A\\B¦A]B¦A^B¦A_B¦A`B¦A{B¦A|B¦A}B¦A~B¦A+B¦A<B¦A=B¦A>B¦A0B¦A9B¦AA¦AAB¦AB¦A'B¦A-B¦ABB¦AC¦B",
            // Order_Trailing.
            "A¦A'¦A-¦A!¦A\"¦A#¦A$¦A%¦A&¦A(¦A)¦A*¦A,¦A.¦A/¦A:¦A;¦A?¦A@¦A[¦A\\¦A]¦A^¦A_¦A`¦A{¦A|¦A}¦A~¦A+¦A<¦A=¦A>¦A0¦A9¦AA¦B",
            // Order_IssueValues.
            "A.B¦AB¦A-B¦TTS.001¦TTSA¦TTS-A¦TTS-HD.001",
            // record/AsCodeKeySigns.al:56.
            "+5¦-+5¦+A¦0¦-0¦5¦-5¦--5¦A¦-A",
        ];
        for list in lists {
            let want: Vec<&str> = list.split('¦').collect();
            for order in [crate::ops::text_key_order, crate::ops::code_key_order] {
                let mut got = want.clone();
                got.reverse();
                got.sort_by(|a, b| order(a, b));
                assert_eq!(got, want);
            }
        }
    }
}
