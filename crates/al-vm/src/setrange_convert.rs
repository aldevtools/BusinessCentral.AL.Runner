//! A value of another type than the field's given to `FieldRef.SetRange`, `FieldRef.SetFilter`, `Record.SetRange` or
//! `Record.SetFilter` from AL code (al-runner issues 8857, 8867, 8868, 8869, 8870): BC converts it into the field's type
//! before it sets the filter, or raises the conversion error (SetFilter without the filter error around it). Lowering (al-sema
//! `setrange_written`) tells the calls apart by how their values are written: a Text into a Guid field raises another error
//! when it is written as a Variant. Only the cells corpus PR 1552 measures convert here
//! (recordref-fieldref/fieldref/SrcSetRangeConvert.al, SrcFilterConvertKinds.al, not yet pinned: assumption
//! setrange-other-type-convert). A value of a pair of types those cells measure is refused in a shape or of a size they do
//! not measure; a cell that passes on the conversion `setrange_filter` and the filter parser give keeps it in the measured
//! shape only (through the same assumption), unless a pin or another assumption covers every shape (`Cell::MainEveryShape`). False into a
//! Code or Text field, and true into one shorter than its Format text, have their own entry (al-runner issue 9026, assumption
//! setrange-bool-into-text). A pair of types they do not
//! measure keeps that conversion.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::fieldref_variant_cells::{nav_of, temporal_value_error, text_evaluate_error, variant_guid_error};
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, RecordState, RtType, Value};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

