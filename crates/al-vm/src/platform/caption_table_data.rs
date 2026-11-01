//! The TableData rows of AllObjWithCaption (2000000058): one per Table row, with the table's name, caption and subtype.
//!
//! Pinned by corpus metadata/lt-all-obj-namespace/AllObjNamespace.al:170-173 and PW_AllObj_TableDataRowPerTableKind (:199-215:
//! `Caption.Get(TableData, id)` finds a row for an own table, a TableType = Temporary table, a Base Application table and
//! platform tables, with the table's "Object Caption" and "Object Subtype"), and by
//! permissions/lt-y59-metadata-permission/Y59MetadataPermissionNames.al:146-156, :257-266 (corpus master 9cf26907): the Lookup
//! FlowFields "Object Name" and "AL Object Name" of a Metadata Permission row of Object Type Table Data read the table's
//! caption (its name without a Caption) and its name from this row. The package ids and the namespace (60, 61, 63) are the
//! Table row's, as in AllObj, whose TableData rows carry the Table row's name, namespace and package ids (AllObjNamespace.al
//! UG_AllObj_TableDataRow_OfContact_AllColumns and PW_AllObj_TableDataRowPerTableKind `samePackage`); "App ID" (62) is not
//! pinned for a TableData row: unknown.

use super::mat::Entry;
use super::{option, pos, unknown_why};
use al_builtins::value::Value;
use al_data::TableDef;

/// `entries` (the AllObjWithCaption rows of the objects) with a TableData row after each Table row.
pub(super) fn with_table_data_rows(t: &TableDef, entries: Vec<Entry>) -> Vec<Entry> {
    let ty = pos(t, 1);
    let mut out = Vec::with_capacity(entries.len() * 2);
    for e in entries {
        let table = matches!(e.row[ty], Value::Option(1, _));
        let data = table.then(|| {
            let mut row = e.row.clone();
            row[ty] = option(t, 1, 0);
            match &e.poison {
                Some(why) => Entry::poisoned(row, why.clone()),
                None => {
                    row[pos(t, 62)] = Value::Unknown(unknown_why(t, 62, "a TableData row"));
                    Entry::ok(row)
                }
            }
        });
        out.push(e);
        out.extend(data);
    }
    out
}
