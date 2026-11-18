//! App Package ID and App Runtime Package ID of a Field row that a key read finds (al-runner issue 8051 and its follow-ups): a
//! read that can find only the row of one key (TableNo, No.) in a form the probes measured is answered as a `Get`, not as a
//! scan. Every pin is in tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al (cited by line below).

use super::fields::declaring_extension;
use super::objects::Catalog;
use super::{Providers, pos};
use crate::Engine;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::platform::Virtual;
use al_builtins::value::Value;
use al_data::{Row, Scan, StoreError, TableDef};
use al_ir::ObjectHead as Object;

/// What a key lookup gives the two package columns of a Field row whose field the program compiled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeyIds {
    /// Null ids, where a scan gives the table's: a field the table declares, of a table no extension of its own app extends
    /// (:569-605 FieldTable_AppPackageIds_KeyLookupOfOwnField_NullUnlessSameAppTableExtension, tables of this app, of Base
    /// Application and of another app; :516-532 System Application's tables; :784-815 every kind of field and a temporary table).
    Null,
    /// The ids a scan gives, on a Get too: a system field 2000000000 to 2000000004 (:618-646) and a field another app's
    /// extension adds (:360-376).
    AsScan,
    /// The ids a scan gives, on a Get too: a field the table declares, of a table that extensions of its own app of a measured
    /// kind extend ([`measured_own_extension`]; :569-605 tables 71397 and Customer, :784-815 table 71401).
    OwnExtended,
    /// Not known: an extension of the table's own app of a kind no probe measured extends the table.
    Unknown,
}

/// The rule of [`KeyIds`] for Field row `r`; `None` for a row of no compiled field and for a field of a platform table.
fn key_ids(c: &Catalog, t: &TableDef, r: &[Value]) -> Option<KeyIds> {
    let (Value::Integer(tid), Value::Integer(no)) = (&r[pos(t, 1)], &r[pos(t, 2)]) else {
        return None;
    };
    let table = c.table_by_id(*tid as u32)?;
    let field = table.table.as_ref()?.fields.iter().find(|f| f.id == *no as u32)?;
    if declaring_extension(c, table, field).is_some() || (2_000_000_000..=2_000_000_004).contains(&field.id) {
        return Some(KeyIds::AsScan);
    }
    if super::objects::platform_object(table) {
        return None;
    }
    let own = c.own_app_extensions(*tid as u32);
    Some(if own.is_empty() {
        KeyIds::Null
    } else if own.iter().all(|&i| measured_own_extension(c, table, i)) {
        KeyIds::OwnExtended
    } else {
        KeyIds::Unknown
    })
}

/// Whether table extension `i` of the table's own app is of a kind the probes measured: it declares a field without an
/// ObsoleteState (extension 71398 on table 71397, :934, and Base Application's extension of Customer, read at :569-605), or it
/// declares no field and every one of its `modify` blocks sets Caption only (extension 71402 on table 71401, :1011, read at
/// :784-815). An extension whose fields all carry an ObsoleteState, one with neither a field nor a modify, one whose modify sets
/// another property and one that was not compiled are not measured.
fn measured_own_extension(c: &Catalog, table: &Object, i: usize) -> bool {
    let fields = table.table.as_ref().map_or(&[][..], |t| &t.fields[..]);
    let mut declared = fields.iter().filter(|f| f.declared_in == Some(i as u32)).peekable();
    if declared.peek().is_some() {
        return declared.any(|f| {
            !f.properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("ObsoleteState"))
        });
    }
    c.full.get(i).is_some_and(|ext| {
        !ext.uncompiled
            && !ext.modified_fields.is_empty()
            && ext
                .modified_fields
                .iter()
                .all(|(_, props)| props.len() == 1 && props[0].eq_ignore_ascii_case("Caption"))
    })
}

/// The positions of App Package ID and App Runtime Package ID in the Field table.
fn package_columns(t: &TableDef) -> Option<(usize, usize)> {
    let col = |name: &str| t.fields.iter().position(|x| x.name == name);
    Some((col("App Package ID")?, col("App Runtime Package ID")?))
}

/// Leaves both package columns of `r` unknown, so that only a read of them is refused.
fn forget_ids(t: &TableDef, r: &mut [Value], why: String) {
    if let Some((pc, rt)) = package_columns(t) {
        let why = Value::Unknown(why.into());
        r[pc] = why.clone();
        r[rt] = why;
    }
}

