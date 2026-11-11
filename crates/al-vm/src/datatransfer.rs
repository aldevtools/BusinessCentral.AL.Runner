//! `DataTransfer` inside install code (al-builtins `datatransfer` hands each call here through `Ctx::ref_op`). The staging
//! calls keep a [`Plan`] in the variable; `CopyRows` and `CopyFields` write the destination rows straight into the store,
//! without the table's triggers. Pinned by install-upgrade/AsDtInstaller.al and AsDtInstallRead.al:86-147:
//! - `CopyRows` inserts one destination row per source row the source filters keep, with the mapped fields and the constants;
//!   the other fields hold their type's default (S1);
//! - `CopyFields` with `AddJoin` changes each destination row the destination filters keep that one source row matches on the
//!   join fields (and the source filters keep); the others stay as they were (S2, S4);
//! - `CopyFields` without a join on one table copies the mapped fields within each row the source filters keep (S3).
//!
//! An unmapped field of a `CopyRows` row gets its InitValue and no table trigger runs (record/Y47DtInstall.al
//! S1-S3 and Y47DtInstallRead.al:54-72; al-runner issue 6586). `Clear` resets the variable: the plan staged before it is
//! dropped (AsDtInstallRead.al:166-180, C1). A destination with a table extension is refused: the file
//! has none. Everything else is refused.

use crate::Session;
use al_builtins::datatransfer::Plan;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{LibraryValue, RtType, Value};
use al_data::{Store, TableDef};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

type Lib = Rc<RefCell<LibraryValue>>;
type Filters = [(i32, String, Vec<Value>)];
type Rec = Rc<RefCell<al_builtins::value::RecordState>>;
/// A table, the rows a scan kept, and the buffer it read them with.
type Scanned = (Arc<TableDef>, Vec<Vec<Value>>, Rec);

fn refuse<T>(what: impl Into<al_builtins::refusal::Refusal>) -> Result<T, Signal> {
    Err(Signal::unsupported(what))
}

fn int(args: &[Value], i: usize, method: &str) -> Result<i32, Signal> {
    match args.get(i) {
        Some(Value::Integer(n)) => Ok(*n),
        _ => refuse(format!("DataTransfer.{method} with an argument that is not an Integer")),
    }
}

fn pos(t: &TableDef, no: i32) -> Result<usize, Signal> {
    t.fields
        .iter()
        .position(|f| i64::from(f.id) == i64::from(no))
        .map_or_else(
            || {
                refuse(format!(
                    "DataTransfer on field {no} that {} does not have (not pinned)",
                    t.name
                ))
            },
            Ok,
        )
}

/// The positions of the fields of an `AddJoin`. Only a join of two Code[20] fields is pinned (install-upgrade/
/// AsDtInstaller.al:163, :195 and record/Y47DtInstall.al:350, read back by AsDtInstallRead.al and Y47DtInstallRead.al); any
/// other pair of field types is refused before a row is written (al-runner issue 7644: a Guid against a Code that holds no
/// Guid counted as no match). How the database matches other types (a Text's collation, a Guid against its text) is not
/// pinned.
fn join_fields(st: &TableDef, s: i32, dt: &TableDef, d: i32) -> Result<(usize, usize), Signal> {
    let (sp, dp) = (pos(st, s)?, pos(dt, d)?);
    let (a, b) = (&st.fields[sp].ty, &dt.fields[dp].ty);
    if *a != RtType::Code(Some(20)) || a != b {
        return refuse(format!(
            "DataTransfer.AddJoin of {} field {} with {} field {} (only a join of two Code[20] fields is pinned)",
            st.name, st.fields[sp].name, dt.name, dt.fields[dp].name
        ));
    }
    Ok((sp, dp))
}

/// Whether the source row `s` matches the destination row `d` on every join field. Two values that do not compare refuse:
/// they are neither a match nor no match (al-runner issue 7644).
fn joined(joins: &[(usize, usize)], s: &[Value], d: &[Value]) -> Result<bool, Signal> {
    for &(sp, dp) in joins {
        match al_builtins::ops::compare(&s[sp], &d[dp]) {
            Ok(o) if o.is_eq() => {}
            Ok(_) => return Ok(false),
            Err(_) => {
                return refuse(format!(
                    "DataTransfer.AddJoin comparing {} with {} (not pinned)",
                    s[sp].describe(),
                    d[dp].describe()
                ));
            }
        }
    }
    Ok(true)
}

