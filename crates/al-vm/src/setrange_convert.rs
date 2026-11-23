//! A value of another type than the field's given to `FieldRef.SetRange`, `FieldRef.SetFilter`, `Record.SetRange` or
//! `Record.SetFilter` from AL code (al-runner issues 8857, 8867, 8868, 8869, 8870): BC converts it into the field's type
//! before it sets the filter, or raises the conversion error (SetFilter without the filter error around it). Lowering (al-sema
//! `setrange_written`) tells the calls apart by how their values are written: a Text into a Guid field raises another error
//! when it is written as a Variant. Only the cells corpus PR 1552 measures convert here
//! (recordref-fieldref/fieldref/SrcSetRangeConvert.al, SrcFilterConvertKinds.al, not yet pinned: assumption
//! setrange-other-type-convert). A value of a pair of types those cells measure is refused in a shape or of a size they do
//! not measure, unless its cell passes on the conversion `setrange_filter` and the filter parser give it; a pair of types
//! they do not measure keeps that conversion.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::fieldref_variant_cells::{temporal_value_error, text_evaluate_error, variant_guid_error};
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, RecordState, RtType, Value};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

/// How the values of a call are written (al-sema `setrange_written`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Written {
    /// Every value typed.
    Typed,
    /// Every value a Variant.
    Variant,
    /// A Variant and a typed value.
    Mixed,
}

/// The catalog id of `id` and, for an id lowering gives by how the values are written, that.
pub(crate) fn catalog_id(id: &str) -> (&str, Option<Written>) {
    use al_builtins::builtin_id::{BuiltinId, ids};
    let Some(b) = BuiltinId::of(id) else { return (id, None) };
    let written = match b {
        ids::FIELDREF_SETRANGETYPED_1 | ids::FIELDREF_SETFILTERTYPED_1 => Some(Written::Typed),
        ids::FIELDREF_SETRANGEVARIANT_1 | ids::FIELDREF_SETFILTERVARIANT_1 => Some(Written::Variant),
        ids::FIELDREF_SETRANGEMIXED_1 => Some(Written::Mixed),
        _ => None,
    };
    (b.written_base().name(), written)
}

/// The call a value is given to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Call {
    FieldRefSetRange,
    /// `FieldRef.SetFilter('%1', X)`: the filter text `%1` and one value.
    FieldRefSetFilter,
    /// `FieldRef.SetFilter` with another filter text or more values.
    FieldRefSetFilterOther,
    RecordSetRange,
    RecordSetFilter,
}

/// What a measured cell says of a value of another type than the field's.
enum Cell {
    /// BC converts it into this value or raises this error, where main's conversion gives another answer.
    Convert(Result<Value, Signal>),
    /// BC gives what main's conversion gives (the cell passes on main).
    Main,
    /// A value of a pair the probe measures, but not this value: refused.
    Unmeasured,
}

/// The measured calls: the routes of the probe (a, aLit, c: typed SetRange; b: Variant SetRange; d1, d2: a Variant and a typed
/// value; e, eLit: typed SetFilter('%1', X); f: Variant SetFilter('%1', V); gr: Record.SetRange(Field, V)) with their number of
/// values. Mixed only on a Date, Time or DateTime field (d1 and d2 run on those only).
fn measured_call(call: Call, written: Option<Written>, values: usize, field: &RtType) -> bool {
    let temporal = matches!(field, RtType::Date | RtType::Time | RtType::DateTime);
    match (call, written) {
        (Call::FieldRefSetRange, Some(Written::Typed)) => values == 1 || values == 2,
        (Call::FieldRefSetRange, Some(Written::Variant)) => values == 1,
        (Call::FieldRefSetRange, Some(Written::Mixed)) => values == 2 && temporal,
        (Call::FieldRefSetFilter, Some(Written::Typed | Written::Variant)) => values == 1,
        (Call::RecordSetRange, Some(Written::Variant)) => values == 1,
        _ => false,
    }
}

