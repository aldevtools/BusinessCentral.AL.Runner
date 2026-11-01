//! A reportextension that changes the run (a modified data item trigger) and adds a field to the request page (al-runner issue
//! 6302): the value a request page handler sets reaches the extension's trigger. Mirrors corpus report/lt-wd/
//! WdReportExtRequestPage.al. SaveValues, a field bound to a report variable and a trigger on the extension's request page are
//! not pinned with such an extension and stay refused.

use al_test_cli::report::Outcome;

fn objects(save_values: bool, ext_trigger: bool) -> String {
    let save = if save_values { "SaveValues = true;" } else { "" };
    let trigger = if ext_trigger {
        "trigger OnOpenPage() begin end;"
    } else {
        ""
    };
    format!(
        r#"
table 62700 "RX Account"
{{
    fields {{ field(1; "No."; Code[20]) {{ }} field(2; Note; Text[50]) {{ }} field(3; Seen; Integer) {{ }} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
}}
report 62701 "RX Statement"
{{
    ProcessingOnly = true; UseRequestPage = true;
    dataset
    {{
        dataitem(Account; "RX Account")
        {{
            trigger OnAfterGetRecord() begin Account.Seen += 1; Account.Modify(); end;
        }}
    }}
    requestpage {{ {save} layout {{ area(Content) {{ group(Options) {{ field(ShowAll; ShowAll) {{ }} }} }} }} }}
    var ShowAll: Boolean;
}}
reportextension 62702 "RX Statement Ext" extends "RX Statement"
{{
    dataset
    {{
        modify(Account)
        {{
            trigger OnAfterAfterGetRecord() begin Account.Note := NoteText; Account.Modify(); end;
        }}
    }}
    requestpage {{ layout {{ addlast(Options) {{ field(NoteText; NoteText) {{ }} }} }} {trigger} }}
    var NoteText: Text[50];
}}
"#
    )
}

fn outcome(save_values: bool, ext_trigger: bool) -> Outcome {
    let src = format!(
        r#"{}
codeunit 62790 T
{{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run()
    var A: Record "RX Account"; R: Report "RX Statement";
    begin
        A.DeleteAll(); A."No." := 'A'; A.Insert(); A."No." := 'B'; A.Insert(); Commit();
        R.UseRequestPage(true); R.Run();
        A.Get('A'); if (A.Note <> 'hello') or (A.Seen <> 1) then Error('A: %1 %2', A.Note, A.Seen);
        A.Get('B'); if (A.Note <> 'hello') or (A.Seen <> 1) then Error('B: %1 %2', A.Note, A.Seen);
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RX Statement")
    begin P.NoteText.SetValue('hello'); P.OK().Invoke(); end;
}}"#,
        objects(save_values, ext_trigger)
    );
    let o = al_testkit::outcomes_in(&src, "T");
    o[0].1.clone()
}

#[test]
fn a_request_page_field_of_an_extension_that_changes_the_run_reaches_its_trigger() {
    assert_eq!(outcome(false, false), Outcome::Pass);
}

#[test]
fn save_values_or_a_trigger_on_the_extension_request_page_stays_refused() {
    let o = outcome(false, true);
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("changes the run and extends the request page")),
        "{o:?}"
    );
    // With SaveValues the close by OK is not pinned (the SaveAsXml and Cancel closes are: corpus al-language-onprem/report/
    // lt-y60-reportext-saveasxml/Y60ReportExtSaveAsXml.al).
    let o = outcome(true, false);
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("SaveValues") && why.contains("OK")),
        "{o:?}"
    );
}

// Y60ReportExtSaveAsXml.al ReportExt_SaveValues_SaveAsXml_NextRun ('second'): a Cancel close of such a page stores nothing, the
// report does not run, and the next run opens with the defaults.
#[test]
fn with_save_values_a_cancel_close_stores_nothing() {
    let src = format!(
        r#"{}
codeunit 62790 T
{{
    Subtype = Test;
    var Step: Integer; Seen: Text;
    [Test] [HandlerFunctions('H')] procedure Run()
    var A: Record "RX Account";
    begin
        A.DeleteAll(); A."No." := 'A'; A.Insert(); Commit();
        Step := 1; Report.Run(Report::"RX Statement");
        Step := 2; Report.Run(Report::"RX Statement");
        A.Get('A');
        if (Seen <> 'note= showall=No') or (A.Seen <> 0) then Error('%1 %2', Seen, A.Seen);
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RX Statement")
    begin
        if Step = 1 then begin P.NoteText.SetValue('hello'); P.ShowAll.SetValue(true); P.Cancel().Invoke(); exit; end;
        Seen := 'note=' + P.NoteText.Value() + ' showall=' + P.ShowAll.Value();
        P.Cancel().Invoke();
    end;
}}"#,
        objects(true, false)
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o[0].1, Outcome::Pass);
}

// Y60ReportExtSaveAsXml.al ReportExt_SaveValues_SaveAsXml_NextRun ('second'): the close by SaveAsXml stores the values of the
// base and of the extension; the next run opens with them.
#[test]
fn with_save_values_a_save_as_xml_close_stores_the_base_and_extension_values() {
    let src = r#"
table 62710 "RY Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
report 62711 "RY Saved"
{
    ProcessingOnly = false;
    dataset { dataitem(Header; "RY Row") { column(HName; Name) { } } }
    requestpage { SaveValues = true; layout { area(Content) { group(Options) { field(ShowAll; ShowAll) { } } } } }
    var ShowAll: Boolean;
}
reportextension 62712 "RY Saved X" extends "RY Saved"
{
    dataset { modify(Header) { trigger OnAfterAfterGetRecord() begin end; } }
    requestpage { layout { addlast(Options) { field(NoteText; NoteText) { } } } }
    var NoteText: Text[50];
}
codeunit 62790 T
{
    Subtype = Test;
    var Step: Integer; Seen: Text;
    [Test] [HandlerFunctions('H')] procedure Run()
    var R: Record "RY Row";
    begin
        R.DeleteAll(); R.Id := 1; R.Name := 'one'; R.Insert(); Commit();
        Step := 1; Report.Run(Report::"RY Saved");
        Step := 2; Report.Run(Report::"RY Saved");
        if Seen <> 'note=hello showall=Yes' then Error(Seen);
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RY Saved")
    begin
        if Step = 1 then begin
            P.NoteText.SetValue('hello'); P.ShowAll.SetValue(true);
            P.SaveAsXml(TemporaryPath() + 'p.xml', TemporaryPath() + 'd.xml');
            exit;
        end;
        Seen := 'note=' + P.NoteText.Value() + ' showall=' + P.ShowAll.Value();
        P.Cancel().Invoke();
    end;
}"#;
    let o = al_testkit::outcomes_in(src, "T");
    assert_eq!(o[0].1, Outcome::Pass);
}
