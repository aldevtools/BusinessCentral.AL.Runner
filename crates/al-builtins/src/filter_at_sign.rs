//! `@` alone and `<>@` alone (al-runner issue 8637): what a filter of `@` and nothing else keeps rests on how BC reads the rows,
//! not on the field's type ([`AtSignRows`]). The cells: open corpus PR 1495, pages/background-tasks/BgTaskQueryCache.al (BQ Tests
//! 9b and 9c) and record/filters/AtSignAloneReads.al. A Text field of a temporary record is pinned (record/filters/
//! WcInvalidFilterExpr.al:66-111: `@` keeps every row, `<>@` none); every other answer here runs under the assumption
//! `at-sign-alone-database-blank`. A reader that knows how its rows are read rewrites the filter here; any other (a platform
//! table its provider serves, a CRM table's connection, a query's own check, CalcSums or a TableRelation over a FlowField filter)
//! keeps the filter as it is: a Text field keeps main's answer and `@` alone on a Code field refuses
//! ([`FilterExpr::AtSignAloneCode`]). The places a filter comes from where `@` alone and `<>@` alone on a Code field are measured
//! are those of [`parse_filter_both_at_signs_measured`] and [`parse_filter_at_sign_measured`].

use crate::assumptions::{Assumption, assume};
use crate::filter::{FieldFilter, FilterExpr};
use crate::value::Value;
use std::borrow::Cow;

/// How BC reads the rows a filter of `@` alone or `<>@` alone selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtSignRows {
    /// A read of the database: `@` keeps the empty values, `<>@` the others. FindSet and Next, FindFirst, FindLast, Find('-'),
    /// Find('+'), Find('<'), Count, IsEmpty, CalcSums, DeleteAll and ModifyAll (AtSignAloneReads.al:229, :265, :314, :399, :429,
    /// :698; BgTaskQueryCache.al:5780), a report's data item and a page (AtSignAloneReads.al:625, :658), CalcFields of a sum or a
    /// count (BgTaskQueryCache.al:5753, :5767; AtSignAloneReads.al:558, :598, :904, :928), a query (BgTaskQueryCache.al:6369-:6419;
    /// AtSignAloneReads.al:861, :871), and Count and IsEmpty with the filter on a lookup, min or max FlowField
    /// (BgTaskQueryCache.al:5809; AtSignAloneReads.al:366, :730; AtSignAloneMethods.al:552).
    Database,
    /// A row of the database BC checks against the filter itself: `@` keeps every row, `<>@` the values that are not empty.
    /// Find('=') (AtSignAloneReads.al:314, :698), and Find, FindFirst, FindLast, FindSet and Next with the filter on a lookup, min
    /// or max FlowField (:366, :730; AtSignAloneMethods.al:552, :827).
    DatabaseRowCheck,
    /// A temporary record: `@` keeps every row, `<>@` none (AtSignAloneReads.al:247, :265, :314, :366, :698, :730).
    Temporary,
}

