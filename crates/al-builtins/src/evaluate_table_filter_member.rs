//! The filter of a TableFilter condition on an Option or Enum field, as `Evaluate` stores it: every member by its caption.
//!
//! Pinned by record/W4PlatformAndMeta.al (TableFilterEvaluate_AcceptedForms, _RejectedForms: one Option member) and, for
//! Enum fields, lists, `<>` and ranges, by corpus branch al-runtime-rs-staging-r6951
//! format-evaluate/tablefilter-neutral/TableFilterEnumForms.al (TableFilterEvaluate_EnumMemberForms, _EnumExpressions,
//! _OptionExpressions; al-runner issue 6951):
//! - one member by its name in any casing, its caption or its ordinal (`5` is the member with ordinal 5);
//! - members joined by `|`, given by name or caption: the captions joined by `|`;
//! - `<>` before one member, `..` between two: the captions with the same operator;
//! - a word of letters that is no member, alone or in a list: `'X' is not an option. The existing options are: <captions>`;
//! - `FILTER(` or `CONST(`: `An error occurred while interpreting the filter: Did not expect '('.`
//!
//! Any other form is `None` (refused by the caller).

/// The stored filter text, `Some(Err(text))` with the error text after `can't be evaluated into type TableFilter.`, or `None` for
/// a form no test pins. `members` are (ordinal, name, caption).
pub(crate) fn member_filter(members: &[(i32, String, String)], v: &str) -> Option<Result<String, String>> {
    let upper = v.to_ascii_uppercase();
    if upper.starts_with("FILTER(") || upper.starts_with("CONST(") {
        return Some(Err(
            "An error occurred while interpreting the filter: Did not expect '('.".into(),
        ));
    }
    let one = |t: &str| -> Option<&str> {
        let by_ordinal = t
            .parse::<i32>()
            .ok()
            .filter(|_| !t.is_empty() && t.chars().all(|ch| ch.is_ascii_digit()))
            .and_then(|o| members.iter().find(|m| m.0 == o));
        by_ordinal
            .or_else(|| members.iter().find(|m| m.1.eq_ignore_ascii_case(t)))
            .or_else(|| members.iter().find(|m| m.2 == t))
            .map(|m| m.2.as_str())
    };
    let not_a_member = |t: &str| -> Option<Result<String, String>> {
        if t.is_empty() || !t.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return None;
        }
        let list = members.iter().map(|m| m.2.as_str()).collect::<Vec<_>>().join(",");
        Some(Err(format!("'{t}' is not an option. The existing options are: {list}")))
    };
    if v.contains('|') {
        let mut captions = Vec::new();
        for part in v.split('|') {
            match one(part) {
                Some(c) => captions.push(c),
                None => return not_a_member(part),
            }
        }
        return Some(Ok(captions.join("|")));
    }
    if let Some(rest) = v.strip_prefix("<>") {
        return one(rest).map(|c| Ok(format!("<>{c}")));
    }
    if let Some((a, b)) = v.split_once("..") {
        return Some(Ok(format!("{}..{}", one(a)?, one(b)?)));
    }
    match one(v) {
        Some(c) => Some(Ok(c.to_string())),
        None => not_a_member(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members() -> Vec<(i32, String, String)> {
        [
            (0, "Labor", "Labor Cap"),
            (1, "Setting", "Setting Cap"),
            (5, "Travel", "Travel Cap"),
        ]
        .iter()
        .map(|(o, n, c)| (*o, n.to_string(), c.to_string()))
        .collect()
    }

    #[test]
    fn unpinned_forms_are_none() {
        let m = members();
        assert_eq!(member_filter(&m, "<>Nope"), None);
        assert_eq!(member_filter(&m, "Labor..Nope"), None);
        assert_eq!(member_filter(&m, "Labor|<>Travel"), None);
        assert_eq!(member_filter(&m, "2"), None);
        assert_eq!(member_filter(&m, "Labor*"), None);
    }
}
