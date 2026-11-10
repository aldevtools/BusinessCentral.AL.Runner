//! A text filter on a Blob field (recordref-fieldref/fieldref/FieldRefFilterEmptyText.al:192-240,
//! FieldRef_SetFilter_EmptyQuotesOnBlob; FieldRefFilterMatrix.al:3063-3250, Filter_Blob_*; al-runner issue 7426).
//!
//! Two single quotes keep the rows whose Blob has no content and read back as `' '`; `<>''` keeps the rows with content and
//! reads back as `<>' '`. `*` keeps the rows with content and `<>*` those without; `1` and `<>1` read as those two. `<>` alone and one single quote raise the
//! filter error. Every other text raises "The value "…" can't be evaluated into type BLOB." naming the first value of the
//! text: without its operator, a quoted value without its quotes, a wildcard pattern and an `@` value as written.

use crate::filter::{FILTER_SYNTAX, FilterExpr, NOT_EVALUATED};
use crate::value::{RtType, Value};

/// Whether `ty` is a Blob field's type.
pub fn is_blob(ty: &RtType) -> bool {
    matches!(ty, RtType::Library(crate::library_kind::LibraryKind::Blob))
}

/// The value `''` stands for on a Blob field: it reads back as `' '` (FieldRefFilterEmptyText.al:226, :229).
fn no_content() -> Value {
    Value::Text(" ".into())
}

/// The filter of `text` (not blank, no placeholders) on a Blob field. An `Err` that starts with a marker of
/// [`crate::filter`] is BC's error; any other is a text no test pins.
pub fn parse(text: &str) -> Result<FilterExpr, String> {
    let t = text.trim();
    match t {
        "''" => return Ok(FilterExpr::Eq(no_content())),
        "<>''" => return Ok(FilterExpr::Ne(no_content())),
        "*" => return Ok(FilterExpr::Like("*".into(), false)),
        "<>*" => return Ok(FilterExpr::NotLike("*".into(), false)),
        // The number 1 reads as `*`, and `<>1` as `<>*` (FieldRefFilterMatrix.al:3097-3098, :3104-3105 `ordinal.Blob`,
        // `neordinal.Blob`). Another number is not pinned.
        "1" => return Ok(FilterExpr::Like("*".into(), false)),
        "<>1" => return Ok(FilterExpr::NotLike("*".into(), false)),
        "<>" => return Err(format!("{FILTER_SYNTAX}A value is expected after the '<>'.")),
        // The wording is BC's (FieldRefFilterMatrix.al:3147 `unterm.Blob`).
        "'" => return Err(format!("{FILTER_SYNTAX}An single quote (') is missing.")),
        _ => {}
    }
    if t.contains(['(', ')', '%']) {
        return Err(format!("text filter `{t}` on a Blob field: not pinned"));
    }
    // The first value BC cannot evaluate: the pieces between `|`, `&` and `..`, each without its operator.
    for piece in t.split(['|', '&']).flat_map(|p| p.split("..")) {
        let value = ["<>", "<=", ">=", "<", ">", "="]
            .iter()
            .find_map(|op| piece.strip_prefix(op))
            .unwrap_or(piece);
        if value.is_empty() || value == "''" || value == "*" {
            continue;
        }
        if value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("text filter `{t}` on a Blob field: a number is not pinned"));
        }
        let value = match value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
            Some(inner) if !inner.is_empty() && !inner.contains('\'') => inner,
            Some(_) => return Err(format!("text filter `{t}` on a Blob field: not pinned")),
            None => value,
        };
        return Err(format!("{NOT_EVALUATED}BLOB\t{value}"));
    }
    Err(format!("text filter `{t}` on a Blob field: not pinned"))
}

/// Whether a Blob field's value passes `e`, a filter [`parse`] made: only whether the row has content counts.
pub(crate) fn matches(e: &FilterExpr, b: &crate::stream::BlobRef) -> bool {
    let has_content = {
        let buf = b.borrow();
        buf.unread_content || !buf.bytes.is_empty() || buf.render_pending.is_some()
    };
    match e {
        FilterExpr::Eq(Value::Text(s)) if s.trim().is_empty() => !has_content,
        FilterExpr::Ne(Value::Text(s)) if s.trim().is_empty() => has_content,
        FilterExpr::Like(p, _) if p == "*" => has_content,
        FilterExpr::NotLike(p, _) if p == "*" => !has_content,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failing(text: &str) -> String {
        let e = parse(text).expect_err(text);
        e.strip_prefix(NOT_EVALUATED)
            .and_then(|r| r.strip_prefix("BLOB\t"))
            .unwrap_or_else(|| panic!("{text}: {e}"))
            .to_string()
    }

    // FieldRefFilterMatrix.al:3064-3150, :3232-3250: the value each error names.
    #[test]
    fn the_error_names_the_first_value() {
        for (text, value) in [
            ("abc", "abc"),
            ("=abc", "abc"),
            ("<>abc", "abc"),
            ("<=abc", "abc"),
            (">=abc", "abc"),
            ("abc..abd", "abc"),
            ("..abd", "abd"),
            ("abc..", "abc"),
            ("abc|abd", "abc"),
            (">abc&<abd", "abc"),
            ("?", "?"),
            ("@abc", "@abc"),
            ("abc*", "abc*"),
            ("*abc", "*abc"),
            ("abc'", "abc'"),
            ("'abc'", "abc"),
        ] {
            assert_eq!(failing(text), value, "{text}");
        }
    }

    #[test]
    fn the_pinned_filters_and_errors() {
        assert!(matches!(parse("''"), Ok(FilterExpr::Eq(_))));
        assert!(matches!(parse("<>''"), Ok(FilterExpr::Ne(_))));
        assert!(matches!(parse("*"), Ok(FilterExpr::Like(..))));
        assert!(matches!(parse("<>*"), Ok(FilterExpr::NotLike(..))));
        assert!(parse("<>").unwrap_err().starts_with(FILTER_SYNTAX));
        assert!(parse("'").unwrap_err().starts_with(FILTER_SYNTAX));
        // Not pinned: refused, not an error of BC.
        let e = parse("(abc)").unwrap_err();
        assert!(!e.starts_with(FILTER_SYNTAX) && !e.starts_with(NOT_EVALUATED), "{e}");
    }
}
