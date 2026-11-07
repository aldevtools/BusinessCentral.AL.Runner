//! An enum member written with its enum, `"ALT Status"::Active`, as the value of a filter in an AL property.
//!
//! Pinned by record/qualified-enum-filters/TestQualifiedEnumFilters.al (al-runner issue 7189, corpus pull request 1166):
//! `const("ALT Status"::Active)` and `filter("ALT Status"::Active)` in a CalcFormula (:CalcFormula_QualifiedEnumConst_*,
//! _QualifiedEnumFilter_*, _QualifiedEnumExist_*, _QualifiedEnumLookup_*) and `const("ALT Status"::Active)` in a page's
//! SourceTableView (:SourceTableView_QualifiedEnumConst_OpensActiveRow) read as the member alone. Lists and ranges of qualified
//! members are pinned by record/enum-lists-ranges/EnumFilterListsRanges.al; other operators after the `::` are not stripped, so they
//! stay refused where they were.

use std::borrow::Cow;

/// `value` without the qualifier when it is `<enum_name>::<member>` of the enum `enum_name` (the name may be namespace
/// qualified, the qualifier may be quoted, in single quotes too: a property view has its double quotes rewritten that way), otherwise `value` as it is.
/// A list `E::A | E::B` and a range `E::A .. E::B` read the same way when every member is qualified with the enum
/// (record/enum-lists-ranges/EnumFilterListsRanges.al: CalcFormula_QualifiedEnumList_And_Range, SourceTableView_QualifiedEnumList,
/// SourceTableView_QualifiedEnumRange); the result is `A|B` and `A..B`. A list or range that mixes a qualified and an
/// unqualified member, and every other operator after the `::`, is returned as it is.
pub fn strip_enum_qualifier<'a>(value: &'a str, enum_name: &str) -> Cow<'a, str> {
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    let wanted = unquote(enum_name.rsplit(['.', '\u{1f}']).next().unwrap_or(enum_name));
    let member_of = |token: &str| -> Option<String> {
        let (qualifier, member) = token.trim().split_once("::")?;
        let member = member.trim();
        let single =
            !member.is_empty() && !member.contains(['|', '&', '<', '>', '(', ')', ':']) && !member.contains("..");
        let given = unquote(qualifier.rsplit('.').next().unwrap_or(qualifier));
        (single && !given.is_empty() && given.eq_ignore_ascii_case(&wanted)).then(|| member.to_string())
    };
    if !value.contains("::") {
        return Cow::Borrowed(value);
    }
    // One member keeps its borrowed text; a list or a range needs a separator (`|` or `..`) and every part qualified.
    if let Some(m) = member_of(value) {
        return Cow::Owned(m);
    }
    let (sep, parts): (&str, Vec<&str>) = if value.contains('|') && !value.contains("..") {
        ("|", value.split('|').collect())
    } else if value.contains("..") && !value.contains('|') {
        ("..", value.split("..").collect())
    } else {
        return Cow::Borrowed(value);
    };
    let members: Option<Vec<String>> = parts.into_iter().map(member_of).collect();
    match members {
        Some(m) if m.len() > 1 => Cow::Owned(m.join(sep)),
        _ => Cow::Borrowed(value),
    }
}

#[cfg(test)]
mod tests {
    use super::strip_enum_qualifier as strip;

    #[test]
    fn strips_one_member_of_the_named_enum() {
        assert_eq!(strip(r#""ALT Status"::Active"#, "ALT Status"), "Active");
        assert_eq!(
            strip(r#""ALT Status"::Draft | "ALT Status"::Active"#, "ALT Status"),
            "Draft|Active"
        );
        assert_eq!(
            strip(r#""ALT Status"::Draft .. "ALT Status"::Active"#, "ALT Status"),
            "Draft..Active"
        );
        assert_eq!(
            strip(r#" ALT.Status::Active "#, "Ns.ALT Status"),
            r#" ALT.Status::Active "#
        );
        assert_eq!(strip("'ALT Status'::Active", "ALT Status"), "Active");
        assert_eq!(strip(r#"Ns."ALT Status"::"In Work""#, "ALT Status"), r#""In Work""#);
    }

    #[test]
    fn keeps_everything_else() {
        assert_eq!(strip(r#""Other"::Active"#, "ALT Status"), r#""Other"::Active"#);
        // A part that is unqualified or of another enum, a mix of `|` and `..`, and other operators stay as written.
        for v in [
            r#""ALT Status"::Active|Draft"#,
            r#""ALT Status"::Active|"Other"::Draft"#,
            r#""ALT Status"::Active|"ALT Status"::Draft..Closed"#,
            r#""ALT Status"::Active&"ALT Status"::Draft"#,
        ] {
            assert_eq!(strip(v, "ALT Status"), v);
        }
        assert_eq!(strip("Active", "ALT Status"), "Active");
    }
}
