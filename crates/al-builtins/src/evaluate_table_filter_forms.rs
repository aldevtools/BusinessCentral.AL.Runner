//! Filter forms of a TableFilter condition that `Evaluate` stores in a form of its own, and the split of the conditions at commas.
//!
//! Pinned by format-evaluate/tablefilter-stored/TableFilterStoredText.al (al-runner issues 7188 and 7190, corpus pull request 1181),
//! which gives the stored text, what the page Table Filter and GetFilters show, and the selected rows:
//! - Text `''` (TableFilter_Text_Empty), `<>''` (_NotEmpty), `?*` (_QuestionStar) and `x*` (_PrefixStar) are kept as typed,
//!   case as written;
//! - Decimal `<>0` (TableFilter_Decimal_NotZero) and `<>5` (_NotFive) are kept; `>=100000`, quoted or not, is stored grouped and
//!   quoted, `">=100,000"` (_QuotedGE, _PlainGE, _QuotedGroupedGE); an unquoted `>=100,000` is the error `The value "<text>" can't be
//!   evaluated into type TableFilter.` (_UnquotedGroupedGE, see [`grouped_split_error`]);
//! - Code `CORP*`, `<>CORP*` and `corp*` are stored in capitals (TableFilter_Code_Prefix, _NotPrefix, _LowerPrefix);
//! - Boolean `Yes` and `true` are stored as `Yes`, `No`, `no` and `false` as `No` (TableFilter_Boolean_*);
//! - an Enum ordinal no member has (2, 7, 10) is kept as the number (TableFilter_Enum_NoMember2, _NoMember7, _NoMember10).
//!
//! A Text value in double quotes that holds a comma, `"A,B"`, is kept as written under the assumption
//! `tablefilter-quoted-comma-text-kept` (TableFilterEnumForms.al:709 pins only the selected row).
//! The Text `<>` without an operand is the error `A value is expected after the '<>'.` (TableFilterEnumForms.al:780).
//! Every other form is `None` (the caller decides).

use crate::value::RtType;

/// The conditions of the text after `<table>:`, split at the commas outside a value in double quotes. A value in double quotes
/// starts right after the `=` of its condition (:695 `Decimal Field=">=100,000"`, :702 followed by `,Entry No.=303`).
pub(crate) fn split_conditions(rest: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut start, mut seen_eq, mut quoted) = (0, false, false);
    let mut prev = ' ';
    for (i, ch) in rest.char_indices() {
        if quoted {
            quoted = ch != '"';
        } else if ch == '"' && seen_eq && prev == '=' {
            quoted = true;
        } else if ch == '=' {
            seen_eq = true;
        } else if ch == ',' {
            parts.push(&rest[start..i]);
            start = i + 1;
            seen_eq = false;
        }
        prev = ch;
    }
    parts.push(&rest[start..]);
    parts
}

/// A number with comma groups of three, `100,000` (en-US grouping, :695).
fn grouped(d: &str) -> bool {
    let mut groups = d.split(',');
    let first = groups.next().unwrap_or("");
    (1..=3).contains(&first.len())
        && first.bytes().all(|b| b.is_ascii_digit())
        && groups.all(|g| g.len() == 3 && g.bytes().all(|b| b.is_ascii_digit()))
}

/// `digits` with a comma before every group of three from the right: `100000` is `100,000`.
fn group_digits(digits: &str) -> String {
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// True when `condition` is an unquoted Decimal `>=` filter cut at its thousands comma, `Dec=>=100`, and `next` is the group of
/// three digits that follows (`000`): the text `Dec=>=100,000` (TableFilterStoredText.al TableFilter_Decimal_UnquotedGroupedGE
/// raises the Evaluate error for it). The condition is the one before the comma.
pub(crate) fn grouped_split_error(condition: &str, next: &str) -> bool {
    let digits = condition
        .trim()
        .split_once('=')
        .and_then(|(_, v)| v.trim().strip_prefix(">="));
    next.trim().len() == 3
        && next.trim().bytes().all(|b| b.is_ascii_digit())
        && digits.is_some_and(|d| (1..=3).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_digit()))
}

