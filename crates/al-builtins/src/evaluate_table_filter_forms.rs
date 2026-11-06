//! Filter forms of a TableFilter condition that `Evaluate` keeps as written, and the split of the conditions at commas.
//!
//! Pinned by format-evaluate/tablefilter-neutral/TableFilterEnumForms.al (al-runner issue 7188, corpus pull request 1165): the
//! stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects the rows the test names. The stored
//! text itself is not pinned (assumption `tablefilter-forms-kept`):
//! - Text `''` (:791) and `<>''` (:773); the wildcard `?*` (:888); a value in double quotes that holds a comma, `"A,B"` (:709);
//! - Decimal `<>0` (:881, also on the sum FlowField :895) and `">=100000"`, `">=100,000"` in double quotes (:688, :695, :702);
//! - Code `CORP*` (:860) and `<>CORP*` (:867), in capitals;
//! - Boolean `Yes` (:874);
//! - an Enum ordinal no member has, `10`, is kept as the number (:799).
//!
//! The Text `<>` without an operand is the error `A value is expected after the '<>'.` (:780).
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

/// The stored text of the filter `v` on a field of type `ty`, or `Some(Err(text))` with the error, or `None`.
/// `enum_ordinals` are the ordinals of an Enum field's members.
pub(crate) fn kept_form(ty: &RtType, v: &str, enum_ordinals: Option<&[i32]>) -> Option<Result<String, String>> {
    let kept = || Some(Ok(v.to_string()));
    match ty {
        RtType::Text(_) => match v {
            "''" | "<>''" | "?*" => kept(),
            "<>" => Some(Err("A value is expected after the '<>'.".into())),
            _ => v
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .filter(|s| {
                    s.contains(',')
                        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == ',' || c == ' ')
                        && !s.contains("  ")
                })
                .and_then(|_| kept()),
        },
        RtType::Decimal => {
            if v == "<>0" {
                return kept();
            }
            let digits = v.strip_prefix('"')?.strip_suffix('"')?.strip_prefix(">=")?;
            let plain = !digits.is_empty() && digits.len() <= 9 && digits.bytes().all(|b| b.is_ascii_digit());
            (plain || grouped(digits)).then(|| Ok(v.to_string()))
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
            fits.then(|| Ok(format!("{}{}", if not { "<>" } else { "" }, w.to_uppercase())))
        }
        RtType::Boolean if v == "Yes" => kept(),
        RtType::Enum(_) => {
            let n: i32 = v.parse().ok().filter(|_| v.bytes().all(|b| b.is_ascii_digit()))?;
            let ordinals = enum_ordinals?;
            (!ordinals.contains(&n)).then(|| Ok(v.to_string()))
        }
        _ => None,
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

    #[test]
    fn decimal_forms() {
        let d = RtType::Decimal;
        assert_eq!(kept_form(&d, r#"">=100,000""#, None), Some(Ok(r#"">=100,000""#.into())));
        assert_eq!(kept_form(&d, r#"">=100000""#, None), Some(Ok(r#"">=100000""#.into())));
        assert_eq!(kept_form(&d, r#"">=1,00""#, None), None);
        assert_eq!(kept_form(&d, "<>0", None), Some(Ok("<>0".into())));
        assert_eq!(kept_form(&d, "<>5", None), None);
    }

    #[test]
    fn code_wildcards_and_enum_ordinal() {
        let c = RtType::Code(Some(20));
        assert_eq!(kept_form(&c, "<>corp*", None), Some(Ok("<>CORP*".into())));
        assert_eq!(kept_form(&c, "*", None), None);
        let e = RtType::Enum(1);
        assert_eq!(kept_form(&e, "10", Some(&[0, 1])), Some(Ok("10".into())));
        assert_eq!(kept_form(&e, "1", Some(&[0, 1])), None);
    }

    #[test]
    fn text_forms() {
        let t = RtType::Text(None);
        assert!(matches!(kept_form(&t, "<>", None), Some(Err(_))));
        assert_eq!(kept_form(&t, r#""A,B""#, None), Some(Ok(r#""A,B""#.into())));
        assert_eq!(kept_form(&t, "?*", None), Some(Ok("?*".into())));
        assert_eq!(kept_form(&t, "x*", None), None);
    }
}
