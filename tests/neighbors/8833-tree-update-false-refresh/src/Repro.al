table 50105 "Repro Tree Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { }
        field(3; Name; Text[30]) { }
        field(4; Indentation; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 50109 "Repro Tree Log"
{
    SingleInstance = true;

    var
        Buffer: Text;

    procedure Add(Entry: Text)
    begin
        Buffer += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buffer;
        Buffer := '';
        exit(Result);
    end;
}

page 50108 "Repro Tree List"
{
    PageType = List;
    SourceTable = "Repro Tree Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = ExpandAll;

                field(Name; Rec.Name)
                {
                    trigger OnValidate()
                    begin
                        Rec.SetRange("Group Code", 'G1');
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "Repro Tree Log";
}
