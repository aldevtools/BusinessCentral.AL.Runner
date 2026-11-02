//! The static filter text of a query (DataItemTableFilter, ColumnFilter) on a Date field. A query has no session dates when its
//! static filters are read, so the texts are read here before the filter parser sees them.

use al_builtins::native::{AlError, Signal};
use al_builtins::value::RtType;
use std::borrow::Cow;

const NO_MATCH: &str = " does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.";

/// The text to parse for a static query filter on a field of type `ty`.
/// - `0D` is the empty date, not today: `filter(0D)`, `filter(= 0D)` match the rows without a date, `filter(<> 0D)` and
///   `filter(<>0D)` the others (filters/lt-y62-date-filter-short/Y62DateFilterShort.al:Query_DataItemTableFilter_NotZeroDate,
///   _ZeroDateForms, Query_ColumnFilter_ZeroDate_AndRuntimeFilters, writer Y62). Other forms with `0D` in them are not pinned.
/// - A date literal (`20260101D`, `20180829D`) or eight digits that are no month, day and year (`29082018`) raise when the query reads
///   (Y62DateFilterShort.al:Query_DataItemTableFilter_ZeroDateForms `filter(0D .. 20260101D)`, `filter(20260101D ..)`;
///   query/lt-cg-query-leads/CgQueryLeads.al:350-389 DataItemTableFilter_DateAsNumberAndAsDateLiteral).
pub(crate) fn static_text<'a>(text: &'a str, ty: &RtType) -> Result<Cow<'a, str>, Signal> {
    if !matches!(ty, RtType::Date) {
        return Ok(Cow::Borrowed(text));
    }
    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        let literal = token.len() > 1
            && token.ends_with(['D', 'd'])
            && token[..token.len() - 1].bytes().all(|b| b.is_ascii_digit())
            && !token.eq_ignore_ascii_case("0D");
        if literal || (token.len() == 8 && token.bytes().all(|b| b.is_ascii_digit()) && !mmddyyyy(token)) {
            return Err(Signal::Error(Box::new(AlError {
                message: format!("The format of the Date '{token}'{NO_MATCH}"),
                code: "InvalidFilterExpression:Evaluate".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
    }
    let bare: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if bare.eq_ignore_ascii_case("0D") || bare.eq_ignore_ascii_case("=0D") {
        return Ok(Cow::Borrowed("''"));
    }
    if bare.eq_ignore_ascii_case("<>0D") {
        return Ok(Cow::Borrowed("<>''"));
    }
    if text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| t.eq_ignore_ascii_case("0D"))
    {
        return Err(Signal::Unsupported(format!(
            "query filter `{text}` with 0D in this form on a Date field (not pinned)"
        )));
    }
    Ok(Cow::Borrowed(text))
}

/// Whether eight digits are a month, a day and a year.
fn mmddyyyy(t: &str) -> bool {
    let (m, d, y): (u32, u32, i32) = (
        t[0..2].parse().unwrap_or(0),
        t[2..4].parse().unwrap_or(0),
        t[4..].parse().unwrap_or(0),
    );
    (1..=12).contains(&m)
        && (1..=31).contains(&d)
        && y >= 1
        && al_builtins::format::civil_from_day(al_builtins::format::day_from_civil(y, m, d)) == (y, m, d)
}

impl crate::Session {
    /// The session's dates for a run-time `Query.SetFilter`: a text there reads as in a record's SetFilter, `0D` today
    /// (Y62DateFilterShort.al:Query_ColumnFilter_ZeroDate_AndRuntimeFilters `SetFilter text 0D` = the row of today).
    pub(crate) fn query_date_context(&self) -> al_builtins::filter::DateFilterContext {
        al_builtins::filter::DateFilterContext {
            today: self.today_day_number(),
            work_date: self.work_date_day().unwrap_or_else(|| self.today_day_number()),
            language: self.global_language,
        }
    }
}
