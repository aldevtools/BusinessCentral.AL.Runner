//! `CurrPage.Update` on a page that shows no row (an empty view, or an editable list on its new line) after a filter change made
//! rows visible: which row the client lands on, and which row triggers run, is not pinned (corpus
//! pageupdate/TestPageUpdateNewRow.al pins only a card's unsaved new row). Refused. Tests-ERM 134022
//! PurchasePaymentAppliedWithToleranceAmount reaches it: General Journal (page 39) opens on an empty batch and the
//! test switches the batch name.

const SRC: &str = r#"
table 69980 "UpdNl Line" {
    fields { field(1; Batch; Code[10]) { } field(2; No; Integer) { } }
    keys { key(PK; Batch, No) { Clustered = true; } }
}
page 69981 "UpdNl List"
{
    PageType = List; SourceTable = "UpdNl Line"; ApplicationArea = All; UsageCategory = None;
    layout {
        area(Content) {
            field(BatchCtl; BatchName) {
                trigger OnValidate()
                begin
                    Rec.FilterGroup(2);
                    Rec.SetRange(Batch, BatchName);
                    Rec.FilterGroup(0);
                    CurrPage.Update(false);
                end;
            }
            repeater(G) { field(NoCtl; Rec.No) { } }
        }
    }
    var BatchName: Code[10];
    trigger OnOpenPage()
    begin
        BatchName := 'A';
        Rec.FilterGroup(2);
        Rec.SetRange(Batch, BatchName);
        Rec.FilterGroup(0);
    end;
}
codeunit 69982 C
{
    procedure Switch(): Text
    var
        R: Record "UpdNl Line";
        P: TestPage "UpdNl List";
    begin
        R.Batch := 'B';
        R.No := 1;
        R.Insert();
        P.OpenEdit();
        P.BatchCtl.SetValue('B');
        exit(P.NoCtl.Value);
    end;
    procedure SwitchInView(): Text
    var
        R: Record "UpdNl Line";
        P: TestPage "UpdNl List";
    begin
        R.Batch := 'B';
        R.No := 1;
        R.Insert();
        P.OpenView();
        P.BatchCtl.SetValue('B');
        exit(P.NoCtl.Value);
    end;
}
"#;

#[test]
fn update_on_the_new_line_after_a_filter_change_that_shows_rows_is_refused() {
    match al_testkit::run_in(SRC, "C", "Switch") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("shows no row"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn update_on_an_empty_view_after_a_filter_change_that_shows_rows_is_refused() {
    match al_testkit::run_in(SRC, "C", "SwitchInView") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("shows no row"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
