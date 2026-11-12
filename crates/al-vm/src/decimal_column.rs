//! The text of a Decimal in the report dataset: which columns write it without trailing zeros (al-runner issue 7440). A row
//! read from the database carries no trailing zeros (al-data `MemStore::as_database`), and `Round` adds none beyond its
//! input (al-builtins `mathfns.rs`); every other column writes the value with its scale.
//!
//! Pinned (reports/dataset/DatasetDecimalPlacesValueOrigins.al, every value origin of :9-10 gives the same text):
//! - a stored field of a data item over a table, by name (:47 `D05`) or with the item's name (:52 `Row.D05`): `1`, not
//!   `1.00000` (:241-247);
//! - a stored field of a global Record variable (:71 `RecLiteral.D05`, never stored): `1` (:364);
//! - a global Decimal variable (:79 `VarLiteral`): `1` (:397);
//! - an expression over a stored field of the data item's row (:53 `D05 * 1`): `1`, because the row is read without trailing
//!   zeros (:241-247); the arithmetic keeps its scale (reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:309
//!   `Dec * 1.5` over the stored 12.345678 is `18.5185170`; DatasetDecimalPlacesValueOrigins.al:505-510 `G.D * 2` over the
//!   stored 1.50 read by Get is `3.0`).
//!
//! Every other shape writes the value with its scale (:425-570: a variable times one, an array element, unary minus,
//! parentheses, procedure and codeunit calls, a field set in a trigger, a FlowField and a temporary data item's field all
//! write what the value holds).

use al_builtins::value::RtType;
use al_ir::{Expr, Place, Program, ReportColumn, SourceSyntax, Stmt};
use rust_decimal::Decimal;

/// The dataset text of Decimal `d` in column `c` of report `object`.
pub(crate) fn text(program: &Program, object: usize, c: &ReportColumn, d: Decimal) -> String {
    if trims(program, object, c) {
        d.normalize().to_string()
    } else {
        d.to_string()
    }
}

/// Whether column `c` has one of the pinned source shapes that write a Decimal without trailing zeros: the syntax (one name,
/// or two joined by a dot) and the lowered routine (`exit` of a load of that place) both say so.
fn trims(program: &Program, object: usize, c: &ReportColumn) -> bool {
    let Some(o) = program.objects.get(object) else {
        return false;
    };
    let (Some(report), Some(routine)) = (o.report.as_ref(), c.routine.and_then(|r| o.routines.get(r))) else {
        return false;
    };
    let item_slots: Vec<usize> = report.items.iter().filter_map(|i| i.slot).collect();
    let own = report
        .items
        .iter()
        .find(|i| i.columns.iter().any(|x| x.routine == c.routine))
        .and_then(|i| i.slot);
    let slot_ty = |g: usize| o.globals.get(g).map(|s| &s.ty);
    routine.body.read(|stmts| {
        let [Stmt::Exit(Some(e))] = stmts else {
            return false;
        };
        let Expr::Load(place) = peel(e) else {
            return false;
        };
        match (c.source_syntax, place) {
            // A global Decimal variable (:79).
            (SourceSyntax::Name, Place::Global(g)) => {
                !item_slots.contains(g) && matches!(slot_ty(*g), Some(RtType::Decimal))
            }
            (SourceSyntax::Name | SourceSyntax::Member, Place::Field(rec, pos)) => {
                let Place::Global(g) = rec.as_ref() else {
                    return false;
                };
                // By name only the own item's field (:47); with a dot the own item (:52) or a global Record variable (:71),
                // not another data item.
                let whose = Some(*g) == own || (c.source_syntax == SourceSyntax::Member && !item_slots.contains(g));
                let Some(RtType::Record(table, false)) = slot_ty(*g) else {
                    return false;
                };
                whose && stored_decimal(program, *table as usize, *pos)
            }
            _ => false,
        }
    })
}

/// A conversion to the routine's Variant result is not part of the source.
fn peel(e: &Expr) -> &Expr {
    match e {
        Expr::Convert(_, inner) => peel(inner),
        e => e,
    }
}

/// Field `pos` of table object `table` is a stored (Normal) Decimal field.
fn stored_decimal(program: &Program, table: usize, pos: usize) -> bool {
    program
        .objects
        .get(table)
        .and_then(|o| o.table.as_ref())
        .and_then(|t| t.fields.get(pos))
        .is_some_and(|f| f.stored && f.ty == RtType::Decimal)
}
