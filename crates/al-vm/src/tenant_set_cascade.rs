//! The platform's delete of a stored Tenant Permission Set row (2000000165). It runs under the assumption
//! `tenant-permission-set-delete-cascade`; the values come from corpus probe permissions/permission-sets/TenantSetDeleteCascade.al,
//! observed on a local BC 27.5 and 28.5 (al-runner issue 7854):
//! - A set whose App ID is zero, deleted by Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete or
//!   RecordRef.DeleteAll: its Tenant Permission rows (2000000166, App ID zero and the same Role ID) go, and so do the Tenant
//!   Permission Set Rel. rows (2000000253) of the set and the ones of another set that include it (TSDC_Delete ... TSDC_RecRefDeleteAll,
//!   `perms gone=0 ... rel of gone=0 rel of kept=0`). The measured rows: permissions of Object Type Table Data, relations with App
//!   ID and Related App ID zero, Related Scope Tenant, Type Include.
//! - A set whose App ID is not zero, deleted by DeleteAll: its permissions stay (TSDC_DeleteAllOtherApp, `perms of that app left=2`).
//!
//! Not measured, refused: another kind of permission or relation row, the delete of a set whose App ID is not zero by anything but
//! DeleteAll, and a delete event or database trigger on Tenant Permission or Tenant Permission Set Rel. (BC runs the Tenant
//! Permission OnAfterDeleteEvent for each row, TSDC_DeleteAllPermSubscriber, with arguments and an order not measured), and an
//! OnAfterDeleteEvent subscriber on the set (the rows go in the store after the set row's delete and its events, an order not
//! measured).

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::platform::{TENANT_PERMISSION, TENANT_PERMISSION_SET, TENANT_PERMISSION_SET_REL, pos};
use crate::records::Rec;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{Row, Store, TableDef};

// Field ids of App ID and Role ID in the set table.
const F_APP_ID: u32 = 1;
const F_ROLE_ID: u32 = 2;
// Object Type Table Data; Related Scope Tenant; Type Include.
const TABLE_DATA: i32 = 0;
const SCOPE_TENANT: i32 = 1;
const TYPE_INCLUDE: i32 = 0;

fn is_zero(v: &Value) -> bool {
    matches!(v, Value::Guid(g) if *g == [0; 16])
}

fn ordinal(v: &Value) -> Option<i32> {
    match v {
        Value::Option(n, _) | Value::Enum(_, n) | Value::Integer(n) => Some(*n),
        _ => None,
    }
}

/// The positions of the named fields of `t`, all of them or none.
fn columns<const N: usize>(t: &TableDef, names: [&str; N]) -> Option<[usize; N]> {
    let mut out = [0; N];
    for (o, n) in out.iter_mut().zip(names) {
        *o = t.fields.iter().position(|f| f.name == n)?;
    }
    Some(out)
}

fn same_text(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) => x.as_str() == y.as_str(),
        _ => false,
    }
}

impl Session {
    /// Whether a row of `t` is a row of the stored Tenant Permission Set table.
    pub(crate) fn is_tenant_set_row(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.table_id(t) == Some(TENANT_PERMISSION_SET)
    }

    /// After the set row `rec` is deleted (`delete_all` says whether a DeleteAll deleted it): see the module.
    pub(crate) fn tenant_set_after_delete(&mut self, t: &TableDef, rec: &Rec, delete_all: bool) -> Result<(), Signal> {
        let (app, role) = {
            let r = rec.borrow();
            (r.fields[pos(t, F_APP_ID)].clone(), r.fields[pos(t, F_ROLE_ID)].clone())
        };
        let (Some(tp), Some(rel)) = (
            self.table_def(TENANT_PERMISSION),
            self.table_def(TENANT_PERMISSION_SET_REL),
        ) else {
            return Ok(());
        };
        let (Some([p_app, p_role, p_kind]), Some([r_app, r_role, r_rapp, r_rrole, r_scope, r_type])) = (
            columns(&tp, ["App ID", "Role ID", "Object Type"]),
            columns(
                &rel,
                [
                    "App ID",
                    "Role ID",
                    "Related App ID",
                    "Related Role ID",
                    "Related Scope",
                    "Type",
                ],
            ),
        ) else {
            return unsupported("the delete of a Tenant Permission Set whose platform tables lack a measured field");
        };
        let cx = self.cx.clone();
        let perms: Vec<Row> = self
            .all_rows(&cx, &tp)?
            .into_iter()
            .filter(|r| same_guid(&r[p_app], &app) && same_text(&r[p_role], &role))
            .collect();
        if !is_zero(&app) {
            // TSDC_DeleteAllOtherApp: a DeleteAll leaves them.
            if delete_all || perms.is_empty() {
                return Ok(());
            }
            return unsupported(
                "a Delete of a Tenant Permission Set whose App ID is not zero that has Tenant Permission rows (not measured)",
            );
        }
        let rels: Vec<Row> = self
            .all_rows(&cx, &rel)?
            .into_iter()
            .filter(|r| {
                (is_zero(&r[r_app]) && same_text(&r[r_role], &role))
                    || (is_zero(&r[r_rapp]) && same_text(&r[r_rrole], &role))
            })
            .collect();
        if perms.is_empty() && rels.is_empty() {
            return Ok(());
        }
        let measured_perm = |r: &Row| ordinal(&r[p_kind]) == Some(TABLE_DATA);
        let measured_rel = |r: &Row| {
            is_zero(&r[r_app])
                && is_zero(&r[r_rapp])
                && ordinal(&r[r_scope]) == Some(SCOPE_TENANT)
                && ordinal(&r[r_type]) == Some(TYPE_INCLUDE)
        };
        let all_measured = perms.iter().all(measured_perm) && rels.iter().all(measured_rel);
        // The rows go after the set's OnAfterDeleteEvent here; where BC deletes them in that order is not measured.
        let quiet =
            self.delete_quiet(&tp)? && self.delete_quiet(&rel)? && !self.listened(t, "onafterdeleteevent", None)?;
        if !(all_measured && quiet) {
            return unsupported(
                "the delete of a Tenant Permission Set with permission or relation rows of a kind, or with delete events or \
                 database triggers on them, that TenantSetDeleteCascade.al did not measure",
            );
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TenantPermissionSetDeleteCascade);
        for (table, row) in perms.iter().map(|r| (&tp, r)).chain(rels.iter().map(|r| (&rel, r))) {
            let pk = table.primary_key(row);
            if let Err(e) = self.store.delete(&cx, table, &pk) {
                return unsupported(format!(
                    "deleting a row a deleted Tenant Permission Set takes with it: {e}"
                ));
            }
        }
        Ok(())
    }

    /// Whether a delete on `t` runs no delete event and no database trigger.
    fn delete_quiet(&mut self, t: &TableDef) -> Result<bool, Signal> {
        Ok(!self.listened(t, "onbeforedeleteevent", None)?
            && !self.listened(t, "onafterdeleteevent", None)?
            && !self.global_enabled(t, GlobalOp::Delete)?)
    }
}

fn same_guid(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::Guid(x), Value::Guid(y)) if x == y)
}
