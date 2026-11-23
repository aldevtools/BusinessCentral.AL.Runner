//! A value of another type than the field's given to `FieldRef.SetRange`, `FieldRef.SetFilter('%1', ...)` or
//! `Record.SetRange` (al-runner issues 8857, 8867, 8868, 8869, 8870): BC converts it into the field's type before it sets
//! the filter, or raises the conversion error (SetFilter without the filter error around it). Lowering (al-sema
//! `setrange_written`) tells the calls apart by how their values are written: a Text into a Guid field raises another
//! error when it is written as a Variant. Only the cells corpus PR 1552 measures convert here
//! (recordref-fieldref/fieldref/SrcSetRangeConvert.al, SrcFilterConvertKinds.al, not yet pinned: assumption
//! setrange-other-type-convert); every other value keeps the conversion `setrange_filter` and the filter parser give it.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
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
    match id {
        "FieldRef.SetRangeTyped#1" => ("FieldRef.SetRange#1", Some(Written::Typed)),
        "FieldRef.SetRangeVariant#1" => ("FieldRef.SetRange#1", Some(Written::Variant)),
        "FieldRef.SetRangeMixed#1" => ("FieldRef.SetRange#1", Some(Written::Mixed)),
        "FieldRef.SetFilterTyped#1" => ("FieldRef.SetFilter#1", Some(Written::Typed)),
        "FieldRef.SetFilterVariant#1" => ("FieldRef.SetFilter#1", Some(Written::Variant)),
        _ => (id, None),
    }
}

/// The call a value is given to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    FieldRefSetRange,
    FieldRefSetFilter,
    RecordSetRange,
}

const MS_PER_DAY: i64 = 86_400_000;

/// The measured calls: the routes of the probe (a, aLit, c: typed SetRange; b: Variant SetRange; d1, d2: a Variant and a typed
/// value; e, eLit: typed SetFilter('%1', X); f: Variant SetFilter('%1', V); gr: Record.SetRange(Field, V)) with their number of
/// values. Mixed only on a Date, Time or DateTime field (d1 and d2 run on those only).
fn measured_call(call: Call, written: Written, values: usize, field: &RtType) -> bool {
    let temporal = matches!(field, RtType::Date | RtType::Time | RtType::DateTime);
    match (call, written) {
        (Call::FieldRefSetRange, Written::Typed) => values == 1 || values == 2,
        (Call::FieldRefSetRange, Written::Variant) => values == 1,
        (Call::FieldRefSetRange, Written::Mixed) => values == 2 && temporal,
        (Call::FieldRefSetFilter, Written::Typed | Written::Variant) => values == 1,
        (Call::RecordSetRange, Written::Variant) => values == 1,
        _ => false,
    }
}

impl Session {
    /// Converts the values of `FieldRef.<method>` on the FieldRef `lib` (`args`, written as `written`) where a measured cell
    /// says so; raises its error. SetFilter: only the filter text `%1` with one value.
    pub(crate) fn fieldref_setrange_convert(
        &mut self,
        method: &str,
        lib: &Rc<RefCell<LibraryValue>>,
        args: &mut [Value],
        written: Written,
    ) -> Result<(), Signal> {
        let (rec, pos) = match &*lib.borrow() {
            LibraryValue::FieldRef(r, p) => (r.clone(), *p),
            _ => return Ok(()),
        };
        let (call, values) = match method {
            "SetRange" => (Call::FieldRefSetRange, &mut args[..]),
            "SetFilter" if args.len() == 2 && crate::filter_text_arg::filter_text(args.first()).as_str() == "%1" => {
                (Call::FieldRefSetFilter, &mut args[1..])
            }
            _ => return Ok(()),
        };
        self.setrange_convert(&rec, pos, values, written, call)
    }

