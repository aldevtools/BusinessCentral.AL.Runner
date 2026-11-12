//! The text a report dataset holds for a Blob column, and which columns are Media (moved out of reports.rs).

use super::*;

/// Whether column `c` of the report itself (`own`, not one a reportextension adds) reads a field of a global record variable
/// of the report: its source is `Global.Field` (`places_table` is set) and it names no field of the data item.
fn global_field(c: &ir::ReportColumn, own: bool) -> bool {
    own && c.field.is_none() && c.item_field.is_none() && c.places_table.is_some()
}

/// Whether `buf` is the Blob of such a field that a stream wrote into while it held nothing, of a global record of the report
/// (`owner`'s globals) that is not temporary and was never read, inserted or given another record's buffer: the shape of
/// `InMemory` of DatasetBlobColumnOfGlobalRecord.al:84-86 (Init, a stream, a write). A write after a Get (also of an empty row,
/// also after Clear of the field) and a write into a temporary record are not pinned.
fn global_written_empty(c: &ir::ReportColumn, owner: &Inst, own: bool, b: &al_builtins::stream::BlobRef) -> bool {
    let buf = b.borrow();
    let shape = global_field(c, own)
        && buf.loaded
        && buf.written_base.as_deref().is_some_and(<[u8]>::is_empty)
        && !buf.temp_row
        && !buf.inserted
        && !buf.persisted
        && !buf.refetch
        && !buf.db_row;
    shape
        && owner.borrow().globals.iter().any(|g| {
            let Value::Record(r) = g else { return false };
            let r = r.borrow();
            let holds = c
                .places_field
                .and_then(|pos| r.fields.get(pos))
                .is_some_and(|v| matches!(v, Value::Blob(x) if std::sync::Arc::ptr_eq(x, b)));
            holds && !r.temporary && r.position.is_none() && r.row_origin.is_none() && !r.inserted
        })
}

impl Session {
    /// The text of a Blob column: the Base64 text of the content, an empty element when there is none (reports/layouts-saveas/
    /// BcXmlBlob.al:93-154, `Hello BC` is `SGVsbG8gQkM=`, the bytes 0, 255, 65 are `AP9B`). A field of the data item that the report
    /// did not calculate shows what the row stores (:102, report 80976 has no CalcFields).
    ///
    /// A field of a global record variable of the report (`GetOnlyRow.Data`, also a temporary record, a second variable of the
    /// data item's table and Company Information.Picture) that was read and not calculated is an empty element, whatever the row
    /// holds (reports/dataset/DatasetBlobColumnOfGlobalRecord.al:231 `GetOnly`, `FindOnly`, `LoadFields`, `EmptyGet`, `TempGet`,
    /// :251 `OtherData`, :281 and :317 `Get only`; BC 27, 28 and 29; al-runner issue 7399). One a stream wrote into while it was
    /// empty and never stored shows what was written (:231 `InMemory`, `global_written_empty`). Other Blobs a stream wrote after a
    /// read, and a column a reportextension adds, are not pinned.
    pub(super) fn blob_column_text(
        &mut self,
        c: &ir::ReportColumn,
        (rec, owner, own): (&Rec, &Inst, bool),
        b: &al_builtins::stream::BlobRef,
    ) -> Result<String, Signal> {
        let written_pinned = b.borrow().written && global_written_empty(c, owner, own, b);
        {
            let buf = b.borrow();
            if let Some(what) = buf.render_pending.as_deref()
                && al_builtins::stream::is_unloaded_write_mark(what)
            {
                return Err(al_builtins::stream::render_pending_error(what));
            }
            if buf.render_pending.is_some() {
                return unsupported(format!(
                    "report column {} holds a Blob a report was saved to in a rendered format (ADR-040)",
                    c.name
                ));
            }
            if buf.written && !written_pinned {
                return unsupported(format!(
                    "report column {} holds a Blob a stream wrote after it was read (not pinned)",
                    c.name
                ));
            }
            if buf.loaded {
                return Ok(al_builtins::platform_native::b64_encode(&buf.bytes, false));
            }
            if global_field(c, own) {
                return Ok(String::new());
            }
        }
        let pos = c
            .field
            .or(c.item_field)
            .filter(|&pos| matches!(rec.borrow().fields.get(pos), Some(Value::Blob(x)) if !x.borrow().loaded && !x.borrow().written));
        let Some(pos) = pos else {
            return unsupported(format!(
                "report column {} holds a Blob that was not calculated and is not a field of the data item (not pinned)",
                c.name
            ));
        };
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let pk = t.primary_key(&rec.borrow().fields);
        let stored =
            al_data::Store::get(self.store_for(rec), &cx, &t, &pk).map_err(|e| Signal::unsupported(format!("{e}")))?;
        match stored.map(|row| row[pos].clone()) {
            Some(Value::Blob(src)) if src.borrow().render_pending.is_none() => {
                Ok(al_builtins::platform_native::b64_encode(&src.borrow().bytes, false))
            }
            _ => unsupported(format!(
                "report column {}: the Blob of a row the store does not hold as written (not pinned)",
                c.name
            )),
        }
    }

    /// Whether column `c` is a Media or MediaSet field of the data item's table, or a Media field of a global record variable
    /// of the report in a column of the report itself (`own`; an empty one is an empty element: reports/dataset/
    /// DatasetBlobColumnOfGlobalRecord.al:231 `MediaGetOnly`, `MediaNeverRead`; al-runner issue 7399). A MediaSet of a global
    /// record and a column a reportextension adds are not pinned and keep the Guid text.
    pub(super) fn media_field(&self, t: &Option<Arc<al_data::TableDef>>, c: &ir::ReportColumn, own: bool) -> bool {
        use al_builtins::library_kind::LibraryKind as L;
        use al_builtins::value::RtType;
        let field_type =
            |t: &Option<Arc<al_data::TableDef>>, pos: usize| t.as_ref()?.fields.get(pos).map(|f| f.ty.clone());
        match c.field.or(c.item_field) {
            Some(pos) => matches!(field_type(t, pos), Some(RtType::Library(L::Media | L::MediaSet))),
            None if own => {
                let global = c.places_table.and_then(|g| self.engine.tables.get(g)?.clone());
                c.places_field
                    .is_some_and(|pos| matches!(field_type(&global, pos), Some(RtType::Library(L::Media))))
            }
            None => false,
        }
    }
}
