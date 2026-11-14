//! The text of a Decimal in the report dataset (al-runner issues 7440 and 7958). The rule, the decimals a Decimal shows, is
//! owned by al-builtins `decimal_scale.rs`, which `Round` reads too; this module writes it and marks the dataset's measured,
//! not yet pinned, shapes (assumption `dataset-decimal-keeps-its-zeros`). A row read from the database carries no trailing
//! zeros (al-data `MemStore::as_database`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::decimal_scale;
use al_ir::{ReportColumn, SourceSyntax};
use rust_decimal::Decimal;

/// The dataset text of Decimal `d` in column `c`: what `decimal_scale::shown` gives. Measured, not pinned: zero written without
/// its zeros, one and minus one without theirs from an expression (`Half + Half` is `1`; pinned only for a name or two names,
/// reports/dataset/DatasetDecimalPlacesValueOrigins.al:364, :397), and the zeros of a variable or a field named as the source.
pub(crate) fn text(c: &ReportColumn, d: Decimal) -> String {
    let shown = decimal_scale::shown(d);
    let named = matches!(c.source_syntax, SourceSyntax::Name | SourceSyntax::Member);
    let trimmed = shown.scale() != d.scale();
    if (trimmed && (d.is_zero() || !named)) || (named && decimal_scale::shows_zeros(d)) {
        assume(Assumption::DatasetDecimalKeepsItsZeros);
    }
    shown.to_string()
}
