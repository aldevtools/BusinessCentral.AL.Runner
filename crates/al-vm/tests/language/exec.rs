//! End to end: AL source → compile → run.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{int, text};
use al_vm::{Engine, Session};

fn run(src: &str, routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    let l = al_testkit::compile(src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.routines.iter().any(|r| r.name == routine))
        .expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, args).map(|(v, _)| v)
}

fn cu(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

#[test]
fn exit_with_arithmetic() {
    assert_eq!(
        int(run(
            &cu("procedure F(): Integer begin exit(2 + 3 * 4); end;"),
            "F",
            vec![]
        )),
        14
    );
}

#[test]
fn named_return_value_and_defaults() {
    assert_eq!(
        int(run(&cu("procedure F() R: Integer begin R := 7; end;"), "F", vec![])),
        7
    );
    assert_eq!(
        int(run(
            &cu("procedure F(): Integer var X: Integer; begin exit(X); end;"),
            "F",
            vec![]
        )),
        0
    );
    assert_eq!(
        int(run(&cu("procedure F(): Integer begin end;"), "F", vec![])),
        0,
        "no exit returns the default"
    );
}

#[test]
fn parameters() {
    assert_eq!(
        int(run(
            &cu("procedure F(A: Integer; B: Integer): Integer begin exit(A - B); end;"),
            "F",
            vec![Value::Integer(9), Value::Integer(4)]
        )),
        5
    );
}

#[test]
fn branches_and_loops() {
    let src = cu("procedure F(): Integer var I: Integer; S: Integer; begin
        for I := 1 to 4 do S += I;
        for I := 3 downto 1 do S += 10;
        while S < 100 do S := S * 2;
        repeat S -= 1; until S mod 7 = 0;
        if S > 1000 then S := 0 else S := S + 1;
        exit(S);
    end;");
    // 1+2+3+4 = 10; +30 = 40; 80, 160; 159,...,154 (154 mod 7 = 0); +1 = 155
    assert_eq!(int(run(&src, "F", vec![])), 155);
}

#[test]
fn case_with_ranges_and_else() {
    let src =
        cu("procedure F(X: Integer): Integer begin case X of 1, 2: exit(10); 3..5: exit(20); else exit(30); end; end;");
    assert_eq!(int(run(&src, "F", vec![Value::Integer(2)])), 10);
    assert_eq!(int(run(&src, "F", vec![Value::Integer(4)])), 20);
    assert_eq!(int(run(&src, "F", vec![Value::Integer(9)])), 30);
}

#[test]
fn in_sets_ternary_and_boolean_ops() {
    let src = cu("procedure F(X: Integer): Integer begin exit((X in [1, 3..5]) and not (X = 4) ? 1 : 0); end;");
    assert_eq!(int(run(&src, "F", vec![Value::Integer(3)])), 1);
    assert_eq!(int(run(&src, "F", vec![Value::Integer(4)])), 0);
}

#[test]
fn break_leaves_the_loop() {
    let src = cu("procedure F(): Integer var I: Integer; begin for I := 1 to 10 do if I = 3 then break; exit(I); end;");
    assert_eq!(int(run(&src, "F", vec![])), 3);
}

#[test]
fn exit_from_inside_a_loop() {
    let src =
        cu("procedure F(): Integer var I: Integer; begin while true do begin I += 1; if I = 5 then exit(I); end; end;");
    assert_eq!(int(run(&src, "F", vec![])), 5);
}

#[test]
fn var_parameters_write_back() {
    let src = cu(
        "procedure Inc(var X: Integer) begin X += 1; end; procedure F(): Integer var V: Integer; begin V := 1; Inc(V); Inc(V); exit(V); end;",
    );
    assert_eq!(int(run(&src, "F", vec![])), 3);
}

#[test]
fn globals_live_on_the_instance() {
    let src = cu(
        "var G: Integer; procedure Bump() begin G += 1; end; procedure F(): Integer begin Bump(); Bump(); exit(G); end;",
    );
    assert_eq!(int(run(&src, "F", vec![])), 2);
}

#[test]
fn codeunit_variables_have_their_own_instances() {
    let src = "codeunit 50101 Counter { var N: Integer; procedure Add(): Integer begin N += 1; exit(N); end; }\n"
        .to_string()
        + &cu(
            "procedure F(): Integer var A: Codeunit Counter; B: Codeunit Counter; begin A.Add(); A.Add(); exit(A.Add() * 10 + B.Add()); end;",
        );
    assert_eq!(int(run(&src, "F", vec![])), 31);
}

#[test]
fn builtins_and_conversions() {
    assert_eq!(
        text(run(
            &cu("procedure F(): Text begin exit(Format(42, 5)); end;"),
            "F",
            vec![]
        )),
        "   42"
    );
    assert_eq!(
        int(run(
            &cu("procedure F(): Integer begin exit(StrLen('abc')); end;"),
            "F",
            vec![]
        )),
        3
    );
    assert_eq!(
        text(run(
            &cu("procedure F(): Code[10] var C: Code[10]; begin C := ' ab '; exit(C); end;"),
            "F",
            vec![]
        )),
        "AB"
    );
}

#[test]
fn error_and_asserterror() {
    let src = cu("procedure F(): Text begin asserterror Error('boom %1', 5); exit(GetLastErrorText()); end;");
    assert_eq!(text(run(&src, "F", vec![])), "boom 5");
    let src = cu("procedure F() begin Error('nope'); end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Error(e)) if e.message == "nope"));
}

#[test]
fn asserterror_without_an_error_is_an_error() {
    let src = cu("procedure F() var I: Integer; begin asserterror I := 1; end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Error(_))));
}

#[test]
fn a_blocked_routine_reports_unsupported() {
    let src = cu("procedure F() begin Debugger.Deactivate(); end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Unsupported(r)) if r.contains("Debugger")));
}

#[test]
fn enum_values_format_by_name() {
    let src = "enum 50900 E { value(0; Alpha) { } value(3; \"Beta Two\") { } }\n".to_string()
        + &cu("procedure F(): Text var X: Enum E; begin X := E::\"Beta Two\"; exit(Format(X)); end;");
    assert_eq!(text(run(&src, "F", vec![])), "Beta Two");
}

#[test]
fn runaway_recursion_raises_stack_overflow_not_a_crash() {
    // Same stack the CLI gives the VM (al-test-cli main).
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(|| {
            let src = cu("procedure F(N: Integer): Integer begin exit(F(N + 1)); end;");
            assert!(
                matches!(run(&src, "F", vec![Value::Integer(0)]), Err(Signal::Error(e)) if e.code == "StackOverflow")
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_character_can_be_assigned_into_a_text_by_position() {
    // handlers/TestPageFieldFormats_Tests.al:58-66: `Nnbsp[1] := 8239` on an empty Text[1] sets
    // its one character (the narrow no-break space), which Replace then finds.
    let src = cu("procedure F(): Text var N: Text[1]; T: Text; begin
                      N[1] := 8239; T := 'a' + N + 'b';
                      exit(Format(StrLen(N)) + Format(StrLen(T)) + Format(StrLen(T.Replace(N, ' ')))); end;");
    assert_eq!(text(run(&src, "F", vec![])), "133");
    // Past the end of the text, past its declared length: errors (types/TestProbeScalarContracts.al:TextChar_AssignByIndex).
    for (body, code) in [
        ("N[2] := 65;", "OutsidePermittedRange"),
        ("N[1] := 65; N[3] := 66;", "OutsidePermittedRange"),
        ("N[1] := 65; N[2] := 66;", "StringLengthExceeded"),
        ("N[0] := 65;", "OutsidePermittedRange"),
    ] {
        let src = cu(&format!(
            "procedure F(): Text var N: Text[1]; begin {body} exit(N); end;"
        ));
        let r = run(&src, "F", vec![]);
        assert!(matches!(&r, Err(Signal::Error(e)) if e.code == code), "{body}: {r:?}");
    }
}

#[test]
fn a_990_deep_chain_completes() {
    // codeunit/TestCodeunitRecursionDepth.al:57-70 (RecursionDepth_JustInsideTheCeiling_Completes):
    // the platform's ceiling lies above 990 calls. Past it the run raises the StackOverflow error
    // (session/TestProbeSessionMiscContracts.al:CallDepth_1001).
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(|| {
            let src = cu(
                "procedure R(N: Integer): Integer begin if N <= 0 then exit(0); exit(1 + R(N - 1)); end;
                          procedure F(N: Integer): Integer begin exit(R(N)); end;",
            );
            let r = run(&src, "F", vec![Value::Integer(990)]);
            assert!(matches!(r, Ok(Value::Integer(990))), "{r:?}");
            assert!(matches!(
                run(&src, "F", vec![Value::Integer(5000)]),
                Err(Signal::Error(e)) if e.code == "StackOverflow"
            ));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn try_function_used_as_a_value_catches_errors() {
    // Doc (TryFunction attribute): the return value tells whether the method ran without errors.
    let src = cu("[TryFunction] procedure T(Fail: Boolean) begin if Fail then Error('inner'); end;
        procedure F(): Text var Ok1: Boolean; Ok2: Boolean; begin Ok1 := T(false); Ok2 := T(true); exit(Format(Ok1) + '/' + Format(Ok2) + '/' + GetLastErrorText()); end;");
    assert_eq!(text(run(&src, "F", vec![])), "Yes/No/inner");
}

#[test]
fn try_function_called_as_a_statement_raises() {
    // Doc: when the return value is not used, the try method behaves like a normal method.
    let src = cu("[TryFunction] procedure T() begin Error('inner'); end; procedure F() begin T(); end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Error(e)) if e.message == "inner"));
}

#[test]
fn single_instance_codeunits_share_state_across_variables() {
    let src = "codeunit 50102 Single { SingleInstance = true; var N: Integer; procedure Add(): Integer begin N += 1; exit(N); end; }\n".to_string()
        + &cu("procedure F(): Integer var A: Codeunit Single; B: Codeunit Single; begin A.Add(); exit(B.Add()); end;");
    assert_eq!(int(run(&src, "F", vec![])), 2);
}

#[test]
fn an_isolated_event_without_subscribers_does_nothing() {
    // Event dispatch is tests/events.rs; an empty publisher body must not hide subscribers,
    // and with none there is nothing to run.
    let src = cu(
        "[IntegrationEvent(false, false, true)] procedure OnX() begin end; procedure F(): Integer begin OnX(); exit(7); end;",
    );
    assert_eq!(int(run(&src, "F", vec![])), 7);
}

#[test]
fn lists() {
    let src = cu("procedure F(): Text var L: List of [Integer]; M: List of [Integer]; I: Integer; S: Text; begin
        L.Add(3); L.Add(1); L.Add(2); L.Insert(1, 9);
        M := L;
        M.Add(4);
        foreach I in L do S += Format(I);
        L.Reverse();
        exit(S + '|' + Format(L.Count()) + Format(L.Get(1)) + Format(L.IndexOf(9)) + Format(L.Contains(7)) + Format(L.Remove(9)) + Format(L.Count()));
    end;");
    // Lists are reference types: M := L shares (ADR-010), so M.Add(4) is visible in L.
    assert_eq!(text(run(&src, "F", vec![])), "93124|545NoYes4");
}

#[test]
fn list_elements_convert_to_the_element_type() {
    let src = cu("procedure F(): Text var L: List of [Code[10]]; begin L.Add('abc'); exit(L.Get(1)); end;");
    assert_eq!(text(run(&src, "F", vec![])), "ABC");
}

#[test]
fn dictionaries() {
    let src = cu(
        "procedure F(): Text var D: Dictionary of [Text, Integer]; V: Integer; K: Text; S: Text; begin
        D.Add('b', 2); D.Add('a', 1); D.Set('b', 20);
        foreach K in D.Keys() do S += K;
        if D.Get('a', V) then S += Format(V);
        exit(S + Format(D.Get('b')) + Format(D.ContainsKey('z')) + Format(D.Remove('a')) + Format(D.Count()));
    end;",
    );
    assert_eq!(text(run(&src, "F", vec![])), "ba120NoYes1");
}

#[test]
fn list_remove_at_past_the_end_raises() {
    // session/TestProbeSessionMiscContracts.al:Dictionary_List_ErrorTexts, `list_removeat_4`.
    let src = cu("procedure F(): Integer var L: List of [Integer]; begin L.RemoveAt(1); exit(0); end;");
    let r = run(&src, "F", vec![]);
    assert!(
        matches!(&r, Err(Signal::Error(e)) if e.code == "ArgumentOutOfRange"),
        "{r:?}"
    );
}

#[test]
fn create_guid_gives_distinct_non_null_guids() {
    let src = cu(
        "procedure F(): Text var A: Guid; B: Guid; N: Guid; begin A := CreateGuid(); B := CreateGuid(); exit(Format(A <> B) + Format(IsNullGuid(A)) + Format(IsNullGuid(N))); end;",
    );
    assert_eq!(text(run(&src, "F", vec![])), "YesNoYes");
}

#[test]
fn unpinned_error_and_commit_behaviors_are_unsupported() {
    // [CommitBehavior(Ignore | Error)] is implemented (tests/transactions.rs) and so is
    // [ErrorBehavior(Collect)] (tests/errorcollect.rs); any other value is refused.
    let src = cu("[ErrorBehavior(ErrorBehavior::Default)] procedure G() begin end; procedure F() begin G(); end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Unsupported(r)) if r.contains("ErrorBehavior")));
    let src = cu("[CommitBehavior(CommitBehavior::Allow)] procedure G() begin end; procedure F() begin G(); end;");
    assert!(matches!(run(&src, "F", vec![]), Err(Signal::Unsupported(r)) if r.contains("CommitBehavior")));
}

#[test]
fn labels_hold_their_text() {
    // Label variables evaluate to their declared text, global or local, with or without
    // properties (error-handling/TestCommitBehaviorAttribute.al raises Error(UnrelatedErr)
    // and matches the text).
    let src = cu("var G: Label 'glob %1', Locked = true;
         procedure P(): Text
         var L: Label 'it''s local', Comment = 'x';
         begin exit(StrSubstNo(G, 1) + '|' + L); end;");
    assert_eq!(text(run(&src, "P", vec![])), "glob 1|it's local");
}

// types/TestVariantExtended.al:Variant_IsRecordId_ReturnsTrue_WhenAssignedRecordId
#[test]
fn a_variant_holding_a_record_id_reports_it() {
    let src = cu(
        "procedure F(): Text var V: Variant; RId: RecordId; begin V := RId; exit(Format(V.IsRecordId()) + Format(V.IsInteger())); end;",
    );
    assert_eq!(text(run(&src, "F", vec![])), "YesNo");
}

// types/TestEnumCaption.al -- Format(enum) is the value's declared Caption; without one, its name.
#[test]
fn enum_values_format_by_caption_when_declared() {
    let src = "enum 50900 E { value(0; Alpha) { Caption = 'The Alpha'; } value(1; Beta) { } value(2; Gamma) { Caption = 'It''s G', Locked = true; } }\n".to_string()
        + &cu("procedure F(): Text var X: Enum E; begin X := E::Alpha; exit(Format(X) + '|' + Format(E::Beta) + '|' + Format(E::Gamma)); end;");
    assert_eq!(text(run(&src, "F", vec![])), "The Alpha|Beta|It's G");
}

// error-handling/TestTryFunctionContracts.al:TryFunction_VarParam_SetBeforeError_VisibleToCaller and
// codeunit/TestScopeIsolationContracts.al:Test03AssertErrorVarParamChangesVisibleToCaller -- a var
// parameter set before the error is visible to the caller that catches it, through any depth.
#[test]
fn var_parameters_set_before_an_error_are_visible_to_the_catching_caller() {
    let src = cu(
        "[TryFunction] procedure T(var X: Integer) begin X := 42; Error('boom'); end;
        procedure Outer(var X: Integer) begin Inner(X); end;
        procedure Inner(var X: Integer) begin X := 7; Error('deep'); end;
        procedure F(): Text var A: Integer; B: Integer; Ok: Boolean; begin
            Ok := T(A);
            asserterror Outer(B);
            exit(Format(Ok) + Format(A) + '|' + Format(B));
        end;",
    );
    assert_eq!(text(run(&src, "F", vec![])), "No42|7");
}

// ------------------------------------------------------------------ the `this` keyword
// Not in a corpus test of its own. Base Application's "Base64 Convert Impl." calls
// `this.EmitLengthWarning(...)` on its own procedure (reached by the media tests 60130, 68120 and
// 69934); `this` is the current object (Microsoft Learn, "this keyword").

#[test]
fn this_calls_a_procedure_of_the_same_object() {
    let src = cu("procedure Add(A: Integer): Integer begin exit(A + 1); end;
         procedure F(): Integer begin exit(this.Add(41)); end;");
    assert_eq!(int(run(&src, "F", vec![])), 42);
}

#[test]
fn this_reads_and_writes_a_global_variable_and_calls_in_statement_position() {
    let src = cu("var G: Integer;
         procedure Bump(N: Integer) begin G := G + N; end;
         procedure F(): Integer begin this.G := 5; this.Bump(2); exit(this.G); end;");
    assert_eq!(int(run(&src, "F", vec![])), 7);
}

#[test]
fn a_member_reached_through_this_can_be_a_var_argument() {
    let src = cu("var G: Integer;
         procedure Twice(var N: Integer) begin N := N * 2; end;
         procedure F(): Integer begin this.G := 4; this.Twice(this.G); exit(G); end;");
    assert_eq!(int(run(&src, "F", vec![])), 8);
}

// ------------------------------------------------------- property forms of builtin members
// `Notif.Id := x`, `Rec.ReadIsolation := x` and `Text[1]` of a call result are plain AL.
// Corpus handlers/TestNotificationHandler.al:55-56 (Notification.Id round trip),
// json/TestJsonXmlDeepContracts.al:71 (`Format(10, 0, '<Integer>')[1]`); the record form is
// used all over Base Application (CostAdjmtSignals.Codeunit.al: `Item.ReadIsolation := ...`).

#[test]
fn a_notification_id_assigned_reads_back() {
    let src = cu("procedure F(): Text var N: Notification; G: Guid; begin
            G := CreateGuid(); N.Id := G; exit(Format(N.Id = G)); end;");
    assert_eq!(text(run(&src, "F", vec![])), "Yes");
}

#[test]
fn a_record_read_isolation_can_be_assigned_and_read() {
    let src = "table 50101 T { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record T; RR: RecordRef; begin
            R.ReadIsolation := IsolationLevel::ReadUncommitted;
            RR.Open(50101); RR.ReadIsolation := IsolationLevel::UpdLock;
            exit(Format(R.ReadIsolation) + '/' + Format(RR.ReadIsolation)); end; }";
    assert_eq!(text(run(src, "F", vec![])), "ReadUncommitted/UpdLock");
}

#[test]
fn a_character_of_a_call_result_can_be_indexed() {
    let src = cu("procedure F(): Text begin exit('a' + Format(10, 0, '<Integer>')[1] + 'b'); end;");
    assert_eq!(text(run(&src, "F", vec![])), "a1b");
}

// ------------------------------------------------------------------------------- ClearAll
// types/TestClearContracts.al:ClearAll_ResetsCodeunitLevelVars and session/TestSystemComplete.al:
// System_ClearAll_DoesNotThrow pin that the call works. That it resets the object's global variables
// and leaves locals alone is the documented meaning (docs/probes-needed.md).

#[test]
fn clear_all_resets_the_global_variables_and_not_the_locals() {
    let src = cu("var G: Integer; T: Text; L: List of [Integer];
         procedure F(): Text var Loc: Integer; begin
            G := 5; T := 'x'; L.Add(1); Loc := 7;
            ClearAll();
            exit(Format(G) + '/' + T + '/' + Format(L.Count()) + '/' + Format(Loc)); end;");
    assert_eq!(text(run(&src, "F", vec![])), "0//0/7");
}

#[test]
fn overloads_that_differ_only_in_the_table_of_a_record_parameter_pick_by_table() {
    // Base Application: Price List Line has CopyTo(var Price Asset) and CopyTo(var Price Source).
    // Microsoft's Tests-ERM (Price List Line UT.T029_ValidateAmountType) relies on the second one.
    let src = "
        table 50960 \"OvA\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        table 50961 \"OvB\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C {
            procedure Fill(var R: Record \"OvA\") begin R.Id := 1; end;
            procedure Fill(var R: Record \"OvB\") begin R.Id := 2; end;
            procedure F(): Text var A: Record \"OvA\"; B: Record \"OvB\"; begin
                Fill(A); Fill(B); exit(Format(A.Id) + Format(B.Id)); end;
        }";
    assert_eq!(text(run(src, "F", vec![])), "12");
}

// codeunit/PWCodeCoverageRows.al:DumpRows (BC 27.5): StrSubstNo of an Option field with an OptionCaption reads the
// caption, as Format does ("Non Applicable", not the member NonApplicable).
#[test]
fn strsubstno_of_an_option_field_reads_its_caption() {
    let src = "table 50300 T { fields { field(1; K; Code[10]) { } field(2; St; Option) { OptionMembers = Covered,NonApplicable; OptionCaption = 'Covered,Non Applicable'; } } keys { key(PK; K) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record T; begin R.St := R.St::NonApplicable;
            exit(Format(R.St) + '|' + StrSubstNo('%1/%2', R.K, R.St)); end; }";
    assert_eq!(text(run(src, "F", vec![])), "Non Applicable|/Non Applicable");
}

// types/ProbeTypeFormsAndFilters.al:Enum_Evaluate_Forms (BC 27.5): Evaluate into an enum reads a member name in any
// case, a caption, or an ordinal (also one no member has); format 9 reads names too; anything else is rejected.
#[test]
fn evaluate_into_an_enum_reads_names_captions_and_ordinals() {
    let src = "enum 50400 \"Kind\" { Extensible = true; value(0; None) { Caption = 'No Kind'; } value(1; Alpha) { } value(5; \"Beta Gamma\") { } }
        codeunit 50100 C { procedure F(): Text var K: Enum Kind; S: Text; begin
            S += Format(Evaluate(K, 'Beta Gamma')) + Format(K, 0, 2) + ',';
            K := K::None; S += Format(Evaluate(K, 'beta gamma')) + Format(K, 0, 2) + ',';
            K := K::Alpha; S += Format(Evaluate(K, 'No Kind')) + ',';
            K := K::None; S += Format(Evaluate(K, '5')) + Format(K, 0, 2) + ',';
            K := K::None; S += Format(Evaluate(K, '5', 9)) + Format(Evaluate(K, 'Alpha', 9)) + ',';
            S += Format(Evaluate(K, 'Nope')) + Format(Evaluate(K, '7')) + Format(K, 0, 2); exit(S); end; }";
    assert_eq!(text(run(src, "F", vec![])), "Yes5,Yes5,Yes,Yes5,YesYes,NoYes7");
}

#[test]
fn a_constant_gives_the_same_value_every_time_it_is_evaluated() {
    // A Decimal keeps its scale, a text is the text, an Option member is the member (constants are built once per session).
    let src = cu(
        "procedure D(): Decimal var i: Integer; d: Decimal; begin for i := 1 to 3 do d := 1.50; exit(d); end;
        procedure T(): Text var i: Integer; t: Text; begin for i := 1 to 3 do t := t + 'ab'; exit(t); end;
        procedure N(): Integer var i: Integer; n: Integer; begin for i := 1 to 4 do n += 2; exit(n); end;",
    );
    match run(&src, "D", vec![]) {
        Ok(Value::Decimal(d)) => assert_eq!((d.to_string(), d.scale()), ("1.50".to_string(), 2)),
        other => panic!("{other:?}"),
    }
    assert_eq!(text(run(&src, "T", vec![])), "ababab");
    assert_eq!(int(run(&src, "N", vec![])), 8);
}

#[test]
fn a_for_loop_reads_its_counter_back_and_fails_on_an_end_beyond_integer() {
    let src = cu("procedure Moved(): Integer var i: Integer; n: Integer; begin for i := 1 to 10 do begin n += 1; i += 2; end; exit(n); end;
        procedure Down(): Integer var i: Integer; n: Integer; begin for i := 5 downto 1 do n := n * 10 + i; exit(n); end;
        procedure Wide(): Integer var i: Integer; n: Integer; b: BigInteger; begin b := 2147483647; b += 1; for i := 2147483646 to b do n += 1; exit(n); end;");
    // The body moves the counter: 1 -> 3 (+1) = 4, 7, 10, 13.
    assert_eq!(int(run(&src, "Moved", vec![])), 4);
    assert_eq!(int(run(&src, "Down", vec![])), 54321);
    // A BigInteger end beyond Integer raises the conversion error before the first round
    // (misc/lt-system-functions/ControlFlow.al:779-830, `end max+1`).
    match run(&src, "Wide", vec![]) {
        Err(Signal::Error(e)) => assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "Overflow under conversion of System.Int64 value 2147483648 to System.Int32.",
                "ConversionCast"
            )
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn foreach_runs_over_a_json_array_variable() {
    let src = cu(
        "procedure F(): Integer var A: JsonArray; T: JsonToken; N: Integer; begin A.Add(1); A.Add(2); A.Add(3); foreach T in A do N += T.AsValue().AsInteger(); exit(N); end;",
    );
    assert_eq!(int(run(&src, "F", vec![])), 6);
}

#[test]
fn foreach_runs_over_the_result_of_a_builtin_method() {
    let src = cu(
        "procedure F(): Integer var D: XmlDocument; E: XmlElement; N: XmlNode; C: Integer; begin XmlDocument.ReadFrom('<a><b/><c/><d/></a>', D); D.GetRoot(E); foreach N in E.GetChildNodes() do C += 1; exit(C); end;",
    );
    assert_eq!(int(run(&src, "F", vec![])), 3);
}

/// platform/lt-platform-members/PlTRowVersionValue.al:MinimumActiveRowVersion_NoWrite_Committed: without writes it is
/// LastUsedRowVersion() + 1. After a write the value is not pinned and the call is refused.
#[test]
fn minimum_active_row_version_is_last_used_plus_one_without_writes() {
    let src = cu("procedure F(): Boolean begin exit(MinimumActiveRowVersion() = LastUsedRowVersion() + 1); end;");
    assert!(matches!(run(&src, "F", vec![]), Ok(Value::Boolean(true))));
}
