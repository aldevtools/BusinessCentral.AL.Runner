//! `@` alone and `<>@` alone (al-runner issue 8637): what a filter of `@` and nothing else keeps rests on how BC reads the rows,
//! not on the field's type ([`AtSignRows`]). The cells: open corpus PR 1495, pages/background-tasks/BgTaskQueryCache.al (BQ Tests
//! 9b and 9c) and record/filters/AtSignAloneReads.al, AtSignAloneMethods.al and AtSignAloneTypes.al. A Text field of a temporary
//! record is pinned (record/filters/WcInvalidFilterExpr.al:66-111: `@` keeps every row, `<>@` none); every other answer here runs
//! under the assumption `at-sign-alone-database-blank`. A reader that knows how its rows are read rewrites the filter here; any
//! other (a platform table its provider serves, a CRM table's connection, a query's own check, CalcSums or a TableRelation over a
//! FlowField filter) keeps the filter as it is: a Text field keeps main's answer and `@` alone on a Code field refuses
//! ([`FilterExpr::AtSignAloneCode`]).
//!
//! al-data `MemStore::at_sign_alone_read` picks the kind of read for the store's `next`, `count` and `delete_all`, for
//! `row_passes` and for `passes_flow_field_filters` (by the Record call that runs). On a Code field the filter is parsed only at
//! the places the cells measure ([`parse_filter_both_at_signs_measured`]); `parse_filter_with` refuses it everywhere else. A
//! CalcFormula's quoted `'@'` or `'<>@'` over a source field that is neither Code nor Text takes BC's rows or error with any
//! method ([`calc_formula_at_sign_other_source`], applied by al-vm `flowfields::calc_filters`); the al-sema lowering admits the two
//! literals only where [`calc_formula_at_sign_measured`] holds. [`code_at_sign_alone_named`] names the filter in the refusals of
//! DataTransfer and FilterPageBuilder.SetView. Entry points: `AtSignRows`, `at_sign_alone_read`,
//! `parse_filter_both_at_signs_measured`, `calc_formula_at_sign_other_source`, `calc_formula_at_sign_measured`.

use crate::assumptions::{Assumption, assume};
use crate::filter::{FieldFilter, FilterExpr};
use crate::native::{Signal, unsupported};
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

