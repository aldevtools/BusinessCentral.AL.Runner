//! The values a Code field the database stores as a number (`SQLDataType`) takes on Insert, Modify and Rename.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<al_builtins::value::RecordState>>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// The fields a position of key `key` is made of: the key's, then the primary key's.
    pub(crate) fn position_fields(t: &TableDef, key: usize) -> Vec<usize> {
        if key == 0 {
            t.keys[0].clone()
        } else {
            t.keys[key].iter().chain(&t.keys[0]).copied().collect()
        }
    }

    /// The order of the values `x` and `y` at place `i` of a position made of `fields`. A Code field stored as a number compares by
    /// number (Y82PkTests.al `PkInt_Typed`, `SecFInt_Typed`: `SetAscending(K, false)` on a typed record).
    pub(crate) fn compare_in_position(
        t: &TableDef,
        fields: &[usize],
        i: usize,
        x: &Value,
        y: &Value,
    ) -> std::cmp::Ordering {
        use al_data::CodeStorage::{BigInteger, Integer, Variant};
        match fields.get(i).map(|&f| t.fields[f].code_storage) {
            Some(Integer | BigInteger | Variant) => t.cmp_key_field(fields[i], x, y),
            _ => al_builtins::ops::compare(x, y).unwrap_or(std::cmp::Ordering::Equal),
        }
    }

    /// A Rename to a value a number-stored Code key cannot hold fails like an Insert and changes nothing (Y82PkTests.al
    /// `PkInt_Rename_ToAbc`, `PkBig_Rename_ToAbc`); a value it takes is stored as the number. Then the checks of the new key's
    /// AutoIncrement fields. The AutoIncrement error is returned, not raised: it comes after the OnRename trigger, and only for a
    /// row that exists (record/lt-r6863-rename-trigger/R6863RenameTrigger.al).
    pub(crate) fn check_rename_values(
        &self,
        rec: &Rec,
        t: &TableDef,
        old: &[Value],
        new: &mut Vec<Value>,
    ) -> Result<Option<Signal>, Signal> {
        let deferred = match self.check_rename_auto_increment(t, old, new) {
            Err(e @ Signal::Error(_)) => Some(e),
            other => {
                other?;
                None
            }
        };
        let held = std::mem::replace(&mut rec.borrow_mut().fields, new.clone());
        let checked = self.check_code_storage(t, rec, false);
        *new = std::mem::replace(&mut rec.borrow_mut().fields, held);
        checked.map(|()| deferred)
    }

    /// A Code field the database stores as a number takes plain numbers and the empty value; the check runs where a record is
    /// Inserted or Modified, and the values are stored as the number they are. Pinned (corpus writer's probe, needs corpus bump,
    /// tests/al-language/record/lt-y82-sqldatatype-code/ Y82InsertTests.al `PkInt_Insert_*`, `PkBig_Insert_*`, `PkVnt_Insert_*`,
    /// `SecFInt_Insert_*`, `SecFBig_Insert_*`):
    /// * `Integer` and `BigInteger`: `0` and a number with a leading zero fail with error code 22928070 (`NDBCS:BadCSideCodeData`);
    ///   letters (also with digits), `-5` and `1.5` fail with 22928072 (`NDBCS:BadSqlIntCodeData`), as does `3000000000` for
    ///   Integer; `+5` and ` 12` are taken and stored as 5 and 12. A Modify of such a value is pinned for Integer
    ///   (record/lt-x-table-props/XTablePropsTests.al:352-353, :368-369, :384-385) and refused for BigInteger.
    /// * `Variant` takes every value; a text of digits is stored as the number (`007` as `7`, `0` stays).
    ///
    /// Any other value is not pinned: refused.
    pub(crate) fn check_code_storage(&self, t: &TableDef, rec: &Rec, modify: bool) -> Result<(), Signal> {
        if !self.engine.table_facts[t.object as usize].code_sql_type {
            return Ok(());
        }
        let schema = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .expect("table");
        for (i, f) in t.fields.iter().enumerate() {
            if !matches!(f.ty, RtType::Code(_)) {
                continue;
            }
            let declared = schema.fields[i]
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("SQLDataType"))
                .map(|(_, v)| v.trim());
            let Some(declared) = declared.filter(|v| !v.eq_ignore_ascii_case("Varchar")) else {
                continue;
            };
            let value = al_builtins::native::text_of(&rec.borrow().fields[i])
                .map(|v| v.to_string())
                .unwrap_or_default();
            match f.code_storage {
                al_data::CodeStorage::Other => {
                    if !value.is_empty() {
                        return unsupported(format!(
                            "a value in the Code field {} with SQLDataType = {declared}: how the database stores and compares it is not pinned",
                            f.name
                        ));
                    }
                }
                al_data::CodeStorage::Variant => {
                    let digits = value.trim_start_matches(' ');
                    if !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit()) {
                        let number = digits.trim_start_matches('0');
                        if number.is_empty() && digits.len() > 1 {
                            return unsupported(format!(
                                "the value '{value}' in the Code field {} with SQLDataType = Variant: not pinned",
                                f.name
                            ));
                        }
                        let stored = if number.is_empty() { "0" } else { number };
                        rec.borrow_mut().fields[i] = Value::Code(stored.into());
                    }
                }
                _ => self.check_number_code(t, rec, i, &value, modify)?,
            }
        }
        Ok(())
    }

    /// `check_code_storage` for field `i`, stored as an Integer or BigInteger.
    fn check_number_code(&self, t: &TableDef, rec: &Rec, i: usize, value: &str, modify: bool) -> Result<(), Signal> {
        let f = &t.fields[i];
        let integer = f.code_storage == al_data::CodeStorage::Integer;
        if value.is_empty() {
            return Ok(());
        }
        let trimmed = value.trim_start_matches(' ');
        let (plus, body) = match trimmed.strip_prefix('+') {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };
        let digits = !body.is_empty() && body.bytes().all(|c| c.is_ascii_digit());
        let bad_cs = ("22928070", "NDBCS:BadCSideCodeData");
        let bad_sql = ("22928072", "NDBCS:BadSqlIntCodeData");
        let number = if digits && !body.starts_with('0') {
            let fits = if integer {
                body.parse::<i32>().is_ok()
            } else {
                body.parse::<i64>().is_ok()
            };
            if fits {
                rec.borrow_mut().fields[i] = Value::Code(body.into());
                return Ok(());
            }
            if !integer {
                return unsupported(format!(
                    "the value '{value}' in the BigInteger Code field {}: not pinned",
                    f.name
                ));
            }
            bad_sql
        } else if digits && !plus && trimmed.len() == value.len() {
            bad_cs
        } else if !plus
            && trimmed.len() == value.len()
            && (value.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && value.bytes().any(|c| c.is_ascii_uppercase())
                || value
                    .strip_prefix('-')
                    .is_some_and(|d| !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit()))
                || value.split_once('.').is_some_and(|(a, b)| {
                    !a.is_empty() && !b.is_empty() && (a.bytes().chain(b.bytes())).all(|c| c.is_ascii_digit())
                }))
        {
            bad_sql
        } else {
            return unsupported(format!(
                "the value '{value}' in the Code field {}, which the database stores as a number: the outcome is not pinned",
                f.name
            ));
        };
        if modify && !integer {
            return unsupported(format!(
                "a Modify of '{value}' into the Code field {} with SQLDataType = BigInteger: the outcome is not pinned",
                f.name
            ));
        }
        // Only a table named as its caption, with one primary key field of type Integer or Code named as its caption, is pinned.
        let pk = &t.keys[0];
        if pk.len() != 1
            || !matches!(t.fields[pk[0]].ty, RtType::Integer | RtType::Code(_))
            || self.caption(t) != t.name
            || self.field_caption(t, pk[0]) != t.fields[pk[0]].name
        {
            return unsupported(format!(
                "the error of a value that is no integer in Code field {} of {}: the identification of this primary key is not pinned",
                f.name, t.name
            ));
        }
        let key = match &rec.borrow().fields[pk[0]] {
            Value::Integer(n) => n.to_string(),
            Value::Code(c) => c.to_string(),
            _ => return unsupported("a primary key field that holds neither an Integer nor a Code"),
        };
        Err(crate::transactions::al_error_code(
            &format!(
                "The {} table contains a value that is not valid in an integer Code field:\\\\{}='{key}'\\Error code: {}",
                t.name, t.fields[pk[0]].name, number.0
            ),
            number.1,
        ))
    }
}
