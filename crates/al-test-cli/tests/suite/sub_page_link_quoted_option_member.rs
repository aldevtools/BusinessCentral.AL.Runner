//! A SubPageLink `filter()` or `const()` on an Option field takes a quoted member (`"Raw material"`) as the member, not as
//! text with quotes (AL Runner issue 6915). Pinned by corpus pages/lt-y42-link-quoted-member/Y42LinkQuotedMember.al:
//! Lq_Option_FilterQuoted (rows `1,2,3,7,8,`), Lq_Option_ConstQuoted, Lq_Option_NotEqualQuoted.

use al_test_cli::report::Outcome;

fn outcome(link: &str, expected_rows: &str) -> Outcome {
    let src = format!(
        r#"
table 62940 "QM Parent" {{ fields {{ field(1; "No."; Code[10]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62941 "QM Entry"
{{
    fields
    {{
        field(1; "Parent no."; Code[10]) {{ }}
        field(2; "Line no."; Integer) {{ }}
        field(3; "Material type"; Option) {{ OptionMembers = "Finished goods","Raw material","Packaging material"; }}
    }}
    keys {{ key(PK; "Parent no.", "Line no.") {{ Clustered = true; }} }}
}}
page 62942 "QM Sub" {{
    PageType = ListPart; SourceTable = "QM Entry";
    layout {{ area(Content) {{ repeater(R) {{ field(L; Rec."Line no.") {{ }} }} }} }}
}}
page 62943 "QM Card" {{
    PageType = Card; SourceTable = "QM Parent";
    layout {{ area(Content) {{ field(No; Rec."No.") {{ }} part(Sub; "QM Sub") {{ SubPageLink = "Parent no." = field("No."), {link}; }} }} }}
}}
codeunit 62944 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: Record "QM Parent"; E: Record "QM Entry"; Card: TestPage "QM Card"; Rows: Text;
    begin
        P."No." := 'P1'; P.Insert();
        Add('P1', 1, E."Material type"::"Finished goods");
        Add('P1', 2, E."Material type"::"Raw material");
        Add('P1', 3, E."Material type"::"Packaging material");
        Card.OpenView();
        if Card.Sub.First() then
            repeat
                Rows += Card.Sub.L.Value() + ',';
            until not Card.Sub.Next();
        Card.Close();
        if Rows <> '{expected_rows}' then Error('rows [%1]', Rows);
    end;
    local procedure Add(Parent: Code[10]; No: Integer; Kind: Option "Finished goods","Raw material","Packaging material")
    var E: Record "QM Entry";
    begin
        E."Parent no." := Parent; E."Line no." := No; E."Material type" := Kind; E.Insert();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_quoted_member_list_selects_its_members() {
    assert_eq!(
        outcome(r#""Material type" = filter("Raw material" | "Finished goods")"#, "1,2,"),
        Outcome::Pass
    );
}

#[test]
fn a_quoted_const_member_selects_it() {
    assert_eq!(
        outcome(r#""Material type" = const("Raw material")"#, "2,"),
        Outcome::Pass
    );
}

#[test]
fn a_not_equal_quoted_member_excludes_it() {
    assert_eq!(
        outcome(r#""Material type" = filter(<> "Raw material")"#, "1,3,"),
        Outcome::Pass
    );
}
