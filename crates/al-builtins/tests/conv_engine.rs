//! The conversions of the run matrix cells (`al_builtins::conv`), through `ops::convert` as the runtime calls it. The values
//! are BC's, from corpus types/lt-y31-conversion-matrix/Y31ConvCv.al.

use al_builtins::conv::variant_into;
use al_builtins::native::Signal;
use al_builtins::ops::convert;
use al_builtins::value::{RtType, Value};
use rust_decimal::Decimal;

fn err(r: Result<Value, Signal>) -> (String, String) {
    match r {
        Err(Signal::Error(e)) => (e.code.clone(), e.message.clone()),
        other => panic!("expected an error, got {other:?}"),
    }
}

fn dec(s: &str) -> Value {
    Value::Decimal(s.parse::<Decimal>().unwrap())
}

fn opt() -> RtType {
    RtType::Option(vec!["A".into(), "B".into(), "C".into()])
}

#[test]
fn numbers_cast_with_the_net_error() {
    // LT Y31 Cv Integer, Integer_To_Char: -1 and 65536 overflow, 256 is U+0100.
    assert_eq!(
        err(convert(Value::Integer(-1), &RtType::Char)),
        (
            "ConversionCast".into(),
            "Overflow under conversion of System.Int32 value -1 to System.Char.".into()
        )
    );
    assert!(matches!(
        convert(Value::Integer(256), &RtType::Char),
        Ok(Value::Char(256))
    ));
    // Decimal_To_Char / Decimal_To_Byte: no rounding.
    assert_eq!(
        err(convert(dec("1.5"), &RtType::Char)).1,
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 1.5 to System.Char."
    );
    assert_eq!(
        err(convert(dec("256"), &RtType::Byte)).1,
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 256 to System.Byte."
    );
    // Decimal_To_Option 2147483648.0.
    assert_eq!(
        err(convert(dec("2147483648.0"), &opt())).1,
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 2147483648.0 to System.Int32."
    );
    // BigInteger_To_Byte -1.
    assert_eq!(
        err(convert(Value::BigInteger(-1), &RtType::Byte)).1,
        "Overflow under conversion of System.Int64 value -1 to System.Byte."
    );
}

#[test]
fn ordinals_and_durations_convert() {
    // Char_To_Option 65, Byte_To_Option 255, Option_To_BigInteger -1, Option_To_Duration 2, Option_To_Byte -1.
    assert!(matches!(convert(Value::Char(65), &opt()), Ok(Value::Option(65, _))));
    assert!(matches!(convert(Value::Byte(255), &opt()), Ok(Value::Option(255, _))));
    let o = |n| Value::Option(n, vec!["A".to_string()].into());
    assert!(matches!(convert(o(-1), &RtType::BigInteger), Ok(Value::BigInteger(-1))));
    assert!(matches!(convert(o(2), &RtType::Duration), Ok(Value::Duration(2))));
    assert_eq!(
        err(convert(o(-1), &RtType::Byte)).1,
        "Overflow under conversion of System.Int32 value -1 to System.Byte."
    );
    // Duration_To_Option 2147483648, Duration_To_Char 255.
    assert_eq!(
        err(convert(Value::Duration(2147483648), &opt())).1,
        "Overflow under conversion of System.Int64 value 2147483648 to System.Int32."
    );
    assert!(matches!(
        convert(Value::Duration(255), &RtType::Char),
        Ok(Value::Char(255))
    ));
}

#[test]
fn char_and_byte_go_through_code_page_437() {
    // Byte_To_Char 255 is U+00A0; Char_To_Byte 255 shows `ÿ` (0x98), 256 `A`, 65535 `?`.
    assert!(matches!(convert(Value::Byte(255), &RtType::Char), Ok(Value::Char(160))));
    assert!(matches!(convert(Value::Byte(97), &RtType::Char), Ok(Value::Char(97))));
    assert!(matches!(
        convert(Value::Char(255), &RtType::Byte),
        Ok(Value::Byte(0x98))
    ));
    assert!(matches!(convert(Value::Char(256), &RtType::Byte), Ok(Value::Byte(65))));
    assert!(matches!(
        convert(Value::Char(65535), &RtType::Byte),
        Ok(Value::Byte(63))
    ));
    // A byte or character the table does not hold is refused, not guessed.
    assert!(matches!(
        convert(Value::Byte(200), &RtType::Char),
        Err(Signal::Unsupported(_))
    ));
    assert!(matches!(
        convert(Value::Char(300), &RtType::Byte),
        Err(Signal::Unsupported(_))
    ));
}

