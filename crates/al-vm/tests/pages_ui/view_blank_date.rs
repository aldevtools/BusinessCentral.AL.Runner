//! A list page whose SourceTableView is `where(<Date field> = filter(''))` shows only the rows with a blank date
//! (pages/lt-cd-testpage-leads/CdTestPageLeads.al:842 SH_SourceTableView_DateFilterBlank_Rows). The same condition in an
//! action's RunPageView stays the registered assumption `view-blank-date-filter`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 50700 "Repro Entry" { fields { field(1; "Entry No."; Integer) { } field(2; "Starting Date"; Date) { } } keys { key(PK; "Entry No.") { } } }
page 50700 "Repro Entries Undated"
{
    PageType = List; SourceTable = "Repro Entry"; SourceTableView = where("Starting Date" = filter('')); InsertAllowed = false;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
}
codeunit 50701 C
{
    procedure F(): Text
    var Entry: Record "Repro Entry"; P: TestPage "Repro Entries Undated"; s: Text;
    begin
        Entry."Entry No." := 1; Entry.Insert();
        Entry."Entry No." := 2; Entry."Starting Date" := 20260101D; Entry.Insert();
        Entry.Get(1);
        P.Trap();
        Page.Run(Page::"Repro Entries Undated", Entry);
        s := P."Entry No.".Value() + '|' + Format(P.Next());
        P.Close();
        exit(s);
    end;
}
"#;

#[test]
fn a_blank_date_view_lists_only_the_blank_row() {
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "1|No");
    assert!(!take_hits().contains(&Assumption::ViewBlankDateFilter));
}

const RUN_VIEW: &str = r#"
table 50700 "Repro Entry" { fields { field(1; "Entry No."; Integer) { } field(2; "Starting Date"; Date) { } } keys { key(PK; "Entry No.") { } } }
page 50700 "Repro Entries"
{
    PageType = List; SourceTable = "Repro Entry"; InsertAllowed = false;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
}
page 50702 "Repro Opener"
{
    PageType = Card; SourceTable = "Repro Entry";
    layout { area(Content) { field(No; Rec."Entry No.") { } } }
    actions { area(Processing) { action(Open) { RunObject = page "Repro Entries"; RunPageView = where("Starting Date" = filter('')); } } }
}
codeunit 50701 C
{
    procedure F(): Text
    var Entry: Record "Repro Entry"; O: TestPage "Repro Opener"; P: TestPage "Repro Entries"; s: Text;
    begin
        Entry."Entry No." := 1; Entry.Insert();
        Entry."Entry No." := 2; Entry."Starting Date" := 20260101D; Entry.Insert();
        O.OpenView();
        P.Trap();
        O.Open.Invoke();
        s := P."Entry No.".Value() + '|' + Format(P.Next());
        P.Close();
        exit(s);
    end;
}
"#;

#[test]
fn a_blank_date_run_page_view_is_the_assumption() {
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(RUN_VIEW, "C", "F")), "1|No");
    assert!(take_hits().contains(&Assumption::ViewBlankDateFilter));
}
