//! A `FieldRef.Value()` assigned to a Date, Time or DateTime variable (al-vm `Compiler::assign`, `Op::ConvertFieldValue`).
//!
//! Every measured cell raises and leaves the variable as it was. This module owns the list of cells; a value, a slot or a
//! target that is not on it gets `None`, and the caller converts as an assignment does (which refuses).

use crate::native::Signal;
use crate::value::{RtType, Value};
use rust_decimal::Decimal;

/// Where the assigned variable is declared. Corpus PR 1511 measured the `var` parameter of an Integer or BigInteger, corpus PR 1522
/// all four slots for a DateTime and the first two for a Date and a Time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalSlot {
    /// A `var` parameter of the routine.
    VarParam,
    /// A local declared in the routine's `var` section (a procedure, not a trigger).
    PlainLocal,
    /// A parameter passed by value.
    ByValueParam,
    /// The named return value.
    NamedReturn,
}

/// The Integer values of the DateTime cells (recordref-fieldref/fieldref-value-convert/FvcDateTimeShapes.al IntField_To_DateTime_Shapes).
const INT_DATETIME: [i32; 4] = [0, 1, 1000, -1];
/// The Integer values of the Date and Time cells (FvcDateTimeShapesDateTime.al IntField_To_Date_Shapes, IntField_To_Time_Shapes).
const INT_DATE_TIME: [i32; 2] = [1, 1000];
/// The BigInteger values of the DateTime cells (FvcDateTimeShapes.al BigField_To_DateTime_Shapes).
const BIG_DATETIME: [i64; 3] = [1, 3_000_000_000, -1];
/// The Decimal values of the DateTime cells, as (mantissa, scale) (FvcDateTimeShapes.al DecField_To_DateTime_Shapes_Small and _Large).
const DEC_DATETIME: [(i64, u32); 9] = [
    (-5, 1),
    (123_456_789, 9),
    (99_999, 2),
    (15, 1),
    (10_005, 1),
    (0, 0),
    (1, 0),
    (5, 0),
    (-100_025, 2),
];
/// The Decimal values of the Date and Time cells (FvcDateTimeShapesDateTime.al DecField_To_Date_Shapes1 and 2, DecField_To_Time_Shapes).
const DEC_DATE_TIME: [(i64, u32); 4] = [(-5, 1), (99_999, 2), (15, 1), (1, 0)];
/// The Duration values in milliseconds of the DateTime cells (FvcDateTimeShapes.al DurField_To_DateTime_Shapes_Small and _Large).
const DUR_DATETIME: [i64; 9] = [1, 65_000, 65_001, 0, -1, 1000, 93_780_000, 86_400_000, 7_200_000];
/// The Duration values of the Date and Time cells (FvcDateTimeShapesDateTime.al DurField_To_Date_Shapes1 and 2, DurField_To_Time_Shapes).
const DUR_DATE_TIME: [i64; 4] = [1, 65_000, 93_780_000, 0];
/// The digit strings of a Text or Code field, for all three targets (recordref-fieldref/fieldref-value-convert/FvcDateTimeDigits.al).
const DIGITS: [&str; 10] = [
    "20260115",
    "20260115120000",
    "202601151200",
    "120000",
    "1200",
    "0115",
    "01152026",
    "260115",
    "1",
    "0",
];

fn in_dec(d: &Decimal, set: &[(i64, u32)]) -> bool {
    set.iter().any(|&(m, s)| *d == Decimal::new(m, s))
}

