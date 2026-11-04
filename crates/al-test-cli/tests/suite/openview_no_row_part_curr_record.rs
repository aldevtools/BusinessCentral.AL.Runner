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

/// Issue 6953: Allocation Account and Bank Acc. Reconciliation are ListPlus pages. Opened for view on no row, their linked
/// list part declares OnNewRecord. `host` is the host's page type, `hidden` hides the part (Allocation Account hides one of
/// its two parts), `grouped` puts the part in a group (Bank Acc. Reconciliation).
fn list_plus_host(host: &str, hidden: bool, grouped: bool) -> (Outcome, String) {
    let visible = if hidden { "Visible = false;" } else { "" };
    let part = format!(r#"part(Lines; "LH Lines") {{ {visible} SubPageLink = "Head No." = field("No."); }}"#);
    let part = if grouped {
        format!("group(G) {{ {part} }}")
    } else {
        part
    };
    let src = format!(
        r#"
table 62940 "LH Head" {{ fields {{ field(1; "No."; Code[10]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62941 "LH Line" {{ fields {{ field(1; "Head No."; Code[10]) {{ }} field(2; "Line No."; Integer) {{ }} field(3; Description; Text[30]) {{ }} }} keys {{ key(PK; "Head No.", "Line No.") {{ Clustered = true; }} }} }}
codeunit 62945 LHLog {{ SingleInstance = true; var Log: Text; procedure Add(t: Text) begin Log += t + '|'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }}
page 62942 "LH Lines" {{
    PageType = ListPart; SourceTable = "LH Line"; DelayedInsert = true; AutoSplitKey = true;
    layout {{ area(Content) {{ repeater(R) {{ field(Description; Rec.Description) {{ }} }} }} }}
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit LHLog; begin L.Add('LP.OnNewRecord:' + Rec."Head No." + ' below=' + Format(BelowxRec)); Rec.Description := 'New-line marker'; end;
    trigger OnAfterGetCurrRecord() var L: Codeunit LHLog; begin L.Add('LP.OnAfterGetCurrRecord:' + Rec."Head No." + '/' + Format(Rec."Line No.") + ' ' + Rec.Description); end;
}}
page 62943 "LH Host" {{
    PageType = {host}; SourceTable = "LH Head"; Caption = 'LH Host';
    layout {{ area(Content) {{ group(General) {{ field("No."; Rec."No.") {{ }} }} {part} }} }}
    trigger OnOpenPage() var L: Codeunit LHLog; begin L.Add('H.OnOpenPage'); end;
}}
codeunit 62944 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: TestPage "LH Host"; L: Codeunit LHLog;
    begin
        P.OpenView();
        if P.Caption() = '' then Error('no caption');
        Error('LOG:' + L.Take());
    end;
}}"#
    );
    let r = al_testkit::outcomes_in(&src, "T")[0].1.clone();
    let log = match &r {
        Outcome::Fail(m) => m.split("LOG:").nth(1).unwrap_or(m).to_string(),
        _ => String::new(),
    };
    (r, log)
}

// pages/parts/PartListPlusHostNoRow.al:288 and :336 (issue 6953):
// ListPlusHost_OpenView_NoRow_PartTriggers `[H.OnOpenPage | LP.OnNewRecord: below=Yes | LP.OnAfterGetCurrRecord:/0 New-line
// marker]`, _PartInGroup the same, _HiddenPart `[H.OnOpenPage]`.
const VISIBLE_PART: &str = "H.OnOpenPage|LP.OnNewRecord: below=Yes|LP.OnAfterGetCurrRecord:/0 New-line marker|";

#[test]
fn list_plus_host_on_no_row_opens_its_part() {
    let (r, log) = list_plus_host("ListPlus", false, false);
    assert!(matches!(r, Outcome::Fail(_)), "{r:?}");
    assert_eq!(log, VISIBLE_PART);
}

#[test]
fn list_plus_host_on_no_row_with_the_part_in_a_group() {
    assert_eq!(list_plus_host("ListPlus", false, true).1, VISIBLE_PART);
}

#[test]
fn list_plus_host_on_no_row_with_a_hidden_part_runs_no_part_trigger() {
    let (r, log) = list_plus_host("ListPlus", true, false);
    assert!(matches!(r, Outcome::Fail(_)), "{r:?}");
    assert_eq!(log, "H.OnOpenPage|");
}
