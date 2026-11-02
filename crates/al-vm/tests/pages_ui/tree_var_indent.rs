//! A tree repeater whose IndentationColumn is a page variable OnAfterGetRecord sets, with no TreeInitialState: it opens collapsed
//! after reading every row, walks the level-0 rows, and Expand(true) shows a row's children (corpus
//! pages/lt-y72-page-probes/Y72TreeDefaultState.al).

use al_testkit::text;

const SRC: &str = r#"
table 69990 "TV Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69991 "TV Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "TV Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "TV Log"; R: Text;
    begin
        if L.FindSet() then
            repeat
                if R <> '' then R += ';';
                R += L.Txt;
            until L.Next() = 0;
        L.DeleteAll();
        exit(R);
    end;
}
table 69992 "TV Row" { fields { field(1; "Line No."; Integer) { } field(2; Indent; Integer) { } } keys { key(PK; "Line No.") { } } }
page 69993 "TV Tree"
{
    PageType = List; SourceTable = "TV Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                IndentationColumn = IndentVar; IndentationControls = LineCtl; ShowAsTree = true;
                field(LineCtl; Rec."Line No.") { }
            }
        }
    }
    var IndentVar: Integer;
    trigger OnAfterGetRecord() var L: Codeunit "TV Log Mgt"; begin IndentVar := Rec.Indent; L.Add('agr ' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "TV Log Mgt"; begin L.Add('agcr ' + Format(Rec."Line No.")); end;
}
page 69995 "TV Curr"
{
    PageType = List; SourceTable = "TV Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                IndentationColumn = Rec.Indent; IndentationControls = LineCtl; ShowAsTree = true;
                field(LineCtl; Rec."Line No.") { }
            }
        }
    }
    trigger OnAfterGetCurrRecord() var L: Codeunit "TV Log Mgt"; begin L.Add('agcr ' + Format(Rec."Line No.")); end;
}
codeunit 69994 C
{
    local procedure Add(Line: Integer; Indent: Integer)
    var R: Record "TV Row";
    begin
        R."Line No." := Line; R.Indent := Indent; R.Insert();
    end;
    local procedure Walk(var P: TestPage "TV Tree"): Text
    var W: Text;
    begin
        P.First();
        W := P.LineCtl.Value();
        while P.Next() do
            W += ',' + P.LineCtl.Value();
        exit(W);
    end;
    procedure CurrOnly(): Text
    var P: TestPage "TV Curr"; L: Codeunit "TV Log Mgt"; R: Text;
    begin
        Add(10, 0); Add(20, 1); Add(30, 1); Add(40, 0); Add(50, 1); Add(60, 0);
        L.Take();
        P.OpenView();
        R := L.Take();
        P.First();
        R += '|' + L.Take();
        P.Expand(true);
        R += '|' + L.Take();
        exit(R);
    end;
    procedure Run(): Text
    var P: TestPage "TV Tree"; L: Codeunit "TV Log Mgt"; R: Text;
    begin
        Add(10, 0); Add(20, 1); Add(30, 1); Add(40, 0); Add(50, 1); Add(60, 0);
        L.Take();
        P.OpenView();
        R := L.Take() + '|' + Format(P.IsExpanded());
        R += '|' + Walk(P);
        P.First();
        L.Take();
        P.Expand(true);
        R += '|' + L.Take() + '|' + Format(P.IsExpanded()) + '|' + Walk(P);
        exit(R);
    end;
}
"#;

#[test]
fn a_tree_over_a_page_variable_opens_collapsed_and_expands() {
    // Y72TreeDefaultState.al TreeDefault_OpenShows, TreeDefault_ExpandCollapse_Row10.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Run")),
        "agr 10;agr 20;agr 30;agr 40;agr 50;agr 60;agr 10;agr 10;agcr 10|No|10,40,60||Yes|10,20,30,40,60"
    );
}

#[test]
fn a_tree_with_only_on_after_get_curr_record_reads_nothing_on_first() {
    // Y72TreeDefaultState.al TreeDefault_RowTriggerVariants_OnAfterGetCurrRecordOnly `first log` empty.
    assert_eq!(text(al_testkit::run_in(SRC, "C", "CurrOnly")), "agcr 10||");
}
