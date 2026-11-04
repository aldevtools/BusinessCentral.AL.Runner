//! A new Document host (OpenNew) with one part linked on its key: a field outside the key typed first stores the host with a
//! blank key; the key typed next runs its OnValidate and then the card shows the blank key again and nothing is renamed (corpus
//! pages/parts/PartLineBeforeHostKey.al:1513 PYB_OpenNew_NameTypedFirst_ThenKey_ThenLine). A Card with a
//! part linked on its key does the same (pages/triggers/NewLineStampsFilters.al:372 PYB_OpenNew_TypeThenNo_Steps).

use al_test_cli::report::Outcome;

fn outcome(page_type: &str) -> Outcome {
    let src = format!(
        r#"
table 62900 "KF Header" {{ fields {{ field(1; "No."; Code[20]) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62901 "KF Line" {{ fields {{ field(1; "Document No."; Code[20]) {{ }} field(2; "Line No."; Integer) {{ }} field(3; Descr; Text[30]) {{ }} }} keys {{ key(PK; "Document No.", "Line No.") {{ Clustered = true; }} }} }}
page 62902 "KF Lines" {{ PageType = ListPart; SourceTable = "KF Line"; layout {{ area(Content) {{ repeater(R) {{ field(Descr; Rec.Descr) {{ }} }} }} }} }}
page 62903 "KF Doc" {{
    PageType = {page_type}; SourceTable = "KF Header";
    layout {{ area(Content) {{ field(No; Rec."No.") {{ }} field(Name; Rec.Name) {{ }} part(Lines; "KF Lines") {{ SubPageLink = "Document No." = field("No."); }} }} }}
}}
codeunit 62904 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: TestPage "KF Doc"; H: Record "KF Header";
    begin
        P.OpenNew();
        P.Name.SetValue('N1');
        P.No.SetValue('D7');
        if P.No.Value() <> '' then Error('card shows %1', P.No.Value());
        if H.Count() <> 1 then Error('headers %1', H.Count());
        H.FindFirst();
        if H."No." <> '' then Error('stored key %1', H."No.");
        P.Close();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_document_shows_its_blank_key_again_after_the_key_is_typed() {
    assert_eq!(outcome("Document"), Outcome::Pass);
}

#[test]
fn a_card_with_a_part_linked_on_its_key_does_the_same() {
    assert_eq!(outcome("Card"), Outcome::Pass);
}
