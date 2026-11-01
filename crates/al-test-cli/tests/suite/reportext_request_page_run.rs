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
    for (save, trigger) in [(true, false), (false, true)] {
        let o = outcome(save, trigger);
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("changes the run and extends the request page")),
            "{save} {trigger}: {o:?}"
        );
    }
}
