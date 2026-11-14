//! A Variant assigned from another Variant that holds a temporary record (al-runner issue 8070). The probe's cells (corpus
//! PR 1406, VariantFromVariantTempRecord.al, codeunit 96471) share the table under the assumption
//! `variant-from-variant-temp-record-keeps-rows`; the other shapes of a temporary record going from a Variant into a Variant
//! keep main's by-value copy, whose field values read as on main and whose rows refuse; a record put into a Variant from a
//! record variable keeps main's by-value copy (recordref-fieldref/recordref/PWRecordMisc.al:234-250).

use al_testkit::text;

const SRC: &str = r#"
table 69890 "VVT Line"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Amount; Decimal) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }

    procedure CopyInTable(P: Variant): Integer
    var
        V: Variant;
        RecRef: RecordRef;
    begin
        V := P;
        RecRef.GetTable(V);
        exit(RecRef.Count());
    end;
}
codeunit 69891 Lines
{
    var
        TempG: Record "VVT Line" temporary;

    procedure BuildLines(): Variant
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Amount := 10;
        TempLine.Insert();
        TempLine."Entry No." := 2;
        TempLine.Amount := 5;
        TempLine.Insert();
        exit(TempLine);
    end;

    procedure Wrap(var T: Record "VVT Line" temporary): Variant
    begin
        exit(T);
    end;

    procedure Echo(P: Variant): Variant
    begin
        exit(P);
    end;

    procedure BuildGlobal(): Variant
    begin
        TempG."Entry No." := 1;
        TempG.Insert();
        exit(TempG);
    end;

    procedure BuildWithReadIsolation(): Variant
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine.ReadIsolation := IsolationLevel::UpdLock;
        exit(TempLine);
    end;

    procedure BuildWithLoadFields(): Variant
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine.SetLoadFields(Amount);
        exit(TempLine);
    end;

    procedure InsertThrough(L: Variant)
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(L);
        RecRef.Field(1).Value := 9;
        RecRef.Insert();
    end;

    procedure BuildStored(): Variant
    var
        Line: Record "VVT Line";
    begin
        Line.DeleteAll();
        Line."Entry No." := 7;
        Line.Insert();
        exit(Line);
    end;

    procedure BuildFiltered(): Variant
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine."Entry No." := 2;
        TempLine.Insert();
        TempLine.SetRange("Entry No.", 2);
        exit(TempLine);
    end;

    procedure InsertNo(L: Variant; No: Integer)
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(L);
        RecRef.Field(1).Value := No;
        RecRef.Insert();
    end;

    procedure Filters(L: Variant): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(L);
        exit(RecRef.GetFilters());
    end;

    procedure CountRows(L: Variant): Integer
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(L);
        exit(RecRef.Count());
    end;

    procedure SumThroughEvent(L: Variant) Total: Decimal
    var
        IsHandled: Boolean;
    begin
        OnSumLines(L, Total, IsHandled);
        if not IsHandled then
            Error('OnSumLines was not handled.');
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSumLines(var TempLines: Record "VVT Line" temporary; var Total: Decimal; var IsHandled: Boolean)
    begin
    end;
}
codeunit 69892 Subscriber
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Lines, OnSumLines, '', false, false)]
    local procedure SumLines(var TempLines: Record "VVT Line" temporary; var Total: Decimal; var IsHandled: Boolean)
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine.Copy(TempLines, true);
        if TempLine.FindSet() then
            repeat
                Total += TempLine.Amount;
            until TempLine.Next() = 0;
        IsHandled := true;
    end;
}
codeunit 69894 Storage
{
    var
        Values: array[10] of Variant;
        N: Integer;

    procedure Enqueue(V: Variant)
    begin
        N += 1;
        Values[N] := V;
    end;

    procedure PeekCount(): Integer
    var
        V: Variant;
        RecRef: RecordRef;
    begin
        V := Values[N];
        RecRef.GetTable(V);
        exit(RecRef.Count());
    end;

    procedure ElementCount(): Integer
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(Values[N]);
        exit(RecRef.Count());
    end;

    procedure Dequeue(var V: Variant)
    begin
        V := Values[N];
        N -= 1;
    end;
}
interface "VVT Source"
{
    procedure Get(): Variant;
}
codeunit 69895 "VVT Impl" implements "VVT Source"
{
    procedure Get(): Variant
    var
        L: Codeunit Lines;
    begin
        exit(L.BuildLines());
    end;
}
codeunit 69896 "Not A Test"
{
    var
        L: Codeunit Lines;

    procedure CopyAndCount(): Integer
    var
        V: Variant;
    begin
        V := L.BuildLines();
        exit(L.CountRows(V));
    end;
}
codeunit 69893 T
{
    Subtype = Test;

    var
        L: Codeunit Lines;
        G: Variant;
        GI: Variant;

    // C1_ResultIntoLocalVariant, the reporter's ReturnValueStoredInVariantVariable.
    procedure ReturnValueStored(): Text
    var
        TheLines: Variant;
    begin
        TheLines := L.BuildLines();
        exit(Format(L.CountRows(TheLines)));
    end;

    procedure ReturnValueStoredThroughEvent(): Text
    var
        TheLines: Variant;
    begin
        TheLines := L.BuildLines();
        exit(Format(L.SumThroughEvent(TheLines)));
    end;

    // C1 in a [Test] method: the count comes back as the error text.
    [Test]
    procedure InTestMethod()
    var
        TheLines: Variant;
    begin
        TheLines := L.BuildLines();
        Error(Format(L.CountRows(TheLines)));
    end;

    // C2_ByValueParameterIntoLocalVariant, the reporter's VariantParameterCopiedToVariantVariable.
    procedure ParameterCopied(): Text
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine."Entry No." := 2;
        TempLine.Insert();
        exit(CountParameterAndCopy(TempLine));
    end;

    local procedure CountParameterAndCopy(P: Variant): Text
    var
        V: Variant;
    begin
        V := P;
        exit(Format(L.CountRows(P)) + '/' + Format(L.CountRows(V)));
    end;

    // C3_VarParameterIntoLocalVariant.
    procedure FromVarParameter(): Text
    var
        W: Variant;
    begin
        W := L.BuildLines();
        exit(CopyVarParameter(W));
    end;

    local procedure CopyVarParameter(var P: Variant): Text
    var
        V: Variant;
    begin
        V := P;
        exit(Format(L.CountRows(V)));
    end;

    // C4_LocalVariantIntoLocalVariant.
    procedure FromLocalVariant(): Text
    var
        W: Variant;
        V: Variant;
    begin
        W := L.BuildLines();
        V := W;
        exit(Format(L.CountRows(V)) + '/' + Format(L.CountRows(W)));
    end;

    // C5_ResultIntoGlobalVariant.
    procedure IntoGlobal(): Text
    begin
        G := L.BuildLines();
        exit(Format(L.CountRows(G)));
    end;

    // C6_WriteThroughEitherSide: one table for both holders.
    procedure WriteThroughEitherSide(): Text
    var
        W: Variant;
        V: Variant;
    begin
        W := L.BuildLines();
        V := W;
        L.InsertNo(V, 3);
        L.InsertNo(W, 4);
        exit(Format(L.CountRows(W)) + '/' + Format(L.CountRows(V)));
    end;

    // C7_FilteredTemporaryRecord: the filter travels.
    procedure FilteredResult(): Text
    var
        V: Variant;
    begin
        V := L.BuildFiltered();
        exit(Format(L.CountRows(V)) + ' ' + L.Filters(V));
    end;

    // C9_TargetAlreadyHoldsAValue.
    procedure IntoAssignedVariant(): Text
    var
        V: Variant;
        I: Integer;
    begin
        I := 7;
        V := I;
        V := L.BuildLines();
        exit(Format(L.CountRows(V)));
    end;

    // The target holds nothing after Clear, as C1's.
    procedure ClearedTarget(): Text
    var
        V: Variant;
    begin
        V := 5;
        Clear(V);
        V := L.BuildLines();
        exit(Format(L.CountRows(V)));
    end;

    procedure SharedThenCallerReads(): Text
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        CountParameterAndCopy(TempLine);
        exit(Format(TempLine.Count()));
    end;

    procedure RecordIntoVariant(): Text
    var
        TempLine: Record "VVT Line" temporary;
        V: Variant;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        V := TempLine;
        exit(Format(L.CountRows(V)));
    end;

    // C8_StoredRecord.
    procedure StoredReturnValue(): Text
    var
        V: Variant;
    begin
        V := L.BuildStored();
        exit(Format(L.CountRows(V)));
    end;

    procedure IntegerReturnValue(): Text
    var
        V: Variant;
        W: Variant;
        I: Integer;
    begin
        I := 4;
        W := I;
        V := W;
        exit(Format(V));
    end;

    procedure FromLocalCodeunitVariable(): Text
    var
        Lc: Codeunit Lines;
        V: Variant;
    begin
        V := Lc.BuildLines();
        exit(Format(L.CountRows(V)));
    end;

    procedure InTableProcedure(): Text
    var
        TempLine: Record "VVT Line" temporary;
        Other: Record "VVT Line";
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        exit(Format(Other.CopyInTable(TempLine)));
    end;

    procedure InNormalCodeunit(): Text
    var
        N: Codeunit "Not A Test";
    begin
        exit(Format(N.CopyAndCount()));
    end;

    procedure FilteredParameter(): Text
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine."Entry No." := 2;
        TempLine.Insert();
        TempLine.SetRange("Entry No.", 2);
        exit(CopyParameter(TempLine));
    end;

    local procedure CopyParameter(P: Variant): Text
    var
        V: Variant;
    begin
        V := P;
        exit(Format(L.CountRows(V)));
    end;

    procedure FromGlobal(): Text
    var
        V: Variant;
    begin
        G := L.BuildLines();
        V := G;
        exit(Format(L.CountRows(V)));
    end;

    procedure IntoGlobalHoldingInteger(): Text
    var
        I: Integer;
    begin
        I := 7;
        GI := I;
        GI := L.BuildLines();
        exit(Format(L.CountRows(GI)));
    end;

    procedure FromWrap(): Text
    var
        TempLine: Record "VVT Line" temporary;
        V: Variant;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        V := L.Wrap(TempLine);
        exit(Format(L.CountRows(V)));
    end;

    procedure FromEcho(): Text
    var
        TempLine: Record "VVT Line" temporary;
        V: Variant;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        V := L.Echo(TempLine);
        exit(Format(L.CountRows(V)));
    end;

    procedure FromCalleeGlobal(): Text
    var
        V: Variant;
    begin
        V := L.BuildGlobal();
        exit(Format(L.CountRows(V)));
    end;

    procedure WithReadIsolation(): Text
    var
        V: Variant;
    begin
        V := L.BuildWithReadIsolation();
        exit(Format(L.CountRows(V)));
    end;

    procedure WithLoadFields(): Text
    var
        V: Variant;
    begin
        V := L.BuildWithLoadFields();
        exit(Format(L.CountRows(V)));
    end;

    procedure IntoArrayElement(): Text
    var
        A: array[2] of Variant;
    begin
        A[1] := L.BuildLines();
        exit(Format(L.CountRows(A[1])));
    end;

    procedure InTryFunction(): Text
    var
        Got: Text;
    begin
        if CopyInTry(Got) then;
        exit(Got);
    end;

    [TryFunction]
    local procedure CopyInTry(var Got: Text)
    var
        V: Variant;
    begin
        V := L.BuildLines();
        Got := Format(L.CountRows(V));
    end;

    procedure FromTernary(): Text
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        exit(CopyTernary(TempLine, true));
    end;

    local procedure CopyTernary(P: Variant; Flag: Boolean): Text
    var
        V: Variant;
    begin
        V := Flag ? P : P;
        exit(Format(L.CountRows(V)));
    end;

    procedure FromInterfaceMethod(): Text
    var
        Impl: Codeunit "VVT Impl";
        S: Interface "VVT Source";
        V: Variant;
    begin
        S := Impl;
        V := S.Get();
        exit(Format(L.CountRows(V)));
    end;

    procedure FieldsOfArrayElement(): Text
    var
        A: array[2] of Variant;
        TempLine: Record "VVT Line" temporary;
    begin
        A[1] := L.BuildLines();
        TempLine := A[1];
        exit(Format(TempLine."Entry No.") + '/' + Format(TempLine.Amount));
    end;

    procedure EnqueueDequeueField(): Text
    var
        S: Codeunit Storage;
        TempLine: Record "VVT Line" temporary;
        Back: Record "VVT Line" temporary;
        V: Variant;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        TempLine."Entry No." := 2;
        TempLine.Insert();
        S.Enqueue(TempLine);
        S.Dequeue(V);
        Back := V;
        exit(Format(Back."Entry No."));
    end;

    procedure WriteToUnmeasuredCopy(): Text
    var
        A: array[2] of Variant;
    begin
        A[1] := L.BuildLines();
        L.InsertThrough(A[1]);
        exit('written');
    end;

    procedure ElementIntoCalleeLocal(): Text
    var
        S: Codeunit Storage;
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        S.Enqueue(TempLine);
        exit(Format(S.PeekCount()));
    end;

    procedure ElementRead(): Text
    var
        S: Codeunit Storage;
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        S.Enqueue(TempLine);
        exit(Format(S.ElementCount()));
    end;

    procedure EnqueueDequeueRows(): Text
    var
        S: Codeunit Storage;
        TempLine: Record "VVT Line" temporary;
        V: Variant;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        S.Enqueue(TempLine);
        S.Dequeue(V);
        exit(Format(L.CountRows(V)));
    end;
}
"#;

