//! The reporters' repros of al-runner issues 7402 and 7481, generalized as the issues give them: the shapes the pinned corpus
//! covers run, the neighbors it does not cover stay refused (crates/al-vm/src/pages/split_key_view.rs, `auto_split_part_pinned`).

use al_testrun::Outcome;

fn outcome_of(src: &str, name: &str) -> Outcome {
    al_testkit::outcomes_in(src, "C")
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, o)| o)
        .unwrap_or_else(|| panic!("no test {name}"))
}

/// Issue 7402: a Card run by Page.RunModal; its ListPart has AutoSplitKey and a SubPageView in a secondary key; the handler
/// adds a row on the empty part, types three values and invokes OK. `RK Process Doc` is the same host as a Document page.
const SRC_7402: &str = r#"
table 69960 "RK Item" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 69961 "RK Process Line"
{
    fields
    {
        field(1; "Item No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Sorting Sequence No."; Integer) { }
        field(4; "Line Type"; Text[30]) { }
        field(5; "No."; Code[20]) { }
        field(6; "Quantity per"; Decimal) { }
    }
    keys { key(PK; "Item No.", "Line No.") { } key(Seq; "Item No.", "Sorting Sequence No.") { } }
}
page 69962 "RK Process Sub"
{
    PageType = ListPart; SourceTable = "RK Process Line"; AutoSplitKey = true;
    layout { area(Content) { repeater(R) { field("Line Type"; Rec."Line Type") { } field("No."; Rec."No.") { } field("Quantity per"; Rec."Quantity per") { } } } }
}
page 69963 "RK Process"
{
    PageType = Card; SourceTable = "RK Item";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            part(Lines; "RK Process Sub")
            {
                SubPageLink = "Item No." = field("No.");
                SubPageView = sorting("Item No.", "Sorting Sequence No.");
                UpdatePropagation = Both;
            }
        }
    }
}
page 69964 "RK Process Doc"
{
    PageType = Document; SourceTable = "RK Item";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RK Process Sub") { SubPageLink = "Item No." = field("No."); SubPageView = sorting("Item No.", "Sorting Sequence No."); UpdatePropagation = Both; } } }
}
codeunit 69965 C
{
    Subtype = Test;

    local procedure Item() Item: Record "RK Item"
    var
        Line: Record "RK Process Line";
    begin
        Line.DeleteAll();
        Item.DeleteAll();
        Item."No." := 'I1';
        Item.Insert();
    end;

    // The reporter's shape, the return value of RunModal used as pages/parts/PartSplitKeyOtherSortKey.al:584 uses it.
    [Test]
    [HandlerFunctions('Handler')]
    procedure Repro7402()
    var
        Line: Record "RK Process Line";
        R: Action;
    begin
        R := Page.RunModal(Page::"RK Process", Item());
        Line.SetRange("Item No.", 'I1');
        if Line.Count() <> 1 then
            Error('count %1', Line.Count());
    end;

    [ModalPageHandler]
    procedure Handler(var MyProcess: TestPage "RK Process")
    begin
        MyProcess.Lines.New();
        MyProcess.Lines."Line Type".Value('Prod. BOM');
        MyProcess.Lines."No.".Value('1896-S');
        MyProcess.Lines."Quantity per".Value('1');
        MyProcess.OK().Invoke();
    end;

    // Not pinned: Page.RunModal as a statement (the page is not in lookup mode).
    [Test]
    [HandlerFunctions('Handler')]
    procedure StatementRunModal()
    begin
        Page.RunModal(Page::"RK Process", Item());
    end;

    // Not pinned: a Document host.
    [Test]
    [HandlerFunctions('HandlerDoc')]
    procedure DocumentHost()
    var
        R: Action;
    begin
        R := Page.RunModal(Page::"RK Process Doc", Item());
    end;

    [ModalPageHandler]
    procedure HandlerDoc(var MyProcess: TestPage "RK Process Doc")
    begin
        MyProcess.Lines.New();
        MyProcess.Lines."Line Type".Value('Prod. BOM');
        MyProcess.Lines."No.".Value('1896-S');
        MyProcess.Lines."Quantity per".Value('1');
        MyProcess.OK().Invoke();
    end;
}
"#;

/// Issue 7481 as the reporter gives it, the host numbered by its table's OnInsert: the shape of pages/parts/
/// PartNewRowPageVariable.al:1944 DelayedSplitPop_NewDocument_RecordFieldsThenVariableWithUpdate with the reporter's host and part.
const SRC_7481: &str = r#"
enum 69970 "RK Type" { value(0; Basic) { } value(1; Extended) { } }
enum 69971 "RK Line Type" { value(0; None) { } value(1; A) { } }
table 69972 "RK Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; Type; Enum "RK Type") { } field(3; "Document Date"; Date) { } }
    keys { key(PK; "No.") { } }
    trigger OnInsert() begin if "No." = '' then "No." := 'D1'; end;
}
table 69973 "RK Line"
{
    fields
    {
        field(1; "Document No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Type; Enum "RK Line Type") { }
        field(4; Code; Code[20]) { }
        field(5; "Actual Value"; Text[250]) { }
    }
    keys { key(PK; "Document No.", "Line No.") { } }
}
page 69974 "RK Lines Part"
{
    PageType = ListPart; SourceTable = "RK Line"; AutoSplitKey = true; DelayedInsert = true; PopulateAllFields = true;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Type; Rec.Type) { }
                field(Code; Rec.Code) { }
                field("Actual Value"; ActValueTxt)
                {
                    trigger OnValidate()
                    begin
                        Rec.Validate("Actual Value", ActValueTxt);
                        CurrPage.Update();
                    end;
                }
            }
        }
    }
    trigger OnAfterGetRecord()
    begin
        ActValueTxt := Rec."Actual Value";
    end;
    var
        ActValueTxt: Text;
}
page 69975 "RK Document"
{
    PageType = Document; SourceTable = "RK Doc";
    layout
    {
        area(Content)
        {
            field(Type; Rec.Type) { }
            field("Document Date"; Rec."Document Date") { }
            part(Lines; "RK Lines Part") { Editable = LinesEditable; SubPageLink = "Document No." = field("No."); Visible = Rec.Type = Rec.Type::Extended; }
        }
    }
    trigger OnAfterGetCurrRecord() begin LinesEditable := true; end;
    var LinesEditable: Boolean;
}
codeunit 69976 C
{
    Subtype = Test;

    [Test]
    procedure Repro7481()
    var
        MyDoc: TestPage "RK Document";
    begin
        MyDoc.OpenNew();
        MyDoc.Type.SetValue("RK Type"::Extended);
        MyDoc."Document Date".SetValue(WorkDate());
        MyDoc.Lines.New();
        MyDoc.Lines.Type.SetValue("RK Line Type"::A);
        MyDoc.Lines.Code.SetValue('E1');
        MyDoc.Lines."Actual Value".SetValue('1');
    end;
}
"#;

#[test]
fn issue_7402_reporter_shape_passes() {
    assert_eq!(outcome_of(SRC_7402, "Repro7402"), Outcome::Pass);
}

#[test]
fn issue_7402_neighbors_the_corpus_does_not_pin_stay_refused() {
    for name in ["StatementRunModal", "DocumentHost"] {
        match outcome_of(SRC_7402, name) {
            Outcome::NotRun(why) => assert!(why.contains("not read in primary key order"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn issue_7481_reporter_shape_passes() {
    assert_eq!(outcome_of(SRC_7481, "Repro7481"), Outcome::Pass);
}
