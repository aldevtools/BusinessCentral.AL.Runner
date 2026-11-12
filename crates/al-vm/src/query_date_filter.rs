//! The static filter text of a query (DataItemTableFilter, ColumnFilter) on a Date field. A query has no session dates when its
//! static filters are read, so the texts are read here before the filter parser sees them.

use al_builtins::native::{AlError, Signal};
use al_builtins::value::RtType;
use std::borrow::Cow;

pub(crate) const NO_MATCH: &str = " does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.";

/// The text to parse for a static query filter on a field of type `ty`.
/// - `0D` is the empty date, not today: `filter(0D)`, `filter(= 0D)` match the rows without a date, `filter(<> 0D)` and
///   `filter(<>0D)` the others (record/filters/Y62DateFilterShort.al:Query_DataItemTableFilter_NotZeroDate,
///   _ZeroDateForms, Query_ColumnFilter_ZeroDate_AndRuntimeFilters, writer Y62). Other forms with `0D` in them are not pinned.
/// - A date literal (`20260101D`, `20180829D`) or eight digits that are no month, day and year (`29082018`) raise when the query reads
///   (Y62DateFilterShort.al:Query_DataItemTableFilter_ZeroDateForms `filter(0D .. 20260101D)`, `filter(20260101D ..)`;
///   queries/CgQueryLeads.al:350-389 DataItemTableFilter_DateAsNumberAndAsDateLiteral).
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
        return Err(Signal::unsupported(format!(
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
            fiscal_year: None,
        }
    }

    /// [`Self::query_date_context`] for a record's filter `text`: a `Y` in it asks for the fiscal year of the work date, which
    /// is read from the Accounting Period rows (table 50, fields 1 Starting Date and 3 New Fiscal Year) the session holds.
    pub(crate) fn date_filter_context_for(&self, text: &str) -> al_builtins::filter::DateFilterContext {
        let mut ctx = self.query_date_context();
        if text.trim().eq_ignore_ascii_case("y") {
            ctx.fiscal_year = self
                .new_fiscal_year_starts()
                .and_then(|rows| al_builtins::filter::fiscal_year_of(&rows, ctx.work_date));
        }
        ctx
    }

    /// The starting dates of the Accounting Period rows with New Fiscal Year, when the table is the Base Application's.
    fn new_fiscal_year_starts(&self) -> Option<Vec<i32>> {
        let t = self.table_def(50)?;
        if !t.name.eq_ignore_ascii_case("Accounting Period") {
            return None;
        }
        let field = |id: u32| t.fields.iter().position(|f| f.id == id);
        let (start, new_year) = (field(1)?, field(3)?);
        let rows = self.all_rows(&self.cx.clone(), &t).ok()?;
        Some(
            rows.iter()
                .filter(|r| matches!(r[new_year], al_builtins::value::Value::Boolean(true)))
                .filter_map(|r| match r[start] {
                    al_builtins::value::Value::Date(d, false) if d > 0 => Some(d),
                    _ => None,
                })
                .collect(),
        )
    }
}

impl crate::Session {
    /// The filter a Text field read by `field(filter(F))` in a CalcFormula sets on a Date source field: the text is read as
    /// SetFilter reads a date filter in the session's language, and a text that is no date raises the Evaluate error
    /// (record/flowfields/FlowFilterDateFilterForms.al:342-347: blank, one date, a range, an open range, two quotes, `abc`).
    /// The error is pinned in English only; in another language it stays refused.
    pub(crate) fn calc_text_date_filter(
        &self,
        text: &str,
        enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
    ) -> Result<al_builtins::filter::FilterExpr, Signal> {
        let ctx = self.date_filter_context_for(text);
        match al_builtins::filter::with_date_context(ctx, || {
            al_builtins::filter::parse_filter_with(text, &[], &RtType::Date, enums)
        }) {
            Ok(e) => Ok(e),
            Err(e) if e.starts_with(al_builtins::filter::INVALID_DATE) && self.global_language == 1033 => {
                let bad = &e[al_builtins::filter::INVALID_DATE.len()..];
                Err(Signal::Error(Box::new(AlError {
                    message: format!("The format of the Date '{bad}'{NO_MATCH}"),
                    code: "InvalidFilterExpression:Evaluate".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })))
            }
            Err(e) => Err(Signal::unsupported(format!("CalcFormula filter: {e}"))),
        }
    }
}
