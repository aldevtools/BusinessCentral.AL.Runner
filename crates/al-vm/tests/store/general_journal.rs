//! Behaviour Microsoft's Tests-General Journal bucket needs (issue 49), in minimal AL. Each test names the
//! Microsoft test that requires it (hard rule 3 exception, owner 2026-10-15) and the probes-needed row.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("codeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, 0, inst, vec![]).map(|(v, _)| v)
}

/// `Customer.Name := CopyStr(CreateGuid(), 1, 50)` (Match General Jnl Lines UT, InsertCustomer: 61 tests of
/// Tests-General Journal): a Guid given where a Text is expected is its text form
/// `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`, the form an assignment gives it (docs/probes-needed.md, "A Guid
/// assigned to a Text or Code").
#[test]
fn a_guid_as_the_text_argument_of_copystr_is_its_text_form() {
    let r = text(run(
        "procedure F(): Text var G: Guid; begin Evaluate(G, '{12345678-ABCD-4DEF-8123-0123456789AB}'); exit(CopyStr(G, 1, 50) + '|' + CopyStr(G, 2, 8) + '|' + Format(StrLen(CopyStr(CreateGuid(), 1, 50)))); end;",
    ));
    assert_eq!(r, "{12345678-ABCD-4DEF-8123-0123456789AB}|12345678|38");
}

// ---- a subscriber to a page event the test client cannot raise (88 tests of Tests-General Journal) ------
// Base Application subscribes to `OnDeleteRecordEvent` of General Journal, Sales Journal and others, so every
// test that only opened one of those pages was refused at the open. The test client has no way to delete a
// page's row itself (a standard Delete action runs no code of ours), so opening the page is fine; invoking the
// standard Delete action is what is refused.
const PAGES: &str = r#"
table 50101 "GJ Row" { fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[30]) { } } keys { key(PK; "No.") { } } }
page 50102 "GJ List"
{
    PageType = List;
    SourceTable = "GJ Row";
    layout { area(Content) { repeater(Lines) { field(No; Rec."No.") { } } } }
    actions { area(Processing) { action(Delete) { ApplicationArea = All; Image = Delete; } action(Mark) { trigger OnAction() begin end; } } }
}
codeunit 50103 "GJ Subs"
{
    [EventSubscriber(ObjectType::Page, Page::"GJ List", 'OnDeleteRecordEvent', '', false, false)]
    local procedure OnDel(var Rec: Record "GJ Row"; var AllowDelete: Boolean) begin AllowDelete := true; end;
}
"#;

