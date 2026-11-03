//! A new Document host, its key typed, closed with a part that shows no row: BC inserts the host and runs no trigger of the part
//! (corpus pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:1468 QH_EmptyPart_OpenNew_KeyTyped, `close` =
//! `[H.OnInsert:Q1]` with no part entry). The runtime relinks the part and runs them, so a part that declares one is refused.

use al_test_cli::report::Outcome;

fn outcome(part_trigger: &str) -> Outcome {
    let src = format!(
        r#"
table 62920 "NC Header" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62921 "NC Line" {{ fields {{ field(1; "Document No."; Code[20]) {{ }} field(2; "Line No."; Integer) {{ }} }} keys {{ key(PK; "Document No.", "Line No.") {{ Clustered = true; }} }} }}
page 62922 "NC Lines" {{
    PageType = ListPart; SourceTable = "NC Line";
    layout {{ area(Content) {{ repeater(R) {{ field(L; Rec."Line No.") {{ }} }} }} }}
    {part_trigger}
}}
page 62923 "NC Doc" {{
    PageType = Document; SourceTable = "NC Header";
    layout {{ area(Content) {{ field(No; Rec."No.") {{ }} part(Lines; "NC Lines") {{ SubPageLink = "Document No." = field("No."); }} }} }}
}}
codeunit 62924 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: TestPage "NC Doc";
    begin
        P.OpenNew();
        P.No.SetValue('Q1');
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
        matches!(&o, Outcome::NotRun(why) if why.contains("closing a new host row")),
        "{o:?}"
    );
}

#[test]
fn a_part_with_on_new_record_is_refused() {
    let o = outcome("trigger OnNewRecord(BelowxRec: Boolean) begin end;");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("closing a new host row")),
        "{o:?}"
    );
}

#[test]
fn a_part_without_triggers_passes() {
    assert_eq!(outcome(""), Outcome::Pass);
}