/// The places a filter comes from where the cells measure both `@` alone and `<>@` alone on a Code field: Record.SetFilter in
/// filter groups 0, 1 and 2 and FieldRef.SetFilter, on stored fields, on a lookup, min or max FlowField and on a FlowFilter that
/// CalcFields applies (AtSignAloneReads.al:457, :475, :538, :698, :730, :792, :904, :928; AtSignAloneMethods.al:489, :552;
/// BgTaskQueryCache.al:5753, :5809), SetView and
/// RecordRef.SetView (AtSignAloneReads.al:497, :538, :792), a CalcFormula's `filter('@')` and `filter('<>@')` (:579, :752), a
/// query's SetFilter and its DataItemTableFilter (BgTaskQueryCache.al:6369, :6403; AtSignAloneReads.al:861, :871), and a
/// CalcFormula's `field(filter())` of a normal field that holds either, with any method, for a host in the database and a temporary
/// host (AtSignAloneReads.al:558; AtSignAloneTypes.al:1027, :1041). The rest ([`crate::filter::parse_filter_with`]) refuse both.
pub fn parse_filter_both_at_signs_measured(
    text: &str,
    args: &[Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    crate::filter::parse_filter_any(text, args, ty, enums)
}

/// `e`, or the refusal of `@` alone or `<>@` alone on a Code field from a place no cell measures ([`crate::filter::parse_filter_with`]).
pub(crate) fn refused_where_not_measured(e: FilterExpr) -> Result<FilterExpr, String> {
    match e {
        FilterExpr::AtSignAloneCode(negated) => Err(format!(
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

/// What BC answers for a CalcFormula's quoted `filter('@')` or `filter('<>@')` (`text`, as the lowering passes it) over a source
/// field of type `ty` that is neither Code nor Text, with any method (AtSignAloneTypes.al:651-:905; AtSignAloneMethods.al:395, :430,
/// :454), whoever reads the FlowField (CalcFields, SetAutoCalcFields and a Find, a filter on the FlowField, a query column, a
/// temporary host: AtSignAloneMethods.al:657).
#[derive(Debug, PartialEq, Eq)]
pub enum OtherSourceAtSign {
    /// A Date field: `@` keeps no row, `<>@` every row (true) (AtSignAloneTypes.al:651).
    Rows(bool),
    /// Reading the FlowField raises this error, for `@` and `<>@` alike (AtSignAloneTypes.al:673-:905), in a session of a
    /// language [`MEASURED_LANGUAGES`] lists ([`calc_formula_at_sign_other_source`] refuses it in another).
    Error { message: String, code: &'static str },
}

/// The session languages the error texts are measured in: 1033, and 1031 and 1030, where a Business Central W1 container gives the
/// texts of 1033 (AtSignAloneTypes.al:935, :974).
const MEASURED_LANGUAGES: [i32; 3] = [1033, 1031, 1030];

/// [`OtherSourceAtSign`] for `text` over `ty` in a session of `language`, or None when the text is not `@` alone or `<>@` alone or
/// no cell measures the type ([`calc_formula_at_sign_measured`]). `captions` gives the captions of an Option or Enum field's members
/// as the session shows them; it is called only for such a field. An error answer in a language no cell measures is refused before
/// the assumption is marked. The lowering admits the text only where [`calc_formula_at_sign_measured`] holds.
pub fn calc_formula_at_sign_other_source(
    text: &str,
    ty: &crate::value::RtType,
    language: i32,
    captions: impl FnOnce() -> Result<Vec<String>, Signal>,
) -> Result<Option<OtherSourceAtSign>, Signal> {
    let negated = match text {
        "@" => false,
        "<>@" => true,
        _ => return Ok(None),
    };
    let Some(answer) = other_source_answer(negated, ty, captions)? else {
        return Ok(None);
    };
    if matches!(answer, OtherSourceAtSign::Error { .. }) && !MEASURED_LANGUAGES.contains(&language) {
        return unsupported(format!(
            "the error of a CalcFormula filter '{text}' in a session of language {language} (not measured)"
        ));
    }
    assume(Assumption::AtSignAloneDatabaseBlank);
    Ok(Some(answer))
}

/// Whether the cells measure a CalcFormula's quoted `filter('@')` or `filter('<>@')` over a source field of type `ty`, with any
/// method: a Code or a Text field (AtSignAloneReads.al:579, :752; AtSignAloneMethods.al:376), and the types of
/// [`calc_formula_at_sign_other_source`] (AtSignAloneTypes.al:651-:905). The lowering refuses the rest.
pub fn calc_formula_at_sign_measured(ty: &crate::value::RtType) -> bool {
    use crate::value::RtType;
    matches!(ty, RtType::Code(_) | RtType::Text(_))
        || matches!(other_source_answer(false, ty, || Ok(Vec::new())), Ok(Some(_)))
}

fn other_source_answer(
    negated: bool,
    ty: &crate::value::RtType,
    captions: impl FnOnce() -> Result<Vec<String>, Signal>,
) -> Result<Option<OtherSourceAtSign>, Signal> {
    use crate::value::RtType;
    let evaluate = |type_name: &str| OtherSourceAtSign::Error {
        message: format!("The value \"@\" can't be evaluated into type {type_name}."),
        code: "InvalidFilterExpression:Evaluate",
    };
    Ok(Some(match ty {
        RtType::Date => OtherSourceAtSign::Rows(negated),
        RtType::Integer => evaluate("Integer"),
        RtType::BigInteger => evaluate("BigInteger"),
        RtType::Decimal => evaluate("Decimal"),
        RtType::Boolean => evaluate("Boolean"),
        RtType::Time => evaluate("Time"),
        RtType::DateTime => evaluate("DateTime"),
        RtType::Duration => evaluate("Duration"),
        RtType::Guid => OtherSourceAtSign::Error {
            message: "Invalid format of GUID string. The correct format of the GUID string is: \
                      CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits."
                .to_string(),
            code: "InvalidFilterExpression:InvalidGuidFormat",
        },
        // The captions, an OptionCaption's or an Enum value's Caption, the name where there is none (AtSignAloneTypes.al:739,
        // :761, :905); a blank member is listed as it is (AtSignAloneMethods.al:717).
        RtType::Option(_) | RtType::Enum(_) => OtherSourceAtSign::Error {
            message: format!(
                "'@' is not an option. The existing options are: {}",
                captions()?.join(",")
            ),
            code: "InvalidFilterExpression:InvalidOptionString",
        },
        _ => return Ok(None),
    }))
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
