//! xRec inside a table procedure that a field's OnValidate calls on Rec (`Rec.MyProc()`): the procedure sees the same
//! before-image the trigger sees (al-runner issue 7785, the reporter's repro). Pinned for `Rec.`, `this.` and the unqualified
//! call by record/validate/XRecTableProcCalls.al:186-251 (and record/TestXRecContracts.al:52, :255 for the unqualified call).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// The reporter's repro, unchanged.
const SRC: &str = r#"
table 50190 "R918 Probe"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Serial; Code[50])
        {
            trigger OnValidate()
            begin
                Rec."Old Serial" := xRec.Serial;          // in the trigger
                Rec.RecordOldSerialInProcedure();         // in a procedure the trigger calls
            end;
        }
        field(3; "Old Serial"; Code[50]) { }
        field(4; "Old Serial (procedure)"; Code[50]) { }
    }
    keys { key(PK; Id) { } }

    procedure RecordOldSerialInProcedure()
    begin
        Rec."Old Serial (procedure)" := xRec.Serial;
    end;
}

codeunit 50191 "R918 xRec Tests"
{
    Subtype = Test;

    [Test]
    procedure XRecInsideTableProcedureFromCode()
    var
        P: Record "R918 Probe";
    begin
        P.Id := 60;
        P.Serial := 'FIRST';
        P.Insert();
        P.Get(60);
        P.Validate(Serial, 'SECOND');
        P.Modify();
        P.Get(60);
        if P."Old Serial (procedure)" <> 'FIRST' then
            Error('xRec.Serial inside the procedure was "%1", in the trigger "%2", expected "FIRST"', P."Old Serial (procedure)", P."Old Serial");
    end;
}
"#;

/// The reporter's TestPage form ("the same holds when the field is set through a TestPage"), the shape of Microsoft's test:
/// OpenView, GoToRecord, SetValue, Close on a card over the table.
const PAGE: &str = r#"
page 50192 "R918 Card"
{
    PageType = Card;
    SourceTable = "R918 Probe";
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
            field(Serial; Rec.Serial) { }
        }
    }
}

codeunit 50193 "R918 Page Tests"
{
    Subtype = Test;

    [Test]
    procedure XRecInsideTableProcedureFromTestPage()
    var
        P: Record "R918 Probe";
        Card: TestPage "R918 Card";
    begin
        P.Id := 61;
        P.Serial := 'FIRST';
        P.Insert();
        Card.OpenView();
        Card.GoToRecord(P);
        Card.Serial.SetValue('SECOND');
        Card.Close();
        P.Get(61);
        if P."Old Serial (procedure)" <> 'FIRST' then
            Error('xRec.Serial inside the procedure was "%1", in the trigger "%2", expected "FIRST"', P."Old Serial (procedure)", P."Old Serial");
    end;
}
"#;

fn only(o: &[(String, Outcome)]) -> Outcome {
    assert_eq!(o.len(), 1, "{o:?}");
    o[0].1.clone()
}

#[test]
fn a_table_procedure_called_on_rec_from_onvalidate_sees_the_triggers_xrec() {
    assert_eq!(only(&outcomes_in(SRC, "R918 xRec Tests")), Outcome::Pass);
}

#[test]
fn the_same_holds_when_a_testpage_sets_the_field() {
    let src = format!("{SRC}\n{PAGE}");
    assert_eq!(only(&outcomes_in(&src, "R918 Page Tests")), Outcome::Pass);
}

/// Calls from triggers that are not the assumed shape: the unqualified call (pinned), and the neighbors no test measures yet
/// (OnModify calling `Rec.Proc()`, OnValidate calling the procedure on another variable, a codeunit that OnValidate gives Rec by
/// var calling `Rec.Proc()`). Each `Mode` value picks one.
const NEIGHBORS: &str = r#"
table 50194 "R918 Neighbor"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Mode; Integer)
        {
            trigger OnValidate()
            var
                Other: Record "R918 Neighbor";
                Helper: Codeunit "R918 Helper";
            begin
                case Mode of
                    1:
                        Seen := ReadXRec();
                    2:
                        begin
                            Other := Rec;
                            Seen := Other.ReadXRec();
                        end;
                    3:
                        Helper.CallOnRec(Rec);
                    4:
                        Seen := Rec.ReadXRec();
                    6:
                        Seen := this.ReadXRec();
                end;
            end;
        }
        field(3; Seen; Text[100]) { }
    }
    keys { key(PK; Id) { } }

    trigger OnModify()
    begin
        if Mode = 5 then
            Seen := Rec.ReadXRec();
    end;

    procedure ReadXRec(): Text
    begin
        exit(Format(xRec.Mode));
    end;
}

codeunit 50195 "R918 Helper"
{
    procedure CallOnRec(var R: Record "R918 Neighbor")
    begin
        R.Seen := R.ReadXRec();
    end;
}

codeunit 50196 "R918 Run"
{
    procedure Run(NewMode: Integer): Text
    var
        N: Record "R918 Neighbor";
    begin
        N.DeleteAll();
        N.Id := 1;
        N.Mode := 9;
        N.Insert();
        N.Get(1);
        if NewMode = 5 then begin
            N.Mode := 5;
            N.Modify(true);
        end else
            N.Validate(Mode, NewMode);
        exit(N.Seen);
    end;
}
"#;

fn seen_in(mode: i32) -> (String, Vec<al_builtins::assumptions::Assumption>) {
    al_builtins::assumptions::take_hits();
    let r = al_testkit::run_with(
        NEIGHBORS,
        "R918 Run",
        "Run",
        vec![al_builtins::value::Value::Integer(mode)],
    );
    let seen = match r {
        Ok((v, _)) => format!("{v:?}"),
        Err(e) => format!("{e:?}"),
    };
    (seen, al_builtins::assumptions::take_hits())
}

#[test]
fn a_call_on_rec_this_or_unqualified_from_the_field_onvalidate_shares_the_triggers_xrec_without_a_mark() {
    // Modes 4, 6 and 1: `Rec.ReadXRec()`, `this.ReadXRec()` and the unqualified call read the trigger's xRec (9), pinned.
    for mode in [4, 6, 1] {
        let (seen, hits) = seen_in(mode);
        assert!(seen.contains("\"9\""), "mode {mode}: {seen}");
        assert!(hits.is_empty(), "mode {mode}: {hits:?}");
    }
}