/// The stored text of the filter `v` on a field of type `ty`, or `Some(Err(text))` with the error, or `None`.
/// `enum_ordinals` are the ordinals of an Enum field's members. `Ok(true)` in the second place marks a form stored under an
/// assumption.
pub(crate) fn kept_form(ty: &RtType, v: &str, enum_ordinals: Option<&[i32]>) -> Option<Result<(String, bool), String>> {
    let kept = || Some(Ok((v.to_string(), false)));
    match ty {
        RtType::Text(_) => match v {
            "''" | "<>''" | "?*" => kept(),
            "<>" => Some(Err("A value is expected after the '<>'.".into())),
            _ => {
                if let Some(stem) = v.strip_suffix('*') {
                    // Pinned for `x*` only; a stem of lowercase letters is the closest shape (capitals are not pinned).
                    if !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_lowercase()) {
                        return kept();
                    }
                }
                v.strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .filter(|s| {
                        s.contains(',')
                            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == ',' || c == ' ')
                            && !s.contains("  ")
                    })
                    .map(|_| Ok((v.to_string(), true)))
            }
        },
        RtType::Decimal => {
            if let Some(n) = v.strip_prefix("<>") {
                let plain = !n.is_empty() && n.len() <= 9 && n.bytes().all(|b| b.is_ascii_digit());
                return plain.then(|| Ok((v.to_string(), false)));
            }
            let quoted = v.strip_prefix('"').and_then(|s| s.strip_suffix('"'));
            let digits = quoted.unwrap_or(v).strip_prefix(">=")?;
            let plain = !digits.is_empty() && digits.len() <= 9 && digits.bytes().all(|b| b.is_ascii_digit());
            if plain && digits.len() >= 4 {
                return Some(Ok((format!("\">={}\"", group_digits(digits)), false)));
            }
            (quoted.is_some() && grouped(digits)).then(|| Ok((v.to_string(), false)))
        }
        RtType::Code(len) => {
            let (not, w) = match v.strip_prefix("<>") {
                Some(w) => (true, w),
                None => (false, v),
            };
            let stem = w.strip_suffix('*')?;
            let fits = !stem.is_empty()
                && stem.bytes().all(|b| b.is_ascii_alphanumeric())
                && len.is_none_or(|n| w.chars().count() <= n as usize);
            fits.then(|| Ok((format!("{}{}", if not { "<>" } else { "" }, w.to_uppercase()), false)))
        }
        RtType::Boolean => match v {
            "Yes" | "true" => Some(Ok(("Yes".into(), false))),
            "No" | "no" | "false" => Some(Ok(("No".into(), false))),
            _ => None,
        },
        RtType::Enum(_) => {
            let n: i32 = v.parse().ok().filter(|_| v.bytes().all(|b| b.is_ascii_digit()))?;
            let ordinals = enum_ordinals?;
            (!ordinals.contains(&n)).then(|| Ok((v.to_string(), false)))
        }
        _ => None,
    }
}

/// A Code form of a TableFilter condition kept as written (the stored text is an assumption, tablefilter-code-chained-kept):
/// alternatives joined by `|` whose values hold `-` or `/`: `BIN-A|BIN/B` selects BIN-A and BIN/B and not BIN/C
/// (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:943 TableFilter_Evaluate_CodeAlternatives_SelectsBothMatchingRows,
/// corpus pull request 1199, al-runner issue 7262). The operands must fit the Code field. Exclusions joined by `&` are
/// [`spaced_chain_form`].
pub(crate) fn code_chained_form(v: &str, len: Option<u32>) -> bool {
    let value = |w: &str| {
        !w.is_empty()
            && len.is_none_or(|n| w.chars().count() <= n as usize)
            && w.chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || matches!(ch, '-' | '/'))
    };
    v.contains('|') && v.split('|').all(value)
}

/// The stored text of a TableFilter condition on a Code or Text field that chains values of capitals, digits and single inner
/// spaces, or `None` (format-evaluate/tablefilter-neutral/TableFilterSpacedChains.al, al-runner issue 7497; Format and the page
/// Table Filter give the text back):
/// - terms joined by `&` or by `|`, not both: `<>A&<>B`, three `<>` (:169), `A|B`, three alternatives, `A|<>B`, `>A&<B`;
/// - a term is a value, `<>` before a value, `>` or `<` before a value inside an `&` chain, a range `A..B`, `..A` or `A..`
///   (:185), and `''` inside a `|` chain (:219, kept as written);
/// - a value in single quotes is stored without them (:219);
/// - `*` and `?` in an unquoted value alone or after `<>`, not in a range: `BIN E*`, `<>BIN E*`, `* EAST`, `BIN*WEST`, `BIN ?AST`
///   (:202);
/// - `@` before lower-case words alone: `@bin east`, stored in capitals on a Code field and as written on a Text field (:202,
///   :269).
///
/// Values must fit the field. The exclusions without spaces (`<>CORP01&<>CORP02`, TableFilterEnumForms.al:936) are the same form.
/// Lower-case values elsewhere, other characters and other operators are not pinned.
pub(crate) fn spaced_chain_form(ty: &RtType, v: &str) -> Option<String> {
    let (code, len) = match ty {
        RtType::Code(l) => (true, *l),
        RtType::Text(l) => (false, *l),
        _ => return None,
    };
    let sep = match (v.contains('&'), v.contains('|')) {
        (true, true) => return None,
        (true, false) => '&',
        _ => '|',
    };
    let terms: Vec<&str> = v.split(sep).collect();
    let chain = Chain {
        code,
        len,
        and: sep == '&',
        many: terms.len() > 1,
    };
    let stored = terms.iter().map(|t| chain.term(t)).collect::<Option<Vec<_>>>()?;
    Some(stored.join(&sep.to_string()))
}

/// How the terms of one condition are read ([`spaced_chain_form`]).
struct Chain {
    code: bool,
    len: Option<u32>,
    and: bool,
    many: bool,
}

