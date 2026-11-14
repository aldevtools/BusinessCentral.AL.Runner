//! A Variant assigned from another Variant that holds a temporary record (al-runner issue 8070). The reporter's shapes keep
//! the rows under the assumption `variant-from-variant-temp-record-keeps-rows`; the other shapes of a temporary record
//! going from a Variant into a Variant are refused; a record put into a Variant from a record variable keeps main's
//! by-value copy (recordref-fieldref/recordref/PWRecordMisc.al:234-250).

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

/// The reporter's three failing tests (al-runner issue 8070; BC 28.5 passes all of them).
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

/// A temporary record going from a Variant into a Variant in a shape the reporter did not measure is refused.
#[test]
fn other_variant_to_variant_shapes_of_a_temporary_record_refuse() {
    for (routine, reason) in [
        ("FromLocalVariant", "the source is no by-value Variant parameter"),
        ("FromVarParameter", "the source is no by-value Variant parameter"),
        (
            "FromLocalCodeunitVariable",
            "the source is no by-value Variant parameter",
        ),
        ("InTableProcedure", "the assignment is not in a procedure of a codeunit"),
        ("IntoGlobal", "the target is no local variable"),
        (
            "IntoAssignedVariant",
            "the target is no declared local Variant that holds nothing yet",
        ),
        ("FilteredParameter", "the record has filters"),
    ] {
        let why = refused(routine);
        assert!(
            why.contains("a Variant holding a temporary record") && why.contains(reason),
            "{routine}: {why}"
        );
    }
}
