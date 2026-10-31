//! CurrPage.Update(false) from an action on a list opened for view whose current record is the draft row New() started
//! (pages/lt-sc-update-empty-row/ScUpdateEmptyRow.al:215-241, codeunit 91255 OpenView_NewDraftRow_ActionUpdate), and
//! Update(true) or no Update on that row, a draft started on another row, and a list over an empty table
//! (pages/lt-sc-update-empty-row/Y6UpdateDraftRow.al:158-342, codeunit 91309, corpus master).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62750 "UD Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 62751 "UD Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ' ; '; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
}
page 62752 "UD List"
{
    PageType = List; SourceTable = "UD Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } } } }
    actions { area(Processing) {
        action(UpdateFalse) { trigger OnAction() begin Log.Add('before id=' + Format(Rec.Id)); CurrPage.Update(false); Log.Add('after id=' + Format(Rec.Id)); end; }
        action(UpdateTrue) { trigger OnAction() begin Log.Add('before id=' + Format(Rec.Id)); CurrPage.Update(true); Log.Add('after id=' + Format(Rec.Id)); end; }
        action(NoUpdate) { trigger OnAction() begin Log.Add('act id=' + Format(Rec.Id)); end; }
    } }
    var Log: Codeunit "UD Log";
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr id=' + Format(Rec.Id) + ' count=' + Format(Rec.Count())); end;
}
"#;

fn run(body: &str) -> Vec<(String, Outcome)> {
    run_on(2, body)
}

fn run_on(rows: usize, body: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "{OBJECTS}\ncodeunit 62790 T {{ Subtype = Test;
        var Log: Codeunit \"UD Log\";
        procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
        procedure Seed(N: Integer) var Row: Record \"UD Row\"; i: Integer; begin Row.DeleteAll(); for i := 1 to N do begin Row.Id := i; Row.Insert(); end; Log.Take(); end;
        [Test] procedure Run() var TP: TestPage \"UD List\"; Row: Record \"UD Row\"; begin Seed({rows}); {body} end;
        }}"
    );
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn update_false_on_the_untouched_draft_row_goes_back_to_the_first_row() {
    let o = run("TP.OpenView(); TP.New(); TP.UpdateFalse.Invoke(); TP.Close();
         Check('agcr id=1 count=2 ; agcr id=0 count=2 ; before id=0 ; after id=0 ; agcr id=1 count=2', Log.Take());
         Check('2', Format(Row.Count()));");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Y6UpdateDraftRow.al:158-198: Update(true), and an action without Update, behave as Update(false).
#[test]
fn update_true_and_no_update_on_the_draft_row_go_back_to_the_first_row() {
    for (action, body) in [("UpdateTrue", "before id=0 ; after id=0"), ("NoUpdate", "act id=0")] {
        let o = run(&format!(
            "TP.OpenView(); TP.New(); TP.{action}.Invoke(); Check('1', TP.Id.Value()); TP.Close();
             Check('agcr id=1 count=2 ; agcr id=0 count=2 ; {body} ; agcr id=1 count=2', Log.Take());
             Check('2', Format(Row.Count()));"
        ));
        assert_eq!(o[0].1, Outcome::Pass, "{action}: {o:?}");
    }
}

// Y6UpdateDraftRow.al:200-264: Update(false) goes back to the row New() started from, Update(true) from the last row to the
// first row.
#[test]
fn update_false_goes_back_to_the_row_new_started_from_and_update_true_from_the_last_row_to_the_first() {
    for (nav, action, id) in [
        ("TP.Last();", "UpdateFalse", "3"),
        ("TP.Next();", "UpdateFalse", "2"),
        ("TP.Last();", "UpdateTrue", "1"),
    ] {
        let o = run_on(
            3,
            &format!(
                "TP.OpenView(); {nav} TP.New(); TP.{action}.Invoke(); Check('{id}', TP.Id.Value()); TP.Close();
                 Check('3', Format(Row.Count()));"
            ),
        );
        assert_eq!(o[0].1, Outcome::Pass, "{nav} {action}: {o:?}");
    }
}

// Y6UpdateDraftRow.al:266-342: over an empty table OpenView reads no row, and an action just runs its body.
#[test]
fn an_action_on_a_list_over_an_empty_table_runs_only_its_body() {
    for (action, body) in [
        ("UpdateFalse", "before id=0 ; after id=0"),
        ("UpdateTrue", "before id=0 ; after id=0"),
        ("NoUpdate", "act id=0"),
    ] {
        let o = run_on(
            0,
            &format!(
                "TP.OpenView(); TP.{action}.Invoke(); Check('', TP.Id.Value()); TP.Close(); Check('{body}', Log.Take());"
            ),
        );
        assert_eq!(o[0].1, Outcome::Pass, "{action}: {o:?}");
    }
    let o = run_on(
        0,
        "TP.OpenView(); TP.New(); TP.UpdateTrue.Invoke(); Check('', TP.Id.Value()); TP.Close();
         Check('agcr id=0 count=0 ; before id=0 ; after id=0', Log.Take()); Check('0', Format(Row.Count()));",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Not pinned: Update(true) on a draft started on a middle row, and an action without Update on a draft started on another
// row than the first.
#[test]
fn update_true_from_a_middle_row_and_no_update_from_another_row_are_refused() {
    for (nav, action) in [
        ("TP.Next();", "UpdateTrue"),
        ("TP.Last();", "NoUpdate"),
        ("TP.Next();", "NoUpdate"),
    ] {
        let o = run_on(
            3,
            &format!("TP.OpenView(); {nav} TP.New(); TP.{action}.Invoke(); TP.Close();"),
        );
        assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{nav} {action}: {o:?}");
    }
}