/// The place a filter comes from where the cells measure `@` alone on a Code field but not `<>@` alone: a CalcFormula's
/// `field(filter())` of a normal field (AtSignAloneReads.al:558). Here `<>@` alone on a Code field refuses; every other place that
/// admits `@` alone admits both ([`parse_filter_both_at_signs_measured`]), and the rest ([`crate::filter::parse_filter_with`])
/// refuse both, as before this module answered them.
pub fn parse_filter_at_sign_measured(
    text: &str,
    args: &[Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    refused_where_not_measured(crate::filter::parse_filter_any(text, args, ty, enums)?, true)
}

/// The places a filter comes from where the cells measure both `@` alone and `<>@` alone on a Code field: Record.SetFilter in
/// filter groups 0, 1 and 2 and FieldRef.SetFilter, on stored fields, on a lookup, min or max FlowField and on a FlowFilter that
/// CalcFields applies (AtSignAloneReads.al:457, :475, :538, :698, :730, :792, :904, :928; AtSignAloneMethods.al:489, :552;
/// BgTaskQueryCache.al:5753, :5809), SetView and
/// RecordRef.SetView (AtSignAloneReads.al:497, :538, :792), a CalcFormula's `filter('@')` and `filter('<>@')` (:579, :752), a
/// query's SetFilter and its DataItemTableFilter (BgTaskQueryCache.al:6369, :6403; AtSignAloneReads.al:861, :871).
pub fn parse_filter_both_at_signs_measured(
    text: &str,
    args: &[Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    crate::filter::parse_filter_any(text, args, ty, enums)
}

/// `e`, or the refusal of `@` alone on a Code field from a place no cell measures: a place that does not admit it (`at_sign`
/// false, [`crate::filter::parse_filter_with`]) refuses `@` alone and `<>@` alone, one of [`parse_filter_at_sign_measured`]
/// refuses `<>@` alone.
pub(crate) fn refused_where_not_measured(e: FilterExpr, at_sign: bool) -> Result<FilterExpr, String> {
    match e {
        FilterExpr::AtSignAloneCode(negated) if negated || !at_sign => Err(format!(
            "`{}` alone on a Code field from a place no cell measures (al-builtins `filter_at_sign`)",
            if negated { "<>@" } else { "@" }
        )),
        e => Ok(e),
    }
}

/// How a refusal names the `@` alone or `<>@` alone on a Code field among `filters`, if one is there: "`@` alone" or "`<>@` alone"
/// (the readers no cell measures, such as DataTransfer and FilterPageBuilder.SetView).
pub fn code_at_sign_alone_named(filters: &[FieldFilter]) -> Option<&'static str> {
    filters.iter().find_map(|f| match f.expr {
        FilterExpr::AtSignAloneCode(negated) => Some(if negated { "`<>@` alone" } else { "`@` alone" }),
        _ => None,
    })
}

/// What BC answers for a CalcFormula's sum with a quoted `filter('@')` or `filter('<>@')` (`text`, as the lowering passes it) over
/// a source field of type `ty` that is neither Code nor Text (AtSignAloneMethods.al:395, :430, :454), whoever reads the FlowField
/// (CalcFields, SetAutoCalcFields and a Find, a filter on the FlowField, a query column, a temporary host: :657).
#[derive(Debug, PartialEq, Eq)]
pub enum OtherSourceAtSign {
    /// A Date field: `@` keeps no row, `<>@` every row (true) (:395).
    Rows(bool),
    /// Reading the FlowField raises this error, for `@` and `<>@` alike (:430, :454; an Option member list with a blank member
    /// :717), in a session of language 1033 ([`calc_formula_at_sign_other_source`] refuses it in another).
    Error { message: String, code: &'static str },
}

/// Whether an Option field with these `properties` (as written) shows its member names: it declares neither OptionCaption nor
/// OptionCaptionML. The error over an Option source field lists the names only then (:430, :717).
pub fn shows_member_names(properties: &[(String, String)]) -> bool {
    !properties
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("OptionCaption") || k.eq_ignore_ascii_case("OptionCaptionML"))
}

/// [`OtherSourceAtSign`] for `text` over `ty` in a session of `language`, or None when the text is not `@` alone or `<>@` alone or
/// no cell measures the type: Date, Integer, Decimal, Boolean, and an Option field that shows its member names (`names_shown`,
/// [`shows_member_names`]). The error texts are measured in language 1033 only: in another language an error answer is the
/// refusal `Err`. The lowering admits the text only where [`calc_formula_at_sign_measured`] holds.
pub fn calc_formula_at_sign_other_source(
    text: &str,
    ty: &crate::value::RtType,
    names_shown: bool,
    language: i32,
) -> Option<Result<OtherSourceAtSign, String>> {
    let negated = match text {
        "@" => false,
        "<>@" => true,
        _ => return None,
    };
    let answer = other_source_answer(negated, ty, names_shown)?;
    if matches!(answer, OtherSourceAtSign::Error { .. }) && language != 1033 {
        return Some(Err(format!(
            "the error of a CalcFormula filter '{text}' in a session of language {language} (not measured)"
        )));
    }
    assume(Assumption::AtSignAloneDatabaseBlank);
    Some(Ok(answer))
}

/// Whether the cells measure a CalcFormula's quoted `filter('@')` or `filter('<>@')` over a source field of type `ty`: a Code or
/// a Text field with any method (AtSignAloneReads.al:579, :752; AtSignAloneMethods.al:376), the types of
/// [`calc_formula_at_sign_other_source`] with a sum (`sum`; AtSignAloneMethods.al:395, :430, :454). The lowering refuses the rest.
pub fn calc_formula_at_sign_measured(ty: &crate::value::RtType, sum: bool, names_shown: bool) -> bool {
    use crate::value::RtType;
    matches!(ty, RtType::Code(_) | RtType::Text(_)) || (sum && other_source_answer(false, ty, names_shown).is_some())
}

fn other_source_answer(negated: bool, ty: &crate::value::RtType, names_shown: bool) -> Option<OtherSourceAtSign> {
    use crate::value::RtType;
    let evaluate = |type_name: &str| OtherSourceAtSign::Error {
        message: format!("The value \"@\" can't be evaluated into type {type_name}."),
        code: "InvalidFilterExpression:Evaluate",
    };
    Some(match ty {
        RtType::Date => OtherSourceAtSign::Rows(negated),
        RtType::Integer => evaluate("Integer"),
        RtType::Decimal => evaluate("Decimal"),
        RtType::Boolean => evaluate("Boolean"),
        RtType::Option(names) if names_shown => OtherSourceAtSign::Error {
            message: format!("'@' is not an option. The existing options are: {}", names.join(",")),
            code: "InvalidFilterExpression:InvalidOptionString",
        },
        _ => return None,
    })
}

impl FilterExpr {
    /// This filter as BC reads `rows` with it ([`AtSignRows`]), or None when it is not `@` alone or `<>@` alone; `blank` is the
    /// field's empty value.
    pub fn at_sign_alone_read(&self, rows: AtSignRows, blank: impl FnOnce() -> Value) -> Option<FilterExpr> {
        let (code, negated) = match self {
            FilterExpr::AtSignAlone(negated) => (false, *negated),
            FilterExpr::AtSignAloneCode(negated) => (true, *negated),
            _ => return None,
        };
        if rows != AtSignRows::Temporary || code {
            assume(Assumption::AtSignAloneDatabaseBlank);
        }
        Some(match (rows, negated) {
            (AtSignRows::Database, false) => FilterExpr::Eq(blank()),
            (AtSignRows::Database | AtSignRows::DatabaseRowCheck, true) => FilterExpr::Ne(blank()),
            (AtSignRows::DatabaseRowCheck | AtSignRows::Temporary, negated) => FilterExpr::AtSignAlone(negated),
        })
    }
}

/// `filters` as BC reads `rows` with them ([`FilterExpr::at_sign_alone_read`]); `blank`: the empty value of a field by its
/// position.
pub fn at_sign_alone_read<'a>(
    filters: &'a [FieldFilter],
    rows: AtSignRows,
    blank: impl Fn(usize) -> Value,
) -> Cow<'a, [FieldFilter]> {
    if !filters
        .iter()
        .any(|f| matches!(f.expr, FilterExpr::AtSignAlone(_) | FilterExpr::AtSignAloneCode(_)))
    {
        return Cow::Borrowed(filters);
    }
    Cow::Owned(
        filters
            .iter()
            .map(|f| match f.expr.at_sign_alone_read(rows, || blank(f.field)) {
                Some(expr) => FieldFilter { expr, ..f.clone() },
                None => f.clone(),
            })
            .collect(),
    )
}
