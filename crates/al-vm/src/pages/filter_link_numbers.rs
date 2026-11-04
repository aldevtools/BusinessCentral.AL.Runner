//! TestFilter.GetFilter of an Option or Enum field filtered by a part's SubPageLink: the part's TestPage Filter reads the member
//! numbers, not the names: `filter(Open | "Bank Acc. Entry Applied" | "Check Entry Applied")` is `0|1|2`, `const("Check Entry Applied")`
//! is `2`, `filter(<> "Bank Acc. Entry Applied")` is `<>1`, an Enum `filter("Inbound Transfer" | "Outbound Transfer")` is `0|1`
//! (corpus pages/lt-y42-link-quoted-member/Y42LinkQuotedMember.al:318 Lq_Option_FilterQuoted, :364 Lq_Enum_FilterQuoted,
//! :386 Lq_Option_ConstQuoted, :408 Lq_Option_NotEqualQuoted). The group of the link is the part's `LINK_GROUP`.

use al_builtins::filter::FilterExpr;
use al_builtins::value::Value;

/// The number of an Option or Enum value.
fn number(v: &Value) -> Option<i32> {
    match v {
        Value::Option(o, _) | Value::Enum(_, o) => Some(*o),
        _ => None,
    }
}

/// The text of a link filter as numbers, or `None` for a form that is not a single value, a range, a `<>` or a list of them.
pub(super) fn link_filter_numbers(expr: &FilterExpr) -> Option<String> {
    match expr {
        FilterExpr::Eq(v) => number(v).map(|n| n.to_string()),
        FilterExpr::Ne(v) => number(v).map(|n| format!("<>{n}")),
        FilterExpr::Range(Some(a), Some(b)) => {
            let (x, y) = (number(a)?, number(b)?);
            Some(if x == y { x.to_string() } else { format!("{x}..{y}") })
        }
        FilterExpr::Or(parts) => parts
            .iter()
            .map(link_filter_numbers)
            .collect::<Option<Vec<_>>>()
            .map(|v| v.join("|")),
        FilterExpr::And(parts) => parts
            .iter()
            .map(|p| matches!(p, FilterExpr::Ne(_)).then(|| link_filter_numbers(p)).flatten())
            .collect::<Option<Vec<_>>>()
            .map(|v| v.join("&")),
        _ => None,
    }
}
