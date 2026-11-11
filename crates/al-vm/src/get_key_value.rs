//! The primary key `Record.Get` looks for: each value handed to it, converted to the type of its key field, and the value of
//! each key field it omits. Also the key values the not-found error names. The Get arm of `records/op_inner.rs` and the
//! permission check of `permissions.rs` build the key here and nowhere else.

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::ops;
use al_builtins::value::RecordState;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// The key a `Get` looks for: one value per primary key field, and how many of them the caller gave (the rest are omitted).
pub(crate) struct GetKey {
    pub(crate) pk: Vec<Value>,
    pub(crate) given: usize,
}

impl Session {
    /// The primary key `Get(args)` on record `rec` of table `t` looks for. `from_id` says the values came from a RecordId
    /// (`Get(RecordId)`, `RecordRef.Get(RecordId)`). The caller has already refused more values than the key has fields.
    pub(crate) fn get_key(&mut self, rec: &Rec, t: &TableDef, args: &[Value], from_id: bool) -> Result<GetKey, Signal> {
        let mut pk = Vec::with_capacity(t.keys[0].len());
        for (n, &field) in t.keys[0].iter().enumerate() {
            pk.push(match args.get(n) {
                Some(v) => self.get_key_value(v, &t.fields[field].ty)?,
                None => self.omitted_key_value(rec, t, field, args.len(), from_id)?,
            });
        }
        Ok(GetKey {
            given: args.len().min(pk.len()),
            pk,
        })
    }

    /// The value a `Get` with fewer values than the primary key has fields searches for in an omitted key field `field`.
    /// Only the shapes the corpus pins run; every other shape refuses (record/crud/GetFewerKeyValues.al, BC 27.5, 28.5, 29.0).
    fn omitted_key_value(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        field: usize,
        given: usize,
        from_id: bool,
    ) -> Result<Value, Signal> {
        let ty = t.fields[field].ty.clone();
        match (self.key_field_has_init_value(t, field), &ty) {
            // An omitted Integer field with InitValue = 5 is searched with 5, on a cleared record variable and after Init: the row
            // with 5 is found, the row with 0 is not (GetFewerKeyValues.al:740-776, Get_OneValue_OmittedFieldHasInitValue). That
            // case gives one plain value to a record that is not temporary and holds the InitValue in the omitted field; every other
            // shape refuses (a RecordId, a temporary record, no value, a record that holds another value there).
            (true, RtType::Integer) => {
                let init = self.initial_fields(t.object as usize)[field].clone();
                let (temporary, held) = {
                    let r = rec.borrow();
                    (r.temporary, r.fields.get(field).cloned())
                };
                let holds_init = matches!((&held, &init), (Some(Value::Integer(h)), Value::Integer(i)) if h == i);
                if from_id || temporary || given == 0 || !holds_init {
                    return unsupported(format!(
                        "Get with fewer values than the primary key has fields, where the omitted key field `{}` has an InitValue, \
                         through a RecordId, on a temporary record, with no value or on a record that holds another value there \
                         (not pinned)",
                        t.fields[field].name
                    ));
                }
                Ok(init)
            }
            (true, _) => unsupported(format!(
                "Get with fewer values than the primary key has fields, where the omitted key field `{}` has an InitValue and is \
                 not an Integer (not pinned)",
                t.fields[field].name
            )),
            // Without an InitValue an omitted field is searched with the blank value of its type, whatever the record variable
            // holds there, whatever filters are set (any filter group) and whatever the current key is
            // (GetFewerKeyValues.al Get_NoValues_AllBlankRowPresent, Get_OneValue_OnlyRowsWithNonBlankRest,
            // Get_OneValue_RecordHoldsValueInOmittedField, Get_OneValue_FilterOnOmittedField, Get_OneValue_FilterOnAnotherField,
            // Get_OneValue_SecondaryKeyIsCurrent; record/crud/RcAGetFind.al:532-577 Get_Fewer_Values_Than_Key_Fields_BQ). Types pinned:
            // Code, Integer, Option, Guid (Get_FewerValues_IntegerOptionGuidOmitted_Found), Boolean, Decimal, BigInteger, DateTime,
            // Time, Text, Duration (Get_OneValue_SevenTypesOmitted), DateFormula and Date (Get_OneValue_DateFormulaOmitted), an
            // Option whose first member has a name (Get_OneValue_OptionWithNamedFirstMemberOmitted), and an Enum, with ordinal 0 even
            // when the enum has no member 0 (Get_OneValue_EnumWithoutOrdinalZeroOmitted).
            (
                false,
                RtType::Code(_)
                | RtType::Text(_)
                | RtType::Integer
                | RtType::BigInteger
                | RtType::Decimal
                | RtType::Boolean
                | RtType::Option(_)
                | RtType::Guid
                | RtType::Date
                | RtType::Time
                | RtType::DateTime
                | RtType::Duration
                | RtType::DateFormula,
            ) => Ok(self.default_value(&ty)),
            (false, RtType::Enum(e)) => Ok(Value::Enum(*e, 0)),
            (false, _) => unsupported(format!(
                "Get with fewer values than the primary key has fields, where the omitted key field `{}` is of a type the corpus \
                 does not pin",
                t.fields[field].name
            )),
        }
    }

