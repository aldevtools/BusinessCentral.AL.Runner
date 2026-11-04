//! Lane T16: a BigInteger end bound of a `for` loop on an Integer counter, `continue;` in an object that declares a
//! procedure named Continue, MaxStrLen of a `var` Text or Code parameter, and `if ... then continue else ...` in a
//! `foreach` nested in another loop.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const FOR_BIG: &str = r#"
codeunit 69901 FB
{
    procedure Run(EndBound: BigInteger): Text
    var
        i: Integer;
        Rounds: Integer;
    begin
        ClearLastError();
        if TryLoop(EndBound, i, Rounds) then
            exit('no error|' + Format(i) + '|' + Format(Rounds));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(i) + '|' + Format(Rounds));
    end;

    procedure RunDown(EndBound: BigInteger): Text
    var
        i: Integer;
    begin
        for i := 1000 downto EndBound do
            exit('ran');
        exit('none');
    end;

    procedure ToIntMax(): Text
    var
        i: Integer;
        Rounds: Integer;
    begin
        for i := 2147483646 to 2147483647 do begin
            Rounds += 1;
            if Rounds = 5 then
                exit('stopped');
        end;
        exit(Format(i) + '|' + Format(Rounds));
    end;

    [TryFunction]
    local procedure TryLoop(EndBound: BigInteger; var i: Integer; var Rounds: Integer)
    begin
        i := 0;
        Rounds := 0;
        for i := 1000 to EndBound do begin
            Rounds += 1;
            if Rounds = 5 then
                exit;
        end;
    end;
}
"#;

fn for_big(routine: &str) -> String {
    text(run_in(FOR_BIG, "FB", routine))
}

