//! The text of a Decimal in the report dataset (al-runner issues 7440 and 7958): the value with the decimals it holds, zeros
//! included, except zero, one and minus one, which are written without decimals.
//!
//! Pinned (reports/dataset/DatasetDecimalPlacesValueOrigins.al): one is `1` whatever made it and whatever the column's
//! source: a stored field by name or with the item's name (:241-247), a field of a global Record variable set to the literal
//! 1.00000, to `0.5 * 2` or by Evaluate of `1.00000` (:364), a global Decimal variable set the same ways (:397), a variable
//! times one, an array element, parentheses, calls and a temporary data item's field over `Round(1, 0.00001)` (:425-570);
//! minus one is `-1` (`-V`, :525-530). Every other value keeps its decimals: an expression column keeps the scale of the
//! calculation (reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:309 `Dec * 1.5` over the stored 12.345678 is
//! `18.5185170`; DatasetDecimalPlacesValueOrigins.al:505-510 `G.D * 2` over the stored 1.50 read by Get is `3.0`). A row
//! read from the database carries no trailing zeros (al-data `MemStore::as_database`).
//!
//! Measured on BC 27.5, 28.5 and 29.0, not pinned yet (assumption `dataset-decimal-keeps-its-zeros`; corpus pull requests 1380,
//! reports/dataset/DatasetDecimalRoundScale.al, and 1377, reports/dataset/DatasetDecimalRoundScaleSources.al): a variable or a
//! field by name keeps the zeros of any other value, with or without the column's own DecimalPlaces (`Round(3, 0.25)` is
//! `3.00`, the literal 2.50 is `2.50`), and zero is `0` (the literal 0.00, `Round(0.001, 0.01)`). One keeps its decimals in
//! the value itself: `0.50 * 2` times 2.50 is `2.5000`.

use al_builtins::assumptions::{Assumption, assume};
use al_ir::{ReportColumn, SourceSyntax};
use rust_decimal::Decimal;

/// The dataset text of Decimal `d` in column `c`.
pub(crate) fn text(c: &ReportColumn, d: Decimal) -> String {
    let trimmed = d.normalize();
    if trimmed.scale() == d.scale() {
        return d.to_string();
    }
    if d.is_zero() {
        assume(Assumption::DatasetDecimalKeepsItsZeros);
        return trimmed.to_string();
    }
    if d.abs() == Decimal::ONE {
        return trimmed.to_string();
    }
    if matches!(c.source_syntax, SourceSyntax::Name | SourceSyntax::Member) {
        assume(Assumption::DatasetDecimalKeepsItsZeros);
    }
    d.to_string()
}
