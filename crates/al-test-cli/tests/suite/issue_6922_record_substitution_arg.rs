//! A Record as a `%1` argument of `Error`, `Message` and `StrSubstNo` is written as `Format(Rec)` writes it: every field in
//! field-number order, joined by a TAB (al-runner issue 6922, the reporter's repro and the texts they measured on BC 29.0).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50110 "Msg Probe Line"
{
    fields
    {
        field(1; "Invoice no."; Code[10]) { }
        field(2; "Line no."; Integer) { }
        field(3; Description; Text[30]) { }
        field(4; Amount; Decimal) { }
        field(5; Posted; Boolean) { }
    }
    keys { key(PK; "Invoice no.", "Line no.") { Clustered = true; } }
}

table 50111 "Msg Probe Other"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 50110 "Msg Probe Tests"
{
    Subtype = Test;

    var
        MessageText: Text;

    local procedure Tab(): Text
    var
        T: Text[1];
    begin
        T[1] := 9;
        exit(T);
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('got [%1], expected [%2]', Actual, Expected);
    end;

    local procedure FillLine(var Line: Record "Msg Probe Line")
    begin
        Line."Invoice no." := 'INV1';
        Line."Line no." := 10;
        Line.Description := 'Gear';
        Line.Amount := 12.5;
        Line.Posted := true;
    end;

    local procedure Filled(): Text
    begin
        exit('INV1' + Tab() + '10' + Tab() + 'Gear' + Tab() + '12.5' + Tab() + 'Yes');
    end;

    local procedure Empty(): Text
    begin
        exit(Tab() + '0' + Tab() + Tab() + '0' + Tab() + 'No');
    end;

    [Test]
    [HandlerFunctions('MessageHandler')]
    procedure Message_WithAFilledRecord()
    var
        Line: Record "Msg Probe Line";
    begin
        FillLine(Line);
        Message('Line %1 is wrong', Line);
        Check(MessageText, 'Line ' + Filled() + ' is wrong');
    end;

    [Test]
    [HandlerFunctions('MessageHandler')]
    procedure Message_WithAnEmptyRecord()
    var
        Line: Record "Msg Probe Line";
    begin
        Message('Line %1 is wrong', Line);
        Check(MessageText, 'Line ' + Empty() + ' is wrong');
    end;

    [Test]
    [HandlerFunctions('MessageHandler')]
    procedure Message_WithARecordOfAnotherTable()
    var
        Other: Record "Msg Probe Other";
    begin
        Message('Line %1 is wrong', Other);
        Check(MessageText, 'Line ' + Tab() + ' is wrong');
    end;

    [Test]
    procedure Error_WithAFilledRecord()
    var
        Line: Record "Msg Probe Line";
    begin
        FillLine(Line);
        asserterror Error('Line %1 is wrong', Line);
        Check(GetLastErrorText(), 'Line ' + Filled() + ' is wrong');
    end;

    [Test]
    procedure Error_WithAnEmptyRecord()
    var
        Line: Record "Msg Probe Line";
    begin
        asserterror Error('Line %1 is wrong', Line);
        Check(GetLastErrorText(), 'Line ' + Empty() + ' is wrong');
    end;

    [Test]
    procedure StrSubstNo_WithAFilledRecord()
    var
        Line: Record "Msg Probe Line";
    begin
        FillLine(Line);
        Check(StrSubstNo('Line %1 is wrong', Line), 'Line ' + Filled() + ' is wrong');
    end;

    [MessageHandler]
    procedure MessageHandler(Msg: Text[1024])
    begin
        MessageText := Msg;
    end;
}
"#;

#[test]
fn a_record_as_a_substitution_argument_is_written_as_format_writes_it() {
    let outcomes = outcomes_in(SRC, "Msg Probe Tests");
    let not_passing: Vec<_> = outcomes
        .iter()
        .filter(|(_, o)| *o != al_testrun::Outcome::Pass)
        .collect();
    assert_eq!(outcomes.len(), 6);
    assert!(not_passing.is_empty(), "{not_passing:#?}");
}

/// format-evaluate/record-format/RecordAsSubstitutionArg.al (corpus branch al-runtime-rs-staging-r6918-22): the primary key
/// fields come first (RecordArg_KeyOnLastField_EmptyAndFilled), a RecordRef and a FieldRef are written as Format writes them
/// (RecordArg_RecordRefAndFieldRef), a #n field of StrSubstNo fills or stars (RecordArg_TwoRecordsAndHashField).
const PROBE: &str = r#"
table 50130 "KeyLast" { fields { field(1; Name; Text[30]) { } field(2; Qty; Integer) { } field(3; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50131 "Line" { fields { field(1; "Invoice no."; Code[10]) { } field(2; "Line no."; Integer) { } field(3; Description; Text[30]) { } field(4; Amount; Decimal) { } field(5; Posted; Boolean) { } } keys { key(PK; "Invoice no.", "Line no.") { } } }
table 50132 "Other" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; Code) { } } }
codeunit 50133 C
{
    local procedure Fill(var L: Record "Line")
    begin
        L."Invoice no." := 'INV1'; L."Line no." := 10; L.Description := 'Gear'; L.Amount := 12.5; L.Posted := true;
    end;
    procedure KeyLast(): Text
    var R: Record "KeyLast"; E: Record "KeyLast";
    begin
        R.Name := 'Last'; R.Qty := 3; R.Id := 7;
        exit(Format(R) + '|' + StrSubstNo('[%1]', R) + '|' + StrSubstNo('[%1]', E));
    end;
    procedure Refs(): Text
    var L: Record "Line"; RR: RecordRef; FR: FieldRef;
    begin
        Fill(L); RR.GetTable(L); FR := RR.Field(3);
        exit(StrSubstNo('[%1]', RR) + '|' + StrSubstNo('[%1]', FR));
    end;
    procedure Hash(): Text
    var L: Record "Line"; O: Record "Other";
    begin
        Fill(L); O.Code := 'C1';
        exit(StrSubstNo('[#1##########]', L) + StrSubstNo('[#1##########]', O) + StrSubstNo('%2 / %1', L, 5));
    end;
}
"#;

#[test]
fn the_probe_texts_of_a_record_argument() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "KeyLast")),
        "7\tLast\t3|[7\tLast\t3]|[0\t\t0]"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "Refs")),
        "[INV1\t10\tGear\t12.5\tYes]|[Gear]"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "Hash")),
        "[************][C1\t         ]5 / INV1\t10\tGear\t12.5\tYes"
    );
}