/// The package ids of a Field row a key lookup finds: a `Get` (and `Find('=')`, :753-772), or a scan in a form
/// [`measured_key_scan`] accepts ([`KeyIds`]). A row of no compiled field and a field of a platform table keep the earlier rule:
/// null on a Get where the two ids are equal (:287-311), the scan's ids on a key scan.
pub(super) fn key_lookup(p: &Providers, t: &TableDef, mut r: Row, key_scan: bool) -> Row {
    let Some((pc, rt)) = package_columns(t) else {
        return r;
    };
    let (Value::Guid(a), Value::Guid(b)) = (&r[pc], &r[rt]) else {
        return r;
    };
    let engine = p.engine();
    let null = match key_ids(&Catalog::new(&engine), t, &r) {
        Some(KeyIds::Null) => true,
        Some(KeyIds::AsScan | KeyIds::OwnExtended) => false,
        Some(KeyIds::Unknown) => {
            forget_ids(
                t,
                &mut r,
                "the package ids of a Field row a key read finds are not measured for a table that an extension of its own app \
                 extends which declares no field without an ObsoleteState and does not only modify Captions"
                    .into(),
            );
            return r;
        }
        None => !key_scan && a == b,
    };
    if null {
        r[pc] = Value::Guid([0; 16]);
        r[rt] = Value::Guid([0; 16]);
    }
    r
}

/// Field row `r`, which a key read of a form no probe measured found (`form` names it): where a `Get` gives other ids than a
/// scan ([`KeyIds::Null`]) or ids no probe measured, both package columns are left unknown. The other rows keep their ids, on
/// which a Get and a scan agree.
pub(crate) fn unmeasured_key_read(engine: &Engine, t: &TableDef, r: &mut [Value], form: &str) {
    if engine.program.objects.heads()[t.object as usize].id != Some(2_000_000_041) {
        return;
    }
    if matches!(
        key_ids(&Catalog::new(engine), t, r),
        Some(KeyIds::Null | KeyIds::Unknown)
    ) {
        forget_ids(
            t,
            r,
            format!(
                "the package ids of a Field row read by {form} are not measured: BC answers a read of one key as a Get or as a \
                 scan by the read's form, and they differ for this row"
            ),
        );
    }
}

/// The row a read found (`method`; for a Find, `which` its direction and `args` what the call named): Find('=') of a Field
/// row answers as a Get (:753-772, `find=`); Find() and a Find of two or three direction characters with `=` are not measured
/// and go to [`unmeasured_key_read`], also where `<` or `>` found the row.
pub(crate) fn unmeasured_find(
    engine: &Engine,
    t: &TableDef,
    found: &mut Option<Row>,
    method: al_builtins::record_op::RecordOp,
    which: &str,
    args: &[Value],
) {
    let form = match which {
        "=" if args.is_empty() => "Find()".to_string(),
        w if w.len() >= 2 && w.contains('=') => format!("Find('{w}')"),
        _ => return,
    };
    if let (al_builtins::record_op::RecordOp::Find, Some(row)) = (method, found) {
        unmeasured_key_read(engine, t, row, &form);
    }
}

/// The (TableNo, No.) a scan's filters set to one value each, outside the cross-column OR group: a scan that can find only the
/// row of that key.
pub(super) fn key_scan(t: &TableDef, filters: &[FieldFilter]) -> Option<(Value, Value)> {
    let one = |id: u32| {
        filters
            .iter()
            .filter(|f| f.field == pos(t, id) && f.group != al_builtins::filter::OR_GROUP)
            .find_map(|f| super::mat::single_value_of(&f.expr))
    };
    Some((one(1)?, one(2)?))
}

/// The filter group of the one single-value filter on Field column `id` among `filters`, if there is exactly one filter on it.
fn single_value_group(t: &TableDef, filters: &[FieldFilter], id: u32) -> Option<i32> {
    match filters.iter().filter(|f| f.field == pos(t, id)).collect::<Vec<_>>()[..] {
        [f] if super::mat::single_value_of(&f.expr).is_some() => Some(f.group),
        _ => None,
    }
}

/// Whether `scan` starts at the start: a Find, or a Next from a blank record, whose position is the blank key (TableNo 0, No. 0).
fn from_start(scan: &Scan) -> bool {
    scan.after
        .is_none_or(|a| a.iter().all(|v| matches!(v, Value::Integer(0))))
}

/// Whether `scan` is a key read the probes measured, which BC answers as a `Get` ([`key_lookup`]): a read in primary key order,
/// either direction, from the start (FindFirst, FindSet, FindLast, Find('-'), Find('+'), Next from a blank record,
/// RecordRef.FindFirst), whose filters are one single-value filter each on TableNo and No. in filter group 0, optionally with a
/// single-value filter on Class in group 0; the Class filter and the group 2 filters (both in group 2, or TableNo in group 2 and
/// No. in group 0) only on an ascending read without a position (FindFirst, FindSet, Find('-')) (:569-605, the ascending
/// FindFirst and FindSet; :662-737, helper ObserveReadForms, read at :753-772, the other forms). Every other read whose filters
/// set the key goes to [`unmeasured_key_read`].
fn measured_key_scan(t: &TableDef, scan: &Scan) -> bool {
    if scan.key != 0 || !from_start(scan) {
        return false;
    }
    let groups = (
        single_value_group(t, scan.filters, 1),
        single_value_group(t, scan.filters, 2),
    );
    // The Class filter and the filters in group 2 are measured on a FindFirst only (:662-737, ObserveReadForms): ascending, no
    // position.
    let find_first = scan.ascending && scan.after.is_none();
    match (groups, scan.filters.len()) {
        ((Some(0), Some(0)), 2) => true,
        ((Some(0), Some(0)), 3) => find_first && single_value_group(t, scan.filters, 7) == Some(0),
        ((Some(2), Some(2)) | (Some(2), Some(0)), 2) => find_first,
        _ => false,
    }
}

