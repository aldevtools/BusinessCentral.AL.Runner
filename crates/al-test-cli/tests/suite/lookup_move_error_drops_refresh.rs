//! A host move that carried the refresh a TestField.Lookup left (al-runner issue 8503): Next or First whose own
//! OnAfterGetCurrRecord raises closes the TestPage and drops the refresh, so the reopened page reads once on open
//! (assumption `lookup-move-error-drops-refresh`, corpus pull request 1467, testpage/LookupLeftMoveStops.al:368
//! Next_MoveReadFails, :450 First_MoveReadFails).
//! Previous, Last and a Next from another row are not measured and refused. The host is the one of
//! `lookup_deferred_refresh_part_move.rs`.

use super::lookup_deferred_refresh_part_move::SRC;
use al_testkit::outcomes_in;

const TESTS: &str = r#"
codeunit 50111 "Repro Move Error Tests"
{
    Subtype = Test;
    var Log: Codeunit "Repro Log";
    [Test] [HandlerFunctions('PickSecond')]
    procedure NextMoveError()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); Commit(); Log.Take();
        Log.SetFailOn('R2');
        asserterror S.Next();
        Check('move', GetLastErrorText(), 'agcr failed on R2');
        Check('move log', Log.Take(), 'agcr R2');
        S.OpenEdit();
        Check('reopen log', Log.Take(), 'agcr R1');
        Check('reopened next', Format(S.Next()) + '|' + S."No.".Value(), 'Yes|R2');
        Check('reopened next log', Log.Take(), 'agcr R2');
        S.Close();
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure FirstMoveError()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.Last(); S.ResourceNo.Lookup(); Commit(); Log.Take();
        Log.SetFailOn('R1');
        asserterror S.First();
        Check('move', GetLastErrorText(), 'agcr failed on R1');
        Check('move log', Log.Take(), 'agcr R1');
        S.OpenEdit();
        Check('reopen log', Log.Take(), 'agcr R1');
        Check('reopened next', Format(S.Next()) + '|' + S."No.".Value(), 'Yes|R2');
        Check('reopened next log', Log.Take(), 'agcr R2');
        S.Close();
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure PreviousMoveError()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.Next(); S.ResourceNo.Lookup(); Commit(); Log.Take();
        Log.SetFailOn('R1');
        asserterror S.Previous();
        Check('move', GetLastErrorText(), 'agcr failed on R1');
        S.OpenEdit(); Error('reopen log: %1', Log.Take());
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure LastMoveError()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); Commit(); Log.Take();
        Log.SetFailOn('R3');
        asserterror S.Last();
        S.OpenEdit(); Error('reopen log: %1', Log.Take());
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure NextFromMiddleMoveError()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.Next(); S.ResourceNo.Lookup(); Commit(); Log.Take();
        Log.SetFailOn('R3');
        asserterror S.Next();
        S.OpenEdit(); Error('reopen log: %1', Log.Take());
    end;
    local procedure Check(What: Text; Actual: Text; Expected: Text)
    begin if Actual <> Expected then Error('%1: [%2], expected [%3]', What, Actual, Expected); end;
    [ModalPageHandler]
    procedure PickSecond(var ResourceList: TestPage "Repro Lookup Resource List")
    begin ResourceList.GoToKey('R2'); ResourceList.OK().Invoke(); end;
    local procedure Setup()
    var Setting: Record "Repro Lookup Setting"; Resource: Record "Repro Lookup Resource";
    begin
        Setting.DeleteAll(); Resource.DeleteAll();
        Insert(Resource, 'R1', 'G1'); Insert(Resource, 'R2', 'G1'); Insert(Resource, 'R3', 'G2');
    end;
    local procedure Insert(var Resource: Record "Repro Lookup Resource"; No: Code[20]; GroupCode: Code[20])
    begin Resource.Init(); Resource."No." := No; Resource."Group Code" := GroupCode; Resource.Insert(); end;
}
"#;

fn source() -> String {
    let failing = SRC
        .replace(
            r#"trigger OnAfterGetCurrRecord() begin Log.Add('agcr ' + Rec."No."); end;"#,
            r#"trigger OnAfterGetCurrRecord() begin Log.Add('agcr ' + Rec."No."); if (Log.FailOn() <> '') and (Log.FailOn() = Rec."No.") then begin Log.SetFailOn(''); Error('agcr failed on %1', Rec."No."); end; end;"#,
        )
        .replace(
            "    var Entries: Text;\n}",
            "    procedure SetFailOn(No: Code[20]) begin Fail := No; end;\n    procedure FailOn(): Code[20] begin exit(Fail); end;\n    var Entries: Text; Fail: Code[20];\n}",
        );
    assert!(
        failing.contains("SetFailOn") && failing.contains("Log.FailOn()"),
        "the host source changed"
    );
    failing + TESTS
}

#[test]
fn a_next_error_that_closes_the_page_drops_the_lookup_refresh() {
    let o = outcomes_in(&source(), "Repro Move Error Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("NextMoveError"), "Some(Pass)");
    // From the last row: LookupLeftMoveStops.al:450 First_MoveReadFails.
    assert_eq!(get("FirstMoveError"), "Some(Pass)");
    // Previous and Last, and Next from the second row, are not measured (main ran the refresh the Lookup left as the page
    // opened again: `agcr R1;agcr R1;agcr R1`).
    for refused in ["PreviousMoveError", "LastMoveError", "NextFromMiddleMoveError"] {
        let got = get(refused);
        assert!(
            got.contains("NotRun") && got.contains("outside the measured cells"),
            "{refused}: {got}"
        );
    }
}
