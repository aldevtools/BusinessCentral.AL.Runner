//! `Evaluate(RecordId, Text, 10)`: the bookmark `Format(RecordId, 0, 10)` writes (al_builtins::bookmark).

use crate::Session;
use al_builtins::bookmark::{self, Read};
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, RtType};

impl Session {
    /// The RecordId a bookmark text names: the table of the number in it, the key read from the bytes after it
    /// (types/lt-y49-recordid-format10/Y49RecordIdFormat10.al:176-262: a Code key and an Option, Code and Integer key read back,
    /// equal the record's RecordId and Get finds the row; a text without `;` is none: `Ok(None)`). A table number no table has,
    /// bytes that do not match the key, and the key shapes the corpus does not pin are refused.
    pub(crate) fn evaluate_record_id_bookmark(&mut self, text: &str) -> Result<Option<LibraryValue>, Signal> {
        let (table_id, rest) = match bookmark::read(text) {
            Read::NotABookmark => return Ok(None),
            Read::Refused(why) => return Err(Signal::Unsupported(why)),
            Read::Bookmark { table_id, rest } => (table_id, rest),
        };
        let objects: Vec<usize> = (self.engine.program.objects.heads().iter().enumerate())
            .filter(|(_, o)| o.table.is_some() && o.id == Some(table_id))
            .map(|(i, _)| i)
            .collect();
        let [object] = objects.as_slice() else {
            return Err(Signal::Unsupported(format!(
                "Evaluate of a bookmark of table number {table_id}, which no single table has (not pinned)"
            )));
        };
        let Some(t) = self.engine.tables.get(*object).cloned().flatten() else {
            return Err(Signal::Unsupported(
                "Evaluate of a bookmark of a table without a compiled schema".into(),
            ));
        };
        let shape: Vec<&RtType> = t.keys[0].iter().map(|&p| &t.fields[p].ty).collect();
        if !bookmark::record_id_shape_pinned(&shape) {
            return Err(Signal::Unsupported(
                "Evaluate of a bookmark of a key of a shape the corpus does not pin".into(),
            ));
        }
        let Some(key) = bookmark::decode_key(&shape, &rest) else {
            return Err(Signal::Unsupported(
                "Evaluate of a bookmark whose bytes do not match the table's key (not pinned)".into(),
            ));
        };
        let o = &self.engine.program.objects[*object];
        Ok(Some(LibraryValue::RecordId {
            table: *object as u32,
            table_no: table_id,
            name: o.name.clone(),
            key,
        }))
    }
}
