//! `Record.AddLink`, `HasLinks`, `DeleteLink`, `DeleteLinks` and `CopyLinks`: they read and write the
//! rows of the Record Link table (2000000068), an ordinary stored table with its real declaration.
//!
//! Pinned by corpus record/TestRecordLinkTable.al (rows written to the table are the links;
//! `AddLink` writes one row, returns its Link ID and shares the table's sequence with direct
//! inserts; a rolled back transaction takes the link with it; `CopyLinks` copies rows and leaves
//! the source alone, also when source and target are one record) and TestRecordLinkColumns.al
//! (`User ID` is the user without a domain, `Created` the current date and time, `Company` the
//! company for a per-company table and empty for another, each row has its own SystemId, a copy
//! keeps `Type`, `User ID`, `To User ID`, `Notify`, `Created`, `Company` and the note).

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

const RECORD_LINK: u32 = 2_000_000_068;

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

impl Session {
    /// A rename moves the record's links to its new key: the current company's rows of a
    /// per-company table, every row of another table (record/TestRecordLinkColumns.al
    /// `Rename_OnAPerCompanyTable_MovesOnlyTheCurrentCompanysRows`).
    pub(crate) fn move_record_links(&mut self, t: &TableDef, rec: &Rec, old_fields: &[Value]) -> Result<(), Signal> {
        if rec.borrow().company.is_some() && self.compiled_table(RECORD_LINK).is_some() {
            return unsupported("Rename of a record in another company moves its record links (not pinned)");
        }
        let Some(link_obj) = self.compiled_table(RECORD_LINK) else {
            return Ok(());
        };
        if t.object as usize == link_obj {
            return Ok(());
        }
        let link_def = self.engine.tables[link_obj]
            .clone()
            .expect("a compiled table has a schema");
        let pos = |id: u32| {
            link_def
                .fields
                .iter()
                .position(|f| f.id == id)
                .expect("Record Link field")
        };
        let old = std::rc::Rc::new(std::cell::RefCell::new(rec.borrow().clone()));
        old.borrow_mut().fields = old_fields.to_vec();
        let old_rid = self.record_op(RecordOp::RecordId, &old, &mut [])?;
        let new_rid = self.record_op(RecordOp::RecordId, rec, &mut [])?;
        let Value::Record(link) = self.default_value(&RtType::Record(link_obj as u32, false)) else {
            unreachable!("a record type makes a record")
        };
        self.record_op(RecordOp::SetRange, &link, &mut [Value::Integer(pos(2) as i32), old_rid])?;
        if t.per_company {
            let company = Value::Text(self.cx.company.clone().into());
            self.record_op(
                RecordOp::SetRange,
                &link,
                &mut [Value::Integer(pos(12) as i32), company],
            )?;
        }
        if !al_builtins::ops::truthy(&self.record_op(RecordOp::IsEmpty, &link, &mut [])?)? {
            self.record_op(
                RecordOp::ModifyAll,
                &link,
                &mut [Value::Integer(pos(2) as i32), new_rid, Value::Boolean(false)],
            )?;
        }
        Ok(())
    }

