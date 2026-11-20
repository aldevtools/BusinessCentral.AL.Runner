//! The field values of a fresh record (`Session::initial_fields`): each field's `InitValue` where declared, else its type's
//! default, worked out once per table and session (`InitialFields`), and a record state built around given field values
//! (`state_with_fields`).

use super::Rec;
use crate::Session;
use al_builtins::ops;
use al_builtins::record_fields::Fields;
use al_builtins::value::{RecordState, Value};
use std::rc::Rc;

impl Session {
    /// Field values of a fresh record: `InitValue` where declared, else the type default.
    pub(crate) fn initial_fields(&mut self, table: usize) -> Fields {
        // The values are the same for every record of the table, so they are worked out once per session and copied
        // (a text shares its buffer, copy on write). A value that is a shared object (a Blob's buffer) is built fresh
        // for each record in its slot of the copy (issue 6133: a local record variable of a table with a Blob field
        // built every field anew on each call).
        if let Some(template) = self.initial_cache.get(&table) {
            let mut fields = template.values.clone();
            if !template.fresh.is_empty() {
                let fresh = Rc::clone(&template.fresh);
                let engine = self.engine.clone();
                if let Some(t) = engine.program.objects[table].table.as_ref() {
                    for &i in fresh.iter() {
                        fields[i] = self.initial_field(&t.fields[i]);
                    }
                }
            }
            self.clear_uninstalled_init_values(table, &mut fields);
            return fields.into();
        }
        let engine = self.engine.clone();
        let Some(t) = engine.program.objects[table].table.as_ref() else {
            return Fields::default();
        };
        let fields: Vec<Value> = t.fields.iter().map(|f| self.initial_field(f)).collect();
        let fresh: Rc<[usize]> = fields
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                !matches!(
                    v,
                    Value::Null
                        | Value::Boolean(_)
                        | Value::Integer(_)
                        | Value::BigInteger(_)
                        | Value::Decimal(_)
                        | Value::Byte(_)
                        | Value::Char(_)
                        | Value::Text(_)
                        | Value::Code(_)
                        | Value::Date(..)
                        | Value::Time(_)
                        | Value::DateTime(_)
                        | Value::Duration(_)
                        | Value::Guid(_)
                        | Value::DateFormula(_)
                        | Value::Option(..)
                        | Value::Enum(..)
                )
            })
            .map(|(i, _)| i)
            .collect();
        // The cache keeps no shared object of this record: its slots are built fresh on every copy, and a Blob buffer the
        // cache held would count as one more holder of this record's buffer (`temp_blob_share.rs`).
        let mut values = fields.clone();
        for &i in fresh.iter() {
            values[i] = Value::Null;
        }
        self.initial_cache.insert(table, InitialFields { values, fresh });
        let mut fields = fields;
        self.clear_uninstalled_init_values(table, &mut fields);
        fields.into()
    }

    /// The InitValue of a table extension field of an app not installed yet does not apply: the field holds its type's
    /// default (install-upgrade/install-order/Y8InstallOrderTests.al:88-110, row 1 that XApp A's install writes has an empty
    /// "Y8 XB Mark", XApp B's InitValue 'B init' applies to B's own install row and at test time).
    fn clear_uninstalled_init_values(&mut self, table: usize, fields: &mut [Value]) {
        if self.events.not_installed_apps.as_ref().is_none_or(|a| a.is_empty()) {
            return;
        }
        let engine = self.engine.clone();
        let Some(t) = engine.program.objects[table].table.as_ref() else {
            return;
        };
        for (i, f) in t.fields.iter().enumerate() {
            if f.init.is_some()
                && let Some(d) = f.declared_in
                && d as usize != table
                && self.object_app_not_installed(d as usize)
                && let Some(slot) = fields.get_mut(i)
            {
                *slot = self.default_value(&f.ty);
            }
        }
    }

    /// The value of field `f` in a fresh record.
    fn initial_field(&mut self, f: &al_ir::Field) -> Value {
        match &f.init {
            Some(c) => ops::convert(crate::constant(c), &f.ty).unwrap_or_else(|_| self.default_value(&f.ty)),
            None => self.default_value(&f.ty),
        }
    }
}

/// The field values of a fresh record of one table (`Session::initial_fields`): the values to copy, and the positions of
/// those that are shared objects and are built fresh for each record instead.
pub(crate) struct InitialFields {
    values: Vec<Value>,
    fresh: Rc<[usize]>,
}

/// A copy of `rec`'s state with `fields` as its field values. Rec's own fields are left out of the copy instead of being
/// copied only to be replaced (issue 6133: NP Retail's tables are wide).
pub(crate) fn state_with_fields(rec: &Rec, fields: Fields) -> RecordState {
    let mut st = match rec.try_borrow_mut() {
        Ok(mut r) => {
            let own = std::mem::take(&mut r.fields);
            let st = r.clone();
            r.fields = own;
            st
        }
        Err(_) => rec.borrow().clone(),
    };
    st.fields = fields;
    st
}
