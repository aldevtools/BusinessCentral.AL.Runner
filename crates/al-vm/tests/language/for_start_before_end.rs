//! A `for` loop assigns the start value to the loop variable before it evaluates the end bound, so an end bound
//! that reads the variable sees the start value (corpus master language/ControlFlow.al:667-743,
//! ForLoop_StartAssignedBeforeEndBoundIsEvaluated; al-runner issue 7458).

const SRC: &str = r#"
codeunit 50100 C
{
    var
        G: Integer;

    local procedure One(): Integer
    begin
        exit(1);
    end;

    local procedure StartSetsGlobal(): Integer
    begin
        G := 100;
        exit(1);
    end;

    local procedure EndReadsGlobal(): Integer
    begin
        exit(G + 2);
    end;

    procedure F(): Text
    var
        i: Integer;
        n: Integer;
        b: BigInteger;
        d: Decimal;
        Dt: Date;
        V: Text;
    begin
        n := 10;
        i := 0;
        for i := One() to i + n - 1 do
            V += Format(i) + ',';
        V += '|';
        i := 0;
        for i := 5 downto i - 2 do
            V += Format(i) + ',';
        V += '|';
        G := 0;
        for G := StartSetsGlobal() to G + 3 do
            V += Format(G) + ',';
        V += '|';
        G := 0;
        for G := One() to EndReadsGlobal() do
            V += Format(G) + ',';
        V += '|';
        G := 7;
        for G := 1 to G do
            V += Format(G) + ',';
        V += '|';
        b := 0;
        for b := 1 to b + 3 do
            V += Format(b) + ',';
        V += '|';
        d := 0;
        for d := 1 to d + 3 do
            V += Format(d) + ',';
        V += '|';
        Dt := 20240101D;
        for Dt := 20240110D to Dt + 2 do
            V += Format(Dt, 0, 9) + ',';
        exit(V);
    end;
}
"#;

#[test]
fn for_loop_assigns_the_start_before_it_evaluates_the_end() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "1,2,3,4,5,6,7,8,9,10,|5,4,3,|1,2,3,4,|1,2,3,|1,|1,2,3,4,|1,2,3,4,|2024-01-10,2024-01-11,2024-01-12,"
    );
}
