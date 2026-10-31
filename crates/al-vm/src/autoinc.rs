//! `AutoIncrement` fields: a per-table counter kept outside the store.
//!
//! Pinned by corpus record/TestAutoIncrementContracts.al, TestAutoIncrementExplicitKey.al,
//! TestAutoIncrementEdges.al and TestTempTableContracts.al; what they leave open is refused.
//! Truncate (which resets the counter) and guarded `Codeunit.Run` rollbacks (which restore it)
//! are not implemented, so no code path here has to model them.

use crate::Session;
use al_builtins::native::{AlError, Signal};
use al_builtins::ops;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn number(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(i64::from(*i)),
        Value::BigInteger(i) => Some(*i),
        _ => None,
    }
}

impl Session {
    /// The AutoIncrement field of the table, if it has one this runtime can drive.
    pub(crate) fn auto_increment_field(&self, t: &TableDef) -> Result<Option<usize>, Signal> {
        let schema = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .expect("table");
        let mut auto = schema.fields.iter().enumerate().filter(|(_, f)| f.auto_increment);
        let Some((pos, f)) = auto.next() else { return Ok(None) };
        if auto.next().is_some() {
            return unsupported("a table with several AutoIncrement fields");
        }
        if !matches!(f.ty, RtType::Integer | RtType::BigInteger) {
            return unsupported("an AutoIncrement field that is not an Integer or BigInteger");
        }
        Ok(Some(pos))
    }

    fn counter_slot(t: &TableDef, company: &str) -> (String, u32) {
        (
            if t.per_company {
                company.to_string()
            } else {
                String::new()
            },
            t.object,
        )
    }

    /// The AutoIncrement counter of a table: what this session has handed out, else the largest
    /// value among the restored rows (test data), else 0. A restored identity column continues
    /// above its rows.
    fn counter(&self, t: &TableDef, slot: &(String, u32)) -> i64 {
        self.autoinc.get(slot).copied().unwrap_or_else(|| {
            self.store
                .backed_rows(&slot.0, t.object)
                .map_or(0, |r| r.auto_increment_floor())
        })
    }

    /// Gives a zero key the next counter value (corpus
    /// record/TestAutoIncrementContracts.al:25: setting the key to 0 requests a new value).
    /// Returns whether a value was taken.
    pub(crate) fn take_auto_increment(
        &mut self,
        t: &TableDef,
        field: usize,
        row: &mut [Value],
        company: &str,
    ) -> Result<bool, Signal> {
        if number(&row[field]).unwrap_or(0) != 0 {
            return Ok(false);
        }
        let slot = Self::counter_slot(t, company);
        let next = self.counter(t, &slot) + 1;
        row[field] = ops::convert(Value::BigInteger(next), &t.fields[field].ty)?;
        self.autoinc.insert(slot, next);
        Ok(true)
    }

    /// After a successful Insert with a key the caller (or OnInsert) chose: a key above
    /// the counter raises it, one at or below leaves it (record/TestAutoIncrementExplicitKey.al:43,98).
    pub(crate) fn note_explicit_key(&mut self, t: &TableDef, field: usize, row: &[Value], company: &str) {
        let Some(k) = number(&row[field]) else { return };
        let slot = Self::counter_slot(t, company);
        let cur = self.counter(t, &slot);
        if k > cur {
            self.autoinc.insert(slot, k);
        }
    }

