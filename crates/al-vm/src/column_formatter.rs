//! The `decimalformatter` of a Decimal report column (the `<Name>Format` column of a TestRequestPage.SaveAsXml data file and
//! the attribute of a Report.SaveAs dataset).

use crate::Signal;
use crate::dataset;

/// The formatter of a Decimal column from its own `DecimalPlaces`, `AutoFormatType` and `AutoFormatExpression`, else the
/// `DecimalPlaces` of its field (`field_places`). Pinned by al-language-onprem/report/lt-y67-saveasxml-columns/
/// Y67SaveAsXmlColumns.al:370 Columns_DecimalFormatters (BC 27.5, 28.5, 29.0):
/// - DecimalPlaces of the column or the field (`dataset::decimal_formatter`): `0 : 0` `#,##0`, `1` `#,##0.0`, `2 : 4`
///   `#,##0.00##`, `0 : 5` `#,##0.#####`, `3` on an expression `#,##0.000`;
/// - AutoFormatType 0 and 1 without an expression: `#,##0.00`; 2 (a unit amount) without one: `#,##0.00###`;
/// - AutoFormatType 10 with the expression `<Precision,a:b>..`: the places `a:b` (`<Precision,3:3><Standard Format,0>` gives
///   `#,##0.000`).
///
/// Every other combination (DecimalPlaces beside AutoFormatType, a currency expression, other types) keeps the rule from before
/// Y67: the AutoFormat properties are taken to leave the formatter of DecimalPlaces (docs/probes-needed.md, "Report", the
/// `decimalformatter` row; the Base Application states AutoFormatType with a currency expression on most decimal columns).
pub(crate) fn column_formatter(
    places: Option<&str>,
    auto_type: Option<&str>,
    auto_expr: Option<&str>,
    field_places: Option<&str>,
) -> Result<String, Signal> {
    if places.is_none() {
        let expr = auto_expr.map(unquote).unwrap_or_default();
        match (auto_type.map(str::trim), expr.trim()) {
            (Some("2"), "") => return dataset::decimal_formatter(Some("2 : 5")),
            (Some("10"), e) => {
                if let Some(p) = precision(e) {
                    return dataset::decimal_formatter(Some(&p));
                }
            }
            _ => {}
        }
    }
    dataset::decimal_formatter(places.or(field_places))
}

/// `column_formatter` of the properties of column `c`.
pub(crate) fn of_column(c: &al_ir::ReportColumn, field_places: Option<&str>) -> Result<String, Signal> {
    let (places, kind, expr) = (
        c.decimal_places.as_deref(),
        c.auto_format_type.as_deref(),
        c.auto_format_expression.as_deref(),
    );
    column_formatter(places, kind, expr, field_places)
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('\'')
        .and_then(|r| r.strip_suffix('\''))
        .unwrap_or(v)
        .replace("''", "'")
}

/// `a:b` of an expression that starts with `<Precision,a:b>`.
fn precision(e: &str) -> Option<String> {
    let rest = e.strip_prefix("<Precision,")?;
    let places = &rest[..rest.find('>')?];
    let (a, b) = places.split_once(':')?;
    (a.trim().parse::<u32>().is_ok() && b.trim().parse::<u32>().is_ok()).then(|| places.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_y67_column_properties_give_their_formatters() {
        let f = |p: Option<&str>, t: Option<&str>, e: Option<&str>, fp: Option<&str>| {
            column_formatter(p, t, e, fp).unwrap()
        };
        assert_eq!(f(None, None, None, Some("2 : 2")), "#,##0.00");
        assert_eq!(f(Some("0 : 0"), None, None, Some("2 : 2")), "#,##0");
        assert_eq!(f(Some("1"), None, None, None), "#,##0.0");
        assert_eq!(f(Some("2 : 4"), None, None, None), "#,##0.00##");
        assert_eq!(f(None, None, None, Some("0 : 5")), "#,##0.#####");
        assert_eq!(f(None, Some("1"), None, Some("2 : 2")), "#,##0.00");
        assert_eq!(f(None, Some("2"), None, Some("2 : 2")), "#,##0.00###");
        assert_eq!(f(None, Some("0"), None, None), "#,##0.00");
        assert_eq!(
            f(None, Some("10"), Some("'<Precision,3:3><Standard Format,0>'"), None),
            "#,##0.000"
        );
        assert_eq!(f(Some("3"), None, None, None), "#,##0.000");
    }

    #[test]
    fn other_shapes_keep_the_decimal_places_rule() {
        let f = |p: Option<&str>, t: Option<&str>, e: Option<&str>| column_formatter(p, t, e, None).unwrap();
        assert_eq!(f(Some("2 : 4"), Some("2"), None), "#,##0.00##");
        assert_eq!(f(None, Some("1"), Some("'GBP'")), "#,##0.00");
        assert_eq!(f(None, Some("10"), Some("'x'")), "#,##0.00");
    }
}
