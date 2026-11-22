//! What the VM asks of a table object on every record operation, worked out once when the engine is built (ADR-035
//! decision 4).

use crate::blob_transfer::FieldKind;
use al_builtins::value::RtType;
use al_ir as ir;

/// Facts about a table object that never change during a run.
#[derive(Debug, Clone, Default)]
pub(crate) struct TableFacts {
    /// Some field has an `InitValue` text that did not evaluate at compile time.
    pub(crate) unresolved_init: bool,
    /// The positions of the BigInteger fields with `SQLTimestamp = true`: they read the row version.
    pub(crate) sql_timestamp: Vec<usize>,
    /// Some Code field declares an `SQLDataType` other than `Varchar`: a write checks its value.
    pub(crate) code_sql_type: bool,
    /// The table has a Blob field or an AutoIncrement field: its Inserts are not queued (`records/insert_buffer.rs`).
    pub(crate) insert_unqueued: bool,
    /// The positions of the Blob fields, ascending, each with its kind (`blob_transfer::FieldKind::of`).
    pub(crate) blob_fields: Box<[(usize, FieldKind)]>,
}

/// The facts of each object of `heads`, by object index; an object that is no table has the default ones.
pub(crate) fn of_heads(heads: &[ir::ObjectHead]) -> Vec<TableFacts> {
    heads
        .iter()
        .enumerate()
        .map(|(i, o)| TableFacts {
            blob_fields: blob_fields(heads, i),
            unresolved_init: o
                .table
                .as_ref()
                .is_some_and(|t| t.fields.iter().any(|f| f.init_text.is_some() && f.init.is_none())),
            sql_timestamp: o.table.as_ref().map_or_else(Vec::new, |t| {
                t.fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| {
                        f.properties.iter().any(|(k, v)| {
                            k.eq_ignore_ascii_case("SQLTimestamp") && v.trim().eq_ignore_ascii_case("true")
                        })
                    })
                    .map(|(i, _)| i)
                    .collect()
            }),
            insert_unqueued: o.table.as_ref().is_none_or(|t| {
                t.fields.iter().any(|f| {
                    f.auto_increment || matches!(f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob))
                })
            }),
            code_sql_type: o.table.as_ref().is_some_and(|t| {
                t.fields.iter().any(|f| {
                    matches!(f.ty, RtType::Code(_))
                        && f.properties.iter().any(|(k, v)| {
                            k.eq_ignore_ascii_case("SQLDataType") && !v.trim().eq_ignore_ascii_case("Varchar")
                        })
                })
            }),
        })
        .collect()
}

/// The Blob fields of table object `table` (`TableFacts::blob_fields`).
fn blob_fields(heads: &[ir::ObjectHead], table: usize) -> Box<[(usize, FieldKind)]> {
    let Some(t) = heads[table].table.as_ref() else {
        return Box::default();
    };
    t.fields
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)))
        .map(|(pos, f)| (pos, FieldKind::of(heads, table, f)))
        .collect()
}