    /// `Truncate`: deletes every row without triggers and, with `reset`, restarts the
    /// AutoIncrement counter (corpus record/TestAutoIncrementContracts.al:83). Guards the
    /// corpus pins raise their message; the rest of the guards BC runs are refused.
    pub(crate) fn truncate(&mut self, t: &TableDef, rec: &crate::records::Rec, reset: bool) -> Result<Value, Signal> {
        let error = |message: &str| -> Result<Value, Signal> {
            Err(Signal::Error(Box::new(AlError {
                message: message.into(),
                code: String::new(),
                call_stack: String::new(),
                ..Default::default()
            })))
        };
        // Texts and code: corpus record/TestCalcFieldsTruncateUniqueContracts.al:Truncate_Refusals_Texts.
        if rec.borrow().temporary {
            let e = AlError {
                message: "Temporary tables does not support truncation.".into(),
                code: "DB:Ok".into(),
                call_stack: String::new(),
                ..Default::default()
            };
            // With the result used it answers false, keeps the rows and leaves the text in GetLastErrorText
            // (record/lt-truncate-guards/TruncateGuards.al:P163_Truncate_Temporary_ReturnValueUsed_BQ).
            if self.ret_used {
                self.last_error = e;
                return Ok(Value::Boolean(false));
            }
            return Err(Signal::Error(Box::new(e)));
        }
        let has_media = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|s| {
                s.fields.iter().any(|f| {
                    matches!(
                        &f.ty,
                        RtType::Library(
                            al_builtins::library_kind::LibraryKind::Media
                                | al_builtins::library_kind::LibraryKind::MediaSet
                        )
                    )
                })
            });
        if has_media {
            // Code DB:Ok (misc/lt-probes-needed-3/PnC03Truncate.al:Truncate_MediaTable, Truncate_MediaTable_Empty;
            // record/lt-truncate-guards/TruncateGuards.al:P163_Truncate_MediaSetTable_AndFilters).
            return Err(Signal::Error(Box::new(AlError {
                message: "Truncate is not supported when the field has Media and/or MediaSet fields.".into(),
                code: "DB:Ok".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        // Corpus record/TestRecordTruncateTryScope.al:34: the scope is inherited by called frames.
        // The computed tables Integer and AllObj: with the result used it answers false, keeps the rows and leaves the text in
        // GetLastErrorText (record/TestCalcFieldsTruncateUniqueContracts.al:311-312; record/lt-truncate-guards/TruncateGuards.al:
        // 222-237). The error code is not pinned; another computed table is refused by the store.
        if matches!(
            self.engine.program.objects[t.object as usize]
                .id
                .and_then(al_builtins::platform::virtual_table),
            Some(al_builtins::platform::Virtual::Integer | al_builtins::platform::Virtual::AllObj)
        ) {
            let e = AlError {
                message: "The table does not support truncation.".into(),
                code: String::new(),
                call_stack: String::new(),
                ..Default::default()
            };
            if self.ret_used {
                self.last_error = e;
                return Ok(Value::Boolean(false));
            }
            return Err(Signal::Error(Box::new(e)));
        }
        if self.tx.try_depth > 0 {
            return error("Truncate is not supported in try functions.");
        }
        // A subscriber that was not compiled is a subscriber to BC all the same.
        let undeclared_delete_subscriber = ["onbeforedeleteevent", "onafterdeleteevent"]
            .iter()
            .any(|e| self.engine.events.missing_for(t.object as usize, e, None).is_some());
        if undeclared_delete_subscriber || self.delete_event_subscribed(&t.name) {
            return error(
                "Truncate is not supported when the OnBeforeDelete and/or OnAfterDelete event is subscribed. Please remove the events subscriptions or use DeleteAll.",
            );
        }
        if !rec.borrow().temporary && self.global_enabled(t, crate::globaltriggers::GlobalOp::Delete)? {
            return unsupported(format!(
                "Truncate on {}, which has the OnDatabaseDelete event enabled (whether BC refuses it is not pinned)",
                t.name
            ));
        }
        let (filters, flow_field_filters) = self.split_filters(rec);
        if !flow_field_filters.is_empty() {
            return error("Truncate does not support filters on FlowFields.");
        }
        let cx = self.cx_for(rec, t)?;
        let store_err = |e: al_data::StoreError| Signal::Unsupported(format!("store error {e}"));
        // A filter on a normal field limits what Truncate deletes (corpus
        // record/TestCalcFieldsTruncateUniqueContracts.al:Truncate_WithFilterOnANormalField_RemovesOnlyTheFilteredRows).
        let slot = Self::counter_slot(t, &cx.company);
        let counter = self.counter(t, &slot);
        self.truncate_undo.entry(slot.clone()).or_insert(counter);
        if reset {
            self.autoinc.insert(slot, 0);
        }
        al_data::Store::delete_all(&mut self.store, &cx, t, &filters).map_err(store_err)?;
        Ok(Value::Boolean(true))
    }

    /// Whether a non-manual subscriber listens to the table's OnBeforeDelete or OnAfterDelete event.
    fn delete_event_subscribed(&self, table: &str) -> bool {
        self.engine
            .delete_subscribed
            .get_or_init(|| tables_with_delete_subscribers(&self.engine.program))
            .contains(&table.to_ascii_lowercase())
    }

    /// A Modify may not change an AutoIncrement field outside the primary key (record/TestAutoIncrementEdges.al:552-566, and :583-600
    /// after a TransferFields from another row). Pinned on a database table without triggers: with a trigger, an event subscriber or
    /// a database trigger the order of the check is not known, and on a temporary record nothing is pinned.
    pub(crate) fn check_modify_auto_increment(
        &mut self,
        t: &TableDef,
        rec: &crate::records::Rec,
        old: &[Value],
        run: bool,
    ) -> Result<(), Signal> {
        let Some(field) = self.auto_increment_field(t)? else {
            return Ok(());
        };
        let new = rec.borrow().fields.clone();
        if t.keys[0].contains(&field) || number(&old[field]) == number(&new[field]) {
            return Ok(());
        }
        let mut trigger = run && self.table_trigger(t, "OnModify").is_some();
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            trigger |= run && !self.ext_triggers(t, n).is_empty();
        }
        if rec.borrow().temporary
            || trigger
            || self.listened(t, "onbeforemodifyevent", None)?
            || self.listened(t, "onaftermodifyevent", None)?
            || self.global_enabled(t, crate::globaltriggers::GlobalOp::Modify)?
        {
            return unsupported(
                "a Modify that changes an AutoIncrement field of a temporary record or a table with triggers or events (not pinned)",
            );
        }
        let br = crate::records::BR;
        Err(Signal::Error(Box::new(AlError {
            message: format!(
                "You cannot change the value of an AutoIncrement field.{br}{br}Field: {}{br}Table: {}{br}{br}Identification fields and values:{br}{}",
                t.fields[field].name,
                self.caption(t),
                self.identification(t, &new)?
            ),
            code: "DB:ModifyOnIdentityField".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// Rename may not change an AutoIncrement value (record/TestAutoIncrementExplicitKey.al:147,
    /// TestAutoIncrementEdges.al:296).
    pub(crate) fn check_rename_auto_increment(&self, t: &TableDef, old: &[Value], new: &[Value]) -> Result<(), Signal> {
        let Some(field) = self.auto_increment_field(t)? else {
            return Ok(());
        };
        if t.keys[0].contains(&field) && number(&old[field]) != number(&new[field]) {
            // Layout and code: corpus record/TestCalcFieldsTruncateUniqueContracts.al:
            // AutoIncrement_Rename_FullTextAndCode (the key shown is the new one).
            let br = crate::records::BR;
            return Err(Signal::Error(Box::new(AlError {
                message: format!(
                    "You cannot change the value of an AutoIncrement field.{br}{br}Field: {}{br}Table: {}{br}{br}Identification fields and values:{br}{}",
                    t.fields[field].name,
                    self.caption(t),
                    self.identification(t, new)?
                ),
                code: "DB:ModifyOnIdentityField".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        Ok(())
    }
}

/// The lower-case names of the tables a non-manual subscriber listens to the OnBeforeDelete or
/// OnAfterDelete event of. Reads every routine's attributes, so it is asked once per engine.
pub(crate) fn tables_with_delete_subscribers(program: &al_ir::Program) -> std::collections::HashSet<String> {
    let mut tables = std::collections::HashSet::new();
    // The heads: every routine with an `EventSubscriber` attribute is among their event routines (ADR-054).
    for o in program.objects.heads() {
        let manual = al_ir::ObjectFacts::of_properties(&o.properties).manual_subscriber;
        if manual {
            continue;
        }
        for a in o.events.iter().flat_map(|r| &r.attributes) {
            if !a.name.eq_ignore_ascii_case("EventSubscriber") {
                continue;
            }
            let text = a.text.to_ascii_lowercase();
            let parts: Vec<&str> = text.split(',').map(|p| p.trim().trim_matches('\'')).collect();
            if text.contains("objecttype::table")
                && parts
                    .get(2)
                    .is_some_and(|e| matches!(*e, "onbeforedeleteevent" | "onafterdeleteevent"))
            {
                if let Some(table) = parts.get(1) {
                    tables.insert(table.trim_start_matches("database::").trim_matches('"').to_string());
                }
            }
        }
    }
    tables
}
