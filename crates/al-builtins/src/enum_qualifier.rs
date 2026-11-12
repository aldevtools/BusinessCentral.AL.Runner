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

/// `value` as the member of the field's enum when it is `<other enum>::<member>` of another enum and the field's enum has a member
/// with the same name and the same ordinal: `"Purchase Document Type"::Order` on a field of enum "Purchase Comment Document Type"
/// (Order is 1 in both) reads as Order (record/qualified-enum-filters/TestQualifiedEnumFilters.al:162
/// SubPageView_CrossEnumQualifier_SelectsPurchaseOrderComment, corpus pull request 1199, al-runner issue 7265). Whether BC
/// matches by name or by ordinal is not pinned, so a member whose name or ordinal differs gives `None` (and stays refused), as
/// does a list, a range, another operator and an enum `members_of` does not know.
pub fn cross_enum_member(
    value: &str,
    field_members: &[(i32, String)],
    members_of: impl Fn(&str) -> Option<Vec<(i32, String)>>,
) -> Option<String> {
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    let (qualifier, member) = value.trim().split_once("::")?;
    if member.contains(['|', '&', '<', '>', '(', ')', ':']) || member.contains("..") || member.trim().is_empty() {
        return None;
    }
    let other = members_of(&unquote(qualifier.rsplit('.').next().unwrap_or(qualifier)))?;
    let name = unquote(member);
    let (ordinal, _) = other.iter().find(|(_, n)| n.eq_ignore_ascii_case(&name))?;
    field_members
        .iter()
        .any(|(o, n)| o == ordinal && n.eq_ignore_ascii_case(&name))
        .then(|| member.trim().to_string())
}

/// The filter value of a link or view property whose operands are qualified constants, with each operand replaced by the text
/// `resolve(qualifier, name)` gives for it (both unquoted, the qualifier without its namespace). Pinned by
/// pages/link-filter/LinkEnumQualifiedAndObjectConstants.al (al-runner issues 7405, 7411), on BC 27.5, 28.5 and 29.0:
/// one operand, `"Enum"::Member` or `Database::"Table"` (Part_EqQual, Part_DbFilterLink, Part_DbFilterView, Part_PageFilterView);
/// `<>` before one operand (Part_NeQual, Part_NeQualView, Part_DbNeView, Action_RunNeQual, Action_RunNeQualView); two operands
/// joined by `|` (Part_OrQual, Part_OrQualView, Part_DbOrView). Any other shape (`<>` before a list, three operands, another
/// operator, a range) and an operand `resolve` does not know give `None`, so the caller keeps its refusal.
pub fn resolve_qualified_operands(value: &str, resolve: impl Fn(&str, &str) -> Option<String>) -> Option<String> {
    let value = value.trim();
    let (prefix, rest) = match value.strip_prefix("<>") {
        Some(rest) => ("<>", rest),
        None => ("", value),
    };
    let operands = split_outside_quotes(rest, '|');
    // One or two operands, or `<>` before one.
    if !matches!((prefix.is_empty(), operands.len()), (true, 1 | 2) | (false, 1)) {
        return None;
    }
    let unquote = |s: &str| {
        let s = s.trim();
        match (s.chars().next(), s.chars().last()) {
            (Some(a @ ('"' | '\'')), Some(b)) if a == b && s.len() > 1 && !s[1..s.len() - 1].contains(a) => {
                Some(s[1..s.len() - 1].to_string())
            }
            _ if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') => Some(s.to_string()),
            _ => None,
        }
    };
    let mut out = Vec::new();
    for operand in operands {
        let at = find_outside_quotes(operand, "::")?;
        let qualifier = operand[..at].trim();
        let qualifier = unquote(split_outside_quotes(qualifier, '.').last()?)?;
        out.push(resolve(&qualifier, &unquote(&operand[at + 2..])?)?);
    }
    Some(format!("{prefix}{}", out.join("|")))
}

/// `text` cut at every `sep` outside double and single quotes.
fn split_outside_quotes(text: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut quote, mut start) = (None, 0);
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if c == sep => {
                parts.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

/// The byte position of the first `pat` outside double and single quotes.
fn find_outside_quotes(text: &str, pat: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, _) if text[i..].starts_with(pat) => return Some(i),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::resolve_qualified_operands as resolve;
    use super::strip_enum_qualifier as strip;

    fn known(q: &str, n: &str) -> Option<String> {
        match (q.to_ascii_lowercase().as_str(), n) {
            ("database", "PG LV Head") => Some("62601".into()),
            ("database", "PG LV Row") => Some("62602".into()),
            ("pg lv doc type", m) => Some(format!("'{m}'")),
            _ => None,
        }
    }

    #[test]
    fn resolves_the_pinned_shapes() {
        assert_eq!(
            resolve(r#"<> "PG LV Doc Type"::"Outbound Document""#, known).as_deref(),
            Some("<>'Outbound Document'")
        );
        assert_eq!(resolve("Database::'PG LV Head'", known).as_deref(), Some("62601"));
        assert_eq!(
            resolve(r#"Database::"PG LV Head" | Database::"PG LV Row""#, known).as_deref(),
            Some("62601|62602")
        );
        assert_eq!(
            resolve(r#"<>Database::"PG LV Head""#, known).as_deref(),
            Some("<>62601")
        );
    }

    #[test]
    fn keeps_other_shapes_unresolved() {
        assert_eq!(
            resolve(r#"<> Database::"PG LV Head" | Database::"PG LV Row""#, known),
            None
        );
        assert_eq!(resolve(r#"Database::"PG LV Head"..Database::"PG LV Row""#, known), None);
        assert_eq!(resolve(r#"Database::"Other""#, known), None);
        assert_eq!(resolve("<> Open", known), None);
    }

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