#[test]
fn a_variant_cell_that_always_errs_raises_bc_text() {
    // Variant_To_Date from an Integer, Variant_To_Integer from a Guid, Variant_To_Guid from a Boolean.
    assert_eq!(
        err(variant_into(Value::Integer(3), &RtType::Date)),
        (
            "TypeConversion".into(),
            "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavDate value.".into()
        )
    );
    assert_eq!(
        err(variant_into(Value::Guid([1; 16]), &RtType::Integer)),
        (
            "Conversion".into(),
            "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.Int32.".into()
        )
    );
    assert_eq!(
        err(variant_into(Value::Boolean(true), &RtType::Guid)).1,
        "The expression Variant cannot be type-converted to a GUID value."
    );
}

#[test]
fn arithmetic_cells_of_the_matrix() {
    use al_builtins::ops::{Arith, arith};
    // LT Y31 Ar Char, Char_Byte: the Byte is a character of code page 437 next to a Char, 255 is 160.
    assert!(matches!(
        arith(Arith::Add, &Value::Char(65), &Value::Byte(255)),
        Ok(Value::Integer(225))
    ));
    assert!(matches!(
        arith(Arith::Sub, &Value::Char(65), &Value::Byte(255)),
        Ok(Value::Integer(-95))
    ));
    assert!(matches!(
        arith(Arith::Add, &Value::Char(65), &Value::Byte(65)),
        Ok(Value::Integer(130))
    ));
    // LT Y31 Ar Option, Option_Option_Same / Option_Enum, LT Y31 Ar Enum, Enum_Enum_Same: an Option or Enum with an Option or
    // Enum gives an Option.
    let o = |n| Value::Option(n, vec!["A".to_string()].into());
    assert!(matches!(arith(Arith::Add, &o(1), &o(3)), Ok(Value::Option(4, _))));
    assert!(matches!(
        arith(Arith::Sub, &o(1), &Value::Enum(0, 5)),
        Ok(Value::Option(-4, _))
    ));
    assert!(matches!(
        arith(Arith::Add, &Value::Enum(0, 1), &Value::Enum(0, 1)),
        Ok(Value::Option(2, _))
    ));
    // With a number it stays a number (Option_Integer, Enum_Integer).
    assert!(matches!(
        arith(Arith::Add, &o(1), &Value::Integer(65)),
        Ok(Value::Integer(66))
    ));
}

#[test]
fn comparison_cells_of_the_matrix() {
    use al_builtins::ops::compare_operator;
    use std::cmp::Ordering;
    let t = |s: &str| Value::Text(s.into());
    let c = |s: &str| Value::Code(s.into());
    // LT Y31 Cmp Text, Text_Char: the Char is a text of one character, in the culture order: 'a' < 'A', not 'A' < 'a'.
    assert_eq!(
        compare_operator(&t("a"), &Value::Char(65), true).unwrap(),
        Ordering::Less
    );
    assert_ne!(
        compare_operator(&t("A"), &Value::Char(97), true).unwrap(),
        Ordering::Less
    );
    assert_ne!(
        compare_operator(&t("abc"), &Value::Char(65), true).unwrap(),
        Ordering::Less
    );
    // Char_Text: 'A' < 'abc'.
    assert_eq!(
        compare_operator(&Value::Char(65), &t("abc"), true).unwrap(),
        Ordering::Less
    );
    // Code10_Char: 'A' < 'a' and 'ABC' < 'a' are false, 'A' = 'a' is false.
    assert_ne!(
        compare_operator(&c("A"), &Value::Char(97), true).unwrap(),
        Ordering::Less
    );
    assert_ne!(
        compare_operator(&c("ABC"), &Value::Char(97), true).unwrap(),
        Ordering::Less
    );
    assert_ne!(
        compare_operator(&c("A"), &Value::Char(97), false).unwrap(),
        Ordering::Equal
    );
    // Text_Guid / Guid_Text: `<` with a Text first is false and with the Guid first true, a text that is no GUID included.
    let g = Value::Guid([7; 16]);
    assert_ne!(compare_operator(&t("a"), &g, true).unwrap(), Ordering::Less);
    assert_eq!(compare_operator(&g, &t("a"), true).unwrap(), Ordering::Less);
    assert_ne!(compare_operator(&c("ABC"), &g, true).unwrap(), Ordering::Less);
}

