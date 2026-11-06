//! An enum member written with its enum, `"ALT Status"::Active`, as the value of a filter in an AL property.
//!
//! Pinned by record/qualified-enum-filters/TestQualifiedEnumFilters.al (al-runner issue 7189, corpus pull request 1166):
//! `const("ALT Status"::Active)` and `filter("ALT Status"::Active)` in a CalcFormula (:CalcFormula_QualifiedEnumConst_*,
//! _QualifiedEnumFilter_*, _QualifiedEnumExist_*, _QualifiedEnumLookup_*) and `const("ALT Status"::Active)` in a page's
//! SourceTableView (:SourceTableView_QualifiedEnumConst_OpensActiveRow) read as the member alone. Only one member is pinned:
//! lists, ranges and operators after the `::` are not stripped, so they stay refused where they were.

/// `value` without the qualifier when it is `<enum_name>::<member>` of the enum `enum_name` (the name may be namespace
/// qualified, the qualifier may be quoted, in single quotes too: a property view has its double quotes rewritten that way), otherwise `value` as it is.
pub fn strip_enum_qualifier<'a>(value: &'a str, enum_name: &str) -> &'a str {
    let v = value.trim();
    let Some((qualifier, member)) = v.split_once("::") else {
        return value;
    };
    let member = member.trim();
    let single = !member.is_empty() && !member.contains(['|', '&', '<', '>', '(', ')', ':']) && !member.contains("..");
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    let wanted = unquote(enum_name.rsplit(['.', '\u{1f}']).next().unwrap_or(enum_name));
    let given = unquote(qualifier.rsplit('.').next().unwrap_or(qualifier));
    if single && !given.is_empty() && given.eq_ignore_ascii_case(&wanted) {
        member
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::strip_enum_qualifier as strip;

    #[test]
    fn strips_one_member_of_the_named_enum() {
        assert_eq!(strip(r#""ALT Status"::Active"#, "ALT Status"), "Active");
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
        assert_eq!(
            strip(r#""ALT Status"::Active|"ALT Status"::Draft"#, "ALT Status"),
            r#""ALT Status"::Active|"ALT Status"::Draft"#
        );
        assert_eq!(strip("Active", "ALT Status"), "Active");
    }
}