    /// A RecordId that holds `values` key values of `t`, for `RecordRef.Get(RecordId)` (the Record form reads the RecordId in
    /// the Get arm and passes `from_id` to [`Self::get_key`]). An omitted key field with an InitValue is not pinned through a
    /// RecordId: refuse it, as [`Self::omitted_key_value`] does.
    pub(crate) fn record_id_omitted_fields(&self, t: &TableDef, values: usize) -> Result<(), Signal> {
        match t.keys[0]
            .iter()
            .skip(values)
            .find(|&&f| self.key_field_has_init_value(t, f))
        {
            Some(&f) => unsupported(format!(
                "Get with fewer values than the primary key has fields, where the omitted key field `{}` has an InitValue, \
                 through a RecordId (not pinned)",
                t.fields[f].name
            )),
            None => Ok(()),
        }
    }

    /// Whether key field `field` of `t` declares an InitValue.
    fn key_field_has_init_value(&self, t: &TableDef, field: usize) -> bool {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|it| it.fields.get(field))
            .is_some_and(|f| f.init.is_some())
    }

    /// The key field values the not-found error of `Get(args)` names, in the record's field slots: `fields` (the record
    /// variable's values) with each key field replaced. Error text and code: RcAGetFind.al:532-577 Get_Fewer_Values_Than_Key_Fields_BQ
    /// and GetRecordId.al P98_Get_RecordId_Missing_*.
    pub(crate) fn get_not_found_ident(
        &self,
        t: &TableDef,
        args: &[Value],
        key: &GetKey,
        mut fields: Vec<Value>,
    ) -> Result<Vec<Value>, Signal> {
        for (n, &field) in t.keys[0].iter().enumerate() {
            fields[field] = match (args.get(n), &t.fields[field].ty) {
                // An omitted key field is named with the value searched for: its blank value
                // (GetFewerKeyValues.al Get_FewerValues_NoRow_ErrorTexts, Get_FewerValues_IntegerOptionGuidOmitted_Missing,
                // Get_AssignmentForm_OneValueOfTwo). For one with an InitValue the error is not pinned.
                // A Get whose result is used answers false on a miss and names nothing, so it runs (GetFewerKeyValues.al:740-776
                // finds its rows; the used miss raises no error).
                (None, _) if !self.ret_used && self.key_field_has_init_value(t, field) => {
                    return unsupported(
                        "the not-found error of a Get with fewer values than the primary key has fields, where an omitted key \
                         field has an InitValue (not pinned)",
                    );
                }
                // The message names the key as the caller gave it: an Option for an Enum key
                // reads 'Three', not the ordinal the enum lacks
                // (corpus types/PEOptionEnumBigText.al:225-259, probed on BC 27.5).
                (Some(o @ Value::Option(..)), RtType::Enum(_)) => o.clone(),
                // An enum of another type is named by the caller's value, its caption in its own enum, also
                // for an ordinal the key's enum lacks (errors/P138EnumGetNotFound.al:82-86,
                // :103, :178-180).
                (Some(e @ Value::Enum(from, _)), RtType::Enum(to)) if from != to => e.clone(),
                // An Integer is named as the key's enum value: its caption, or the number for an ordinal the
                // enum lacks (P138EnumGetNotFound.al:126-130, :153-155).
                (Some(Value::Integer(n)), RtType::Enum(to)) => Value::Enum(*to, *n),
                _ => key.pk[n].clone(),
            };
        }
        Ok(fields)
    }

    /// The key value `v` for a key field of type `ty`.
    pub(crate) fn get_key_value(&self, v: &Value, ty: &RtType) -> Result<Value, Signal> {
        Ok(match v {
            // A text for an Integer or BigInteger key reads as Evaluate reads it: surrounding spaces and a sign are fine, anything
            // else raises (GetWithTextArgument.al:Get_TextForIntegerKey, Get_TextVariableDirect; record/Y70Probes.al
            // RecordGet_TextOnIntegerKey `int_text5`, `int_padded`, `int_plus`, `int_x`, `int_empty`, `int_decimal_text`, and
            // RecordGet_TextOnBigIntegerKey; BC 27 names the type unknown, BC 28 and later say the value can't be evaluated). A one-character
            // literal is a Char in a Variant and reads as the text of that character (`int_text5` passes '5', `int_x` passes 'x').
            Value::Text(_) | Value::Code(_) | Value::Char(_) if matches!(ty, RtType::Integer | RtType::BigInteger) => {
                let s = match v {
                    Value::Char(c) => String::from_utf16_lossy(&[*c]),
                    other => al_builtins::native::text_of(other)
                        .map(|t| t.to_string())
                        .unwrap_or_default(),
                };
                let (name, parsed) = match ty {
                    RtType::Integer => ("Integer", s.trim_matches(' ').parse::<i32>().ok().map(Value::Integer)),
                    _ => (
                        "BigInteger",
                        s.trim_matches(' ').parse::<i64>().ok().map(Value::BigInteger),
                    ),
                };
                match parsed {
                    Some(n) => n,
                    None if self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28 => {
                        return Err(crate::transactions::al_error_code(
                            &format!("The value \"{s}\" can't be evaluated into type {name}."),
                            "Evaluate:Format",
                        ));
                    }
                    None => {
                        return Err(crate::transactions::al_error_code(
                            &format!("The type Nav{name} is unknown."),
                            "NotSupportedType",
                        ));
                    }
                }
            }
            // A non-negative Integer for a Text key is its digits: Get(1) finds the row '1', Get(0) the row '0'
            // (record/Y70Probes.al RecordGet_BooleanOnTextKey `text_int1`, `text_int0`). A negative one and one longer
            // than the field are not pinned.
            Value::Integer(i) if matches!(ty, RtType::Text(_)) => {
                let digits = i.to_string();
                let fits = matches!(ty, RtType::Text(len) if len.is_none_or(|l| digits.len() <= l as usize));
                if *i < 0 || !fits {
                    return Err(Signal::Unsupported(
                        "Get with a negative or too long Integer on a Text key (not pinned)".into(),
                    ));
                }
                Value::Text(digits.into())
            }
            // An Integer (literal or variable) for a Code key reads as its digits: found, not found and the
            // not-found error (record/crud/AtGetIntegerOnCodeKey.al:86-140, Integer 123 on a
            // Code[20] key). A negative number, one longer than the field and a Text key are not pinned.
            Value::Integer(i) if matches!(ty, RtType::Code(_)) => {
                let digits = i.to_string();
                let fits = matches!(ty, RtType::Code(len) if len.is_none_or(|l| digits.len() <= l as usize));
                if *i < 0 || !fits {
                    return Err(Signal::Unsupported(
                        "Get with a negative or too long Integer on a Code key (not pinned)".into(),
                    ));
                }
                Value::Code(digits.into())
            }
            // An Option or Enum value for a Code key looks for the member's name: `ClientType::Web`, an own Option variable and an
            // Enum value find the row `WEB`, not the row of the ordinal (types/operators-expressions/Y71ValueConversions.al:
            // 218-328). An Enum member with a caption of its own, a name longer than the field and a Text key are not pinned.
            Value::Option(..) | Value::Enum(..) if matches!(ty, RtType::Code(_)) => {
                let name = self.member_name_for_key(v)?;
                let fits = matches!(ty, RtType::Code(len) if len.is_none_or(|l| name.chars().count() <= l as usize));
                if !fits {
                    return Err(Signal::Unsupported(
                        "Get with an Option or Enum value whose name is longer than the Code key (not pinned)".into(),
                    ));
                }
                Value::Code(name.to_uppercase().into())
            }
            v => ops::convert(v.clone(), ty)?,
        })
    }

    fn member_name_for_key(&self, v: &Value) -> Result<String, Signal> {
        let refused = || {
            Signal::Unsupported("Get with an Option or Enum value of a member without a name, or with a caption of its own, on a Code key (not pinned)".into())
        };
        match v {
            Value::Option(o, members) => members
                .get(*o as usize)
                .filter(|m| !m.is_empty())
                .cloned()
                .ok_or_else(refused),
            Value::Enum(id, value) => {
                if let Some(why) = self.enum_ml_unpinned(*id, Some(*value), "the key value of an enum") {
                    return Err(Signal::unsupported(why));
                }
                let o = &self.engine.program.objects[*id as usize];
                let name = o
                    .enum_values
                    .iter()
                    .find(|(x, _)| x == value)
                    .map(|(_, n)| n.clone())
                    .ok_or_else(refused)?;
                match o.enum_captions.iter().find(|(x, _)| x == value) {
                    Some((_, c)) if !c.eq_ignore_ascii_case(&name) => Err(refused()),
                    _ => Ok(name),
                }
            }
            _ => Err(refused()),
        }
    }
}