#[test]
fn date_and_datetime_arithmetic_edges() {
    use al_builtins::ops::{Arith, arith};
    // LT Y31 Ar Date, Date_Integer: 2026-01-31 - 2147483647 overflows.
    let d = Value::Date(739647, false);
    let e = err(arith(Arith::Sub, &d, &Value::Integer(2147483647)));
    assert_eq!(
        e,
        (
            "AL:Overflow".into(),
            "Arithmetic operation resulted in an overflow.".into()
        )
    );
    // LT Y31 Ar DateTime, DateTime_Decimal: DT - 1.5 names -1.5.
    let dt = Value::DateTime(63_904_000_000_000);
    assert_eq!(
        err(arith(Arith::Sub, &dt, &dec("1.5"))).1,
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value -1.5 to System.Int64."
    );
}

#[test]
fn byte_plus_char_of_a_compound_assignment() {
    use al_builtins::ops::{Arith, arith};
    // LT Y31 Ar Char, Char_Byte `compound_add.A.255`: Byte 255 += 'A' is 225.
    assert!(matches!(
        arith(Arith::Add, &Value::Byte(255), &Value::Char(65)),
        Ok(Value::Integer(225))
    ));
}

#[test]
fn variant_cells_that_give_values() {
    // LT Y31 Cv Variant: a Variant holding false/true into a number is 0/1, an Enum its ordinal, a Byte into a Text its
    // character, a DateTime into a Date its date.
    assert!(matches!(
        variant_into(Value::Boolean(false), &RtType::Integer),
        Ok(Value::Integer(0))
    ));
    assert!(matches!(variant_into(Value::Boolean(false), &RtType::Decimal), Ok(Value::Decimal(d)) if d.is_zero()));
    assert!(matches!(
        variant_into(Value::Boolean(true), &RtType::Duration),
        Ok(Value::Duration(1))
    ));
    assert!(matches!(
        variant_into(Value::Boolean(true), &opt()),
        Ok(Value::Option(1, _))
    ));
    assert!(matches!(
        variant_into(Value::Enum(0, 2), &RtType::BigInteger),
        Ok(Value::BigInteger(2))
    ));
    assert!(matches!(
        variant_into(Value::Enum(0, 2), &RtType::Char),
        Ok(Value::Char(2))
    ));
    match variant_into(Value::Byte(65), &RtType::Code(Some(3))) {
        Ok(Value::Code(s)) => assert_eq!(s.as_str(), "A"),
        other => panic!("{other:?}"),
    }
    // CreateDateTime(20260131D, 120000T) into a Date is 2026-01-31.
    let noon = Value::DateTime(739_647 * 86_400_000 - 86_400_000 + 43_200_000 + 1);
    assert!(matches!(
        variant_into(noon, &RtType::Date),
        Ok(Value::Date(739_647, false))
    ));
}