    /// The object index of the table with platform id `id`, if the program compiled it.
    pub(crate) fn compiled_table(&self, id: u32) -> Option<usize> {
        self.engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Table) && o.id == Some(id) && o.table.is_some())
    }

    pub(crate) fn record_links(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        t: &TableDef,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        // The Record Link reads below are the platform's own: they use their results, whether or not the statement
        // `CopyLinks(..)` uses its (none) (Bank Account Ledger Entry.CopyLinks in every journal posting).
        let used = std::mem::replace(&mut self.ret_used, true);
        let r = self.record_links_inner(method, rec, t, args);
        self.ret_used = used;
        r
    }

    fn record_links_inner(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        t: &TableDef,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        if rec.borrow().company.is_some() {
            return unsupported(format!("{method} on a record in another company (not pinned)"));
        }
        let Some(link_obj) = self.compiled_table(RECORD_LINK) else {
            return unsupported(format!("{method}: the Record Link table was not compiled"));
        };
        let link_def = self.engine.tables[link_obj]
            .clone()
            .expect("a compiled table has a schema");
        let pos = |id: u32| {
            link_def
                .fields
                .iter()
                .position(|f| f.id == id)
                .expect("Record Link field")
        };
        let rid = self.record_op(RecordOp::RecordId, rec, &mut [])?;
        // A fresh Record Link record, filtered to the links of record `of`, and to `company` when
        // one is given.
        let linked = |s: &mut Session, of: &Value, company: Option<&str>| -> Result<Rec, Signal> {
            let Value::Record(link) = s.default_value(&RtType::Record(link_obj as u32, false)) else {
                unreachable!("a record type makes a record")
            };
            s.record_op(
                RecordOp::SetRange,
                &link,
                &mut [Value::Integer(pos(2) as i32), of.clone()],
            )?;
            if let Some(c) = company {
                s.record_op(
                    RecordOp::SetRange,
                    &link,
                    &mut [Value::Integer(pos(12) as i32), Value::Text(c.to_string().into())],
                )?;
            }
            Ok(link)
        };
        // HasLinks and DeleteLinks look at the current company's rows of a per-company table and at
        // every row of another table (TestRecordLinkColumns.al, `HasLinks_*`, `DeleteLinks_*`).
        let own_company = self.cx.company.clone();
        let scoped = t.per_company.then_some(&*own_company);
        match method {
            RecordOp::HasLinks => {
                let link = linked(self, &rid, scoped)?;
                let empty = self.record_op(RecordOp::IsEmpty, &link, &mut [])?;
                Ok(Value::Boolean(!al_builtins::ops::truthy(&empty)?))
            }
            RecordOp::DeleteLinks => {
                let link = linked(self, &rid, scoped)?;
                self.record_op(RecordOp::DeleteAll, &link, &mut [Value::Boolean(false)])?;
                Ok(Value::Null)
            }
            // DeleteLink compares the company with the session company even for a table that is not
            // per company, so it leaves the row AddLink wrote there (same file, `DeleteLink_*`).
            RecordOp::DeleteLink => {
                let id = args.first().cloned().unwrap_or(Value::Null);
                let link = linked(self, &rid, Some(&*own_company))?;
                self.record_op(RecordOp::SetRange, &link, &mut [Value::Integer(pos(1) as i32), id])?;
                self.record_op(RecordOp::DeleteAll, &link, &mut [Value::Boolean(false)])?;
                Ok(Value::Null)
            }
            RecordOp::AddLink => {
                let text = |i: usize, max: usize| {
                    al_builtins::native::text_of(args.get(i).unwrap_or(&Value::Null))
                        .unwrap_or_default()
                        .chars()
                        .take(max)
                        .collect::<String>()
                };
                let (url, description) = (text(0, 2048), text(1, 250));
                let user = self.host.identity.user_id();
                let user = user.rsplit('\\').next().unwrap_or(&user).to_string();
                let created = crate::hostvalues::datetime_of_unix_millis(self.now_millis());
                let Value::Record(link) = self.default_value(&RtType::Record(link_obj as u32, false)) else {
                    unreachable!("a record type makes a record")
                };
                {
                    let mut l = link.borrow_mut();
                    l.fields[pos(2)] = rid;
                    l.fields[pos(3)] = Value::Text(url.into());
                    l.fields[pos(7)] = Value::Text(description.into());
                    l.fields[pos(10)] = created;
                    l.fields[pos(11)] = Value::Text(user.into());
                    l.fields[pos(12)] = Value::Text(if t.per_company {
                        own_company.clone().into()
                    } else {
                        String::new().into()
                    });
                }
                self.record_op(RecordOp::Insert, &link, &mut [Value::Boolean(false)])?;
                let id = link.borrow().fields[pos(1)].clone();
                Ok(id)
            }
            RecordOp::CopyLinks => {
                let from = match args.first() {
                    Some(Value::Record(from)) => from.clone(),
                    Some(Value::Library(_, lib)) => match &*lib.borrow() {
                        al_builtins::value::LibraryValue::RecordRef(from) => from.clone(),
                        other => return unsupported(format!("CopyLinks from {}", other.describe())),
                    },
                    other => return unsupported(format!("CopyLinks from {}", other.describe())),
                };
                let from_t = self.table(&from)?;
                let from_scope = from_t.per_company.then_some(&*own_company);
                let from_rid = self.record_op(RecordOp::RecordId, &from, &mut [])?;
                // Copying a record's links onto itself changes nothing
                // (TestRecordLinkTable.al `CopyLinks_SourceOntoItself_DoesNotDuplicateTheRows`).
                if al_builtins::ops::compare(&from_rid, &rid).is_ok_and(|o| o.is_eq()) {
                    return Ok(Value::Null);
                }
                let source = linked(self, &from_rid, from_scope)?;
                self.record_op(RecordOp::SetCurrentKey, &source, &mut [Value::Integer(pos(1) as i32)])?;
                // Read the rows first, then write: the copies must not join the scan.
                let mut rows = Vec::new();
                if al_builtins::ops::truthy(&self.record_op(RecordOp::FindSet, &source, &mut [])?)? {
                    loop {
                        self.record_op(RecordOp::CalcFields, &source, &mut [Value::Integer(pos(9) as i32)])?;
                        rows.push(source.borrow().fields.clone());
                        if !matches!(self.record_op(RecordOp::Next, &source, &mut [])?, Value::Integer(n) if n != 0) {
                            break;
                        }
                    }
                }
                for fields in rows {
                    let Value::Record(link) = self.default_value(&RtType::Record(link_obj as u32, false)) else {
                        unreachable!("a record type makes a record")
                    };
                    {
                        let mut l = link.borrow_mut();
                        for id in [3, 7, 8, 9, 10, 11, 13, 14] {
                            l.fields[pos(id)] = fields[pos(id)].clone();
                        }
                        l.fields[pos(2)] = rid.clone();
                        // The copy belongs to the company of the target (same file,
                        // `CopyLinks_OntoAPerCompanyTable_WritesTheCompany`, `..._LeavesCompanyEmpty`).
                        l.fields[pos(12)] = Value::Text(if t.per_company {
                            own_company.clone().into()
                        } else {
                            String::new().into()
                        });
                    }
                    self.record_op(RecordOp::Insert, &link, &mut [Value::Boolean(false)])?;
                }
                Ok(Value::Null)
            }
            other => unsupported(format!("Record.{other}")),
        }
    }
}
