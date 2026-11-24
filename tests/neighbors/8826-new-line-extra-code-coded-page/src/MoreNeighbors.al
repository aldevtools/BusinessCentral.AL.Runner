// More neighbors of ListNewLineExtraCodeCodedPage.al (review of al-runner PR 8834, 21:06Z): pages of the probe's shape
// with one change each, and other opens and calls. Each test holds Business Central's answer, measured on a local BC 29.1
// container (3 runs of 3), and runs in a codeunit of its own.

// Point 1, first shape: a key of (Code, Integer) and no code.
table 96876 "LT NG3 Plain Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Name; Text[30])
        {
        }
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Line No.") { Clustered = true; }
    }
}

page 96877 "LT NG3 Plain Info"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Plain Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

page 96878 "LT NG3 Plain Picture"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Plain Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

page 96879 "LT NG3 Plain Usage"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Plain Line";

    layout
    {
        area(Content)
        {
            field(Mark; Rec.Mark) { ApplicationArea = All; }
        }
    }
}

page 96880 "LT NG3 Plain Lines"
{
    PageType = List;
    SourceTable = "LT NG3 Plain Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG3 Plain Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
            part(Picture; "LT NG3 Plain Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
            part(Usage; "LT NG3 Plain Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    var
        Opened: Boolean;
        Touched: Boolean;
}

// Point 1, second shape: a key of (Code, AutoIncrement Integer), a table field OnValidate that sets another field and a control OnValidate that sets a variable: no insert, no save.
table 96882 "LT NG3 Auto Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Line No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            begin
                Note := 'Set';
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Line No.") { Clustered = true; }
    }
}

page 96883 "LT NG3 Auto Info"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Auto Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

page 96884 "LT NG3 Auto Picture"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Auto Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

page 96885 "LT NG3 Auto Usage"
{
    PageType = CardPart;
    SourceTable = "LT NG3 Auto Line";

    layout
    {
        area(Content)
        {
            field(Mark; Rec.Mark) { ApplicationArea = All; }
        }
    }
}

page 96886 "LT NG3 Auto Lines"
{
    PageType = List;
    SourceTable = "LT NG3 Auto Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Touched := true;
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG3 Auto Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
            part(Picture; "LT NG3 Auto Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
            part(Usage; "LT NG3 Auto Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Line No." = field("Line No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    var
        Opened: Boolean;
        Touched: Boolean;
}

// The repro page with DelayedInsert.
page 96890 "LT NG3 Delayed Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page with a SourceTableView in another order.
page 96892 "LT NG3 Desc Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    SourceTableView = sorting("Group Code", "Entry No.") order(descending);
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page without OnDeleteRecord.
page 96894 "LT NG3 No Delete Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's first FactBox without OnAfterGetRecord.
page 96896 "LT NG3 Quiet Info"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Mark; Rec.Mark) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    var
        Info: Text;
}

// The repro page whose first FactBox declares no trigger.
page 96897 "LT NG3 Quiet Part Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG3 Quiet Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's second FactBox, not editable.
page 96899 "LT NG3 Locked Picture"
{
    PageType = CardPart;
    Editable = false;
    SourceTable = "LT NZ Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearValue)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Value := '';
                    Rec.Modify();
                end;
            }
        }
    }
}

