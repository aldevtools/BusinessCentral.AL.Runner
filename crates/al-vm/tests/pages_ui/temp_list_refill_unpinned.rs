//! The read-only temporary List of `pages::temp_list_refill` (al-runner issue 7419): the corpus pins one page
//! (testpage/TempListExpressionRefill.al, page 66411), whose OnOpenPage and actions only refill Rec. A page whose OnOpenPage or
//! action also sets a key, an order or a filter, moves to a row, or changes a row without a refill is not that page: an
//! expression-bound control read after its action keeps the earlier refusal.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69949 "TLR Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Field No."; Integer) { } field(3; "Old Value"; Text[50]) { } }
    keys { key(PK; "Entry No.") { } key(ByField; "Field No.") { } }
    procedure GetLocalOld(): Text
    begin
        if "Old Value" = '' then exit('');
        exit('L:' + "Old Value");
    end;
}
page 69950 "TLR Pinned"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; }  } }
    trigger OnOpenPage() begin Fill('open');  end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69951 "TLR OpenKey"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; }  } }
    trigger OnOpenPage() begin Fill('open'); Rec.SetCurrentKey("Field No."); end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69952 "TLR OpenDesc"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; }  } }
    trigger OnOpenPage() begin Fill('open'); Rec.Ascending(false); end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69953 "TLR OpenFilter"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; }  } }
    trigger OnOpenPage() begin Fill('open'); Rec.SetFilter("Field No.", '>6'); end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69954 "TLR OpenFindFirst"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; }  } }
    trigger OnOpenPage() begin Fill('open'); Rec.FindFirst(); end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69955 "TLR ActKey"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other'); Rec.SetCurrentKey("Field No."); end; }  } }
    trigger OnOpenPage() begin Fill('open');  end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69956 "TLR ActFilter"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other'); Rec.SetRange("Field No.", 7, 9); end; }  } }
    trigger OnOpenPage() begin Fill('open');  end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
page 69957 "TLR ActChange"
{
    PageType = List; SourceTable = "TLR Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(EntryNoCtl; Rec."Entry No.") { } field(OldLocalCtl; Rec.GetLocalOld()) { } } } }
    actions { area(Processing) { action(ThreeOther) { trigger OnAction() begin Fill('three other');  end; } action(Change) { trigger OnAction() begin Rec.FindFirst(); Rec."Old Value" := 'chg'; Rec.Modify(); end; } } }
    trigger OnOpenPage() begin Fill('open');  end;
    local procedure Fill(Name: Text)
    begin
        if not Rec.IsEmpty() then Rec.DeleteAll();
        case Name of
            'open': begin AddRow(1, 7, 'a1'); AddRow(2, 8, 'a2'); end;
            'three other': begin AddRow(11, 7, 'm1'); AddRow(12, 9, 'm2'); AddRow(13, 7, 'm3'); end;
        end;
    end;
    local procedure AddRow(EntryNo: Integer; FieldNo: Integer; OldValue: Text[50])
    begin
        Rec.Init(); Rec."Entry No." := EntryNo; Rec."Field No." := FieldNo; Rec."Old Value" := OldValue; Rec.Insert();
    end;
}
codeunit 69960 C
{
    procedure Pinned(): Text
    var P: TestPage "TLR Pinned";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure OpenKey(): Text
    var P: TestPage "TLR OpenKey";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure OpenDesc(): Text
    var P: TestPage "TLR OpenDesc";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure OpenFilter(): Text
    var P: TestPage "TLR OpenFilter";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure OpenFindFirst(): Text
    var P: TestPage "TLR OpenFindFirst";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure ActKey(): Text
    var P: TestPage "TLR ActKey";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure ActFilter(): Text
    var P: TestPage "TLR ActFilter";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
    procedure ActChange(): Text
    var P: TestPage "TLR ActChange";
    begin
        P.OpenView();
        P.ThreeOther.Invoke();
        exit(P.OldLocalCtl.Value());
    end;
}
"#;

fn refused(routine: &str) {
    let r = run_in(SRC, "C", routine);
    assert!(
        matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
        "{routine}: {r:?}"
    );
}

/// testpage/TempListExpressionRefill.al:313-319 (the action ThreeOther: the control reads the row the refill left Rec on).
#[test]
fn the_pinned_page_serves_the_control_after_a_refill() {
    assert_eq!(text(run_in(SRC, "C", "Pinned")), "L:m3");
}

#[test]
fn a_key_set_in_onopenpage_keeps_the_refusal() {
    refused("OpenKey");
}

#[test]
fn a_descending_order_set_in_onopenpage_keeps_the_refusal() {
    refused("OpenDesc");
}

#[test]
fn a_filter_set_in_onopenpage_keeps_the_refusal() {
    refused("OpenFilter");
}

#[test]
fn a_find_in_onopenpage_keeps_the_refusal() {
    refused("OpenFindFirst");
}

#[test]
fn a_key_set_in_the_action_keeps_the_refusal() {
    refused("ActKey");
}

#[test]
fn a_filter_set_in_the_action_keeps_the_refusal() {
    refused("ActFilter");
}

#[test]
fn an_action_that_modifies_a_row_without_a_refill_keeps_the_refusal() {
    refused("ActChange");
}
