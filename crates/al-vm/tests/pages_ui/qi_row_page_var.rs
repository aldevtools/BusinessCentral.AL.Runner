//! corpus pages/row-page-variable/QiRowPageVar.al (codeunit 86864): a repeater control over a page variable shows, on every row,
//! what the variable held after that row's OnAfterGetRecord (and OnAfterGetCurrRecord for the current row). The fixture objects
//! are the corpus file's.

use al_testkit::text;

const SRC: &str = r#"
table 86860 "LT QI RV Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; Id; Integer) { }
        field(2; "Host Code"; Code[10]) { }
        field(3; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 86865 "LT QI RV Host"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Code"; Code[10]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

page 86861 "LT QI RV List"
{
    PageType = List;
    SourceTable = "LT QI RV Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(IdCtl; Rec.Id) { ApplicationArea = All; }
                field(NameCtl; Rec.Name) { ApplicationArea = All; }
                field(AgrVarCtl; AgrVar) { ApplicationArea = All; Editable = false; }
                field(CurVarCtl; CurVar) { ApplicationArea = All; Editable = false; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SetVarAct)
            {
                ApplicationArea = All;
                Caption = 'Set variables';

                trigger OnAction()
                begin
                    AgrVar := 'action';
                    CurVar := 'action';
                end;
            }
            action(SetVarUpdateAct)
            {
                ApplicationArea = All;
                Caption = 'Set variables and update';

                trigger OnAction()
                begin
                    AgrVar := 'action';
                    CurVar := 'action';
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        AgrVar := 'agr ' + Format(Rec.Id);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurVar := 'cur ' + Format(Rec.Id);
    end;

    var
        AgrVar: Text;
        CurVar: Text;
}

page 86862 "LT QI RV Part"
{
    PageType = ListPart;
    SourceTable = "LT QI RV Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(IdCtl; Rec.Id) { ApplicationArea = All; }
                field(NameCtl; Rec.Name) { ApplicationArea = All; }
                field(AgrVarCtl; AgrVar) { ApplicationArea = All; Editable = false; }
                field(CurVarCtl; CurVar) { ApplicationArea = All; Editable = false; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SetVarAct)
            {
                ApplicationArea = All;
                Caption = 'Set variables';

                trigger OnAction()
                begin
                    AgrVar := 'action';
                    CurVar := 'action';
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        AgrVar := 'agr ' + Format(Rec.Id);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurVar := 'cur ' + Format(Rec.Id);
    end;

    var
        AgrVar: Text;
        CurVar: Text;
}

page 86863 "LT QI RV Host Card"
{
    PageType = Card;
    SourceTable = "LT QI RV Host";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(CodeCtl; Rec."Code") { ApplicationArea = All; }
            }
            part(Lines; "LT QI RV Part")
            {
                ApplicationArea = All;
                SubPageLink = "Host Code" = field("Code");
            }
        }
    }
}
codeunit 86864 "QI RV Test"
{
    local procedure Seed()
    var
        Row: Record "LT QI RV Row";
        Host: Record "LT QI RV Host";
        i: Integer;
    begin
        Row.DeleteAll();
        Host.DeleteAll();
        Host."Code" := 'H';
        Host.Insert();
        for i := 1 to 3 do begin
            Row.Init();
            Row.Id := i;
            Row."Host Code" := 'H';
            Row.Name := 'row ' + Format(i);
            Row.Insert();
        end;
    end;

    local procedure ReadPart(var Host: TestPage "LT QI RV Host Card"): Text
    begin
        exit(Host.Lines.IdCtl.Value() + ' | ' + Host.Lines.AgrVarCtl.Value() + ' | ' + Host.Lines.CurVarCtl.Value() + ';');
    end;

    procedure PartNavigation(): Text
    var
        Row: Record "LT QI RV Row";
        Host: TestPage "LT QI RV Host Card";
        T: Text;
    begin
        Seed();
        Host.OpenView();
        T := ReadPart(Host);
        Host.Lines.First();
        T += ReadPart(Host);
        Host.Lines.Next();
        T += ReadPart(Host);
        Host.Lines.Last();
        T += ReadPart(Host);
        Host.Lines.Previous();
        T += ReadPart(Host);
        Row.Get(1);
        Host.Lines.GoToRecord(Row);
        T += ReadPart(Host);
        Host.Lines.Next();
        Host.Lines.SetVarAct.Invoke();
        T += ReadPart(Host);
        Host.Lines.Next();
        T += ReadPart(Host);
        Host.Close();
        exit(T);
    end;
}
"#;

/// QiRowPageVar.al:297 QI_RV_Part_Navigation.
#[test]
fn each_part_row_shows_its_own_page_variable_values() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "QI RV Test", "PartNavigation")),
        "1 | agr 1 | cur 1;1 | agr 1 | cur 1;2 | agr 2 | cur 2;3 | agr 3 | cur 3;2 | agr 2 | cur 2;1 | agr 1 | cur 1;2 | agr 2 | cur 2;3 | agr 3 | cur 3;"
    );
}