// The repro page whose second FactBox declares Editable = false.
page 96900 "LT NG3 Locked Part Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NG3 Locked Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose first part control has a Caption.
page 96902 "LT NG3 Captioned Part Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                Caption = 'Details';
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose OnOpenPage writes another table.
page 96904 "LT NG3 Open Writer Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    var
        Detail: Record "LT NZ Detail";
    begin
        Opened := true;
        Detail.Init();
        Detail.Text := 'Opened';
        Detail.Insert();
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's third FactBox, whose OnAfterGetCurrRecord also writes another table on the blank record.
page 96906 "LT NG3 Writer Usage"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

    layout
    {
        area(Content)
        {
            field(Usage; Usage) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
        Detail: Record "LT NZ Detail";
    begin
        Usage := '';
        if Rec."Entry No." = 0 then begin
            Detail.Init();
            Detail.Text := 'Blank';
            Detail.Insert();
            exit;
        end;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NZ Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The repro page whose third FactBox writes another table at the open.
page 96907 "LT NG3 Open Part Writer Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NG3 Writer Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose control sets a field of Rec between the save and the Update.
page 96909 "LT NG3 Late Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Rec.Mark := 'Late';
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose control writes another table between the save and the Update.
page 96911 "LT NG3 Between Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Detail: Record "LT NZ Detail";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Detail.Init();
                        Detail.Text := 'Between';
                        Detail.Insert();
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose first action has a Caption.
page 96913 "LT NG3 Action Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;
                Caption = 'Mark the line';

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's third FactBox, which enqueues no background task.
page 96915 "LT NG3 Idle Usage"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

    layout
    {
        area(Content)
        {
            field(Usage; Usage) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
    begin
        Usage := '';
        if Rec."Entry No." = 0 then
            exit;
        Usage := Format(Rec."Entry No." <> 0);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The repro page whose third FactBox enqueues no background task.
page 96916 "LT NG3 Idle Part Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NG3 Idle Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro page whose control calls CurrPage.Update(true) instead of the save.
page 96920 "LT NG3 Update True Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.Update(true);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's table with a secondary key on Name.
table 96923 "LT NG4 Keyed Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            begin
                if "Entry No." = 0 then
                    Insert(true);
                CopyDefaults();
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
        key(ByName; Name) { }
    }

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NZ Detail";
        Index: Integer;
    begin
        for Index := 1 to 2 do begin
            Detail.Init();
            Detail."Entry No." := 0;
            Detail."Line Entry No." := "Entry No.";
            Detail.Text := Name + Format(Index);
            Detail.Insert();
        end;
    end;
}

// The repro's first FactBox over the table with a secondary key.
page 96924 "LT NG4 Keyed Info"
{
    PageType = CardPart;
    SourceTable = "LT NG4 Keyed Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Mark; Rec.Mark) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

// The repro's second FactBox over the table with a secondary key.
page 96925 "LT NG4 Keyed Picture"
{
    PageType = CardPart;
    SourceTable = "LT NG4 Keyed Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearValue)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Value := '';
                    Rec.Modify();
                end;
            }
        }
    }
}

