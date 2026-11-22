// The corpus repro ListNewLineFactboxesRepro.al of corpus pull request 1517 (al-runner issue 8434, the reporter's repro as filed), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the pages, tables and codeunits of the
//   reporter's repro, are what is under test
// Fixtures used: own tables 97372 to 97377, part pages 97378 to 97380, background task codeunit 97381, calculation codeunit
//   97382, page 97383, opener codeunit 97384, test codeunit 97385
// Probe (al-runner issue 8434, the reporter's repro as filed, object names and ids changed only): ListNewLineFactboxesCode.al
//   measures CurrPage.SaveRecord() and CurrPage.Update(false) on the new line of a List page with code and FactBoxes, on its
//   own objects, whose code logs each step. This file keeps the repro's own objects, whose code differs: no log, no Value field
//   of its own on the Where-Used part, a header table the calculation counts in. The page is opened with Page.RunModal on a
//   record filtered to one header with no line; the handler starts a new line, sets the attribute name (the table field inserts
//   the row, the control's OnValidate saves it, calculates and refreshes), reads it back, sets the value (the table field
//   writes a change entry, the control's OnValidate saves, reads Rec again, calculates and refreshes), reads it back and
//   invokes OK. Readings: what the page shows after each step, the line, the header's calculation count and the change entries
//   after OK.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 97372 "LT NFR Header"
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

table 97373 "LT NFR Setup"
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

table 97374 "LT NFR Kind"
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

table 97375 "LT NFR Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Header No."; Code[20])
        {
            TableRelation = "LT NFR Header"."No.";
        }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; "Attribute Name"; Text[250])
        {
            NotBlank = true;
            TableRelation = "LT NFR Kind".Name where(Blocked = const(false));
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
                Change: Record "LT NFR Change";
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
        Last: Record "LT NFR Line";
    begin
        Last.SetCurrentKey("Sorting");
        Last.SetRange("Header No.", "Header No.");
        Last.SetLoadFields("Sorting");
        if Last.FindLast() then
            "Sorting" := Last."Sorting" + 10;
    end;

    trigger OnDelete()
    var
        Change: Record "LT NFR Change";
    begin
        Change.SetRange("Header No.", "Header No.");
        Change.SetRange("Line Entry No.", "Entry No.");
        Change.DeleteAll();
    end;

    local procedure CopyKind()
    var
        Kind: Record "LT NFR Kind";
    begin
        Kind.SetRange(Name, "Attribute Name");
        if Kind.FindFirst() then begin
            "Kind ID" := Kind.ID;
            "Kind Type" := "Kind Type"::Option;
        end;
    end;
}

table 97376 "LT NFR Value"
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

table 97377 "LT NFR Change"
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

page 97378 "LT NFR Rules"
{
    Caption = ' ', Locked = true;
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NFR Line";

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
        Change: Record "LT NFR Change";
    begin
        Change.SetRange("Header No.", Rec."Header No.");
        Change.SetRange("Line Entry No.", Rec."Entry No.");
        RulesText := Format(Change.Count());
    end;

    var
        RulesText: Text;
}

page 97379 "LT NFR Picture"
{
    Caption = 'Attribute Picture';
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NFR Line";

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

page 97380 "LT NFR Where-Used"
{
    Caption = 'Attribute Where-Used';
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NFR Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NFR Where-Used Task", Parameters);
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

codeunit 97381 "LT NFR Where-Used Task"
{
    trigger OnRun()
    var
        Change: Record "LT NFR Change";
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

codeunit 97382 "LT NFR Change Svc"
{
    procedure CalculateAfterChange(var Line: Record "LT NFR Line")
    var
        Header: Record "LT NFR Header";
    begin
        if Line.GetBySystemId(Line.SystemId) then;
        Header.Get(Line."Header No.");
        Header."Calc Count" += 1;
        Header.Modify();
        if Line.GetBySystemId(Line.SystemId) then;
    end;
}

page 97383 "LT NFR Lines"
{
    AutoSplitKey = true;
    Caption = 'Attribute lines';
    Editable = true;
    PageType = List;
    SourceTable = "LT NFR Line";
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
                    TableRelation = "LT NFR Kind".Name where(Blocked = const(false));
                    ToolTip = 'Specifies the attribute.';

                    trigger OnValidate()
                    var
                        Kind: Record "LT NFR Kind";
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
                    TableRelation = "LT NFR Value".Value where("Kind ID" = field("Kind ID"), Blocked = const(false));

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
            part(Rules; "LT NFR Rules")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Header No." = field("Header No."), "Entry No." = field("Entry No.");
                UpdatePropagation = SubPart;
            }
            part(Picture; "LT NFR Picture")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Header No." = field("Header No."), "Entry No." = field("Entry No.");
            }
            part(WhereUsed; "LT NFR Where-Used")
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
        ChangeSvc: Codeunit "LT NFR Change Svc";
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
        ChangeSvc: Codeunit "LT NFR Change Svc";
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
        Setup: Record "LT NFR Setup";
        Header: Record "LT NFR Header";
        IsOption: Boolean;
        IsSaaS: Boolean;
        ValueEditable: Boolean;
        VariantEditable: Boolean;
        CannotChangeErr: Label 'Attribute can not be changed';
        UnknownErr: Label 'Unknown attribute';
        CustomHeader: array[30] of Text[30];
        CustomValue: array[30] of Text[30];
}

codeunit 97384 "LT NFR Opener"
{
    procedure Open(HeaderNo: Code[20])
    var
        Line: Record "LT NFR Line";
    begin
        Line.SetRange("Header No.", HeaderNo);
        Page.RunModal(Page::"LT NFR Lines", Line);
    end;
}

codeunit 97385 "LT NFR Line Tests"
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
        Header: Record "LT NFR Header";
        Line: Record "LT NFR Line";
        Change: Record "LT NFR Change";
        Opener: Codeunit "LT NFR Opener";
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
        Header: Record "LT NFR Header";
        Setup: Record "LT NFR Setup";
        Kind: Record "LT NFR Kind";
        Line: Record "LT NFR Line";
        Change: Record "LT NFR Change";
        AttrValue: Record "LT NFR Value";
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
    procedure AttrLinesHandler(var Lines: TestPage "LT NFR Lines")
    begin
        Lines.New();
        Lines."Attribute Name".SetValue('Station');
        NameShown := Lines."Attribute Name".Value();
        Lines.Value.SetValue('ST-07');
        ValueShown := Lines.Value.Value();
        Lines.OK().Invoke();
    end;
}
