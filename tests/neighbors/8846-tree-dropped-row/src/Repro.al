table 50148 "Repro Tree Drop Row"
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

page 50149 "Repro Tree Drop List"
{
    PageType = List;
    SourceTable = "Repro Tree Drop Row";
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
}