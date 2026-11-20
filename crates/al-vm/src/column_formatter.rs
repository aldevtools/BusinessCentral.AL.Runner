//! The `decimalformatter` of a Decimal report column (the `<Name>Format` column of a TestRequestPage.SaveAsXml data file and
//! the attribute of a Report.SaveAs dataset).

use crate::dataset;
use crate::{Session, Signal};
use std::sync::Arc;

/// The formatter of a Decimal column from its own `DecimalPlaces`, `AutoFormatType` and `AutoFormatExpression`, else the
/// `DecimalPlaces` of its field (`field_places`). Pinned by al-language-onprem/reports/dataset/
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
/// A type the runtime does not model at all (`pagefmt::unmodelled_auto_format_type`) is refused.
pub(crate) fn column_formatter(
    places: Option<&str>,
    auto_type: Option<&str>,
    auto_expr: Option<&str>,
    field_places: Option<&str>,
) -> Result<String, Signal> {
    if let Some(why) = auto_type.and_then(al_builtins::pagefmt::unmodelled_auto_format_type) {
        return Err(Signal::unsupported(format!(
            "the decimalformatter of a report column with {why}"
        )));
    }
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

/// Property `name` of field `pos` of table object `table` (`ReportColumn::places_table`, `places_field`).
fn field_property(program: &al_ir::Program, (table, pos): (usize, usize), name: &str) -> Option<String> {
    let field = program.objects.get(table)?.table.as_ref()?.fields.get(pos)?;
    field
        .properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

impl Session {
    /// The `decimalformatter` of a Decimal column: from `DecimalPlaces` of the column, else of the field (`dataset::decimal_formatter`).
    /// Without either it is `#,##0.00` on every version (reports/runtime/YReportPropsTests.al:40, 1234567.891;
    /// reports/W3ReportObjectIdPreviewLanguage.al:448). BC follows General Ledger Setup "Amount Decimal Places" as the
    /// session first read it; the corpus tests read it at the default 2:2 after SelectLatestVersion (corpus bump 28: the
    /// earlier BC 28 `#,##0.00###` was a value left by an earlier test).
    ///
    /// A column that states no AutoFormatType takes its field's: a type the runtime does not model is refused (al-runner issue
    /// 8011); a modelled one keeps the rule above. The column's type, else its field's, is refused while a subscriber could write
    /// the AutoFormat result (`auto_format_writer`).
    pub(crate) fn decimal_column_formatter(
        &self,
        t: &Option<Arc<al_data::TableDef>>,
        c: &al_ir::ReportColumn,
    ) -> Result<String, Signal> {
        let from_field = |name: &str| match (t, c.field, c.places_table.zip(c.places_field)) {
            (Some(t), Some(pos), _) => self.field_property(t, pos, name),
            (_, None, Some(at)) => field_property(&self.engine.program, at, name),
            _ => None,
        };
        let field_type = from_field("AutoFormatType");
        if c.auto_format_type.is_none() {
            if let Some(why) = field_type
                .as_deref()
                .and_then(al_builtins::pagefmt::unmodelled_auto_format_type)
            {
                return Err(Signal::unsupported(format!(
                    "the decimalformatter of a report column over a field with {why}"
                )));
            }
        }
        if let Some(why) = c
            .auto_format_type
            .as_deref()
            .or(field_type.as_deref())
            .and_then(|k| self.auto_format_writer_refusal(k, false))
        {
            return Err(Signal::unsupported(format!(
                "the decimalformatter of a report column: {why}"
            )));
        }
        of_column(c, from_field("DecimalPlaces").as_deref())
    }
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

    /// A column whose AutoFormatType the runtime does not model is refused (al-runner issue 8011), DecimalPlaces beside it or not.
    #[test]
    fn an_unmodelled_type_is_refused() {
        for p in [None, Some("2 : 4")] {
            let e = format!("{:?}", column_formatter(p, Some("50733"), None, None));
            assert!(e.contains("Unsupported") && e.contains("AutoFormatType 50733"), "{e}");
        }
    }
}