// The repro's third FactBox over the table with a secondary key.
page 96926 "LT NG4 Keyed Usage"
{
    PageType = CardPart;
    SourceTable = "LT NG4 Keyed Line";

    layout
    {
        area(Content)
        {
            field(Usage; Usage) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
    begin
        Usage := '';
        if Rec."Entry No." = 0 then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NZ Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The repro page over the table with a secondary key.
page 96927 "LT NG4 Keyed Lines"
{
    PageType = List;
    SourceTable = "LT NG4 Keyed Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG4 Keyed Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NG4 Keyed Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NG4 Keyed Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NG4 Keyed Line")
    var
        Other: Record "LT NG4 Keyed Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}


// Point 1, first shape. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96881 "LT NG3 Plain Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PlainPageNameThenOK_SavesTheRow()
    var
        Line: Record "LT NG3 Plain Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Plain Lines", Line);
        Check('all', 'r1=A/// | rows=G1:A///;| details=', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Plain Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG3 Plain Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// Point 1, second shape. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96887 "LT NG3 Auto Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageWithCodeThatNeitherInsertsNorSaves_SavesTheRow()
    var
        Line: Record "LT NG3 Auto Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Auto Lines", Line);
        Check('all', 'r1=A/Set// | rows=G1:A/Set//;| details=', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Auto Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG3 Auto Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// The probe's repro page: Previous(), then Next(), after the Value SetValue. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96888 "LT NG3 Prev Next Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PreviousThenNextAfterTheValue_SavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NZ Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/V previous=No next=Yes r2=/// | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Shown += 'previous=' + Format(Lines.Previous()) + ' ';
        Shown += 'next=' + Format(Lines.Next()) + ' ';
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// The probe's repro page: Close() after the Value SetValue. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96889 "LT NG3 Close Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_CloseAfterTheValue_RunModalFails()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        asserterror Page.RunModal(Page::"LT NZ Lines", Line);
        Errors := '| error=' + GetLastErrorText();
        Check('all', 'r1=A/Resolved/Stamped/V | rows=| details=| error=The RunModal procedure could not close the page 96853 as it has already been closed.', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Close();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A page property the cells do not declare. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96891 "LT NG3 Delayed Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageWithDelayedInsert_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Delayed Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Delayed Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A sort order the cells do not have. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96893 "LT NG3 Desc Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageWithDescendingView_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Desc Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Desc Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A subset of the cells' page triggers. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96895 "LT NG3 No Delete Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageWithoutOnDeleteRecord_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 No Delete Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 No Delete Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// Other trigger names on a part. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96898 "LT NG3 Quiet Part Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartWithoutTrigger_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Quiet Part Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Quiet Part Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A part page property the cells do not declare. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96901 "LT NG3 Locked Part Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartPageProperty_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Locked Part Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Locked Part Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A part control property the cells do not declare. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96903 "LT NG3 Captioned Part Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartControlProperty_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Captioned Part Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Captioned Part Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// OnOpenPage writes another table. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96905 "LT NG3 Open Writer Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_OnOpenPageWritesAnotherTable_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Open Writer Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=Opened;A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Open Writer Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A part trigger writes another table at the open. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96908 "LT NG3 Open Part Writer Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartTriggerAtTheOpenWritesAnotherTable_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Open Part Writer Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=Blank;Blank;A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Open Part Writer Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// A field of Rec set between the save and the Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96910 "LT NG3 Late Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_FieldOfRecSetBetweenSaveAndUpdate_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Late Lines", Line);
        Check('all', 'r1=A/Resolved// r2=A/Resolved//V | rows=G1:A/Resolved//V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Late Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// Another table written between the save and the Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96912 "LT NG3 Between Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_AnotherTableWrittenBetweenSaveAndUpdate_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Between Lines", Line);
        Check('all', 'r1=A/Resolved// r2=A/Resolved//V | rows=G1:A/Resolved//V;| details=A1;A2;Between;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Between Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// An action property the cells do not declare. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96914 "LT NG3 Action Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ActionProperty_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Action Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Action Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// No part enqueues a page background task. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96917 "LT NG3 Idle Part Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartEnqueuesNoBackgroundTask_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Idle Part Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Idle Part Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// The repro page opened by OpenEdit. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96918 "LT NG3 Open Edit Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    procedure NewLine_OpenEdit_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
        Lines: TestPage "LT NZ Lines";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Close();
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// The repro page opened by Page.RunModal on a record without a filter. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96919 "LT NG3 Unfiltered Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_RunModalWithoutFilter_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Page.RunModal(Page::"LT NZ Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// CurrPage.Update(true) on this page. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96921 "LT NG3 Update True Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_UpdateTrue_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG3 Update True Lines", Line);
        Check('all', 'r1=A/Resolved// r2=A/Resolved//V | rows=G1:A/Resolved//V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG3 Update True Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// The logging page whose control's OnValidate does nothing (mode 0). Its own codeunit, so that no earlier refused test hides its result.
codeunit 96922 "LT NG3 Idle Control Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_LoggingPageControlDoesNothing_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NZ Var Lines", Line);
        Check('all', 'r1=A/// r2=A///V | rows=G1:A///;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// Page.RunModal on a record in the secondary key on Name. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96928 "LT NG4 Keyed Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_RecordInASecondaryKey_ShowsAndSavesTheRow()
    var
        Line: Record "LT NG4 Keyed Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetCurrentKey(Name);
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NG4 Keyed Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG4 Keyed Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG4 Keyed Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}


// Page.RunModal on a record sorted by Note, a field no key begins with. Its own codeunit, so that no earlier refused test hides its result.
codeunit 96929 "LT NG4 Ad Hoc Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;
        Errors: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_RecordInAnAdHocSortOrder_ShowsAndSavesTheRow()
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
        Errors := '';
        Line.SetCurrentKey(Note);
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NZ Lines", Line);
        Check('all', 'r1=A/Resolved/Stamped/ r2=A/Resolved/Stamped/V | rows=G1:A/Resolved/Stamped/V;| details=A1;A2;', Shown + '| rows=' + Rows() + '| details=' + Details() + Errors);
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'r1=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'r2=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NZ Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