impl Chain {
    /// Words of the characters `ok` allows, with single inner spaces, that fit the field.
    fn words(&self, w: &str, ok: impl Fn(char) -> bool) -> bool {
        !w.is_empty()
            && self.len.is_none_or(|n| w.chars().count() <= n as usize)
            && w.trim_matches(' ') == w
            && !w.contains("  ")
            && w.chars().all(|c| c == ' ' || ok(c))
    }

    /// A value of capitals and digits, plain or in single quotes, as stored (without the quotes).
    fn value<'a>(&self, w: &'a str) -> Option<&'a str> {
        let inner = w.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')).unwrap_or(w);
        self.words(inner, |c| c.is_ascii_uppercase() || c.is_ascii_digit())
            .then_some(inner)
    }

    fn term(&self, t: &str) -> Option<String> {
        if t == "''" {
            return (!self.and && self.many).then(|| t.to_string());
        }
        if let Some(w) = t.strip_prefix('@') {
            let ok = !self.many && self.words(w, |c| c.is_ascii_lowercase() || c.is_ascii_digit());
            return ok.then(|| format!("@{}", if self.code { w.to_uppercase() } else { w.to_string() }));
        }
        if let Some((a, b)) = t.split_once("..") {
            let a = if a.is_empty() { "" } else { self.value(a)? };
            let b = if b.is_empty() { "" } else { self.value(b)? };
            return (!(a.is_empty() && b.is_empty())).then(|| format!("{a}..{b}"));
        }
        let (op, w) = ["<>", ">", "<"]
            .iter()
            .find_map(|op| t.strip_prefix(op).map(|w| (*op, w)))
            .unwrap_or(("", t));
        if matches!(op, ">" | "<") && !self.and {
            return None;
        }
        if let Some(w) = self.value(w) {
            return Some(format!("{op}{w}"));
        }
        let wild = matches!(op, "" | "<>")
            && w.contains(['*', '?'])
            && self.words(w, |c| {
                c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '*' | '?')
            });
        wild.then(|| format!("{op}{w}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_outside_a_quoted_value_only() {
        assert_eq!(
            split_conditions(r#" A=">=100,000",B=3"#),
            vec![r#" A=">=100,000""#, "B=3"]
        );
        assert_eq!(split_conditions("A='x,y'"), vec!["A='x", "y'"]);
        assert_eq!(split_conditions("A=1,B=2"), vec!["A=1", "B=2"]);
    }

    fn ok(s: &str) -> Option<Result<(String, bool), String>> {
        Some(Ok((s.to_string(), false)))
    }

    #[test]
    fn decimal_forms() {
        let d = RtType::Decimal;
        assert_eq!(kept_form(&d, r#"">=100,000""#, None), ok(r#"">=100,000""#));
        assert_eq!(kept_form(&d, r#"">=100000""#, None), ok(r#"">=100,000""#));
        assert_eq!(kept_form(&d, ">=100000", None), ok(r#"">=100,000""#));
        assert_eq!(kept_form(&d, ">=1234567", None), ok(r#"">=1,234,567""#));
        assert_eq!(kept_form(&d, ">=40", None), None);
        assert_eq!(kept_form(&d, r#"">=1,00""#, None), None);
        assert_eq!(kept_form(&d, "<>0", None), ok("<>0"));
        assert_eq!(kept_form(&d, "<>5", None), ok("<>5"));
        assert_eq!(kept_form(&d, "<>5.5", None), None);
    }

    #[test]
    fn unquoted_grouped_greater_or_equal_is_cut_at_its_comma() {
        assert!(grouped_split_error(" Dec=>=100", "000"));
        assert!(!grouped_split_error("Dec=>=100", "00"));
        assert!(!grouped_split_error("Dec=100", "000"));
        assert!(!grouped_split_error("Dec=>=1000", "000"));
    }

    #[test]
    fn boolean_forms() {
        let b = RtType::Boolean;
        for (v, stored) in [
            ("Yes", "Yes"),
            ("true", "Yes"),
            ("No", "No"),
            ("no", "No"),
            ("false", "No"),
        ] {
            assert_eq!(kept_form(&b, v, None), ok(stored));
        }
        assert_eq!(kept_form(&b, "YES", None), None);
    }

    #[test]
    fn code_wildcards_and_enum_ordinal() {
        let c = RtType::Code(Some(20));
        assert_eq!(kept_form(&c, "<>corp*", None), ok("<>CORP*"));
        assert_eq!(kept_form(&c, "*", None), None);
        let e = RtType::Enum(1);
        assert_eq!(kept_form(&e, "10", Some(&[0, 1])), ok("10"));
        assert_eq!(kept_form(&e, "1", Some(&[0, 1])), None);
    }

    #[test]
    fn text_forms() {
        let t = RtType::Text(None);
        assert!(matches!(kept_form(&t, "<>", None), Some(Err(_))));
        assert_eq!(kept_form(&t, r#""A,B""#, None), Some(Ok((r#""A,B""#.into(), true))));
        assert_eq!(kept_form(&t, "?*", None), ok("?*"));
        assert_eq!(kept_form(&t, "x*", None), ok("x*"));
        assert_eq!(kept_form(&t, "*", None), None);
        assert_eq!(kept_form(&t, "A*", None), None);
    }
}