fn run_page(body: &str) -> Result<Value, Signal> {
    let src = format!("{PAGES}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, 0, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn opening_a_page_that_has_a_delete_record_subscriber_is_not_refused() {
    let r = text(run_page(
        "procedure F(): Text var R: Record \"GJ Row\"; P: TestPage \"GJ List\"; begin R.\"No.\" := 'A'; R.Insert(); P.OpenView(); P.Mark.Invoke(); exit(P.No.Value()); end;",
    ));
    assert_eq!(r, "A");
}

// ---- a new page row takes the equality filters of the page's FilterGroup(2) (Payment Journal) -----------
const JOURNAL: &str = r#"
table 50111 "GJ Line" { fields { field(1; Tmpl; Code[10]) { } field(2; Batch; Code[10]) { } field(3; "Line No."; Integer) { } field(4; Acc; Code[20]) { } } keys { key(PK; Tmpl, Batch, "Line No.") { } } }
page 50112 "GJ Journal"
{
    PageType = List;
    SourceTable = "GJ Line";
    layout { area(Content) { repeater(Lines) { field(Acc; Rec.Acc) { } field(BatchCtl; Rec.Batch) { } } } }
    var Seen: Text;
    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange(Tmpl, 'T');
        Rec.SetRange(Batch, 'B');
        Rec.FilterGroup(0);
    end;
    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Seen := Rec.Tmpl + '/' + Rec.Batch;
        Rec."Line No." := 10000;
    end;
    procedure GetSeen(): Text begin exit(Seen); end;
}
"#;

#[test]
fn a_new_row_of_a_page_starts_with_the_equality_filters_of_filter_group_2() {
    let src = format!(
        "{JOURNAL}\ncodeunit 50100 C {{ procedure F(): Text var P: TestPage \"GJ Journal\"; R: Record \"GJ Line\"; begin P.OpenEdit(); P.New(); P.Acc.SetValue('X'); P.Close(); R.FindFirst(); exit(R.Tmpl + '/' + R.Batch); end; }}"
    );
    let r = run_src(&src);
    assert_eq!(text(r), "T/B");
}

fn run_src(src: &str) -> Result<Value, Signal> {
    let l = al_testkit::compile(src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, 0, inst, vec![]).map(|(v, _)| v)
}

const JOURNAL2: &str = r#"
table 50113 "GJ Line2" { fields { field(1; Tmpl; Code[10]) { } field(2; Batch; Code[10]) { } field(3; "Line No."; Integer) { } field(4; Acc; Code[20]) { } } keys { key(PK; Tmpl, Batch, "Line No.") { } } }
page 50114 "GJ Journal2"
{
    PageType = List;
    SourceTable = "GJ Line2";
    layout { area(Content) { repeater(Lines) { field(Acc; Rec.Acc) { } } } }
    trigger OnOpenPage()
    var Mgt: Codeunit "GJ Mgt";
    begin
        Mgt.OpenJnl(Rec);
    end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec."Line No." := 10000; end;
}
codeunit 50115 "GJ Mgt"
{
    procedure OpenJnl(var L: Record "GJ Line2")
    var T: Code[10]; B: Code[10];
    begin
        T := L.GetRangeMax(Tmpl);
        B := L.GetRangeMax(Batch);
        L.FilterGroup := 2;
        L.SetRange(Tmpl, T);
        L.SetRange(Batch, B);
        L.FilterGroup := 0;
    end;
}
"#;

/// `PAGE.Run(PAGE::"Payment Journal", GenJournalLine)` with equality filters on the template and the batch
/// (Incom. Doc. Attach. FactBox, PaymentJournal_Multiline_Attach_Prev_Next): the new row takes both.
#[test]
fn a_new_row_takes_every_equality_filter_of_the_record_the_page_was_run_with() {
    let src = format!(
        "{JOURNAL2}\ncodeunit 50100 C {{ procedure F(): Text var P: TestPage \"GJ Journal2\"; L: Record \"GJ Line2\"; R: Record \"GJ Line2\"; begin L.SetRange(Tmpl, 'T'); L.SetRange(Batch, 'B'); Commit(); P.Trap(); Page.Run(Page::\"GJ Journal2\", L); P.Acc.SetValue('X'); P.Close(); R.FindFirst(); exit(R.Tmpl + '/' + R.Batch); end; }}"
    );
    assert_eq!(text(run_src(&src)), "T/B");
}

/// `GenJnlManagement.OpenJnl` reads `GenJnlLine.GetRangeMax("Journal Template Name")` in filter group 0 right after
/// `TemplateSelection` set the template in filter group 2 (34 tests of Tests-General Journal; Microsoft's own
/// code, so BC's GetRangeMax sees the other group; corpus record/filters/GetRangeFilterGroups.al).
#[test]
fn get_range_max_sees_a_filter_of_another_group_and_still_fails_with_none() {
    let ok = text(run_src(
        "table 50120 \"GJ T\" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }\ncodeunit 50100 C { procedure F(): Text var R: Record \"GJ T\"; begin R.FilterGroup(2); R.SetRange(K, 'T'); R.FilterGroup(0); exit(R.GetRangeMax(K) + '/' + R.GetRangeMin(K)); end; }",
    ));
    assert_eq!(ok, "T/T");
    // No filter at all: DB:NoFilter (corpus record/filters/GetRangeFilterGroups.al: P168_GetRange_FilterOnlyInGroup2).
    match run_src(
        "table 50120 \"GJ T\" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }\ncodeunit 50100 C { procedure F(): Text var R: Record \"GJ T\"; begin exit(R.GetRangeMax(K)); end; }",
    ) {
        Err(Signal::Error(e)) => {
            assert_eq!(e.code, "DB:NoFilter");
            assert_eq!(e.message, "Specify a filter for the K field in the GJ T table.");
        }
        other => panic!("{other:?}"),
    }
}

// ---- CopyLinks of a record without links (every posting of a journal line: Bank Account Ledger Entry.CopyLinks) --
// `BankAccLedgEntry.CopyLinks(GenJnlLine)` is a statement; the source has no link. The internal FindSet on the Record
// Link table must not raise "There is no Record Link within the filter" because the statement's result is unused
// (Tests-General Journal, 80 tests; record/TestRecordLinkTable.al pins CopyLinks, not the empty source).
const LINKS: &str = r#"
table 2000000068 "Record Link" {
    fields {
        field(1; "Link ID"; Integer) { }
        field(2; "Record ID"; RecordID) { }
        field(3; URL1; Text[250]) { }
        field(7; Type; Option) { OptionMembers = Link,Note; }
        field(8; Description; Text[250]) { }
        field(9; Note; Blob) { }
        field(10; "User ID"; Code[132]) { }
        field(11; Created; DateTime) { }
        field(12; Company; Text[30]) { }
        field(13; Notify; Boolean) { }
        field(14; "To User ID"; Code[132]) { }
    }
    keys { key(PK; "Link ID") { } }
}
table 50105 "Linked" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
"#;

#[test]
fn copy_links_from_a_record_without_links_raises_nothing() {
    let src = format!(
        "{LINKS}\ncodeunit 50100 C {{ procedure F(): Text var A: Record Linked; B: Record Linked; begin A.Id := 1; A.Insert(); B.Id := 2; B.Insert(); B.CopyLinks(A); exit(Format(B.HasLinks())); end; }}"
    );
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    assert_eq!(text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v)), "No");
}
