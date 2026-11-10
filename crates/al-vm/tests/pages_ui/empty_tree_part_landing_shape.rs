//! The action of an empty tree part in another key than the primary key that leaves rows: the reads are pinned for one
//! shape only (pages/parts/R6889RealShape.al:1213 RunStep4, :1400 PG6889RS_Step4_; Part4 at :370, Host4 at :605): a ListPlus
//! host opened with OpenEdit, the part with UpdatePropagation = Both, an action that does not call CurrPage.Update. The same
//! action with CurrPage.Update, on a Card or Document host, without UpdatePropagation = Both or on a host opened for view
//! is not pinned and refused.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 60 "RT Doc" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 61 "RT Line"
{
    fields
    {
        field(1; "Header No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Order No."; Integer) { }
        field(4; Level; Integer) { }
        field(5; Name; Text[30]) { }
    }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } key(Ord; "Header No.", "Order No.") { } }
}
codeunit 62 "RT Log"
{
    SingleInstance = true;
    var
        L: Text;
    procedure Add(T: Text) begin if L <> '' then L += ';'; L += T; end;
    procedure Take() R: Text begin R := L; L := ''; end;
}
page 63 "RT Part"
{
    PageType = ListPart; SourceTable = "RT Line"; SourceTableView = sorting("Header No.", "Order No.");
    AutoSplitKey = true; InsertAllowed = false; DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                ShowAsTree = true; IndentationColumn = Rec.Level; TreeInitialState = ExpandAll;
                field(Level; Rec.Level) { }
                field("Line No."; Rec."Line No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Plain) { trigger OnAction() begin TwoRows(); end; }
            action(WithUpdateFalse) { trigger OnAction() begin TwoRows(); CurrPage.Update(false); end; }
            action(WithUpdateTrue) { trigger OnAction() begin TwoRows(); CurrPage.Update(true); end; }
        }
    }
    local procedure TwoRows()
    var
        Log: Codeunit "RT Log";
        H: Code[20];
    begin
        Log.Add('ACT');
        H := 'D2';
        Rec.Init(); Rec."Header No." := H; Rec."Line No." := 10000; Rec."Order No." := 10000; Rec.Insert();
        Rec.Init(); Rec."Header No." := H; Rec."Line No." := 20000; Rec."Order No." := 20000; Rec.Level := 1; Rec.Insert();
    end;
    trigger OnAfterGetRecord() var Log: Codeunit "RT Log"; begin Log.Add('R' + Format(Rec."Line No." div 1000)); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "RT Log"; begin Log.Add('C' + Format(Rec."Line No." div 1000)); end;
}
page 64 "RT Host Plus Both"
{
    PageType = ListPlus; SourceTable = "RT Doc";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RT Part") { SubPageLink = "Header No." = field("No."); UpdatePropagation = Both; } } }
}
page 65 "RT Host Card"
{
    PageType = Card; SourceTable = "RT Doc";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RT Part") { SubPageLink = "Header No." = field("No."); } } }
}
page 66 "RT Host Doc"
{
    PageType = Document; SourceTable = "RT Doc";
    layout { area(Content) { group(G) { field("No."; Rec."No.") { } } part(Lines; "RT Part") { SubPageLink = "Header No." = field("No."); } } }
}
page 67 "RT Host Plus"
{
    PageType = ListPlus; SourceTable = "RT Doc";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RT Part") { SubPageLink = "Header No." = field("No."); } } }
}
page 68 "RT Host Card Both"
{
    PageType = Card; SourceTable = "RT Doc";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RT Part") { SubPageLink = "Header No." = field("No."); UpdatePropagation = Both; } } }
}
codeunit 69 C
{
    local procedure Seed()
    var D: Record "RT Doc"; L: Record "RT Line";
    begin
        D.DeleteAll(); L.DeleteAll();
        D."No." := 'D2'; D.Insert();
    end;

    procedure PlusBoth_Plain(): Text
    var P: TestPage "RT Host Plus Both"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure PlusBoth_UpdateFalse(): Text
    var P: TestPage "RT Host Plus Both"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.WithUpdateFalse.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure PlusBoth_UpdateTrue(): Text
    var P: TestPage "RT Host Plus Both"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.WithUpdateTrue.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure Card_Plain(): Text
    var P: TestPage "RT Host Card"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure CardBoth_Plain(): Text
    var P: TestPage "RT Host Card Both"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure Doc_Plain(): Text
    var P: TestPage "RT Host Doc"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure Plus_Plain(): Text
    var P: TestPage "RT Host Plus"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
    procedure PlusBoth_View_Plain(): Text
    var P: TestPage "RT Host Plus Both"; Log: Codeunit "RT Log";
    begin
        Seed(); P.OpenView(); P.GoToKey('D2'); Log.Take();
        P.Lines.Plain.Invoke();
        exit(P.Lines."Line No.".Value() + '|' + Log.Take());
    end;
}
"#;

fn refused(routine: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains("an action of a tree part in another key than the primary key that showed no row before it"),
            "{routine}: {why}"
        ),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

// R6889RealShape.al:1400 PG6889RS_Step4_ `line after action` and the reads of `log action`.
#[test]
fn the_pinned_host_reads_the_rows_the_action_left() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "PlusBoth_Plain")),
        "10000|ACT;R20;R20;R20;R10;R20;R20;R20;C20;R10;R10;R10;C10"
    );
}

#[test]
fn an_action_that_calls_currpage_update_is_refused() {
    refused("PlusBoth_UpdateFalse");
    refused("PlusBoth_UpdateTrue");
}

#[test]
fn another_host_type_or_propagation_is_refused() {
    refused("Card_Plain");
    refused("CardBoth_Plain");
    refused("Doc_Plain");
    refused("Plus_Plain");
}

#[test]
fn a_host_opened_for_view_is_refused() {
    refused("PlusBoth_View_Plain");
}
