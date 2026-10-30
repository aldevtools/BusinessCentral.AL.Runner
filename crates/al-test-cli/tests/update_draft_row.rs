//! CurrPage.Update(false) from an action on a list opened for view whose current record is the draft row New() started
//! (pages/lt-sc-update-empty-row/ScUpdateEmptyRow.al:215-241, codeunit 91255 OpenView_NewDraftRow_ActionUpdate).

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
    let src = format!(
        "{OBJECTS}\ncodeunit 62790 T {{ Subtype = Test;
        var Log: Codeunit \"UD Log\";
        procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
        procedure Seed() var Row: Record \"UD Row\"; begin Row.DeleteAll(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert(); Log.Take(); end;
        [Test] procedure Run() var TP: TestPage \"UD List\"; Row: Record \"UD Row\"; begin Seed(); {body} end;
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

#[test]
fn update_true_on_the_draft_row_is_refused() {
    let o = run("TP.OpenView(); TP.New(); TP.UpdateTrue.Invoke(); TP.Close();");
    assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{o:?}");
}