/// Whether `scan` is the key read with a package filter the probe measured: a FindFirst or FindSet (ascending, primary key, no
/// position) whose filters are one single-value filter each on TableNo and No. and one `=` or `<>` filter on App Package ID of
/// the table's id (the id the listing's row `keyed` holds) or the empty id, all in filter group 0 (:827-842, helper
/// ObservePackageFilters, read at :844-868).
fn measured_key_package_scan(t: &TableDef, scan: &Scan, keyed: &[Value]) -> bool {
    let Some((pc, _)) = package_columns(t) else {
        return false;
    };
    let measured_value = |v: &Value| match (v, &keyed[pc]) {
        (Value::Guid(g), Value::Guid(table)) => g == table || *g == [0; 16],
        _ => false,
    };
    let package = scan.filters.iter().filter(|f| f.field == pc).collect::<Vec<_>>();
    scan.key == 0
        && scan.ascending
        && scan.after.is_none()
        && scan.filters.len() == 3
        && single_value_group(t, scan.filters, 1) == Some(0)
        && single_value_group(t, scan.filters, 2) == Some(0)
        && matches!(package[..], [f] if f.group == 0
        && match &f.expr {
            FilterExpr::Ne(v) => measured_value(v),
            e => super::mat::single_value_of(e).is_some_and(|v| measured_value(&v)),
        })
}

/// Whether the filters read App Package ID or App Runtime Package ID.
fn filters_package(t: &TableDef, filters: &[FieldFilter]) -> bool {
    filters.iter().any(|f| {
        matches!(
            t.fields[f.field].name.as_str(),
            "App Package ID" | "App Runtime Package ID"
        )
    })
}

impl Providers {
    /// A scan of the Field table whose filters set TableNo and No. to one value each ([`key_scan`]): in a shape the probes
    /// measured, BC answers it as a `Get` ([`measured_key_scan`], [`key_lookup`]); in any other shape the row goes to
    /// [`unmeasured_key_read`]. With a filter on a package column too, of a row a Get gives other ids than a scan or ids no probe
    /// measured, or of a declared field of a table extensions of its own app extend: in the measured shape the filter is tested
    /// against the ids of the Get ([`measured_key_package_scan`]); a read after the one row the key filters leave finds nothing;
    /// every other shape is refused. Of the other rows the Get and the scan agree, and the scan tests the filter.
    pub(super) fn field_key_scan(&self, t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
        let listed = self.listed(t, Virtual::Field)?;
        let engine = self.engine();
        let c = Catalog::new(&engine);
        if filters_package(t, scan.filters)
            && let Some((table_no, no)) = key_scan(t, scan.filters)
            && let Some(keyed) = listed.get(t, &[table_no.clone(), no.clone()])?
        {
            match key_ids(&c, t, &keyed) {
                Some(KeyIds::AsScan) => {}
                Some(KeyIds::Null | KeyIds::OwnExtended) if measured_key_package_scan(t, scan, &keyed) => {
                    let row = key_lookup(self, t, keyed, true);
                    return Ok(al_builtins::filter::row_passes_checked(&row, scan.filters).then_some(row));
                }
                _ if scan.key == 0
                    && scan.after.is_some_and(|a| {
                        al_builtins::filter::key_cmp(a.get(..2).unwrap_or(a), &[table_no, no])
                            == std::cmp::Ordering::Equal
                    }) =>
                {
                    return Ok(None);
                }
                _ => {
                    return Err(StoreError::Refused(
                        "Field: a read that sets TableNo and No. to one value each and filters on App Package ID or App Runtime Package ID is measured only as a FindFirst or FindSet with an = or <> filter on App Package ID of the table's id or the empty id, all in filter group 0, on a table whose own app's extensions are of a measured kind".into(),
                    ));
                }
            }
        }
        let row = listed.next(t, scan)?;
        Ok(row.map(|mut row| {
            if measured_key_scan(t, scan) {
                key_lookup(self, t, row, true)
            } else {
                unmeasured_key_read(
                    &engine,
                    t,
                    &mut row,
                    "a read whose filters set TableNo and No. to one value each in a form no corpus test measures",
                );
                row
            }
        }))
    }
}
