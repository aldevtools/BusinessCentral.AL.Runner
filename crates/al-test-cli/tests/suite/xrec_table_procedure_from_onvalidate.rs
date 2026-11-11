//! xRec inside a table procedure that a field's OnValidate calls on Rec (`Rec.MyProc()`): the procedure sees the same
//! before-image the trigger sees (al-runner issue 7785, the reporter's repro). No corpus test pins the `Rec.` call; Microsoft's
//! Subscription Billing test `Service Object Test.CheckChangeServiceObjectSN` needs it (assumption
//! `xrec-table-proc-from-field-validate`, a flagged pass). The unqualified call from OnValidate is pinned
//! (record/TestXRecContracts.al:52, :255 `internal xRec=A/stored`).

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

fn hits_of(mode: i32) -> (String, Vec<al_builtins::assumptions::Assumption>) {
    al_builtins::assumptions::clear_hits();
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
fn only_a_call_on_rec_from_the_tables_field_onvalidate_takes_the_assumption() {
    use al_builtins::assumptions::Assumption::XrecTableProcFromFieldValidate as A;
    // Mode 4: `Rec.ReadXRec()` in the field's OnValidate reads the trigger's xRec (9), on the assumption.
    let (seen, hits) = hits_of(4);
    assert!(seen.contains("\"9\""), "{seen}");
    assert!(hits.contains(&A), "{hits:?}");
    // Mode 6: `this.ReadXRec()` runs as the unqualified call does (main's behavior, untouched): the trigger's xRec, no mark.
    let (seen, hits) = hits_of(6);
    assert!(seen.contains("\"9\""), "{seen}");
    assert!(!hits.contains(&A), "{hits:?}");
    // Mode 1: the unqualified call, pinned (record/TestXRecContracts.al:52, :255): the trigger's xRec, no mark.
    let (seen, hits) = hits_of(1);
    assert!(seen.contains("\"9\""), "{seen}");
    assert!(!hits.contains(&A), "{hits:?}");
    // Modes 2, 3 and 5: another variable, a codeunit given Rec by var, OnModify. Not the assumed shape: no mark.
    for mode in [2, 3, 5] {
        let (seen, hits) = hits_of(mode);
        assert!(!hits.contains(&A), "mode {mode}: {seen} {hits:?}");
    }
}
