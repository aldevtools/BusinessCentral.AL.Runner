//! A SubPageLink `filter()` or `const()` with quoted Option or Enum members (the Bank Acc. Reconciliation part link, issue
//! 6953). Pinned by corpus pages/link-quoted-member/Y42LinkQuotedMember.al: the quotes are dropped and the members match
//! (:7), and TestPage Filter.GetFilter of the part reads the link filter as option numbers (:335 `0|1|2`, :379 `0|1`,
//! :401 `2`, :423 `<>1`).

use al_test_cli::report::Outcome;

fn part_filter(link: &str) -> Outcome {
    part_filter_with(link, "")
}

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

#[test]
fn an_option_filter_with_quoted_members_matches_and_reads_as_numbers() {
    assert_eq!(
        part_filter(r#"Status = filter(Open | "Bank Acc. Entry Applied" | "Check Entry Applied")"#),
        fail("1,2,3, 0|1|2 ")
    );
}

#[test]
fn an_option_const_and_a_not_equal_with_a_quoted_member_read_as_numbers() {
    assert_eq!(part_filter(r#"Status = const("Check Entry Applied")"#), fail("3, 2 "));
    assert_eq!(
        part_filter(r#"Status = filter(<> "Bank Acc. Entry Applied")"#),
        fail("1,3,4, <>1 ")
    );
}

#[test]
fn an_enum_filter_with_quoted_members_reads_as_numbers() {
    assert_eq!(
        part_filter(r#"Kind = filter("Inbound Transfer" | "Outbound Transfer")"#),
        fail("1,2,4,  0|1")
    );
}

#[test]
fn the_part_setting_the_same_values_in_on_open_page_still_reads_as_numbers() {
    // Y42LinkQuotedMember.al:357 Lq_Option_PartAlsoSetsFilter `0|1|2` (the part page sets the filter in OnOpenPage, page 379 shape).
    let set = "trigger OnOpenPage() begin Rec.SetFilter(Status, '%1|%2|%3', Rec.Status::Open, Rec.Status::\"Bank Acc. Entry Applied\", Rec.Status::\"Check Entry Applied\"); end;";
    assert_eq!(
        part_filter_with(
            r#"Status = filter(Open | "Bank Acc. Entry Applied" | "Check Entry Applied")"#,
            set
        ),
        fail("1,2,3, 0|1|2 ")
    );
}
