//! TestPage.Close() reports CloseAction OK on a NavigatePage (temporary or real source, after a footer action, opened with
//! OpenView), a CardPart and a ListPart, and Cancel on a StandardDialog and a ConfirmationDialog; the dialogs' OK/Cancel and
//! Yes/No actions report their own name (corpus branch al-runtime-rs-staging-y65,
//! testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al:505-626; al-runner issue 6670; assumption
//! close-action-ok-navigate-and-parts until that file is on the pinned corpus).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69990 "CAN Row" { fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 69990 "CAN Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
}
page 69990 "CAN Nav"
{
    PageType = NavigatePage; SourceTable = "CAN Row"; SourceTableTemporary = true; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { } } }
    actions { area(Processing) { action(ActionNext) { InFooterBar = true; trigger OnAction() begin Rec.Name := 'Next'; Rec.Modify(); end; } } }
    trigger OnOpenPage() begin Rec.Init(); Rec."Code" := 'W'; Rec.Insert(); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
    trigger OnClosePage() var G: Codeunit "CAN Log"; begin G.Add('close'); end;
}
page 69991 "CAN Nav Real"
{
    PageType = NavigatePage; SourceTable = "CAN Row"; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
}
page 69992 "CAN CardPart"
{
    PageType = CardPart; SourceTable = "CAN Row"; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
}
page 69993 "CAN ListPart"
{
    PageType = ListPart; SourceTable = "CAN Row"; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field(Name; Rec.Name) { } } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
}
page 69994 "CAN Std"
{
    PageType = StandardDialog; SourceTable = "CAN Row"; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
}
page 69995 "CAN Conf"
{
    PageType = ConfirmationDialog; SourceTable = "CAN Row"; UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var G: Codeunit "CAN Log"; begin G.Add('qc ' + Format(CloseAction)); exit(true); end;
}
codeunit 69996 C
{
    procedure Nav(): Text var P: TestPage "CAN Nav"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure NavFooter(): Text var P: TestPage "CAN Nav"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.ActionNext.Invoke(); P.Close(); exit(G.Take()); end;
    procedure NavView(): Text var P: TestPage "CAN Nav"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenView(); P.Close(); exit(G.Take()); end;
    procedure NavReal(): Text var P: TestPage "CAN Nav Real"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure CardPart(): Text var P: TestPage "CAN CardPart"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure ListPart(): Text var P: TestPage "CAN ListPart"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure Std(): Text var P: TestPage "CAN Std"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure StdOK(): Text var P: TestPage "CAN Std"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.OK().Invoke(); exit(G.Take()); end;
    procedure StdCancel(): Text var P: TestPage "CAN Std"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Cancel().Invoke(); exit(G.Take()); end;
    procedure Conf(): Text var P: TestPage "CAN Conf"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Close(); exit(G.Take()); end;
    procedure ConfYes(): Text var P: TestPage "CAN Conf"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.Yes().Invoke(); exit(G.Take()); end;
    procedure ConfNo(): Text var P: TestPage "CAN Conf"; G: Codeunit "CAN Log"; begin G.Take(); P.OpenEdit(); P.No().Invoke(); exit(G.Take()); end;
}
"#;

fn log(routine: &str) -> String {
    text(run_in(SRC, "C", routine))
}

#[test]
fn close_of_a_navigate_page_reports_ok() {
    // Y65ClosePageTypes.al:505 Y65_Close_Navigate, :518 _AfterFooterAction, :532 _OpenView.
    assert_eq!(log("Nav"), "qc OK;close;");
    assert_eq!(log("NavFooter"), "qc OK;close;");
    assert_eq!(log("NavView"), "qc OK;close;");
    // :545 Y65_Close_NavigateRealTable.
    assert_eq!(log("NavReal"), "qc OK;");
}

#[test]
fn close_of_a_card_part_and_a_list_part_reports_ok() {
    // Y65ClosePageTypes.al:636 Y65_Close_CardPart, :649 Y65_Close_ListPart.
    assert_eq!(log("CardPart"), "qc OK;");
    assert_eq!(log("ListPart"), "qc OK;");
}

#[test]
fn the_dialogs_keep_cancel_on_close_and_report_their_actions() {
    // Y65ClosePageTypes.al:558-634.
    assert_eq!(log("Std"), "qc Cancel;");
    assert_eq!(log("StdOK"), "qc OK;");
    assert_eq!(log("StdCancel"), "qc Cancel;");
    assert_eq!(log("Conf"), "qc Cancel;");
    assert_eq!(log("ConfYes"), "qc Yes;");
    assert_eq!(log("ConfNo"), "qc No;");
}