fn for_big_with(end: i64) -> String {
    let (v, _) =
        al_testkit::run_with(FOR_BIG, "FB", "Run", vec![al_builtins::value::Value::BigInteger(end)]).expect("runs");
    match v {
        al_builtins::value::Value::Text(t) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

// corpus language/ControlFlow.al:779-830, LT_BU_ForLoop_BigIntegerEndBoundOnInteger.
#[test]
fn a_big_integer_end_bound_beyond_integer_on_an_integer_counter_fails_after_the_counter_takes_the_start() {
    assert_eq!(
        for_big_with(100_000_000_000_000),
        "error:Overflow under conversion of System.Int64 value 100000000000000 to System.Int32.|ConversionCast|1000|0"
    );
    assert_eq!(
        for_big_with(2_147_483_648),
        "error:Overflow under conversion of System.Int64 value 2147483648 to System.Int32.|ConversionCast|1000|0"
    );
    assert_eq!(for_big_with(5), "no error|1000|0");
    assert_eq!(for_big("ToIntMax"), "2147483647|2");
}

#[test]
fn a_downto_loop_with_a_big_integer_bound_beyond_integer_is_refused() {
    let r = al_testkit::run_with(
        FOR_BIG,
        "FB",
        "RunDown",
        vec![al_builtins::value::Value::BigInteger(-3_000_000_000)],
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const CONTINUE_PROC: &str = r#"
codeunit 69902 CP
{
    var
        Calls: Integer;

    procedure Continue()
    begin
        Calls += 1;
    end;

    procedure ForLoop(): Text
    var
        i: Integer;
        S: Text;
    begin
        Calls := 0;
        for i := 1 to 3 do begin
            if i = 2 then
                continue;
            S += Format(i);
        end;
        exit(S + '|calls=' + Format(Calls));
    end;

    procedure WhileLoop(): Text
    var
        i: Integer;
        S: Text;
    begin
        Calls := 0;
        while i < 3 do begin
            i += 1;
            if i = 2 then
                continue;
            S += Format(i);
        end;
        exit(S + '|calls=' + Format(Calls));
    end;
}
"#;

// corpus types/PUContinue.al:166-180 (Continue_WithProcedureNamedContinue) with codeunit 82150 at :221-272: `continue;` calls
// the procedure.
#[test]
fn continue_calls_a_procedure_named_continue_of_the_same_object() {
    assert_eq!(text(run_in(CONTINUE_PROC, "CP", "ForLoop")), "123|calls=1");
    assert_eq!(text(run_in(CONTINUE_PROC, "CP", "WhileLoop")), "123|calls=1");
}

#[test]
fn continue_with_a_procedure_named_continue_that_takes_parameters_is_refused() {
    let src = r#"
codeunit 69903 CQ
{
    procedure Continue(x: Integer)
    begin
    end;

    procedure F(): Integer
    var
        i: Integer;
    begin
        for i := 1 to 3 do
            continue;
        exit(i);
    end;
}
"#;
    let r = run_in(src, "CQ", "F");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const MAXSTRLEN: &str = r#"
table 69904 MT
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Description; Text[100]) { }
    }
}

codeunit 69905 MS
{
    procedure Lengths(): Text
    var
        Item: Record MT;
        T30: Text[30];
        T: Text;
        C20: Code[20];
    begin
        exit(Format(MaxOfVarText(Item.Description)) + '|' + Format(MaxOfVarText(T30)) + '|' + Format(MaxOfVarText(T)) + '|' +
            Format(MaxOfVarText30(Item.Description)) + '|' + Format(MaxOfVarText30(T30)) + '|' +
            Format(MaxOfText(Item.Description)) + '|' + Format(MaxOfText(C20)) + '|' + Format(MaxOfText30(Item.Description)) + '|' +
            Format(MaxOfVarCode30(C20)));
    end;

    procedure Chained(): Text
    var
        T30: Text[30];
    begin
        exit(Format(PassOn(T30)));
    end;

    local procedure PassOn(var s: Text): Integer
    begin
        exit(MaxOfVarText(s));
    end;

    local procedure MaxOfVarText(var s: Text): Integer
    begin
        exit(MaxStrLen(s));
    end;

    local procedure MaxOfVarText30(var s: Text[30]): Integer
    begin
        exit(MaxStrLen(s));
    end;

    local procedure MaxOfText(s: Text): Integer
    begin
        exit(MaxStrLen(s));
    end;

    local procedure MaxOfText30(s: Text[30]): Integer
    begin
        exit(MaxStrLen(s));
    end;

    local procedure MaxOfVarCode30(var s: Code[30]): Integer
    begin
        exit(MaxStrLen(s));
    end;
}
"#;

// corpus text/text-methods/MaxStrLen.Codeunit.al:91-151, LT_BU_MaxStrLen_TextParameters: a `var` parameter reports the
// declared length of the caller's variable or field, a by-value parameter its own.
#[test]
fn max_str_len_of_a_var_parameter_is_the_length_of_the_callers_variable() {
    assert_eq!(
        text(run_in(MAXSTRLEN, "MS", "Lengths")),
        "100|30|2147483647|100|30|2147483647|2147483647|30|20"
    );
}

// corpus text/text-methods/MaxStrLen.Codeunit.al:146-170, PYC_MaxStrLen_VarParameterHops (`two hops text30` = 30).
#[test]
fn max_str_len_of_a_var_parameter_handed_on_from_another_var_parameter_is_the_first_callers_length() {
    assert_eq!(text(run_in(MAXSTRLEN, "MS", "Chained")), "30");
}

const CONTINUE_ELSE: &str = r#"
codeunit 69906 CE
{
    procedure Nested(): Text
    var
        List: List of [Text];
        Item: Text;
        FieldNo: Integer;
        Seen: Text;
    begin
        List.Add('A');
        List.Add('B');
        for FieldNo := 1 to 2 do
            foreach Item in List do
                if Item = 'A' then
                    continue
                else
                    Seen += Item;
        exit(Seen);
    end;

    procedure TopLevelBlock(): Text
    var
        List: List of [Text];
        Item: Text;
        Seen: Text;
    begin
        List.Add('A');
        List.Add('B');
        List.Add('C');
        foreach Item in List do begin
            if Item = 'B' then
                continue
            else
                Seen += Item;
            Seen += '.';
        end;
        exit(Seen);
    end;
}
"#;

// corpus types/PUContinue.al:182-218, Continue_ForeachInFor_IfContinueElse: BC skips the else branch in the nested shape
// (microsoft/AL issue 8339), which nothing explains yet, so the shape is refused; the block in a foreach that is not nested
// behaves as continue does elsewhere (`A.C.`).
#[test]
fn continue_else_in_a_foreach_nested_in_a_loop_is_refused() {
    let r = run_in(CONTINUE_ELSE, "CE", "Nested");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
    assert_eq!(text(run_in(CONTINUE_ELSE, "CE", "TopLevelBlock")), "A.C.");
}

// `#undef x` after `#define X`: preprocessor symbols are case-sensitive in `#if`
// (permissions/BmPreprocessorDefine.al:83), but whether `#undef` matches the other case is not pinned:
// the file is refused (docs/probes-needed.md, "Loops, continue and MaxStrLen").
#[test]
fn undef_of_a_symbol_defined_only_in_another_case_is_refused() {
    let src = "#define LTX\n#undef ltx\ncodeunit 69907 PU { procedure F(): Text begin\n#if LTX\nexit('defined');\n#else\nexit('not defined');\n#endif\nend; }\n";
    // The workspace reports the directive (so `al_testkit::compile` would stop here) and every routine of the file is refused.
    let (_, l) = al_sema::lower::compile(vec![al_testkit::app(src)], &Default::default());
    let reasons: Vec<String> = al_testkit::blocked(&l).into_iter().flatten().flatten().collect();
    assert!(reasons.iter().any(|r| r.contains("#undef ltx")), "{reasons:?}");
    let same_case = "#define LTX\n#undef LTX\ncodeunit 69908 PV { procedure F(): Text begin\n#if LTX\nexit('defined');\n#else\nexit('not defined');\n#endif\nend; }\n";
    assert_eq!(text(run_in(same_case, "PV", "F")), "not defined");
}
