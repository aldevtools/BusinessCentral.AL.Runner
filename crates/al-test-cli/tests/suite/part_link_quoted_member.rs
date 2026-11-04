//! A part that sets its own filter on an Option field next to its SubPageLink filter (the Bank Acc. Reconciliation shape,
//! page 379; issue 6953). The link alone is covered by `sub_page_link_quoted_option_member.rs`. Pinned by corpus
//! pages/lt-y42-link-quoted-member/Y42LinkQuotedMember.al:342 Lq_Option_PartAlsoSetsFilter: with equal filters TestPage
//! Filter.GetFilter reads the numbers `0|1|2`; other combinations are not pinned and stay refused.

use al_test_cli::report::Outcome;

fn part_filter_with(link: &str, part_trigger: &str) -> Outcome {
    let src = format!(
        r#"
enum 62950 "QM Kind" {{ Extensible = false; value(0; "Inbound Transfer") {{ }} value(1; "Outbound Transfer") {{ }} value(2; Other) {{ }} }}
table 62951 "QM Head" {{ fields {{ field(1; "No."; Code[10]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
table 62952 "QM Entry" {{
    fields {{
        field(1; "Entry No."; Integer) {{ }} field(2; "Acc No."; Code[10]) {{ }}
        field(3; Status; Option) {{ OptionMembers = Open,"Bank Acc. Entry Applied","Check Entry Applied",Closed; }}
        field(4; Kind; Enum "QM Kind") {{ }}
    }}
    keys {{ key(PK; "Entry No.") {{ Clustered = true; }} }}
}}
page 62953 "QM Part" {{
    PageType = ListPart; SourceTable = "QM Entry";
    layout {{ area(Content) {{ repeater(R) {{ field("Entry No."; Rec."Entry No.") {{ }} }} }} }}
    {part_trigger}
}}
page 62954 "QM Card" {{
    PageType = Card; SourceTable = "QM Head";
    layout {{ area(Content) {{ field("No."; Rec."No.") {{ }} part(P; "QM Part") {{ SubPageLink = "Acc No." = field("No."), {link}; }} }} }}
}}
codeunit 62955 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var H: Record "QM Head"; E: Record "QM Entry"; P: TestPage "QM Card"; Rows: Text; i: Integer;
    begin
        H."No." := 'A1'; H.Insert();
        for i := 0 to 3 do begin
            E."Entry No." := i + 1; E."Acc No." := 'A1'; E.Status := i; E.Kind := "QM Kind".FromInteger(i mod 3); E.Insert();
        end;
        P.OpenView();
        P.GoToRecord(H);
        if P.P.First() then
            repeat
                Rows += P.P."Entry No.".Value() + ',';
            until not P.P.Next();
        Error('%1 %2 %3', Rows, P.P.Filter.GetFilter(Status), P.P.Filter.GetFilter(Kind));
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

fn fail(text: &str) -> Outcome {
    Outcome::Fail(text.to_string())
}

const LINK: &str = r#"Status = filter(Open | "Bank Acc. Entry Applied" | "Check Entry Applied")"#;

#[test]
fn the_part_setting_the_same_values_in_on_open_page_reads_as_numbers() {
    let set = "trigger OnOpenPage() begin Rec.SetFilter(Status, '%1|%2|%3', Rec.Status::Open, Rec.Status::\"Bank Acc. Entry Applied\", Rec.Status::\"Check Entry Applied\"); end;";
    assert_eq!(part_filter_with(LINK, set), fail("1,2,3, 0|1|2 "));
}

#[test]
fn the_part_setting_other_values_stays_refused() {
    let set = "trigger OnOpenPage() begin Rec.SetFilter(Status, '%1', Rec.Status::Open); end;";
    assert!(matches!(part_filter_with(LINK, set), Outcome::NotRun(_)));
}