impl Session {
    /// One `DataTransfer` call inside install code.
    pub(crate) fn datatransfer_op(&mut self, method: &str, lib: &Lib, args: &mut [Value]) -> Result<Value, Signal> {
        if method == "SetTables" {
            let (source, destination) = (int(args, 0, method)?, int(args, 1, method)?);
            for n in [source, destination] {
                if self.compiled_table(n as u32).is_none() {
                    return refuse("DataTransfer.SetTables on a table outside the workspace (not pinned)");
                }
            }
            // A second SetTables without Clear keeps the field mappings of the first (AsDtInstallRead.al:182-195, C2).
            let fields = match &*lib.borrow() {
                LibraryValue::DataTransferPlan(p) => Some(p.fields.clone()),
                _ => None,
            };
            *lib.borrow_mut() = LibraryValue::DataTransferPlan(Box::new(Plan {
                source,
                destination,
                restaged: fields.is_some(),
                fields: fields.unwrap_or_default(),
                ..Default::default()
            }));
            return Ok(Value::Null);
        }
        let mut plan = match &*lib.borrow() {
            LibraryValue::DataTransferPlan(p) => (**p).clone(),
            // A variable Clear reset asks for SetTables, also in install code (AsDtInstallRead.al:198-208, C3). A cleared
            // variable holds the same value as one never staged.
            LibraryValue::Empty => return al_builtins::datatransfer::before_set_tables(),
            _ => {
                return refuse(format!(
                    "DataTransfer.{method} before SetTables inside install code (not pinned)"
                ));
            }
        };
        if plan.restaged && matches!(method, "CopyRows" | "CopyFields") {
            return refuse(format!(
                "DataTransfer.{method} after a second SetTables without Clear (only the kept field mappings are pinned)"
            ));
        }
        match method {
            "AddSourceFilter" | "AddDestinationFilter" => {
                let field = int(args, 0, method)?;
                let Some(text) = args.get(1).and_then(al_builtins::native::text_of) else {
                    return refuse(format!("DataTransfer.{method} with a filter that is not a text"));
                };
                let entry = (field, text.to_string(), args.get(2..).unwrap_or_default().to_vec());
                match method {
                    "AddSourceFilter" => plan.source_filters.push(entry),
                    _ => plan.destination_filters.push(entry),
                }
            }
            "AddFieldValue" => {
                let (from, to) = (int(args, 0, method)?, int(args, 1, method)?);
                // After a second SetTables, mapping a destination field the kept mappings write raises
                // (AsDtInstallRead.al:191, C2). Within one SetTables it is not pinned.
                if plan.fields.iter().any(|&(_, d)| d == to) {
                    if plan.restaged {
                        return al_builtins::datatransfer::same_destination_twice();
                    }
                    return refuse("DataTransfer.AddFieldValue onto a destination field already mapped (not pinned)");
                }
                plan.fields.push((from, to));
            }
            "AddJoin" => plan.joins.push((int(args, 0, method)?, int(args, 1, method)?)),
            "AddConstantValue" => plan
                .constants
                .push((args.first().cloned().unwrap_or(Value::Null), int(args, 1, method)?)),
            "CopyRows" => self.dt_copy_rows(&plan)?,
            "CopyFields" => self.dt_copy_fields(&plan)?,
            other => return refuse(format!("DataTransfer.{other} inside install code (not pinned)")),
        }
        *lib.borrow_mut() = LibraryValue::DataTransferPlan(Box::new(plan));
        Ok(Value::Null)
    }

    /// The rows of table `no` that `filters` keep, in primary key order, with the table and a buffer of it.
    fn dt_rows(&mut self, no: i32, filters: &Filters) -> Result<Scanned, Signal> {
        let Some(object) = self.compiled_table(no as u32) else {
            return refuse("DataTransfer on a table outside the workspace (not pinned)");
        };
        let Value::Record(rec) = self.default_value(&RtType::Record(object as u32, false)) else {
            return refuse("a record buffer for DataTransfer");
        };
        let t = self.table(&rec)?;
        for (field, text, values) in filters {
            let mut a = vec![
                Value::Integer(pos(&t, *field)? as i32),
                Value::Text(text.clone().into()),
            ];
            a.extend(values.iter().cloned());
            self.record_op(RecordOp::SetFilter, &rec, &mut a)?;
        }
        // The reads answer as when their result is used: an empty table answers false, it does not raise.
        let used = std::mem::replace(&mut self.ret_used, true);
        let rows = self.dt_scan(&rec);
        self.ret_used = used;
        Ok((t, rows?, rec))
    }

    fn dt_scan(&mut self, rec: &Rec) -> Result<Vec<Vec<Value>>, Signal> {
        let mut rows = Vec::new();
        if al_builtins::ops::truthy(&self.record_op(RecordOp::FindSet, rec, &mut [])?)? {
            loop {
                rows.push(rec.borrow().fields.clone());
                if matches!(self.record_op(RecordOp::Next, rec, &mut [])?, Value::Integer(0)) {
                    break;
                }
            }
        }
        Ok(rows)
    }

    /// Writes the mapped fields of `source` (a row of `st`) and the constants into `row` (a row of `dt`).
    fn dt_apply(plan: &Plan, st: &TableDef, source: &[Value], dt: &TableDef, row: &mut [Value]) -> Result<(), Signal> {
        for (sf, df) in &plan.fields {
            let (sp, dp) = (pos(st, *sf)?, pos(dt, *df)?);
            if st.fields[sp].ty != dt.fields[dp].ty {
                return refuse("DataTransfer.AddFieldValue between fields of different types (not pinned)");
            }
            row[dp] = source[sp].clone();
        }
        for (v, df) in &plan.constants {
            let dp = pos(dt, *df)?;
            row[dp] = al_builtins::ops::convert(v.clone(), &dt.fields[dp].ty)?;
        }
        Ok(())
    }

