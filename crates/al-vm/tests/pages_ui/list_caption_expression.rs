//! The caption of a List or ListPart with `DataCaptionExpression` (gap G5-18), pinned by
//! pages/lt-y50-list-caption-expr/Y50ListCaptionExpr.al: `<caption> - <value>` evaluated at each read; an empty value or no
//! row shows the caption alone; page DataCaptionFields add nothing; a ListPart shows the value alone.

const SRC: &str = r#"
table 61960 "LCE Item"
{
    DataCaptionFields = "No.";
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
table 61961 "LCE Host"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
page 61962 "LCE Global"
{
    PageType = List; SourceTable = "LCE Item"; DataCaptionExpression = CaptionTxt; DataCaptionFields = Name;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
    var
        CaptionTxt: Text;
    procedure SetCaption(T: Text)
    begin
        CaptionTxt := T;
    end;
}
page 61963 "LCE Row"
{
    PageType = List; SourceTable = "LCE Item"; DataCaptionExpression = 'Row ' + Rec.Name;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
}
page 61964 "LCE Part"
{
    PageType = ListPart; SourceTable = "LCE Item"; DataCaptionExpression = 'Part ' + Rec.Name;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
}
page 61965 "LCE Card"
{
    PageType = Card; SourceTable = "LCE Host";
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "LCE Part") { } } }
}
codeunit 61966 C
{
    local procedure Seed()
    var
        I: Record "LCE Item";
        H: Record "LCE Host";
    begin
        I."No." := 'A1';
        I.Name := 'Alpha';
        I.Insert();
        I."No." := 'A2';
        I.Name := 'Beta';
        I.Insert();
        H.Code := 'H1';
        H.Insert();
    end;

    procedure Run(): Text
    var
        I: Record "LCE Item";
        Pg: Page "LCE Global";
        Pg2: Page "LCE Global";
        G: TestPage "LCE Global";
        R: TestPage "LCE Row";
        H: TestPage "LCE Card";
        T: Text;
    begin
        Seed();
        Pg.SetCaption('X1');
        G.Trap();
        Pg.Run();
        T := G.Caption();
        G.Close();
        G.Trap();
        Pg2.Run();
        T += '|' + G.Caption();
        G.Close();
        R.OpenView();
        T += '|' + R.Caption();
        R.Next();
        T += '|' + R.Caption();
        R.Close();
        H.OpenView();
        T += '|' + H.Lines.Caption();
        H.Close();
        I.DeleteAll();
        R.OpenView();
        T += '|' + R.Caption();
        R.Close();
        exit(T);
    end;
}
"#;

#[test]
fn a_list_caption_shows_its_data_caption_expression() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Run")),
        "LCE Global - X1|LCE Global|LCE Row - Row Alpha|LCE Row - Row Beta|Part Alpha|LCE Row"
    );
}
