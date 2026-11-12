//! A ListPart with page triggers inside a Document, whose page extension's OnAfterValidate reads xRec: the base trigger's xRec
//! is pinned only for a part whose page triggers are exactly OnAfterGetRecord, OnNewRecord and OnAfterGetCurrRecord
//! (pages/currpage/XRecPartWithTriggersSaveRecord.al:363-506). A part with only some of them is not measured and stays refused,
//! as before that pin (review of runner PR 7800).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn src(part_triggers: &str) -> String {
    format!(
        r#"
table 50400 "VX Head" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 50401 "VX Line" {{ fields {{ field(1; "Doc No."; Code[20]) {{ }} field(2; "Line No."; Integer) {{ }} field(3; Qty; Integer) {{ }} }} keys {{ key(PK; "Doc No.", "Line No.") {{ Clustered = true; }} }} }}
codeunit 50402 "VX Log" {{ SingleInstance = true; var Log: Text; procedure Add(V: Text) begin Log += V + ';'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }}
page 50403 "VX Lines"
{{
    PageType = ListPart; SourceTable = "VX Line"; AutoSplitKey = true; DelayedInsert = true; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ repeater(Rows) {{
        field("Line No."; Rec."Line No.") {{ ApplicationArea = All; Editable = false; }}
        field(Qty; Rec.Qty) {{ ApplicationArea = All; trigger OnValidate() begin Log.Add('base x=' + Format(xRec.Qty)); CurrPage.SaveRecord(); end; }} }} }} }}
    var Log: Codeunit "VX Log";
    {part_triggers}
}}
page 50404 "VX Doc"
{{
    PageType = Document; SourceTable = "VX Head"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ field("No."; Rec."No.") {{ ApplicationArea = All; }} part(Lines; "VX Lines") {{ ApplicationArea = All; SubPageLink = "Doc No." = field("No."); }} }} }}
}}
pageextension 50405 "VX Lines Ext" extends "VX Lines"
{{
    layout {{ modify(Qty) {{ trigger OnAfterValidate() begin Log.Add('after x=' + Format(xRec.Qty)); end; }} }}
    var Log: Codeunit "VX Log";
}}
codeunit 50406 "VX Tests"
{{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure StoredLineQty()
    var Head: Record "VX Head"; Line: Record "VX Line"; Doc: TestPage "VX Doc"; Log: Codeunit "VX Log";
    begin
        Head.DeleteAll(); Line.DeleteAll();
        Head."No." := 'H1'; Head.Insert();
        Line."Doc No." := 'H1'; Line."Line No." := 10000; Line.Qty := 1; Line.Insert();
        Doc.OpenEdit(); Doc.GoToRecord(Head);
        Log.Take();
        Doc.Lines.First(); Doc.Lines.Qty.SetValue(5);
        Doc.Close();
    end;
}}
"#
    )
}

const AGR: &str = "trigger OnAfterGetRecord() begin Log.Add('agr'); end;";
const NEW: &str = "trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;";
const AGCR: &str = "trigger OnAfterGetCurrRecord() begin Log.Add('agcr'); end;";

#[test]
fn a_part_with_only_some_of_the_three_triggers_is_refused() {
    for triggers in [AGR.to_string(), format!("{AGR} {AGCR}"), format!("{NEW} {AGCR}")] {
        let o = outcomes_in(&src(&triggers), "VX Tests");
        assert!(matches!(o.as_slice(), [(_, Outcome::NotRun(_))]), "{triggers}: {o:?}");
    }
    // All three: the measured shape runs.
    let o = outcomes_in(&src(&format!("{AGR} {NEW} {AGCR}")), "VX Tests");
    assert!(matches!(o.as_slice(), [(_, Outcome::Pass)]), "{o:?}");
}
