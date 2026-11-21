// The repro of al-runner issue 8434 (its app and test files), green on BC 29.1 (1/1).
table 50100 "Repro Attr Header"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Calc Count"; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50101 "Repro Attr Setup"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Lock Values"; Boolean) { }
        field(3; "Custom 1 Caption"; Text[30]) { }
    }

    keys
    {
        key(PK; "Primary Key") { Clustered = true; }
    }
}

table 50102 "Repro Attr Kind"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; ID; Integer) { }
        field(2; Name; Text[250]) { }
        field(3; Blocked; Boolean) { }
    }

    keys
    {
        key(PK; ID) { Clustered = true; }
    }
}

table 50103 "Repro Attr Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Header No."; Code[20])
        {
            TableRelation = "Repro Attr Header"."No.";
        }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; "Attribute Name"; Text[250])
        {
            NotBlank = true;
            TableRelation = "Repro Attr Kind".Name where(Blocked = const(false));
            ValidateTableRelation = false;

            trigger OnValidate()
            begin
                if "Entry No." = 0 then
                    Insert(true);
                CopyKind();
            end;
        }
        field(4; "Attribute Value"; Text[250])
        {
            trigger OnValidate()
            var
                Change: Record "Repro Attr Change";
            begin
                if "Attribute Value" <> xRec."Attribute Value" then begin
                    Variant := '';
                    Change.Init();
                    Change."Header No." := "Header No.";
                    Change."Line Entry No." := "Entry No.";
                    Change."Old Value" := xRec."Attribute Value";
                    Change."New Value" := "Attribute Value";
                    Change.Insert();
                end;
            end;
        }
        field(5; "Kind ID"; Integer) { }
        field(6; "Sorting"; Integer) { }
        field(7; Variant; Code[10]) { }
        field(8; Image; MediaSet) { }
        field(9; Style; Text[30]) { }
        field(10; Quantity; Decimal) { }
        field(11; "Has Quantity"; Boolean) { }
        field(12; "Kind Type"; Option)
        {
            OptionMembers = Text,Option;
        }
        field(13; Description; Text[100]) { }
        field(14; "Show Blank"; Boolean) { }
    }

    keys
    {
        key(PK; "Header No.", "Entry No.") { Clustered = true; }
        key(BySorting; "Header No.", "Sorting") { }
    }

    trigger OnInsert()
    var
        Last: Record "Repro Attr Line";
    begin
        Last.SetCurrentKey("Sorting");
        Last.SetRange("Header No.", "Header No.");
        Last.SetLoadFields("Sorting");
        if Last.FindLast() then
            "Sorting" := Last."Sorting" + 10;
    end;

    trigger OnDelete()
    var
        Change: Record "Repro Attr Change";
    begin
        Change.SetRange("Header No.", "Header No.");
        Change.SetRange("Line Entry No.", "Entry No.");
        Change.DeleteAll();
    end;

    local procedure CopyKind()
    var
        Kind: Record "Repro Attr Kind";
    begin
        Kind.SetRange(Name, "Attribute Name");
        if Kind.FindFirst() then begin
            "Kind ID" := Kind.ID;
            "Kind Type" := "Kind Type"::Option;
        end;
    end;
}

table 50112 "Repro Attr Value"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Kind ID"; Integer) { }
        field(2; Value; Text[250]) { }
        field(3; Blocked; Boolean) { }
    }

    keys
    {
        key(PK; "Kind ID", Value) { Clustered = true; }
    }
}

