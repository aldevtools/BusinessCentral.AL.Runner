//! A Document host opened for view on no row, with a part: the part runs OnNewRecord once on the blank row. Whether and when
//! it runs OnAfterGetCurrRecord is pinned by corpus pages/parts/PartLineBeforeHostKey.al:1813
//! QH_OpenView_NoRow_TraceHost (`CP.OnAfterGetCurrRecord:/0` after `CP.OnNewRecord`) and the runtime runs it.

use al_test_cli::report::Outcome;

fn outcome(part_trigger: &str) -> Outcome {
    let src = format!(
        r#"
table 62910 "OV Header" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62911 "OV Line" {{ fields {{ field(1; "Document No."; Code[20]) {{ }} field(2; "Line No."; Integer) {{ }} }} keys {{ key(PK; "Document No.", "Line No.") {{ Clustered = true; }} }} }}
page 62912 "OV Lines" {{
    PageType = ListPart; SourceTable = "OV Line";
    layout {{ area(Content) {{ repeater(R) {{ field(L; Rec."Line No.") {{ }} }} }} }}
    {part_trigger}
}}
page 62913 "OV Doc" {{
    PageType = Document; SourceTable = "OV Header";
    layout {{ area(Content) {{ field(No; Rec."No.") {{ }} part(Lines; "OV Lines") {{ SubPageLink = "Document No." = field("No."); }} }} }}
}}
codeunit 62914 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: TestPage "OV Doc";
    begin
        P.OpenView();
        P.Close();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_part_with_on_after_get_curr_record_runs_it_once_on_the_blank_row_and_the_open_passes() {
    // Pinned: PartLineBeforeHostKey.al:1813 QH_OpenView_NoRow_TraceHost `CP.OnAfterGetCurrRecord:/0`.
    assert_eq!(outcome("trigger OnAfterGetCurrRecord() begin end;"), Outcome::Pass);
}

#[test]
fn the_trigger_runs_once_after_on_new_record_with_the_blank_row() {
    // QH_OpenView_NoRow_TraceHost: `CP.OnNewRecord: below=Yes | CP.OnAfterGetCurrRecord:/0`.
    let src = r#"
table 62920 "OV2 Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 62921 "OV2 Line" { fields { field(1; "Document No."; Code[20]) { } field(2; "Line No."; Integer) { } } keys { key(PK; "Document No.", "Line No.") { Clustered = true; } } }
codeunit 62925 OV2Log { SingleInstance = true; var Log: Text; procedure Add(t: Text) begin Log += t + '|'; end; procedure Get(): Text begin exit(Log); end; }
page 62922 "OV2 Lines" {
    PageType = ListPart; SourceTable = "OV2 Line";
    layout { area(Content) { repeater(R) { field(D; Rec."Line No.") { } } } }
    trigger OnNewRecord() var L: Codeunit OV2Log; begin L.Add('new'); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit OV2Log; begin L.Add('curr:' + Rec."Document No." + '/' + Format(Rec."Line No.")); end;
}
page 62923 "OV2 Doc" {
    PageType = Document; SourceTable = "OV2 Header";
    layout { area(Content) { field(No; Rec."No.") { } part(Lines; "OV2 Lines") { SubPageLink = "Document No." = field("No."); } } }
}
codeunit 62924 T
{
    Subtype = Test;
    [Test] procedure Run()
    var P: TestPage "OV2 Doc"; L: Codeunit OV2Log;
    begin
        P.OpenView();
        if L.Get() <> 'new|curr:/0|' then Error('log %1', L.Get());
        if not P.Lines.D.Visible() then Error('not visible');
        P.Close();
    end;
}"#;
    assert_eq!(al_testkit::outcomes_in(src, "T")[0].1.clone(), Outcome::Pass);
}

#[test]
fn a_part_without_it_still_passes() {
    assert_eq!(outcome(""), Outcome::Pass);
}

fn reporter(rows: bool) -> Outcome {
    // Issue 6878: a Document over a header table, a linked ListPart with OnAfterGetCurrRecord, OpenView, Visible() of a part column.
    let insert = if rows { "H.\"No.\" := 'H1'; H.Insert();" } else { "" };
    let src = format!(
        r#"
table 62930 "VP Header" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62931 "VP Line" {{ fields {{ field(1; "Header No."; Code[20]) {{ }} field(2; "Line No."; Integer) {{ }} field(3; Description; Text[50]) {{ }} }} keys {{ key(PK; "Header No.", "Line No.") {{ Clustered = true; }} }} }}
page 62932 "VP Part" {{
    PageType = ListPart; SourceTable = "VP Line";
    layout {{ area(Content) {{ repeater(Lines) {{ field(Description; Rec.Description) {{ }} }} }} }}
    trigger OnAfterGetCurrRecord() begin HasRow := Rec."Line No." <> 0; end;
    var HasRow: Boolean;
}}
page 62933 "VP Doc" {{
    PageType = Document; SourceTable = "VP Header"; InsertAllowed = false;
    layout {{ area(Content) {{ field("No."; Rec."No.") {{ }} part(Lines; "VP Part") {{ SubPageLink = "Header No." = field("No."); }} }} }}
}}
codeunit 62934 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var H: Record "VP Header"; P: TestPage "VP Doc";
    begin
        H.DeleteAll();
        {insert}
        P.OpenView();
        if not P.Lines.Description.Visible() then Error('hidden');
        P.Close();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn reporter_repro_no_row() {
    assert_eq!(reporter(false), Outcome::Pass);
}

#[test]
fn reporter_repro_one_row() {
    assert_eq!(reporter(true), Outcome::Pass);
}
