//! A Document host opened for view on no row, with a part: the part runs OnNewRecord once on the blank row. Whether and when
//! it runs OnAfterGetCurrRecord is pinned by corpus pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:1594
//! QH_OpenView_NoRow_TraceHost (`CP.OnAfterGetCurrRecord:/0` after `CP.OnNewRecord`) but the runtime does not run it yet, so a
//! part that declares it is refused instead of run with a log that misses the entry (hard rule 4).

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
fn a_part_with_on_after_get_curr_record_is_refused() {
    let o = outcome("trigger OnAfterGetCurrRecord() begin end;");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("OnAfterGetCurrRecord of a list part whose host was opened for view on no row")),
        "{o:?}"
    );
}

#[test]
fn a_part_without_it_still_passes() {
    assert_eq!(outcome(""), Outcome::Pass);
}