#[test]
fn numbers_by_ordinal_code_and_milliseconds() {
    use al_builtins::ops::{Arith, arith, compare_operator};
    use std::cmp::Ordering;
    let d = Value::Date(739_647, false);
    // LT Y31 Ar Date: Date_Enum + B is one day, Date_Char + 'A' 65 days, Date_Duration + 65 ms 65 days.
    assert!(matches!(
        arith(Arith::Add, &d, &Value::Enum(0, 1)),
        Ok(Value::Date(739_648, false))
    ));
    assert!(matches!(
        arith(Arith::Sub, &d, &Value::Char(65)),
        Ok(Value::Date(739_582, false))
    ));
    assert!(matches!(
        arith(Arith::Add, &d, &Value::Duration(65)),
        Ok(Value::Date(739_712, false))
    ));
    // Char_Enum 'A' + B is 66, Enum_Byte B - 255 is -254, Duration_Char 65 ms - 'a' is the BigInteger -32.
    assert!(matches!(
        arith(Arith::Add, &Value::Char(65), &Value::Enum(0, 1)),
        Ok(Value::Integer(66))
    ));
    assert!(matches!(
        arith(Arith::Sub, &Value::Enum(0, 1), &Value::Byte(255)),
        Ok(Value::Integer(-254))
    ));
    assert!(matches!(
        arith(Arith::Sub, &Value::Duration(65), &Value::Char(97)),
        Ok(Value::BigInteger(-32))
    ));
    // Decimal_Duration 1.5 - 65 ms is the Decimal -63.5.
    assert!(
        matches!(arith(Arith::Sub, &dec("1.5"), &Value::Duration(65)), Ok(Value::Decimal(x)) if x == "-63.5".parse::<Decimal>().unwrap())
    );
    // Guid_Char and Guid_Guid_Same join as texts.
    let g = Value::Guid([0; 16]);
    match arith(Arith::Add, &g, &Value::Char(65)) {
        Ok(Value::Text(s)) => assert_eq!(s.as_str(), "{00000000-0000-0000-0000-000000000000}A"),
        other => panic!("{other:?}"),
    }
    // LT Y31 Cmp: Option_Char 1 < 'A', Duration_Char 65 ms = 'A', Enum_Byte B < 65.
    let o = Value::Option(1, vec!["A".to_string()].into());
    assert_eq!(compare_operator(&o, &Value::Char(65), true).unwrap(), Ordering::Less);
    assert_eq!(
        compare_operator(&Value::Duration(65), &Value::Char(65), false).unwrap(),
        Ordering::Equal
    );
    assert_eq!(
        compare_operator(&Value::Enum(0, 1), &Value::Byte(65), true).unwrap(),
        Ordering::Less
    );
    // Guid_Char: `<` with the Guid first is true.
    assert_eq!(compare_operator(&g, &Value::Char(65), true).unwrap(), Ordering::Less);
}

#[test]
fn variant_text_into_numbers() {
    let t = |s: &str| Value::Text(s.into());
    // LT Y31 Cv Variant: '12' converts, 'abc' raises TypeConversion:Format (Duration: AL:Format).
    assert!(matches!(variant_into(t("12"), &RtType::Byte), Ok(Value::Byte(12))));
    assert!(matches!(
        variant_into(t("12"), &RtType::Boolean),
        Ok(Value::Boolean(true))
    ));
    assert!(matches!(variant_into(t("12"), &opt()), Ok(Value::Option(12, _))));
    assert_eq!(
        err(variant_into(t("abc"), &RtType::Char)),
        (
            "TypeConversion:Format".into(),
            "The expression Variant cannot be type-converted to a Char value.".into()
        )
    );
    assert_eq!(
        err(variant_into(Value::Code("ABC".into()), &RtType::Duration)),
        (
            "AL:Format".into(),
            "The input string 'ABC' was not in a correct format.".into()
        )
    );
}

#[test]
fn variant_numbers_into_boolean() {
    // LT Y31 Cv Variant `Variant_To_Boolean`: 3, 300, 2.0, 'A', Byte 65, option B, enum C are true; 1.5 raises ConversionCast.
    assert!(matches!(
        variant_into(Value::Integer(3), &RtType::Boolean),
        Ok(Value::Boolean(true))
    ));
    assert!(matches!(
        variant_into(dec("2.0"), &RtType::Boolean),
        Ok(Value::Boolean(true))
    ));
    assert!(matches!(
        variant_into(Value::Char(65), &RtType::Boolean),
        Ok(Value::Boolean(true))
    ));
    assert_eq!(
        err(variant_into(dec("1.5"), &RtType::Boolean)).1,
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 1.5 to System.Int32."
    );
    // Zero is not tried.
    assert!(matches!(
        variant_into(Value::Integer(0), &RtType::Boolean),
        Err(Signal::Unsupported(_))
    ));
}

#[test]
fn a_variant_enum_into_boolean() {
    // LT Y31 Cv Variant `Variant_To_Boolean` enumC: true.
    assert!(matches!(
        variant_into(Value::Enum(0, 2), &RtType::Boolean),
        Ok(Value::Boolean(true))
    ));
}
