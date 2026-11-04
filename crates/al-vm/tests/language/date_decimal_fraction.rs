//! A Date plus or minus a Decimal with a fraction, and `Next` with one, raise ConversionCast
//! (errors/PnF11DateDecimalArithmetic.al).

fn run(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

const PROCS: &str = "[TryFunction] local procedure TryPlus(D: Date; N: Decimal; var R: Date) begin R := D + N; end; \
    [TryFunction] local procedure TryMinus(D: Date; N: Decimal; var R: Date) begin R := D - N; end; \
    local procedure Show(Ok: Boolean; R: Date): Text begin if Ok then exit(Format(R, 0, 9)); \
    exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;";

#[test]
fn a_date_plus_or_minus_a_fraction_raises_a_conversion_error() {
    let src = format!(
        "codeunit 50100 C {{ {PROCS} procedure F(): Text var D: Date; R: Date; T: Text; begin D := DMY2Date(10, 3, 2024); \
         T := Show(TryPlus(D, 2.5, R), R) + '#' + Show(TryPlus(D, 12 / 5, R), R) + '#' + Show(TryMinus(D, 0.4, R), R) + '#' + \
         Show(TryMinus(D, -2.5, R), R) + '#' + Show(TryPlus(D, 2.0, R), R); exit(T); end; }}"
    );
    let cast = |v: &str| {
        format!(
            "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {v} to System.Int32.|ConversionCast"
        )
    };
    assert_eq!(
        run(&src),
        [
            cast("2.5"),
            cast("2.4"),
            cast("-0.4"),
            cast("2.5"),
            "2024-03-12".to_string()
        ]
        .join("#")
    );
}

#[test]
fn next_with_a_fraction_raises_a_conversion_error_and_a_whole_decimal_steps() {
    let src = "table 50100 Row { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        codeunit 50100 C { [TryFunction] local procedure TryNext(var Row: Record Row; S: Decimal; var M: Integer) \
        begin M := Row.Next(S); end; \
        procedure F(): Text var Row: Record Row; i: Integer; M: Integer; T: Text; begin \
        for i := 1 to 10 do begin Row.Id := i; Row.Insert(); end; \
        Row.Get(3); if TryNext(Row, 12 / 4, M) then T := Format(M) + '|' + Format(Row.Id); \
        Row.Get(6); if not TryNext(Row, -1.5, M) then T += '#' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Row.Id); \
        exit(T); end; }";
    assert_eq!(
        run(src),
        "3|6#Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value -1.5 to System.Int32.|ConversionCast|6"
    );
}
