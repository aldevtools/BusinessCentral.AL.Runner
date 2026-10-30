//! The caption of a page with `DataCaptionExpression`: the page caption, ` - ` and the expression's value, evaluated when
//! the caption is read. Pinned on a card with a source table (pages/lt-page-autoformat-lookups/P113DataCaption.al:83-88,
//! :264-279, P113_DC_DataCaptionExpression: `P113 Expr - EXPR-Alpha`, after GoToKey `... - EXPR-Beta`). An expression whose
//! value is empty, a page with no source table (al-runner issue 6320), a new row and a list page are not pinned and are
//! refused.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "DC Row" { fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "Code") { } } }
page 50100 "DC Card"
{
    PageType = Card; SourceTable = "DC Row"; UsageCategory = None; Caption = 'DC Expr';
    DataCaptionExpression = 'EXPR-' + Rec.Name;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } }
}
page 50101 "DC Editor"
{
    PageType = Card; Caption = 'Message Editor'; UsageCategory = None;
    DataCaptionExpression = GetCaption();
    layout { area(Content) { field(SubjectCtl; Subject) { } } }
    var Subject: Text;
    procedure SetSubject(NewSubject: Text) begin Subject := NewSubject; end;
    local procedure GetCaption(): Text begin exit(Subject); end;
}
page 50102 "DC List"
{
    PageType = List; SourceTable = "DC Row"; UsageCategory = None; Caption = 'DC List';
    DataCaptionExpression = 'EXPR-' + Rec.Name;
    layout { area(Content) { repeater(R) { field("Code"; Rec."Code") { } } } }
}
codeunit 50103 T
{
    Subtype = Test;
    var Expected: Text;
    local procedure Seed()
    var Row: Record "DC Row";
    begin
        Row.DeleteAll();
        Row."Code" := 'C1'; Row.Name := 'Alpha'; Row.Insert();
        Row."Code" := 'C2'; Row.Name := 'Beta'; Row.Insert();
    end;
    [Test] procedure CardWithRows()
    var Card: TestPage "DC Card"; First: Text;
    begin
        Seed();
        Card.OpenView();
        First := Card.Caption();
        Card.GoToKey('C2');
        if First + '|' + Card.Caption() <> 'DC Expr - EXPR-Alpha|DC Expr - EXPR-Beta' then
            Error('%1', First + '|' + Card.Caption());
        Card.Close();
    end;
    [Test] [HandlerFunctions('EditorHandler')]
    procedure EditorWithoutSourceTable()
    var Editor: Page "DC Editor";
    begin
        Expected := 'Message Editor - Hello';
        Editor.SetSubject('Hello');
        Editor.RunModal();
    end;
    [Test] [HandlerFunctions('EditorHandler')]
    procedure EmptyExpressionIsRefused()
    var Editor: Page "DC Editor";
    begin
        Expected := 'x';
        Editor.RunModal();
    end;
    [Test] procedure NewRowIsRefused()
    var Card: TestPage "DC Card";
    begin
        Seed();
        Card.OpenNew();
        if Card.Caption() = '' then;
    end;
    [Test] procedure ListIsRefused()
    var List: TestPage "DC List";
    begin
        Seed();
        List.OpenView();
        if List.Caption() = '' then;
    end;
    [ModalPageHandler]
    procedure EditorHandler(var Editor: TestPage "DC Editor")
    begin
        if Editor.Caption() <> Expected then
            Error('caption %1', Editor.Caption());
    end;
}
"#;

#[test]
fn the_caption_of_a_page_with_a_data_caption_expression() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("CardWithRows"), "Some(Pass)");
    for t in [
        "EditorWithoutSourceTable",
        "EmptyExpressionIsRefused",
        "NewRowIsRefused",
        "ListIsRefused",
    ] {
        let g = get(t);
        assert!(g.contains("NotRun") && g.contains("DataCaptionExpression"), "{t}: {g}");
    }
}