table 50104 "Repro Attr Change"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(2; "Header No."; Code[20]) { }
        field(3; "Line Entry No."; Integer) { }
        field(4; "Old Value"; Text[250]) { }
        field(5; "New Value"; Text[250]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

page 50105 "Repro Attr Rules"
{
    Caption = ' ', Locked = true;
    Editable = false;
    PageType = CardPart;
    SourceTable = "Repro Attr Line";

    layout
    {
        area(Content)
        {
            field(Lines; RulesText)
            {
                ApplicationArea = All;
                ShowCaption = false;
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Change: Record "Repro Attr Change";
    begin
        Change.SetRange("Header No.", Rec."Header No.");
        Change.SetRange("Line Entry No.", Rec."Entry No.");
        RulesText := Format(Change.Count());
    end;

    var
        RulesText: Text;
}

page 50106 "Repro Attr Picture"
{
    Caption = 'Attribute Picture';
    Editable = false;
    PageType = CardPart;
    SourceTable = "Repro Attr Line";

    layout
    {
        area(Content)
        {
            field(Image; Rec.Image)
            {
                ApplicationArea = All;
                ShowCaption = false;
            }
        }
    }
}

page 50107 "Repro Attr Where-Used"
{
    Caption = 'Attribute Where-Used';
    Editable = false;
    PageType = CardPart;
    SourceTable = "Repro Attr Line";

    layout
    {
        area(Content)
        {
            field(Changes; UsageCount)
            {
                ApplicationArea = All;
                Caption = 'Changes';
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
    begin
        if (HeaderNo = Rec."Header No.") and (EntryNo = Rec."Entry No.") and not Stale then
            exit;
        HeaderNo := Rec."Header No.";
        EntryNo := Rec."Entry No.";
        Stale := false;
        if TaskId <> 0 then
            CurrPage.CancelBackgroundTask(TaskId);
        Parameters.Add('HeaderNo', HeaderNo);
        Parameters.Add('EntryNo', Format(EntryNo));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"Repro Attr Where-Used Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId <> TaskId then
            exit;
        if Results.ContainsKey('Count') then
            UsageCount := Results.Get('Count');
    end;

    var
        HeaderNo: Code[20];
        EntryNo: Integer;
        TaskId: Integer;
        Stale: Boolean;
        UsageCount: Text;

    procedure Invalidate()
    begin
        Stale := true;
    end;
}

codeunit 50108 "Repro Attr Where-Used Task"
{
    trigger OnRun()
    var
        Change: Record "Repro Attr Change";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        EntryNo: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Evaluate(EntryNo, Parameters.Get('EntryNo'));
        Change.SetRange("Header No.", Parameters.Get('HeaderNo'));
        Change.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Change.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50109 "Repro Attr Change Svc"
{
    procedure CalculateAfterChange(var Line: Record "Repro Attr Line")
    var
        Header: Record "Repro Attr Header";
    begin
        if Line.GetBySystemId(Line.SystemId) then;
        Header.Get(Line."Header No.");
        Header."Calc Count" += 1;
        Header.Modify();
        if Line.GetBySystemId(Line.SystemId) then;
    end;
}

page 50110 "Repro Attr Lines"
{
    AutoSplitKey = true;
    Caption = 'Attribute lines';
    Editable = true;
    PageType = List;
    SourceTable = "Repro Attr Line";
    SourceTableView = sorting("Sorting") order(ascending);
    UsageCategory = None;
    ApplicationArea = All;
    Extensible = false;

    layout
    {
        area(Content)
        {
            repeater(Group)
            {
                IndentationColumn = 0;
                Caption = 'Group';
                field("Attribute Name"; Rec."Attribute Name")
                {
                    ApplicationArea = Basic, Suite;
                    StyleExpr = Rec.Style;
                    TableRelation = "Repro Attr Kind".Name where(Blocked = const(false));
                    ToolTip = 'Specifies the attribute.';

                    trigger OnValidate()
                    var
                        Kind: Record "Repro Attr Kind";
                    begin
                        if xRec."Attribute Name" <> '' then
                            if Rec."Attribute Name" <> xRec."Attribute Name" then
                                Error(CannotChangeErr);
                        if Rec."Attribute Name" <> '' then begin
                            Kind.SetRange(Name, Rec."Attribute Name");
                            if Kind.FindFirst() then
                                Rec."Kind ID" := Kind.ID
                            else
                                Error(UnknownErr);
                        end;
                        CurrPage.SaveRecord();
                        CalculateAndRefresh();
                    end;
                }
                field(Value; Rec."Attribute Value")
                {
                    ApplicationArea = Basic, Suite;
                    Editable = ValueEditable;
                    TableRelation = "Repro Attr Value".Value where("Kind ID" = field("Kind ID"), Blocked = const(false));

                    trigger OnValidate()
                    begin
                        CurrPage.SaveRecord();
                        Rec.GetBySystemId(Rec.SystemId);
                        CalculateAndRefresh();
                    end;
                }
                field(Variant; Rec.Variant)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Variant';
                    Editable = VariantEditable;

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field(Quantity; Rec.Quantity)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Quantity';
                    BlankZero = true;
                    Editable = Rec."Has Quantity";

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field(Description; Rec.Description)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Description';
                    Editable = false;
                }
                field("Sorting"; Rec."Sorting")
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Sorting';
                }
                field(Custom1; CustomValue[1])
                {
                    ApplicationArea = Basic, Suite;
                    CaptionClass = CustomHeader[1];
                    Editable = false;
                    ToolTip = 'Specifies the first custom value.';
                }
                field("Show Blank"; Rec."Show Blank")
                {
                    ApplicationArea = Basic, Suite;
                    Visible = false;
                    Editable = true;

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field("Kind Type"; Rec."Kind Type")
                {
                    ApplicationArea = Basic, Suite;
                    Visible = (Rec."Kind Type" = Rec."Kind Type"::Option);
                    Editable = true;

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
            }
        }
        area(FactBoxes)
        {
            systempart(Notes; Notes)
            {
                ApplicationArea = Basic, Suite;
            }
            part(Rules; "Repro Attr Rules")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Header No." = field("Header No."), "Entry No." = field("Entry No.");
                UpdatePropagation = SubPart;
            }
            part(Picture; "Repro Attr Picture")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Header No." = field("Header No."), "Entry No." = field("Entry No.");
            }
            part(WhereUsed; "Repro Attr Where-Used")
            {
                SubPageLink = "Header No." = field("Header No."), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            group(Process)
            {
                Caption = 'Process';
                action(Recalculate)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Recalculate';
                    Enabled = IsOption;
                    Image = Calculate;
                    ToolTip = 'Recalculates the lines.';
                    Visible = Rec."Kind ID" <> 0;

                    trigger OnAction()
                    begin
                        CalculateAndRefresh();
                    end;
                }
            }
        }
        area(Promoted)
        {
            actionref(Recalculate_Promoted; Recalculate)
            {
            }
        }
    }

    trigger OnDeleteRecord(): Boolean
    var
        ChangeSvc: Codeunit "Repro Attr Change Svc";
    begin
        Rec.Delete(true);
        ChangeSvc.CalculateAfterChange(Rec);
        RefreshWhereUsed();
        exit(false);
    end;

    trigger OnOpenPage()
    var
        EnvironmentInformation: Codeunit "Environment Information";
    begin
        Setup.Get();
        IsSaaS := EnvironmentInformation.IsSaaSInfrastructure();
    end;

    trigger OnAfterGetRecord()
    begin
        Setup.Get();
        Header.Get(Rec."Header No.");
        IsOption := Rec."Kind Type" = Rec."Kind Type"::Option;
        ValueEditable := not Setup."Lock Values";
        VariantEditable := Rec."Kind ID" <> 0;
        if Setup."Custom 1 Caption" <> '' then begin
            CustomHeader[1] := Setup."Custom 1 Caption";
            CustomValue[1] := Format(Rec."Kind ID");
        end;
    end;

    local procedure RefreshWhereUsed()
    begin
        CurrPage.WhereUsed.Page.Invalidate();
        CurrPage.Update(false);
    end;

    local procedure CalculateAndRefresh()
    var
        ChangeSvc: Codeunit "Repro Attr Change Svc";
    begin
        ChangeSvc.CalculateAfterChange(Rec);
        RefreshWhereUsed();
    end;

    local procedure SaveChangeAndCalculate()
    begin
        if Rec."Entry No." = 0 then
            exit;
        CurrPage.SaveRecord();
        CalculateAndRefresh();
    end;

    var
        Setup: Record "Repro Attr Setup";
        Header: Record "Repro Attr Header";
        IsOption: Boolean;
        IsSaaS: Boolean;
        ValueEditable: Boolean;
        VariantEditable: Boolean;
        CannotChangeErr: Label 'Attribute can not be changed';
        UnknownErr: Label 'Unknown attribute';
        CustomHeader: array[30] of Text[30];
        CustomValue: array[30] of Text[30];
}

codeunit 50111 "Repro Attr Opener"
{
    procedure Open(HeaderNo: Code[20])
    var
        Line: Record "Repro Attr Line";
    begin
        Line.SetRange("Header No.", HeaderNo);
        Page.RunModal(Page::"Repro Attr Lines", Line);
    end;
}

codeunit 50120 "Repro Attr Line Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        NameShown: Text;
        ValueShown: Text;

    [Test]
    [HandlerFunctions('AttrLinesHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure NewLine_NameThenValue_SavesTheLine()
    var
        Header: Record "Repro Attr Header";
        Line: Record "Repro Attr Line";
        Change: Record "Repro Attr Change";
        Opener: Codeunit "Repro Attr Opener";
    begin
        Initialize();

        Opener.Open('H1');

        Line.SetRange("Header No.", 'H1');
        if Line.Count() <> 1 then
            Error('Expected 1 line, found %1.', Line.Count());
        Line.FindFirst();
        if (Line."Attribute Name" <> 'Station') or (Line."Attribute Value" <> 'ST-07') or (Line."Kind ID" <> 7) then
            Error('Expected Station/ST-07/7, found %1/%2/%3.', Line."Attribute Name", Line."Attribute Value", Line."Kind ID");
        Header.Get('H1');
        if Header."Calc Count" <> 2 then
            Error('Expected 2 calculations, found %1.', Header."Calc Count");
        if Change.Count() <> 1 then
            Error('Expected 1 change entry, found %1.', Change.Count());
        if NameShown <> 'Station' then
            Error('Expected the page to show Station after the Name, found %1.', NameShown);
        if ValueShown <> 'ST-07' then
            Error('Expected the page to show ST-07 after the Value, found %1.', ValueShown);
    end;

    local procedure Initialize()
    var
        Header: Record "Repro Attr Header";
        Setup: Record "Repro Attr Setup";
        Kind: Record "Repro Attr Kind";
        Line: Record "Repro Attr Line";
        Change: Record "Repro Attr Change";
        AttrValue: Record "Repro Attr Value";
    begin
        Line.DeleteAll();
        Change.DeleteAll();
        AttrValue.DeleteAll();
        Kind.DeleteAll();
        Header.DeleteAll();
        Setup.DeleteAll();
        Setup.Init();
        Setup."Custom 1 Caption" := 'Width';
        Setup.Insert();
        Header.Init();
        Header."No." := 'H1';
        Header.Insert();
        Kind.Init();
        Kind.ID := 7;
        Kind.Name := 'Station';
        Kind.Insert();
        AttrValue.Init();
        AttrValue."Kind ID" := 7;
        AttrValue.Value := 'ST-07';
        AttrValue.Insert();
        Commit();
        NameShown := '';
        ValueShown := '';
    end;

    [ModalPageHandler]
    procedure AttrLinesHandler(var Lines: TestPage "Repro Attr Lines")
    begin
        Lines.New();
        Lines."Attribute Name".SetValue('Station');
        NameShown := Lines."Attribute Name".Value();
        Lines.Value.SetValue('ST-07');
        ValueShown := Lines.Value.Value();
        Lines.OK().Invoke();
    end;
}
