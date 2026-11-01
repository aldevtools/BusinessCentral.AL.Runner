//! An array passed to a parameter declared with more elements than the argument has (Tests-ERM
//! codeunit 134831 passes an `array[2]` variable to `var DimensionValue: array[9]`) panicked in the
//! interpreter (issue 18). BC raises `Index out of bounds.` (code IndexOutOfBounds) for an element beyond the argument
//! (misc/lt-probes-needed-5/PnE01Arrays.al:SmallerArray_VarParameter, SmallerArray_ByValue, Integer arrays), with the elements
//! that exist reading and writing as before. `ArrayLen` inside the procedure is the
//! argument's length (types/W7VarArray.al), so a loop up to `ArrayLen` stays within it.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62700 "AS Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; N; Integer) { } }
    keys { key(PK; "No.") { } }
}
codeunit 62702 "AS Helper"
{
    procedure FillNine(var Rows: array[9] of Record "AS Row") var i: Integer; begin for i := 1 to 9 do Rows[i].N := i; end;
    procedure FillTwo(var Rows: array[2] of Record "AS Row") var i: Integer; begin for i := 1 to ArrayLen(Rows) do Rows[i].N := i; end;
    procedure ClearThenFill(var Vals: array[3] of Decimal) var i: Integer; begin Clear(Vals); for i := 1 to ArrayLen(Vals) do Vals[i] := i; end;
    procedure Calc14(var Amounts: array[14] of Decimal; var Total: Decimal) var i: Integer; begin Clear(Amounts); for i := 1 to ArrayLen(Amounts) do Amounts[i] := Calc(i); Total := Calc(0); end;
    procedure Calc(i: Integer): Decimal begin exit(i); end;
    procedure FillIntegers(var Vals: array[5] of Integer) begin Vals[1] := 1; Vals[5] := 5; end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62701 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn an_element_beyond_the_argument_raises_the_index_error_not_a_panic() {
    let o = outcomes(
        "[Test] procedure TooSmall() var Rows: array[2] of Record \"AS Row\"; H: Codeunit \"AS Helper\"; begin H.FillNine(Rows); end;
         [Test] procedure TooSmallIntegers() var Vals: array[2] of Integer; H: Codeunit \"AS Helper\"; begin H.FillIntegers(Vals); end;
         [Test] procedure SameSize() var Rows: array[2] of Record \"AS Row\"; H: Codeunit \"AS Helper\"; begin
             H.FillTwo(Rows); if (Rows[1].N <> 1) or (Rows[2].N <> 2) then Error('values'); end;",
    );
    for (name, outcome) in &o[..2] {
        assert!(
            matches!(outcome, Outcome::Fail(why) if why.contains("Index out of bounds.")),
            "{name}: {outcome:?}"
        );
    }
    assert_eq!(o[2].1, Outcome::Pass, "{:?}", o[2]);
}

/// A parameter declared `array[14]` takes the caller's `array[16]`; `Clear` of it keeps the argument's length and the loop to
/// `ArrayLen` writes every element (Tests-Cash Flow ERMCashFlowUnitTests: CalculateAllAmounts; types/W7VarArray.al).
#[test]
fn clear_of_a_larger_argument_keeps_its_length() {
    let o = outcomes(
        "[Test] procedure Larger() var Vals: array[5] of Decimal; H: Codeunit \"AS Helper\"; begin
             H.ClearThenFill(Vals); if (Vals[4] <> 4) or (Vals[5] <> 5) then Error('values'); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}

#[test]
fn a_sixteen_element_argument_in_a_fourteen_element_parameter() {
    let o = outcomes(
        "[Test] procedure Sixteen() var Vals: array[16] of Decimal; T: Decimal; H: Codeunit \"AS Helper\"; i: Integer; begin
             H.Calc14(Vals, T); for i := 1 to ArrayLen(Vals) do if (Vals[i] <> i) then Error('values %1', i); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}
