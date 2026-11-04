//! An Integer beyond a Byte assigned to a Byte raises ConversionCast
//! (errors/PnBErrorsCollections.al:560-583, Arithmetic_ErrorTexts `byteOverflow`).

#[test]
fn an_integer_beyond_a_byte_raises_a_conversion_error() {
    let src = "codeunit 50100 C { procedure F(): Text var Small: Integer; B: Byte; begin Small := 300; \
               asserterror B := Small; exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Overflow under conversion of System.Int32 value 300 to System.Byte.|ConversionCast"
    );
}