impl Session {
    /// Converts the values of `FieldRef.<method>` on the FieldRef `lib` (`args`, written as `written`, `None` for a call lowering
    /// keeps the catalog id for) where a measured cell says so; raises its error; refuses a value of another type than the
    /// field's that no cell measures in this shape.
    pub(crate) fn fieldref_setrange_convert(
        &mut self,
        method: &str,
        lib: &Rc<RefCell<LibraryValue>>,
        args: &mut [Value],
        written: Option<Written>,
    ) -> Result<(), Signal> {
        // The probe reads a FieldRef of `RecordRef.Field`; one of `KeyRef.FieldIndex` is not measured.
        let (rec, pos, written) = match &*lib.borrow() {
            LibraryValue::FieldRef(r, p) => (r.clone(), *p, written),
            LibraryValue::KeyFieldRef(r, p) => (r.clone(), *p, None),
            _ => return Ok(()),
        };
        let (call, values) = match method {
            "SetRange" => (Call::FieldRefSetRange, &mut args[..]),
            "SetFilter" if args.len() == 2 && crate::filter_text_arg::filter_text(args.first()).as_str() == "%1" => {
                (Call::FieldRefSetFilter, &mut args[1..])
            }
            "SetFilter" if args.len() >= 2 => (Call::FieldRefSetFilterOther, &mut args[1..]),
            _ => return Ok(()),
        };
        self.setrange_convert(&rec, pos, values, written, call)
    }

    /// The same for `Record.SetRange(Field, ...)` and `Record.SetFilter(Field, Text, ...)` (`args`: the field position and the
    /// rest); `written` is `Some(Variant)` for `Record.SetRange(Field, V)` with `V` written as a Variant, `None` for every other
    /// shape (lowering keeps the catalog id).
    pub(crate) fn record_setrange_convert(
        &mut self,
        call: Call,
        rec: &Rc<RefCell<RecordState>>,
        args: &mut [Value],
        written: Option<Written>,
    ) -> Result<(), Signal> {
        let Some((Value::Integer(p), rest)) = args.split_first_mut() else {
            return Ok(());
        };
        let Ok(pos) = usize::try_from(*p) else { return Ok(()) };
        let values = match call {
            Call::RecordSetFilter => match rest.split_first_mut() {
                Some((_, values)) => values,
                None => return Ok(()),
            },
            _ => rest,
        };
        self.setrange_convert(rec, pos, values, written, call)
    }

    fn setrange_convert(
        &mut self,
        rec: &Rc<RefCell<RecordState>>,
        pos: usize,
        values: &mut [Value],
        written: Option<Written>,
        call: Call,
    ) -> Result<(), Signal> {
        if values.is_empty() {
            return Ok(());
        }
        let t = self.table(rec)?;
        let Some(f) = t.fields.get(pos) else { return Ok(()) };
        let state_measured = {
            let r = rec.borrow();
            !r.temporary && r.filter_group == 0
        };
        // The probe's facts: an en-US session, a stored field of the table's own (no FlowField, FlowFilter or system field)
        // on a Record or RecordRef that is not temporary, in filter group 0.
        let measured = self.global_language() == 1033
            && f.stored
            && f.id < 2_000_000_000
            && state_measured
            && measured_call(call, written, values.len(), &f.ty);
        // `Record.SetRange(Field, Value)` with one value of a type other than Variant.
        let record_typed_one = call == Call::RecordSetRange && written.is_none() && values.len() == 1;
        for v in values.iter_mut() {
            match self.setrange_cell(v, &f.ty, written, record_typed_one) {
                None | Some(Cell::Main) => {}
                Some(Cell::Convert(converted)) if measured => {
                    assume(Assumption::SetRangeOtherTypeConvert);
                    *v = converted?;
                }
                Some(Cell::Convert(_) | Cell::Unmeasured) => {
                    return Err(Signal::unsupported(format!(
                        "SetRange or SetFilter of {} into a {} field in a shape no cell measures (assumption setrange-other-type-convert)",
                        v.describe(),
                        f.ty.describe()
                    )));
                }
            }
        }
        Ok(())
    }