/// Runs `routine` of codeunit `T` and returns its text and whether it marked the assumption of the issue.
fn run(routine: &str) -> (String, bool) {
    al_builtins::assumptions::clear_hits();
    let got = text(al_testkit::run_in(SRC, "T", routine));
    (got, marked())
}

fn marked() -> bool {
    al_builtins::assumptions::take_hits()
        .iter()
        .any(|a| a.info().id == "variant-from-variant-temp-record-keeps-rows")
}

fn refused(routine: &str) -> String {
    let r = al_testkit::run_in(SRC, "T", routine);
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

/// The reporter's three failing tests (al-runner issue 8070; real Business Central passes all of them), the probe's C1
/// and C2.
#[test]
fn the_reporters_shapes_keep_the_temporary_rows() {
    assert_eq!(run("ReturnValueStored"), ("2".into(), true));
    assert_eq!(run("ReturnValueStoredThroughEvent"), ("15".into(), true));
    assert_eq!(run("ParameterCopied"), ("2/2".into(), true));
}

/// The probe's other cells (corpus PR 1406, VariantFromVariantTempRecord.al, codeunit 96471): a var parameter (C3), a
/// local (C4), a global target (C5), a filtered record (C7), a target holding an Integer (C9); C1 in a `[Test]` method.
#[test]
fn the_probes_cells_keep_the_temporary_rows() {
    assert_eq!(run("FromVarParameter"), ("2".into(), true));
    assert_eq!(run("FromLocalVariant"), ("2/2".into(), true));
    assert_eq!(run("IntoGlobal"), ("2".into(), true));
    assert_eq!(run("FilteredResult"), ("1 Entry No.: 2".into(), true));
    assert_eq!(run("IntoAssignedVariant"), ("2".into(), true));
    assert_eq!(run("ClearedTarget"), ("2".into(), true));
    al_builtins::assumptions::clear_hits();
    match al_testkit::run_in(SRC, "T", "InTestMethod") {
        Err(al_builtins::native::Signal::Error(e)) => assert_eq!(e.message, "2"),
        other => panic!("InTestMethod: {other:?}"),
    }
    assert!(marked());
}

/// C6: one table for both holders, a write through either shows through the other; the caller's record reads it too.
#[test]
fn both_holders_share_one_table() {
    assert_eq!(run("WriteThroughEitherSide"), ("4/4".into(), true));
    assert_eq!(run("SharedThenCallerReads"), ("1".into(), true));
}

/// Main's results: a record variable into a Variant is a copy over an empty table (PWRecordMisc.al:234-250), a stored
/// record keeps reading the stored rows (C8), a value that is no record is copied.
#[test]
fn shapes_outside_the_assumption_keep_mains_result() {
    assert_eq!(run("RecordIntoVariant"), ("0".into(), false));
    assert_eq!(run("StoredReturnValue"), ("1".into(), false));
    assert_eq!(run("IntegerReturnValue"), ("4".into(), false));
}

const ROWS: &str = "whether its table holds the source's rows on real Business Central is not measured";

/// A temporary record going from a Variant into a Variant in a shape the probe did not measure gets main's by-value copy:
/// a read of its rows or a write refuses (main counted 0 rows there).
#[test]
fn unmeasured_shapes_of_a_temporary_record_refuse_the_rows() {
    for routine in [
        "FromLocalCodeunitVariable",
        "InTableProcedure",
        "InNormalCodeunit",
        "FilteredParameter",
        "FromGlobal",
        "IntoGlobalHoldingInteger",
        "FromWrap",
        "FromEcho",
        "FromCalleeGlobal",
        "WithReadIsolation",
        "WithLoadFields",
        "IntoArrayElement",
        "FromTernary",
        "FromInterfaceMethod",
        "InTryFunction",
        "EnqueueDequeueRows",
        "ElementIntoCalleeLocal",
        "ElementRead",
        "WriteToUnmeasuredCopy",
    ] {
        let why = refused(routine);
        assert!(why.contains(ROWS) && why.contains("issue 8070"), "{routine}: {why}");
    }
}

/// Main's results there: the field values of the copy (the source's current row) read as on main, also after the
/// Enqueue and Dequeue moves of a library that keeps Variants in an array.
#[test]
fn field_values_of_an_unmeasured_copy_read_as_on_main() {
    assert_eq!(run("FieldsOfArrayElement"), ("2/5".into(), false));
    assert_eq!(run("EnqueueDequeueField"), ("2".into(), false));
}