    /// Converts the value of `Record.SetRange(Field, V)`, `V` written as a Variant (`args`: the field position and the value),
    /// where a measured cell says so; raises its error.
    pub(crate) fn record_setrange_variant_convert(
        &mut self,
        rec: &Rc<RefCell<RecordState>>,
        args: &mut [Value],
    ) -> Result<(), Signal> {
        let Some((Value::Integer(p), values)) = args.split_first_mut() else {
            return Ok(());
        };
        let Ok(pos) = usize::try_from(*p) else { return Ok(()) };
        self.setrange_convert(rec, pos, values, Written::Variant, Call::RecordSetRange)
    }

    fn setrange_convert(
        &mut self,
        rec: &Rc<RefCell<RecordState>>,
        pos: usize,
        values: &mut [Value],
        written: Written,
        call: Call,
    ) -> Result<(), Signal> {
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
        if !measured {
            return Ok(());
        }
        for v in values.iter_mut() {
            if let Some(converted) = self.setrange_cell(v, &f.ty, written) {
                assume(Assumption::SetRangeOtherTypeConvert);
                *v = converted?;
            }
        }
        Ok(())
    }

    /// The measured cell of the value `v` into a field of type `ty`; `None` for one the probe does not measure or one the
    /// conversion before already gives (a digit text into a number field, a Decimal text into a Decimal field, a Boolean into
    /// a Text or Code field, a Date into a DateTime field, a Text into a Guid field written as a Text, a value of the field's
    /// own type).
    fn setrange_cell(&self, v: &Value, ty: &RtType, written: Written) -> Option<Result<Value, Signal>> {
        let from28 = self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28);
        match ty {
            // SrcFilterConvertKinds.al:452-686 (Int5_Cod .. DecNeg125_Txt): the Format text, `1,000.5` and `-1.25` too.
            RtType::Code(_) | RtType::Text(_) => number_into_text(v, ty, self),
            // SrcFilterConvertKinds.al:729-1381 (CodeABC_Int .. TxtEmpty_Dec, Code1000_Int, Code1000_Big).
            RtType::Integer | RtType::BigInteger | RtType::Decimal => match v {
                Value::Text(s) | Value::Code(s) if no_number_text(s.as_str(), ty) => {
                    Some(Err(text_evaluate_error(s.as_str(), ty, from28)?))
                }
                _ => None,
            },
            // SrcFilterConvertKinds.al:1406-1598 (Code123_Bool .. Int0_Bool).
            RtType::Boolean => match v {
                Value::Text(s) | Value::Code(s) if digits(s.as_str()) => {
                    Some(Ok(Value::Boolean(s.as_str().bytes().any(|b| b != b'0'))))
                }
                Value::Text(s) | Value::Code(s) if letters(s.as_str()) => {
                    Some(Err(text_evaluate_error(s.as_str(), ty, from28)?))
                }
                Value::Integer(n @ (0 | 1)) => Some(Ok(Value::Boolean(*n == 1))),
                _ => None,
            },
            // SrcFilterConvertKinds.al:1730-1849 (CodeABC_G .. TxtGuid_G) routes b, f and gr; the typed routes keep the Guid
            // format error and the Guid a valid text gives.
            RtType::Guid => match (v, written) {
                (Value::Text(s) | Value::Code(s), Written::Variant) if !s.is_empty() => {
                    Some(Err(variant_guid_error(s.as_str(), from28)))
                }
                _ => None,
            },
            RtType::Date | RtType::Time | RtType::DateTime => self.temporal_cell(v, ty, from28),
            _ => None,
        }
    }

    /// Into a Date, Time or DateTime field (SrcSetRangeConvert.al:920-5680; SrcFilterConvertKinds.al:1616-1714 CodeABC_D,
    /// TxtXY_D).
    fn temporal_cell(&self, v: &Value, ty: &RtType, from28: bool) -> Option<Result<Value, Signal>> {
        let formatted = || al_builtins::format::try_format(v, 0, 0, self).ok();
        // The .NET type BC 27 names and the text BC 28 evaluates (an Integer and a BigInteger as digits without grouping).
        let (nav, text) = match v {
            Value::Integer(n) if *n >= 0 => ("NavInteger", n.to_string()),
            Value::BigInteger(n) if *n >= 0 => ("NavBigInteger", n.to_string()),
            Value::Decimal(d) if *d >= Decimal::ZERO && *d < Decimal::from(1000) && d.scale() <= 2 => {
                ("NavDecimal", formatted()?)
            }
            Value::Boolean(true) => ("NavBoolean", formatted()?),
            Value::Text(s) => ("NavText", s.as_str().to_string()),
            Value::Code(s) => ("NavCode", s.as_str().to_string()),
            Value::Guid(_) => ("NavGuid", formatted()?),
            Value::Duration(ms) if (1..60_000).contains(ms) => ("NavDuration", formatted()?),
            Value::Option(..) => ("NavOption", formatted()?),
            Value::Enum(id, o) => {
                let text = formatted()?;
                let name = self.engine.program.objects.get(*id as usize)?.enum_values.iter().find(|(n, _)| n == o)?;
                if name.1 != text {
                    return None;
                }
                ("NavOption", text)
            }
            // SrcSetRangeConvert.al Date_T, Time_D, DT_D, DT_T and the undefined values (DateZero_T, TimeZero_D, TimeZero_DT,
            // DTZero_D, DTZero_T): the same on every major.
            Value::Date(0, false) if matches!(ty, RtType::Time) => return Some(Ok(Value::Time(0))),
            Value::Time(0) if matches!(ty, RtType::Date) => return Some(Ok(Value::Date(0, false))),
            Value::Time(0) if matches!(ty, RtType::DateTime) => return Some(Ok(Value::DateTime(0))),
            Value::DateTime(0) if matches!(ty, RtType::Date) => return Some(Ok(Value::Date(0, false))),
            Value::DateTime(0) if matches!(ty, RtType::Time) => return Some(Ok(Value::Time(0))),
            Value::Date(_, false) if matches!(ty, RtType::Time) => {
                return al_builtins::fieldref_variant_cells::variant_into_field(v, ty, from28, self);
            }
            Value::Time(_) if matches!(ty, RtType::Date) => {
                return al_builtins::fieldref_variant_cells::variant_into_field(v, ty, from28, self);
            }
            // Time_DT: that time on 0001-01-02 (`01/02/0001 10:00 AM`).
            Value::Time(x) if matches!(ty, RtType::DateTime) => return Some(Ok(Value::DateTime(*x + MS_PER_DAY))),
            Value::DateTime(_) if matches!(ty, RtType::Date | RtType::Time) => {
                return al_builtins::fieldref_variant_cells::variant_into_field(v, ty, from28, self);
            }
            _ => return None,
        };
        Some(Err(temporal_value_error(nav, &text, ty, from28)))
    }
}

/// An Integer, BigInteger or Decimal into a Code or Text field: its Format text, of the sizes the probe measures (5, 0, 1.5,
/// 1000.5, -1.25: at most 999,999 in size, at most two decimals). A text longer than the field is not measured: refused.
fn number_into_text(v: &Value, ty: &RtType, ctx: &Session) -> Option<Result<Value, Signal>> {
    let limit = Decimal::from(1_000_000);
    let measured = match v {
        Value::Integer(n) => n.unsigned_abs() < 1_000_000,
        Value::BigInteger(n) => n.unsigned_abs() < 1_000_000,
        Value::Decimal(d) => d.abs() < limit && d.scale() <= 2,
        _ => false,
    };
    if !measured {
        return None;
    }
    match al_builtins::ops::number_into_text_field(v, ty, ctx)? {
        Ok(text) => Some(Ok(text)),
        Err(Signal::Error(_)) => Some(Err(Signal::unsupported(
            "SetRange or SetFilter of a number whose Format text is longer than the Code or Text field (not measured)",
        ))),
        Err(e) => Some(Err(e)),
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