    /// The measured cell of the value `v` into a field of type `ty`; `None` for a pair of types the probe does not measure (a
    /// value of the field's own type among them): main's conversion stays.
    fn setrange_cell(&self, v: &Value, ty: &RtType, written: Option<Written>, record_typed_one: bool) -> Option<Cell> {
        let from28 = self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28);
        let text = match v {
            Value::Text(s) | Value::Code(s) => Some(s.as_str()),
            _ => None,
        };
        match ty {
            // SrcFilterConvertKinds.al:452-686 (Int5_Cod .. DecNeg125_Txt): the Format text, `1,000.5` and `-1.25` too; true is
            // Yes (BoolTrue_Cod, BoolTrue_Txt: main's answer).
            RtType::Code(_) | RtType::Text(_) => match v {
                Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) => Some(number_into_text(v, ty, self)),
                Value::Boolean(true) => Some(Cell::Main),
                Value::Boolean(false) => Some(Cell::Unmeasured),
                _ => None,
            },
            // SrcFilterConvertKinds.al:729-1381 (CodeABC_Int .. TxtEmpty_Dec, Code1000_Int, Code1000_Big); digits (Code123_Int)
            // and, into a Decimal, `1.5` and `1,000` (Txt15_Dec, Code1000_Dec) are main's answer.
            RtType::Integer | RtType::BigInteger | RtType::Decimal => {
                let s = text?;
                let decimal = matches!(ty, RtType::Decimal);
                Some(if no_number_text(s, ty) {
                    Cell::Convert(Err(text_evaluate_error(s, ty, from28)?))
                } else if digits(s) || (decimal && (decimal_point(s) || one_group(s))) {
                    Cell::Main
                } else {
                    Cell::Unmeasured
                })
            }
            // SrcFilterConvertKinds.al:1406-1598 (Code123_Bool .. Int0_Bool).
            RtType::Boolean => match (v, text) {
                (_, Some(s)) if digits(s) => Some(Cell::Convert(Ok(Value::Boolean(s.bytes().any(|b| b != b'0'))))),
                (_, Some(s)) if letters(s) => Some(Cell::Convert(Err(text_evaluate_error(s, ty, from28)?))),
                (Value::Integer(n @ (0 | 1)), _) => Some(Cell::Convert(Ok(Value::Boolean(*n == 1)))),
                (Value::Integer(_), _) | (_, Some(_)) => Some(Cell::Unmeasured),
                _ => None,
            },
            // SrcFilterConvertKinds.al:1730-1849 (CodeABC_G .. TxtGuid_G) routes b, f and gr raise the Evaluate error; the typed
            // routes (a, e) keep the Guid format error and the Guid a valid text gives (main's answer). `Record.SetRange` of one
            // Text that holds a Guid filters on that Guid (main's answer): a text literal in
            // tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al:221, a Label in
            // tests/al-language-onprem/metadata/platform-tables/Y35StoredTables.al:569.
            RtType::Guid => {
                let s = text?;
                Some(match written {
                    Some(Written::Variant) if !s.is_empty() => Cell::Convert(Err(variant_guid_error(s, from28))),
                    Some(Written::Typed) if !s.is_empty() => Cell::Main,
                    None if record_typed_one && al_builtins::ops::parse_guid(s).is_some() => Cell::Main,
                    _ => Cell::Unmeasured,
                })
            }
            RtType::Date | RtType::Time | RtType::DateTime => self.temporal_cell(v, ty, from28),
            _ => None,
        }
    }

    /// Into a Date, Time or DateTime field (SrcSetRangeConvert.al:920-5680; SrcFilterConvertKinds.al:1616-1714 CodeABC_D,
    /// TxtXY_D).
    fn temporal_cell(&self, v: &Value, ty: &RtType, from28: bool) -> Option<Cell> {
        let formatted = || al_builtins::format::try_format(v, 0, 0, self).ok();
        let variant_cell = || match al_builtins::fieldref_variant_cells::variant_into_field(v, ty, from28, self) {
            Some(r) => Cell::Convert(r),
            None => Cell::Unmeasured,
        };
        // The .NET type BC 27 names and the text BC 28 evaluates (an Integer and a BigInteger as digits without grouping).
        let (nav, text) = match v {
            Value::Integer(n) if *n >= 0 => ("NavInteger", Some(n.to_string())),
            Value::BigInteger(n) if *n >= 0 => ("NavBigInteger", Some(n.to_string())),
            Value::Decimal(d) if *d >= Decimal::ZERO && *d < Decimal::from(1000) && d.scale() <= 2 => {
                ("NavDecimal", formatted())
            }
            Value::Boolean(true) => ("NavBoolean", formatted()),
            Value::Text(s) => ("NavText", Some(s.as_str().to_string())),
            Value::Code(s) => ("NavCode", Some(s.as_str().to_string())),
            Value::Guid(_) => ("NavGuid", formatted()),
            Value::Duration(ms) if (1..60_000).contains(ms) => ("NavDuration", formatted()),
            Value::Option(..) => ("NavOption", formatted()),
            Value::Enum(id, o) => {
                let text = formatted();
                let name = self.engine.program.objects.get(*id as usize).and_then(|obj| {
                    obj.enum_values
                        .iter()
                        .find(|(n, _)| n == o)
                        .map(|(_, name)| name.clone())
                });
                match (text, name) {
                    (Some(text), Some(name)) if name == text => ("NavOption", Some(text)),
                    _ => return Some(Cell::Unmeasured),
                }
            }
            // A negative number, false, a Decimal out of the measured size, a longer Duration.
            Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Boolean(false)
            | Value::Duration(_) => {
                return Some(Cell::Unmeasured);
            }
            // Date_DT, DateZero_DT: that day at midnight, main's answer.
            Value::Date(..) if matches!(ty, RtType::DateTime) => return Some(Cell::Main),
            // SrcSetRangeConvert.al Date_T, Time_D, DT_D, DT_T and the undefined values (DateZero_T, TimeZero_D, TimeZero_DT,
            // DTZero_D, DTZero_T): the same on every major.
            Value::Date(0, false) if matches!(ty, RtType::Time) => return Some(Cell::Convert(Ok(Value::Time(0)))),
            Value::Time(0) if matches!(ty, RtType::Date) => return Some(Cell::Convert(Ok(Value::Date(0, false)))),
            Value::Time(0) if matches!(ty, RtType::DateTime) => return Some(Cell::Convert(Ok(Value::DateTime(0)))),
            Value::DateTime(0) if matches!(ty, RtType::Date) => return Some(Cell::Convert(Ok(Value::Date(0, false)))),
            Value::DateTime(0) if matches!(ty, RtType::Time) => return Some(Cell::Convert(Ok(Value::Time(0)))),
            Value::Date(_, false) if matches!(ty, RtType::Time) => return Some(variant_cell()),
            Value::Time(_) if matches!(ty, RtType::Date) => return Some(variant_cell()),
            // Time_DT (SrcSetRangeConvert.al) measures that time on 0001-01-02 and reads it back as `01/02/0001 10:00 AM`; the
            // filter text al-runner writes for a DateTime of year 1 is `01/02/1`, so this cell stays refused.
            Value::Time(_) if matches!(ty, RtType::DateTime) => return Some(Cell::Unmeasured),
            Value::DateTime(_) if matches!(ty, RtType::Date | RtType::Time) => return Some(variant_cell()),
            // A closing date (`Date(_, true)`) into a Time field.
            Value::Date(..) if matches!(ty, RtType::Time) => return Some(Cell::Unmeasured),
            _ => return None,
        };
        Some(match text {
            Some(text) => Cell::Convert(Err(temporal_value_error(nav, &text, ty, from28))),
            None => Cell::Unmeasured,
        })
    }
}

