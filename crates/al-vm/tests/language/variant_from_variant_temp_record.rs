//! A Variant assigned from another Variant that holds a temporary record (al-runner issue 8070). The reporter's shapes keep
//! the rows under the assumption `variant-from-variant-temp-record-keeps-rows`, and a write to the shared table refuses; the
//! other shapes of a temporary record going from a Variant into a Variant keep main's by-value copy, whose field values read
//! as on main and whose rows refuse; a record put into a Variant from a record variable keeps main's by-value copy
//! (recordref-fieldref/recordref/PWRecordMisc.al:234-250).

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
codeunit 69893 T
{
    var
        L: Codeunit Lines;
        G: Variant;

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

    procedure FromLocalVariant(): Text
    var
        W: Variant;
        V: Variant;
    begin
        W := L.BuildLines();
        V := W;
        exit(Format(L.CountRows(V)));
    end;

    procedure IntoGlobal(): Text
    begin
        G := L.BuildLines();
        exit(Format(L.CountRows(G)));
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

    procedure IntoAssignedVariant(): Text
    var
        V: Variant;
        I: Integer;
    begin
        I := 1;
        V := I;
        V := L.BuildLines();
        exit(Format(L.CountRows(V)));
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

    procedure SharedThenWriteThroughCopy(): Text
    var
        TheLines: Variant;
    begin
        TheLines := L.BuildLines();
        L.InsertThrough(TheLines);
        exit('written');
    end;

    procedure SharedThenCallerInserts(): Text
    var
        TempLine: Record "VVT Line" temporary;
    begin
        TempLine."Entry No." := 1;
        TempLine.Insert();
        CountParameterAndCopy(TempLine);
        TempLine."Entry No." := 3;
        TempLine.Insert();
        exit('written');
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

    procedure ClearedTarget(): Text
    var
        V: Variant;
    begin
        V := 5;
        Clear(V);
        V := L.BuildLines();
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

    procedure FromLocalVariantField(): Text
    var
        V: Variant;
        TempLine: Record "VVT Line" temporary;
    begin
        G := L.BuildLines();
        V := G;
        TempLine := V;
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
    begin
        G := L.BuildLines();
        L.InsertThrough(G);
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
}
"#;

/// Runs `routine` of codeunit `T` and returns its text and whether it marked the assumption of the issue.
fn run(routine: &str) -> (String, bool) {
    al_builtins::assumptions::clear_hits();
    let got = text(al_testkit::run_in(SRC, "T", routine));
    let marked = al_builtins::assumptions::take_hits()
        .iter()
        .any(|a| a.info().id == "variant-from-variant-temp-record-keeps-rows");
    (got, marked)
}

fn refused(routine: &str) -> String {
    let r = al_testkit::run_in(SRC, "T", routine);
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

/// The reporter's three failing tests (al-runner issue 8070; real Business Central passes all of them).
#[test]
fn a_variant_from_a_returned_variant_keeps_the_temporary_rows() {
    assert_eq!(run("ReturnValueStored"), ("2".into(), true));
    assert_eq!(run("ReturnValueStoredThroughEvent"), ("15".into(), true));
}

#[test]
fn a_variant_from_a_variant_parameter_keeps_the_temporary_rows() {
    assert_eq!(run("ParameterCopied"), ("2/2".into(), true));
}

/// Main's results: a record variable into a Variant is a copy over an empty table (PWRecordMisc.al:234-250), a stored
/// record keeps reading the stored rows, a value that is no record is copied.
#[test]
fn shapes_outside_the_assumption_keep_mains_result() {
    assert_eq!(run("RecordIntoVariant"), ("0".into(), false));
    assert_eq!(run("StoredReturnValue"), ("1".into(), false));
    assert_eq!(run("IntegerReturnValue"), ("4".into(), false));
}

const ROWS: &str = "whether its table holds the source's rows on real Business Central is not measured";
const WRITES: &str = "whether the write shows through the other holder is not measured";

/// A temporary record going from a Variant into a Variant in a shape the reporter did not measure gets main's by-value
/// copy: a read of its rows refuses (main counted 0 rows there).
#[test]
fn other_variant_to_variant_shapes_of_a_temporary_record_refuse_the_rows() {
    for routine in [
        "FromLocalVariant",
        "FromVarParameter",
        "FromLocalCodeunitVariable",
        "InTableProcedure",
        "IntoGlobal",
        "IntoAssignedVariant",
        "FilteredParameter",
        "FromWrap",
        "FromEcho",
        "FromCalleeGlobal",
        "ClearedTarget",
        "WithReadIsolation",
        "WithLoadFields",
        "IntoArrayElement",
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
    assert_eq!(run("FromLocalVariantField"), ("2/5".into(), false));
    assert_eq!(run("EnqueueDequeueField"), ("2".into(), false));
}

/// In the reporter's shapes the table is shared: a write through either holder refuses, a read runs.
#[test]
fn a_write_to_the_shared_table_refuses() {
    assert!(refused("SharedThenWriteThroughCopy").contains(WRITES));
    assert!(refused("SharedThenCallerInserts").contains(WRITES));
    assert_eq!(run("SharedThenCallerReads"), ("1".into(), true));
}
