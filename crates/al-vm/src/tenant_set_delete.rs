//! The delete of a stored Tenant Permission Set row (2000000165) whose App ID is zero, while Tenant Permission rows (2000000166)
//! or Tenant Permission Set Rel. rows (2000000253) name it: refused. BC deletes those rows with the set (corpus probe
//! permissions/permission-sets/TenantSetDeleteCascade.al, corpus pull request 1340, not yet in the pinned corpus; al-runner
//! issue 7854), which the runner does not do yet. Every other delete of the table keeps its result.

use crate::Session;
use crate::platform::{TENANT_PERMISSION, TENANT_PERMISSION_SET, TENANT_PERMISSION_SET_REL, pos};
use crate::records::Rec;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef, position};

// Field ids of App ID and Role ID in the set table (its primary key).
const F_APP_ID: u32 = 1;
const F_ROLE_ID: u32 = 2;

const REFUSED: &str = "the delete of a Tenant Permission Set with App ID zero that Tenant Permission or Tenant Permission Set \
                       Rel. rows name (BC deletes them with the set: corpus probe TenantSetDeleteCascade.al; not implemented)";

fn column(t: &TableDef, name: &str) -> Option<usize> {
    t.fields.iter().position(|f| f.name == name)
}

fn is_zero(v: &Value) -> bool {
    matches!(v, Value::Guid(g) if *g == [0; 16])
}

fn same_text(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) => x.as_str() == y.as_str(),
        _ => false,
    }
}

impl Session {
    /// Whether `t` with `temporary` is the stored Tenant Permission Set table.
    pub(crate) fn is_stored_tenant_set(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.engine.program.objects[t.object as usize].id == Some(TENANT_PERMISSION_SET)
    }

    /// Refuses when the stored set row `row` of `t` has App ID zero and a Tenant Permission row (App ID zero, same Role ID) or a
    /// Tenant Permission Set Rel. row (App ID zero and the same Role ID, or Related App ID zero and the same Related Role ID)
    /// names it (see the module).
    pub(crate) fn tenant_set_delete_refuses(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        row: &[Value],
    ) -> Result<(), Signal> {
        let role = &row[pos(t, F_ROLE_ID)];
        if !is_zero(&row[pos(t, F_APP_ID)]) {
            return Ok(());
        }
        if let Some(tp) = self.table_def(TENANT_PERMISSION) {
            let (Some(app), Some(rid)) = (column(&tp, "App ID"), column(&tp, "Role ID")) else {
                return unsupported(REFUSED);
            };
            if self
                .all_rows(cx, &tp)?
                .iter()
                .any(|r| is_zero(&r[app]) && same_text(&r[rid], role))
            {
                return unsupported(REFUSED);
            }
        }
        if let Some(rel) = self.table_def(TENANT_PERMISSION_SET_REL) {
            let cols = (
                column(&rel, "App ID"),
                column(&rel, "Role ID"),
                column(&rel, "Related App ID"),
                column(&rel, "Related Role ID"),
            );
            let (Some(app), Some(rid), Some(rapp), Some(rrid)) = cols else {
                return unsupported(REFUSED);
            };
            let names = |r: &Vec<Value>| {
                (is_zero(&r[app]) && same_text(&r[rid], role)) || (is_zero(&r[rapp]) && same_text(&r[rrid], role))
            };
            if self.all_rows(cx, &rel)?.iter().any(names) {
                return unsupported(REFUSED);
            }
        }
        Ok(())
    }

    /// For a Delete of `rec`: refuses when the stored set row it deletes is one `tenant_set_delete_refuses` refuses. A Delete
    /// that finds no stored row keeps its result.
    pub(crate) fn tenant_set_delete_one_refuses(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        rec: &Rec,
    ) -> Result<(), Signal> {
        if !self.is_stored_tenant_set(t, rec.borrow().temporary) {
            return Ok(());
        }
        let pk: Vec<Value> = t.keys[0].iter().map(|&f| rec.borrow().fields[f].clone()).collect();
        let stored = self
            .store_for(rec)
            .get(cx, t, &pk)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        match stored {
            Some(row) => self.tenant_set_delete_refuses(cx, t, &row),
            None => Ok(()),
        }
    }

    /// For a DeleteAll of `rec`: refuses when one of the rows its filters select is one `tenant_set_delete_refuses` refuses.
    pub(crate) fn tenant_set_delete_all_refuses(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        rec: &Rec,
    ) -> Result<(), Signal> {
        if !self.is_stored_tenant_set(t, rec.borrow().temporary) {
            return Ok(());
        }
        let key = rec.borrow().key;
        let mut after = None;
        while let Some(row) = self.scan(rec, t, true, after.take())? {
            self.tenant_set_delete_refuses(cx, t, &row)?;
            after = Some(position(t, key, &row));
        }
        Ok(())
    }
}