/// The entry of the cells of corpus PR 1552.
const ENTRY: Assumption = Assumption::SetRangeOtherTypeConvert;

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
    // Only the FieldRef SetRange and SetFilter ids are written ids; skip the table search for every other ref call.
    if !id.starts_with("FieldRef.Set") {
        return (id, None);
    }
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
    /// BC gives what main's conversion gives (the cell passes on main) in the shape the cells measure, through this entry
    /// (setrange-other-type-convert, or setrange-bool-into-text for false into a Code or Text field); refused in another.
    Main(Assumption),
    /// Main's answer, pinned in the measured shape of this call: run there without the flag; refused in another.
    MainPinned,
    /// Main's conversion in every shape: pinned there, or run through another assumption entry.
    MainEveryShape,
    /// BC raises this error in the shape the cells of this entry measure; refused in another.
    Raise(Assumption, Signal),
    /// A value of a pair the probe measures, but not this value: refused.
    Unmeasured,
    /// A value of the pair of this entry that its cells do not measure: refused, the message names this entry.
    Refused(Assumption),
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
        let state_measured = || {
            let r = rec.borrow();
            !r.temporary && r.filter_group == 0
        };
        let at = CellAt {
            call,
            written,
            values: values.len(),
        };
        // The probe's facts: an en-US session, a stored field of the table's own (no FlowField, FlowFilter or system field)
        // on a Record or RecordRef that is not temporary, in filter group 0.
        // Read only for a cell that runs in the measured shape alone (most values have no cell).
        let measured = || {
            self.global_language() == 1033
                && f.stored
                && f.id < 2_000_000_000
                && state_measured()
                && measured_call(call, written, at.values, &f.ty)
        };
        for v in values.iter_mut() {
            match self.setrange_cell(v, &f.ty, at) {
                None | Some(Cell::MainEveryShape) => {}
                // Main's answer, measured by the open probe alone.
                Some(Cell::Main(entry)) if measured() => assume(entry),
                Some(Cell::MainPinned) if measured() => {}
                Some(Cell::Convert(converted)) if measured() => {
                    assume(Assumption::SetRangeOtherTypeConvert);
                    *v = converted?;
                }
                Some(Cell::Raise(entry, e)) if measured() => {
                    assume(entry);
                    return Err(e);
                }
                Some(
                    cell @ (Cell::Main(_)
                    | Cell::MainPinned
                    | Cell::Convert(_)
                    | Cell::Raise(..)
                    | Cell::Unmeasured
                    | Cell::Refused(_)),
                ) => {
                    let entry = match cell {
                        Cell::Refused(entry) => entry,
                        _ => ENTRY,
                    };
                    return Err(Signal::unsupported(format!(
                        "SetRange or SetFilter of {} into a {} field in a shape no cell measures (assumption {})",
                        v.describe(),
                        f.ty.describe(),
                        entry.info().id
                    )));
                }
            }
        }
        Ok(())
    }

    /// The measured cell of the value `v` into a field of type `ty`, given to the call `at`; `None` for a pair of types the probe
    /// does not measure (a value of the field's own type among them): main's conversion stays.
    fn setrange_cell(&self, v: &Value, ty: &RtType, at: CellAt) -> Option<Cell> {
        let from28 = self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28);
        let text = match v {
            Value::Text(s) | Value::Code(s) => Some(s.as_str()),
            _ => None,
        };
        match ty {
            // SrcFilterConvertKinds.al:452-686 (Int5_Cod .. DecNeg125_Txt): the Format text, `1,000.5` and `-1.25` too; true is
            // Yes (BoolTrue_Cod, BoolTrue_Txt: main's answer); other lengths and false have their own evidence (`bool_into_text`).
            RtType::Code(_) | RtType::Text(_) => match v {
                Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) => Some(number_into_text(v, ty, self)),
                Value::Boolean(b) => Some(bool_into_text(*b, ty)),
                _ => None,
            },
            // SrcFilterConvertKinds.al:729-1381 (CodeABC_Int .. TxtEmpty_Dec, Code1000_Int, Code1000_Big); digits (Code123_Int,
            // Code123_Dec) and, into a Decimal, `1.5` and `1,000` (Txt15_Dec, Code1000_Dec) are main's answer. Digits into an
            // Integer or BigInteger field run through the assumption text-digits-to-integer (al-builtins `ops.rs`) in every
            // shape; into a Decimal field no other entry covers them, so they run in the measured shape only. The Text `1.5` in a
            // Variant given to FieldRef.SetRange alone is pinned: FieldRefFilterMatrix.al:819,822 (Filter_Dec_Range `rtext.Dec`);
            // a Code or another Decimal text there runs through the entry.
            RtType::Integer | RtType::BigInteger | RtType::Decimal => {
                let s = text?;
                let decimal = matches!(ty, RtType::Decimal);
                let whole = matches!(ty, RtType::Integer | RtType::BigInteger);
                Some(if no_number_text(s, ty) {
                    Cell::Convert(Err(text_evaluate_error(s, ty, from28)?))
                } else if whole && digits(s) {
                    Cell::MainEveryShape
                } else if decimal && matches!(v, Value::Text(t) if t.as_str() == "1.5") && variant_setrange(at) {
                    Cell::MainPinned
                } else if decimal && (digits(s) || decimal_point(s) || one_group(s)) {
                    Cell::Main(ENTRY)
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
            RtType::Guid => text.map(|s| guid_cell(s, at, from28)),
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
        // The text BC 28 evaluates (an Integer and a BigInteger as digits without grouping); BC 27 names the .NET type
        // (`nav_of`).
        let text = match v {
            Value::Integer(n) if *n >= 0 => Some(n.to_string()),
            Value::BigInteger(n) if *n >= 0 => Some(n.to_string()),
            Value::Decimal(d) if *d >= Decimal::ZERO && *d < Decimal::from(1000) && d.scale() <= 2 => formatted(),
            Value::Boolean(true) | Value::Guid(_) => formatted(),
            Value::Text(s) | Value::Code(s) => Some(s.as_str().to_string()),
            Value::Duration(ms) if (1..60_000).contains(ms) => formatted(),
            // Opt_* and Enum_* measure the member `One`: a member whose Format text is its name of letters (`letters`).
            Value::Option(..) | Value::Enum(..) => match member_text(formatted(), self.member_name(v)) {
                Some(text) => Some(text),
                None => return Some(Cell::Unmeasured),
            },
            // A negative number, false, a Decimal out of the measured size, a longer Duration.
            Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Boolean(false)
            | Value::Duration(_) => {
                return Some(Cell::Unmeasured);
            }
            // Date_DT, DateZero_DT: that day at midnight, main's answer.
            Value::Date(..) if matches!(ty, RtType::DateTime) => return Some(Cell::Main(ENTRY)),
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
        Some(match (nav_of(v), text) {
            (Some(nav), Some(text)) => Cell::Convert(Err(temporal_value_error(nav, &text, ty, from28))),
            _ => Cell::Unmeasured,
        })
    }

    /// The name of the member an Option or Enum value holds.
    fn member_name(&self, v: &Value) -> Option<String> {
        match v {
            Value::Option(o, names) => usize::try_from(*o).ok().and_then(|i| names.get(i)).cloned(),
            Value::Enum(id, o) => self.engine.program.objects.get(*id as usize).and_then(|obj| {
                obj.enum_values
                    .iter()
                    .find(|(n, _)| n == o)
                    .map(|(_, name)| name.clone())
            }),
            _ => None,
        }
    }
}

/// The call a value is given to, how its values are written and how many there are.
#[derive(Clone, Copy)]
struct CellAt {
    call: Call,
    written: Option<Written>,
    values: usize,
}

/// `FieldRef.SetRange(V)`: one value, written as a Variant.
fn variant_setrange(at: CellAt) -> bool {
    at.call == Call::FieldRefSetRange && at.written == Some(Written::Variant) && at.values == 1
}

/// A Text or Code `s` into a Guid field. SrcFilterConvertKinds.al:1730-1849 (CodeABC_G, Code123_G, TxtXY_G, TxtGuid_G): routes b,
/// f and gr raise the Evaluate error; the typed SetRange routes (a, c) raise the Guid format error or give the Guid a valid text
/// gives (main's answer); the typed SetFilter('%1', X) route (e) gives the Guid of a valid text (main's answer) and raises the
/// Guid format error itself for `ABC`, `123` and `XY`, without the filter error around it; 32 hexadecimal digits without
/// hyphens (main's Guid for an assignment) are not measured there. `Record.SetRange` of one Text that
/// holds a Guid filters on that Guid (main's answer) in every shape: a text literal in
/// tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al:221, a Label in
/// tests/al-language-onprem/metadata/platform-tables/Y35StoredTables.al:569.
fn guid_cell(s: &str, at: CellAt, from28: bool) -> Cell {
    let guid = || al_builtins::ops::parse_guid(s).is_some();
    let hex32 = || al_builtins::ops::parse_guid_without_hyphens(s).is_some();
    match (at.call, at.written) {
        (_, Some(Written::Variant)) if !s.is_empty() => Cell::Convert(Err(variant_guid_error(s, from28))),
        (Call::FieldRefSetRange, Some(Written::Typed)) if !s.is_empty() => Cell::Main(ENTRY),
        (Call::FieldRefSetFilter, Some(Written::Typed)) if guid() => Cell::Main(ENTRY),
        (Call::FieldRefSetFilter, Some(Written::Typed)) if hex32() => Cell::Unmeasured,
        (Call::FieldRefSetFilter, Some(Written::Typed)) if letters(s) || digits(s) => {
            Cell::Convert(al_builtins::native::invalid_guid_format())
        }
        (Call::RecordSetRange, None) if at.values == 1 && guid() => Cell::MainEveryShape,
        _ => Cell::Unmeasured,
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

/// A Boolean `b` into a Code or Text field `ty`, on the field lengths corpus PR 1575 measures (SrcFilterBoolIntoText.al, the same on
/// the Business Central majors 27, 28 and 29, on every route of `measured_call`), through the entry setrange-bool-into-text:
/// false into a Code[20], Code[2] or Text[50] field is main's answer, `NO` in a Code, `No` in a Text (178-224: BoolFalse_Cod,
/// BoolFalse_Txt, BoolFalse_Cod2); a Format text longer than the field (false into Code[1] or Text[1], true into Code[2],
/// Code[1] or Text[1]) raises the string-length error with the Format text, not upper case in a Code field (226-304:
/// BoolFalse_Cod1 .. BoolTrue_Txt1). True into a Code[20] or Text[50] field is main's answer (corpus PR 1552, BoolTrue_Cod,
/// BoolTrue_Txt), and so is true into a field of 3 or more characters (main's conversion, which the cells do not measure).
/// Every other length is refused.
fn bool_into_text(b: bool, ty: &RtType) -> Cell {
    let len = match ty {
        RtType::Code(Some(n)) | RtType::Text(Some(n)) => *n,
        _ => return Cell::Refused(Assumption::SetRangeBoolIntoText),
    };
    let code = matches!(ty, RtType::Code(_));
    let format = if b { "Yes" } else { "No" };
    match (b, code, len) {
        (false, true, 2 | 20) | (false, false, 50) => Cell::Main(Assumption::SetRangeBoolIntoText),
        (false, _, 1) | (true, true, 1 | 2) | (true, false, 1) => {
            // `string_length_error` raises for every length it is given here (the Format text is longer than the field).
            Cell::Raise(
                Assumption::SetRangeBoolIntoText,
                al_builtins::ops::string_length_error::<()>(format, len).unwrap_err(),
            )
        }
        (true, _, 3..) => Cell::Main(ENTRY),
        _ => Cell::Refused(Assumption::SetRangeBoolIntoText),
    }
}

/// A text the probe measures as no number of the field's type: letters, empty, and, into an Integer or BigInteger field, digits
/// with one decimal point (`1.5`) or one thousands group (`1,000`).
fn no_number_text(s: &str, ty: &RtType) -> bool {
    let whole = matches!(ty, RtType::Integer | RtType::BigInteger);
    s.is_empty() || letters(s) || (whole && (decimal_point(s) || one_group(s)))
}

/// The Format text of an Option or Enum member where it is the member's name and that name is letters (`letters`).
fn member_text(text: Option<String>, name: Option<String>) -> Option<String> {
    match (text, name) {
        (Some(text), Some(name)) if name == text && letters(&name) => Some(text),
        _ => None,
    }
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