    /// Refuses a copy into table `no` when it has a table extension of another app. A table extension of the table's own app
    /// is pinned: its fields are copied, filtered and joined like the table's, an unmapped one gets its InitValue on CopyRows,
    /// and no trigger of the table or of an extension runs (record/Y47DtInstall.al S4-S7 on Y57DtExtTables.al,
    /// Y47DtInstallRead.al Y57_DtExt_Outcomes, Y57_DtExt_Rows).
    fn dt_refuse_extensions(&self, no: i32) -> Result<(), Signal> {
        let Some(object) = self.compiled_table(no as u32) else {
            return Ok(());
        };
        let objects = &self.engine.program.objects;
        let app = objects[object].app;
        if objects[object]
            .table
            .as_ref()
            .is_some_and(|t| t.extensions.iter().any(|&e| objects[e].app != app))
        {
            return refuse("DataTransfer into a table that has a table extension of another app (not pinned)");
        }
        Ok(())
    }

    fn dt_copy_rows(&mut self, plan: &Plan) -> Result<(), Signal> {
        if !plan.joins.is_empty() || !plan.destination_filters.is_empty() {
            return refuse("DataTransfer.CopyRows with AddJoin or AddDestinationFilter (not pinned)");
        }
        let (st, rows, _) = self.dt_rows(plan.source, &plan.source_filters)?;
        let (dt, _, drec) = self.dt_rows(plan.destination, &[])?;
        let object = self.compiled_table(plan.destination as u32).expect("checked");
        let fresh = self.initial_fields(object);
        self.dt_refuse_extensions(plan.destination)?;
        let cx = self.cx_for(&drec, &dt)?;
        for source in rows {
            let mut row = fresh.clone();
            Self::dt_apply(plan, &st, &source, &dt, &mut row)?;
            let pk: Vec<Value> = dt.keys[0].iter().map(|&p| row[p].clone()).collect();
            if matches!(Store::get(&self.store, &cx, &dt, &pk), Ok(Some(_))) {
                return refuse("DataTransfer.CopyRows onto a row that already exists (not pinned)");
            }
            Store::insert(&mut self.store, &cx, &dt, row)
                .map_err(|e| Signal::unsupported(format!("DataTransfer.CopyRows: {e}")))?;
        }
        Ok(())
    }

    fn dt_copy_fields(&mut self, plan: &Plan) -> Result<(), Signal> {
        // CopyFields into a table with a table extension, of its own app or of another, copies, filters and joins the extension's
        // fields like the table's and runs no trigger (record/Y47DtInstallRead.al Y65_DtItemAttribute_Outcomes,
        // writer Y65, a Base Application table with another app's extension; Y57_DtExt_* for the own app's). CopyRows stays refused.
        let (st, sources, _) = self.dt_rows(plan.source, &plan.source_filters)?;
        let changed: Vec<Vec<Value>> = if plan.joins.is_empty() {
            if plan.source != plan.destination || !plan.destination_filters.is_empty() {
                return refuse(
                    "DataTransfer.CopyFields without AddJoin between two tables or with AddDestinationFilter (not pinned)",
                );
            }
            let mut out = Vec::new();
            for source in &sources {
                let mut row = source.clone();
                Self::dt_apply(plan, &st, source, &st, &mut row)?;
                out.push(row);
            }
            out
        } else {
            let (dt, rows, _) = self.dt_rows(plan.destination, &plan.destination_filters)?;
            let joins: Vec<(usize, usize)> = plan
                .joins
                .iter()
                .map(|(s, d)| join_fields(&st, *s, &dt, *d))
                .collect::<Result<_, Signal>>()?;
            let mut out = Vec::new();
            for row in rows {
                let mut matches: Vec<&Vec<Value>> = Vec::new();
                for s in &sources {
                    if joined(&joins, s, &row)? {
                        matches.push(s);
                    }
                }
                match matches.as_slice() {
                    [] => {}
                    [source] => {
                        let mut row = row.clone();
                        Self::dt_apply(plan, &st, source, &dt, &mut row)?;
                        out.push(row);
                    }
                    _ => {
                        return refuse(
                            "DataTransfer.CopyFields where several source rows match one destination row (not pinned)",
                        );
                    }
                }
            }
            out
        };
        let (dt, _, drec) = self.dt_rows(plan.destination, &[])?;
        let cx = self.cx_for(&drec, &dt)?;
        for row in changed {
            Store::modify(&mut self.store, &cx, &dt, row)
                .map_err(|e| Signal::unsupported(format!("DataTransfer.CopyFields: {e}")))?;
        }
        Ok(())
    }
}