/// An Integer, BigInteger or Decimal into a Code or Text field: its Format text, of the sizes the probe measures (5, 0, 1.5,
/// 1000.5, -1.25: at most 999,999 in size, at most two decimals). A text longer than the field is not measured: refused.
fn number_into_text(v: &Value, ty: &RtType, ctx: &Session) -> Cell {
    let limit = Decimal::from(1_000_000);
    let measured = match v {
        Value::Integer(n) => n.unsigned_abs() < 1_000_000,
        Value::BigInteger(n) => n.unsigned_abs() < 1_000_000,
        Value::Decimal(d) => d.abs() < limit && d.scale() <= 2,
        _ => false,
    };
    if !measured {
        return Cell::Unmeasured;
    }
    match al_builtins::ops::number_into_text_field(v, ty, ctx) {
        Some(Ok(text)) => Cell::Convert(Ok(text)),
        Some(Err(Signal::Error(_))) => Cell::Unmeasured,
        Some(Err(e)) => Cell::Convert(Err(e)),
        None => Cell::Unmeasured,
    }
}

/// A text the probe measures as no number of the field's type: letters, empty, and, into an Integer or BigInteger field, digits
/// with one decimal point (`1.5`) or one thousands group (`1,000`).
fn no_number_text(s: &str, ty: &RtType) -> bool {
    let whole = matches!(ty, RtType::Integer | RtType::BigInteger);
    s.is_empty() || letters(s) || (whole && (decimal_point(s) || one_group(s)))
}

/// Two or more ASCII letters and nothing else (`ABC`, `XY`, `Yes`).
fn letters(s: &str) -> bool {
    s.len() >= 2 && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// One to nine ASCII digits (`123`, `0`).
fn digits(s: &str) -> bool {
    (1..=9).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// Digits, one `.`, digits (`1.5`).
fn decimal_point(s: &str) -> bool {
    s.split_once('.').is_some_and(|(a, b)| digits(a) && digits(b))
}

/// One to three digits, `,`, three digits (`1,000`).
fn one_group(s: &str) -> bool {
    s.split_once(',')
        .is_some_and(|(a, b)| (1..=3).contains(&a.len()) && digits(a) && b.len() == 3 && digits(b))
}
