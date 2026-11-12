//! Tenant Media rows that an AL `Insert(false)` stores in the record store, next to the rows the media store computes.
//! Pinned by corpus streams/media/Y48TenantMediaRows.al:634-649 (Record_Insert_TenantMediaDescriptionOnly_ReadsBack):
//! `Init`, an `ID` and a `Description`, `Insert(false)`, then `Get`, `TestField(Description)` and `Delete()` of that row.
//! The provider refuses a scan while such a row is there (`platform::Providers::tenant_media_scan`) and the store refuses
//! its Modify (`VirtualTables::modifies_stored_rows`); this module checks the Record calls that name one row, and owns the
//! predicate `is_stored_tenant_media_id` with which a Media or MediaSet field holding such an ID is refused
//! (`Session::release_media`, `MediaSet.Insert`).

use super::*;

impl Session {
    /// Whether `t` is the platform table Tenant Media.
    pub(crate) fn is_tenant_media(&self, t: &TableDef) -> bool {
        self.engine.program.objects.get(t.object as usize).and_then(|o| o.id) == Some(crate::permissions::TENANT_MEDIA)
    }

    /// `RecordRef.Get(RecordId)` of Tenant Media while a row an AL Insert stored is there: the case reads that row with a
    /// Record `Get` (Y48TenantMediaRows.al:646) only. Every other table and a temporary record keep their path.
    pub(crate) fn tenant_media_recordref_get_check(&self, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        if !rec.borrow().temporary && self.is_tenant_media(t) && self.tenant_media_has_stored_rows(t, rec)? {
            return tenant_media_read_refusal();
        }
        Ok(())
    }

    /// Whether the store holds a Tenant Media row an AL Insert stored.
    fn tenant_media_has_stored_rows(&self, t: &TableDef, rec: &Rec) -> Result<bool, Signal> {
        let cx = self.cx_for(rec, t)?;
        Ok(!self.store.stored_rows(&cx, t).is_empty())
    }

    /// Whether the key of `rec` names a row an AL Insert stored, not one the media store computes.
    pub(super) fn tenant_media_stored_key(&self, t: &TableDef, rec: &Rec) -> Result<bool, Signal> {
        let cx = self.cx_for(rec, t)?;
        let key = t.primary_key(&rec.borrow().fields);
        Ok(self
            .store
            .stored_rows(&cx, t)
            .iter()
            .any(|row| super::stale_rule::same_key(&t.primary_key(row), &key)))
    }

    /// Whether `id` is the ID of a Tenant Media row an AL Insert stored. The media store does not hold such a row, so what a
    /// Media or MediaSet field that holds its ID does with it is not pinned (Y48TenantMediaRows.al:634-649 sets no field).
    pub(crate) fn is_stored_tenant_media_id(&self, id: [u8; 16]) -> bool {
        let Some(t) = self.table_def(crate::permissions::TENANT_MEDIA) else {
            return false;
        };
        let Some(&key) = t.keys.first().and_then(|k| k.first()) else {
            return false;
        };
        self.store
            .stored_rows(&self.cx, &t)
            .iter()
            .any(|row| matches!(row.get(key), Some(Value::Guid(g)) if *g == id))
    }

    /// The Record calls on a stored Tenant Media row: an Insert is the measured one or refused; a Delete of a stored row is
    /// the measured `Delete()` or refused; a CalcFields of one is refused; while such a row is there, a Find is refused (a
    /// RecordRef Get: `tenant_media_recordref_get_check`). The case makes both writes through a Record
    /// variable in their statement form: through a RecordRef or with the return value used they are refused. Every other
    /// table and a temporary record keep their path.
    pub(super) fn tenant_media_row_check(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        if rec.borrow().temporary || !self.is_tenant_media(t) {
            return Ok(());
        }
        match method {
            RecordOp::Insert => self.tenant_media_insert_check(t, rec, args),
            RecordOp::Delete if self.tenant_media_stored_key(t, rec)? => self.tenant_media_delete_check(t, args),
            RecordOp::CalcFields if self.tenant_media_stored_key(t, rec)? => {
                unsupported("CalcFields of a Tenant Media row an AL Insert stored (not pinned)")
            }
            // The case reads the stored row with a Record `Get` (:646) only.
            RecordOp::Find if self.tenant_media_has_stored_rows(t, rec)? => tenant_media_read_refusal(),
            _ => Ok(()),
        }
    }

    /// `Insert(false)` (Y48TenantMediaRows.al:644), a Record statement, of a row whose `ID` is a GUID no row has (:642, a fixed GUID that
    /// Initialize, :650-656, removed first), with a `Description` (:643) and every other field as `Init` left it (:641).
    /// Any non-null ID and any non-empty Description are accepted on purpose: they are data the row stores as given and the
    /// Get reads back (BC keeps an AL-inserted row as inserted, :469 and :475), not a shape of the call.
    /// No insert subscriber or global insert trigger: the case has none.
    fn tenant_media_insert_check(&mut self, t: &TableDef, rec: &Rec, args: &[Value]) -> Result<(), Signal> {
        let measured_call = args.len() == 1
            && !self.through_recordref
            && !self.ret_used
            && !arg_bool(args, 0)?
            && !self.listened(t, "onbeforeinsertevent", None)?
            && !self.listened(t, "onafterinsertevent", None)?
            && !self.global_enabled(t, GlobalOp::Insert)?;
        if !measured_call {
            return unsupported("an Insert into Tenant Media other than the measured Insert(false) (not pinned)");
        }
        let measured_row = {
            let record = rec.borrow();
            t.fields
                .iter()
                .zip(&record.fields)
                .all(|(field, value)| match (field.id, value) {
                    (1, Value::Guid(id)) => *id != [0; 16],
                    (2, Value::Text(text)) => !text.is_empty(),
                    (3, Value::Blob(blob)) => {
                        let content = blob.borrow();
                        content.bytes.is_empty() && content.render_pending.is_none()
                    }
                    // The system fields are the platform's (SystemId is given at the insert).
                    (id, _) if id >= 2_000_000_000 => true,
                    (_, value) => ops::compare(value, &Value::default_for(&field.ty)).is_ok_and(|order| order.is_eq()),
                })
        };
        if !measured_row {
            return unsupported(
                "an Insert into Tenant Media with a null ID, no Description, content or other metadata (not pinned)",
            );
        }
        let cx = self.cx_for(rec, t)?;
        let key = t.primary_key(&rec.borrow().fields);
        let exists = self
            .store
            .get(&cx, t, &key)
            .map_err(|e| Signal::unsupported(format!("{e}")))?
            .is_some();
        if exists {
            return unsupported("an Insert into Tenant Media of an ID that has a row (not pinned)");
        }
        Ok(())
    }

    /// `Delete()` without an argument (Y48TenantMediaRows.al:648), a Record statement, and no delete subscriber or global delete trigger.
    fn tenant_media_delete_check(&mut self, t: &TableDef, args: &[Value]) -> Result<(), Signal> {
        if args.is_empty()
            && !self.through_recordref
            && !self.ret_used
            && !self.listened(t, "onbeforedeleteevent", None)?
            && !self.listened(t, "onafterdeleteevent", None)?
            && !self.global_enabled(t, GlobalOp::Delete)?
        {
            return Ok(());
        }
        unsupported("a Delete of a Tenant Media row an AL Insert stored other than the measured Delete() (not pinned)")
    }
}

fn tenant_media_read_refusal() -> Result<(), Signal> {
    unsupported("a Find or a RecordRef Get of Tenant Media while a row an AL Insert stored is there (not pinned)")
}