/// A `FieldRef.Value()` of a field whose value the probes measured, assigned to a DateTime, Date or Time of `slot`: an error,
/// the variable keeps its value. From BC 28 (`from28`, the gate FieldRefTextNumberEvaluateErrorFrom28) the Evaluate error of
/// the value's text, code Evaluate:Conversion (a Date names its device date settings); in BC 27
/// `Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav<Type> to System.DateTime.`, code Conversion, for all three targets.
///
/// The cells (assumption fieldref-value-temporal-error; corpus PRs 1511 and 1522, recordref-fieldref/fieldref-value-convert/):
/// - an Integer or BigInteger of any value into each target, `VarParam` only (FvcIntegerField.al, FvcBigIntegerField.al, PR 1511);
/// - into a DateTime, every slot: the Integer, BigInteger, Decimal and Duration values listed above, a Boolean, and the ten digit
///   strings of a Text or Code field (FvcDateTimeShapes.al, FvcDateTimeDigits.al);
/// - into a Date or Time, `VarParam` and `PlainLocal`: the Integer, Decimal and Duration values listed above and the ten digit
///   strings (FvcDateTimeShapesDateTime.al, FvcDateTimeDigits.al).
///
/// Every cell runs in the en-US session only (`format::en_us_session`), the language of an error text changes with it. `None`: any other value, slot or target.
pub fn fieldref_value_into_temporal(
    v: &Value,
    t: &RtType,
    slot: TemporalSlot,
    from28: bool,
    ctx: &dyn crate::format::FormatCtx,
) -> Option<Result<Value, Signal>> {
    let dt = matches!(t, RtType::DateTime);
    let temporal = matches!(t, RtType::DateTime | RtType::Date | RtType::Time);
    let two = matches!(slot, TemporalSlot::VarParam | TemporalSlot::PlainLocal);
    let var = slot == TemporalSlot::VarParam;
    // The cell's `Nav<Type>` name and its measured text.
    let (nav, text) = match v {
        Value::Integer(n)
            if temporal && (var || (dt && INT_DATETIME.contains(n)) || (two && INT_DATE_TIME.contains(n))) =>
        {
            ("NavInteger", n.to_string())
        }
        Value::BigInteger(n) if temporal && (var || (dt && BIG_DATETIME.contains(n))) => {
            ("NavBigInteger", n.to_string())
        }
        Value::Decimal(d) if (dt && in_dec(d, &DEC_DATETIME)) || (temporal && two && in_dec(d, &DEC_DATE_TIME)) => {
            ("NavDecimal", crate::format::try_format(v, 0, 0, ctx).ok()?)
        }
        Value::Duration(ms) if (dt && DUR_DATETIME.contains(ms)) || (temporal && two && DUR_DATE_TIME.contains(ms)) => {
            ("NavDuration", crate::format::try_format(v, 0, 0, ctx).ok()?)
        }
        Value::Boolean(_) if dt => ("NavBoolean", crate::format::try_format(v, 0, 0, ctx).ok()?),
        Value::Text(s) if temporal && (dt || two) && DIGITS.contains(&s.as_str()) => ("NavText", s.as_str().to_owned()),
        Value::Code(s) if temporal && (dt || two) && DIGITS.contains(&s.as_str()) => ("NavCode", s.as_str().to_owned()),
        _ => return None,
    };
    if !crate::format::en_us_session(ctx) {
        return None;
    }
    crate::assumptions::assume(crate::assumptions::Assumption::FieldRefValueTemporalError);
    if !from28 {
        return Some(crate::native::raise(
            format!("Unable to convert from Microsoft.Dynamics.Nav.Runtime.{nav} to System.DateTime."),
            "Conversion",
        ));
    }
    let message = match t {
        RtType::Date => crate::fieldref_variant_cells::date_settings_error(&text),
        RtType::Time => format!("The value \"{text}\" can't be evaluated into type Time."),
        _ => format!("The value \"{text}\" can't be evaluated into type DateTime."),
    };
    Some(crate::native::raise(message, "Evaluate:Conversion"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Session(i32);
    impl crate::format::FormatCtx for Session {
        fn enum_name(&self, _: u32, _: i32) -> String {
            String::new()
        }
        fn language(&self) -> i32 {
            self.0
        }
    }

    fn run(v: &Value, t: &RtType, slot: TemporalSlot, lang: i32) -> bool {
        fieldref_value_into_temporal(v, t, slot, true, &Session(lang)).is_some()
    }

    #[test]
    fn every_cell_runs_in_en_us_only() {
        for v in [
            Value::Integer(1),
            Value::BigInteger(1),
            Value::Decimal(Decimal::new(15, 1)),
            Value::Boolean(true),
        ] {
            assert!(
                run(&v, &RtType::DateTime, TemporalSlot::VarParam, 1033),
                "{v:?} in 1033"
            );
            assert!(
                !run(&v, &RtType::DateTime, TemporalSlot::VarParam, 1031),
                "{v:?} in 1031"
            );
        }
    }

    #[test]
    fn a_slot_or_value_off_the_lists_gets_none() {
        assert!(!run(
            &Value::Integer(1),
            &RtType::Date,
            TemporalSlot::ByValueParam,
            1033
        ));
        assert!(!run(
            &Value::Decimal(Decimal::new(2, 0)),
            &RtType::DateTime,
            TemporalSlot::VarParam,
            1033
        ));
    }
}
