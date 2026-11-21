// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, the page's own triggers, its FactBoxes and the
//   procedure its controls call are what is under test
// Fixtures used: shared Assert (60021); own tables 97480 to 97484, log codeunit 97485, subscriber codeunit 97486, calculation
//   codeunit 97487, background task codeunit 97488, part pages 97489 to 97491, pages 97492 and 97493, opener codeunit 97494, test
//   codeunit 97495
// Probe (al-runner issue 8434, the follow-up of 8285 measured in ListNewLineSaveRecordUpdate.al): a List page opened with
//   Page.RunModal on a record filtered to one group stands on its new line (New()). SetValue on Name runs the table field's
//   OnValidate, which inserts Rec itself (Insert(true), with an AutoIncrement key field), and then the control's OnValidate, which
//   calls CurrPage.SaveRecord() and a procedure: a codeunit writes another table, writes the line itself without its trigger
//   when the Value marked it (Modify(false)), and reads Rec again by SystemId, the procedure
//   calls a procedure of a FactBox part, then CurrPage.Update(false). SetValue on Value then runs the table field's OnValidate
//   (it inserts a row in another table and marks the line) and the control's OnValidate: CurrPage.SaveRecord(), Rec.GetBySystemId and the same
//   procedure. Unlike the pages of ListNewLineSaveRecordUpdate.al, the page declares Caption, Editable, Extensible and a
//   SourceTableView in a secondary key, OnOpenPage, OnAfterGetRecord (reads two tables, sets the variables of control
//   expressions) and OnDeleteRecord; its controls carry Caption, ToolTip, StyleExpr, Style, TableRelation, Editable, Visible,
//   BlankZero, DrillDown, Lookup, CaptionClass and Obsolete properties, are bound to fields and to an array variable, and have
//   OnValidate triggers the test does not reach; it has actions (Processing, Prompting, Promoted); its FactBoxes are the Notes
//   system part and three CardParts linked to the line's key: one with OnAfterGetRecord and UpdatePropagation SubPart, one
//   showing a MediaSet field, one without ApplicationArea that enqueues a page background task in OnAfterGetCurrRecord (a control
//   with OnDrillDown, a cue group of fields of a temporary record with StyleExpr, OnPageBackgroundTaskCompleted and
//   OnPageBackgroundTaskError); each part declares UsageCategory, ApplicationArea and Extensible, one a MultiLine control, one
//   two actions and a control without ApplicationArea. The table and the page have manual event subscribers that the tests do
//   not bind, except one. The background task reads only a table the tests do not change while the page is open.
//   Readings: the page, part and table triggers each step runs, the row the page shows, what the parts show, the rows the tables
//   hold. Cells (all on Page.RunModal of a record filtered to group G1, then OK): the page in the secondary key and the same page
//   without SourceTableView (the primary key), each on an empty view, with and without reads of the parts between the steps; the
//   secondary key with a row before the line; and neighbors, each in the secondary key on an empty view: a Name the Kind table
//   does not hold (the control's OnValidate raises after the insert), a Name of a blocked kind (the page control's TableRelation
//   is not checked), a Value SetValue of the value already there, a Value SetValue before any Name (the page inserts the line), a
//   second Value SetValue, and the manual subscriber bound.
//   Measured on a local BC 29.1 OnPrem w1 container (11 of 11 green).
// BC versions: 27, 28, 29

table 97480 "LT NF Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; Name; Text[30])
        {
            NotBlank = true;
            TableRelation = "LT NF Kind".Name where(Blocked = const(false));
            ValidateTableRelation = false;

            trigger OnValidate()
            var
                Log: Codeunit "LT NF Log";
            begin
                Log.Add('T.VN(' + Name + ')');
                if "Entry No." = 0 then
                    Insert(true);
                CopyKind();
            end;
        }
        field(4; Value; Text[30])
        {
            trigger OnValidate()
            var
                Change: Record "LT NF Change";
                Log: Codeunit "LT NF Log";
            begin
                Log.Add('T.VV(' + xRec.Value + '>' + Value + ')');
                if Value <> xRec.Value then begin
                    Variant := '';
                    Change.Init();
                    Change."Group Code" := "Group Code";
                    Change."Line Entry No." := "Entry No.";
                    Change."Old Value" := xRec.Value;
                    Change."New Value" := Value;
                    Change.Insert();
                end;
                "Calc State" := "Calc State"::Pending;
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
        field(15; "Kind Group"; Text[30]) { }
        field(16; Billable; Boolean) { }
        field(17; Active; Boolean) { }
        field(18; "Old Flag"; Boolean) { }
        field(19; "Calc State"; Option)
        {
            OptionMembers = " ",Pending,Processed;
        }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
        key(BySorting; "Group Code", "Sorting") { }
    }

    trigger OnInsert()
    var
        Last: Record "LT NF Line";
        Log: Codeunit "LT NF Log";
    begin
        Last.SetCurrentKey("Sorting");
        Last.SetRange("Group Code", "Group Code");
        Last.SetLoadFields("Sorting");
        if Last.FindLast() then
            "Sorting" := Last."Sorting" + 10;
        Log.Add('T.I(' + Name + '/' + Format("Sorting") + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT NF Log";
    begin
        Log.Add('T.M(' + Name + '/' + Value + ')');
    end;

    trigger OnDelete()
    var
        Change: Record "LT NF Change";
    begin
        Change.SetRange("Group Code", "Group Code");
        Change.SetRange("Line Entry No.", "Entry No.");
        Change.DeleteAll();
    end;

    local procedure CopyKind()
    var
        Kind: Record "LT NF Kind";
    begin
        Kind.SetRange(Name, Name);
        if Kind.FindFirst() then begin
            "Kind ID" := Kind.ID;
            "Kind Type" := "Kind Type"::Option;
        end;
    end;
}

table 97481 "LT NF Kind"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; ID; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Blocked; Boolean) { }
    }

    keys
    {
        key(PK; ID) { Clustered = true; }
    }
}

table 97482 "LT NF Setup"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Lock Values"; Boolean) { }
        field(3; "Custom 1 Caption"; Text[30]) { }
        field(4; "Calc Count"; Integer) { }
    }

    keys
    {
        key(PK; "Primary Key") { Clustered = true; }
    }
}

table 97483 "LT NF Change"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(2; "Group Code"; Code[10]) { }
        field(3; "Line Entry No."; Integer) { }
        field(4; "Old Value"; Text[30]) { }
        field(5; "New Value"; Text[30]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 97484 "LT NF Value"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Kind ID"; Integer) { }
        field(2; Value; Text[30]) { }
        field(3; Blocked; Boolean) { }
    }

    keys
    {
        key(PK; "Kind ID", Value) { Clustered = true; }
    }
}

codeunit 97485 "LT NF Log"
{
    SingleInstance = true;

    var
        Buffer: Text;

    procedure Reset()
    begin
        Buffer := '';
    end;

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

// Manual subscribers to the table and to page 97492: they run only while a test binds them (the SubscriberBound test).
codeunit 97486 "LT NF Spy"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Table, Database::"LT NF Line", 'OnAfterModifyEvent', '', false, false)]
    local procedure LineOnAfterModify(var Rec: Record "LT NF Line"; var xRec: Record "LT NF Line"; RunTrigger: Boolean)
    var
        Log: Codeunit "LT NF Log";
    begin
        Log.Add('E.M(' + Rec.Name + '/' + Rec.Value + ')');
    end;

    [EventSubscriber(ObjectType::Page, Page::"LT NF Lines", 'OnModifyRecordEvent', '', false, false)]
    local procedure LinesOnModifyRecord(var Rec: Record "LT NF Line"; var xRec: Record "LT NF Line"; var AllowModify: Boolean)
    var
        Log: Codeunit "LT NF Log";
    begin
        Log.Add('E.PM(' + Rec.Name + '/' + Rec.Value + ')');
    end;
}

codeunit 97487 "LT NF Calc"
{
    procedure CalculateAfterChange(var Line: Record "LT NF Line")
    var
        Setup: Record "LT NF Setup";
        Log: Codeunit "LT NF Log";
    begin
        if Line.GetBySystemId(Line.SystemId) then;
        Log.Add('S.CALC(' + Line.Name + '/' + Line.Value + '/' + Format(Line."Calc State", 0, 2) + ')');
        Setup.Get();
        Setup."Calc Count" += 1;
        Setup.Modify();
        // As a recalculation does: the line the Value marked is processed and written without its trigger, then read again.
        if Line."Calc State" = Line."Calc State"::Pending then begin
            Line."Calc State" := Line."Calc State"::Processed;
            Line.Modify(false);
        end;
        if Line.GetBySystemId(Line.SystemId) then;
    end;
}

// The background task reads only the Kind table, which no test changes while its page is open.
codeunit 97488 "LT NF Usage Task"
{
    trigger OnRun()
    var
        Kind: Record "LT NF Kind";
        Results: Dictionary of [Text, Text];
    begin
        Results.Add('Count', 'K' + Format(Kind.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

page 97489 "LT NF Rules"
{
    Caption = ' ', Locked = true;
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NF Line";
    UsageCategory = None;
    ApplicationArea = All;
    Extensible = false;

    layout
    {
        area(Content)
        {
            field(Lines; RulesText)
            {
                ApplicationArea = All;
                MultiLine = true;
                ShowCaption = false;
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Change: Record "LT NF Change";
        Log: Codeunit "LT NF Log";
    begin
        Change.SetRange("Group Code", Rec."Group Code");
        Change.SetRange("Line Entry No.", Rec."Entry No.");
        RulesText := Format(Change.Count());
        Log.Add('R.AGR(' + Rec.Name + '/' + RulesText + ')');
    end;

    var
        RulesText: Text;
}

page 97490 "LT NF Picture"
{
    Caption = 'Picture';
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NF Line";
    UsageCategory = None;
    ApplicationArea = All;
    Extensible = false;

    layout
    {
        area(Content)
        {
            field(Image; Rec.Image)
            {
                ShowCaption = false;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ImportPicture)
            {
                Caption = 'Import';
                Image = Import;
                ToolTip = 'Imports a picture.';

                trigger OnAction()
                begin
                    Clear(Rec.Image);
                end;
            }
            action(ClearPicture)
            {
                Caption = 'Clear';
                Image = Delete;
                ToolTip = 'Clears the picture.';

                trigger OnAction()
                begin
                    Clear(Rec.Image);
                end;
            }
        }
    }
}

page 97491 "LT NF Usage"
{
    Caption = 'Usage';
    Editable = false;
    Extensible = false;
    PageType = CardPart;
    SourceTable = "LT NF Line";
    UsageCategory = None;
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Uses; UsageText)
            {
                ApplicationArea = All;
                Caption = 'Uses';

                trigger OnDrillDown()
                begin
                    Stale := true;
                end;
            }
            cuegroup(Kinds)
            {
                Caption = 'Kinds';

                field(KindName; TempKind.Name)
                {
                    Caption = 'Kind';
                    DrillDown = true;
                    Style = Unfavorable;
                    StyleExpr = Stale;

                    trigger OnDrillDown()
                    begin
                        Stale := true;
                    end;
                }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "LT NF Log";
        Parameters: Dictionary of [Text, Text];
    begin
        if Stale then
            Log.Add('W.AGCR(' + Rec.Name + '/s)')
        else
            Log.Add('W.AGCR(' + Rec.Name + ')');
        if (GroupCode = Rec."Group Code") and (EntryNo = Rec."Entry No.") and not Stale then
            exit;
        GroupCode := Rec."Group Code";
        EntryNo := Rec."Entry No.";
        Stale := false;
        if TaskId <> 0 then
            CurrPage.CancelBackgroundTask(TaskId);
        Parameters.Add('GroupCode', GroupCode);
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NF Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Log: Codeunit "LT NF Log";
    begin
        if CompletedTaskId <> TaskId then
            exit;
        if Results.ContainsKey('Count') then
            UsageText := Results.Get('Count');
        Log.Add('W.DONE(' + UsageText + ')');
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    var
        Log: Codeunit "LT NF Log";
    begin
        Log.Add('W.ERR(' + ErrorCode + ')');
        IsHandled := true;
    end;

    var
        GroupCode: Code[10];
        EntryNo: Integer;
        TaskId: Integer;
        Stale: Boolean;
        UsageText: Text;
        TempKind: Record "LT NF Kind" temporary;

    procedure Invalidate()
    begin
        Stale := true;
    end;
}

// The secondary key (SourceTableView), with the page's own triggers, control expressions, actions and four FactBoxes.
page 97492 "LT NF Lines"
{
    AutoSplitKey = true;
    Caption = 'Lines';
    Editable = true;
    PageType = List;
    SourceTable = "LT NF Line";
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
                field(Name; Rec.Name)
                {
                    ApplicationArea = Basic, Suite;
                    StyleExpr = Rec.Style;
                    TableRelation = "LT NF Kind".Name where(Blocked = const(false));
                    ToolTip = 'Specifies the kind.';

                    trigger OnValidate()
                    var
                        Kind: Record "LT NF Kind";
                    begin
                        if xRec.Name <> '' then
                            if Rec.Name <> xRec.Name then
                                Error(CannotChangeErr);
                        if Rec.Name <> '' then begin
                            Kind.SetRange(Name, Rec.Name);
                            if Kind.FindFirst() then
                                Rec."Kind ID" := Kind.ID
                            else
                                Error(UnknownErr);
                        end;
                        Log.Add('P.VN(' + Rec.Name + ')');
                        CurrPage.SaveRecord();
                        Log.Add('P.SR');
                        CalculateAndRefresh();
                        Log.Add('P.VN.end');
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = Basic, Suite;
                    Editable = ValueEditable;
                    TableRelation = "LT NF Value".Value where("Kind ID" = field("Kind ID"), Blocked = const(false));

                    trigger OnValidate()
                    begin
                        Log.Add('P.VV(' + Rec.Value + ')');
                        CurrPage.SaveRecord();
                        Log.Add('P.SR');
                        Rec.GetBySystemId(Rec.SystemId);
                        CalculateAndRefresh();
                        Log.Add('P.VV.end');
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
                field(Active; Rec.Active)
                {
                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field("Kind Group"; Rec."Kind Group")
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Group';
                    Editable = true;
                    Style = None;
                    DrillDown = false;
                    Lookup = false;

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

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field(Billable; Rec.Billable)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Billable';
                    Visible = IsGroupKind;

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
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
                field("Old Flag"; Rec."Old Flag")
                {
                    ApplicationArea = Basic, Suite;
                    Visible = false;
                    Editable = true;
                    ObsoleteState = Pending;
                    ObsoleteTag = '28.0';
                    ObsoleteReason = 'Not shown any more.';
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
            part(Rules; "LT NF Rules")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
                UpdatePropagation = SubPart;
            }
            part(Picture; "LT NF Picture")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NF Usage")
            {
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
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
        area(Prompting)
        {
            action(Suggest)
            {
                ApplicationArea = Basic, Suite;
                Caption = 'Suggest';
                Image = Sparkle;
                ToolTip = 'Suggests lines.';
                Visible = IsSaaS;

                trigger OnAction()
                begin
                    CalculateAndRefresh();
                end;
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
        Calc: Codeunit "LT NF Calc";
    begin
        Rec.Delete(true);
        Calc.CalculateAfterChange(Rec);
        RefreshUsage();
        exit(false);
    end;

    trigger OnOpenPage()
    var
        EnvironmentInformation: Codeunit "Environment Information";
    begin
        Log.Add('P.OPEN');
        Setup.Get();
        IsSaaS := EnvironmentInformation.IsSaaSInfrastructure();
    end;

    trigger OnAfterGetRecord()
    var
        Kind: Record "LT NF Kind";
    begin
        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');
        Setup.Get();
        IsOption := Rec."Kind Type" = Rec."Kind Type"::Option;
        IsGroupKind := false;
        if Kind.Get(Rec."Kind ID") then
            IsGroupKind := Kind.Blocked;
        ValueEditable := not Setup."Lock Values";
        VariantEditable := Rec."Kind ID" <> 0;
        if Setup."Custom 1 Caption" <> '' then begin
            CustomHeader[1] := Setup."Custom 1 Caption";
            CustomValue[1] := Format(Rec."Kind ID");
        end;
    end;

    local procedure RefreshUsage()
    begin
        CurrPage.Usage.Page.Invalidate();
        Log.Add('P.UPD');
        CurrPage.Update(false);
        Log.Add('P.UPD.end');
    end;

    local procedure CalculateAndRefresh()
    var
        Calc: Codeunit "LT NF Calc";
    begin
        Calc.CalculateAfterChange(Rec);
        RefreshUsage();
    end;

    local procedure SaveChangeAndCalculate()
    begin
        if Rec."Entry No." = 0 then
            exit;
        CurrPage.SaveRecord();
        CalculateAndRefresh();
    end;

    var
        Setup: Record "LT NF Setup";
        Log: Codeunit "LT NF Log";
        IsOption: Boolean;
        IsGroupKind: Boolean;
        IsSaaS: Boolean;
        ValueEditable: Boolean;
        VariantEditable: Boolean;
        CannotChangeErr: Label 'The kind cannot be changed.';
        UnknownErr: Label 'Unknown kind.';
        CustomHeader: array[30] of Text[30];
        CustomValue: array[30] of Text[30];
}
// Page 97492 without its SourceTableView: the primary key.
page 97493 "LT NF Lines Pk"
{
    AutoSplitKey = true;
    Caption = 'Lines';
    Editable = true;
    PageType = List;
    SourceTable = "LT NF Line";
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
                field(Name; Rec.Name)
                {
                    ApplicationArea = Basic, Suite;
                    StyleExpr = Rec.Style;
                    TableRelation = "LT NF Kind".Name where(Blocked = const(false));
                    ToolTip = 'Specifies the kind.';

                    trigger OnValidate()
                    var
                        Kind: Record "LT NF Kind";
                    begin
                        if xRec.Name <> '' then
                            if Rec.Name <> xRec.Name then
                                Error(CannotChangeErr);
                        if Rec.Name <> '' then begin
                            Kind.SetRange(Name, Rec.Name);
                            if Kind.FindFirst() then
                                Rec."Kind ID" := Kind.ID
                            else
                                Error(UnknownErr);
                        end;
                        Log.Add('P.VN(' + Rec.Name + ')');
                        CurrPage.SaveRecord();
                        Log.Add('P.SR');
                        CalculateAndRefresh();
                        Log.Add('P.VN.end');
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = Basic, Suite;
                    Editable = ValueEditable;
                    TableRelation = "LT NF Value".Value where("Kind ID" = field("Kind ID"), Blocked = const(false));

                    trigger OnValidate()
                    begin
                        Log.Add('P.VV(' + Rec.Value + ')');
                        CurrPage.SaveRecord();
                        Log.Add('P.SR');
                        Rec.GetBySystemId(Rec.SystemId);
                        CalculateAndRefresh();
                        Log.Add('P.VV.end');
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
                field(Active; Rec.Active)
                {
                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field("Kind Group"; Rec."Kind Group")
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Group';
                    Editable = true;
                    Style = None;
                    DrillDown = false;
                    Lookup = false;

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

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
                }
                field(Billable; Rec.Billable)
                {
                    ApplicationArea = Basic, Suite;
                    Caption = 'Billable';
                    Visible = IsGroupKind;

                    trigger OnValidate()
                    begin
                        SaveChangeAndCalculate();
                    end;
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
                field("Old Flag"; Rec."Old Flag")
                {
                    ApplicationArea = Basic, Suite;
                    Visible = false;
                    Editable = true;
                    ObsoleteState = Pending;
                    ObsoleteTag = '28.0';
                    ObsoleteReason = 'Not shown any more.';
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
            part(Rules; "LT NF Rules")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
                UpdatePropagation = SubPart;
            }
            part(Picture; "LT NF Picture")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NF Usage")
            {
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
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
        area(Prompting)
        {
            action(Suggest)
            {
                ApplicationArea = Basic, Suite;
                Caption = 'Suggest';
                Image = Sparkle;
                ToolTip = 'Suggests lines.';
                Visible = IsSaaS;

                trigger OnAction()
                begin
                    CalculateAndRefresh();
                end;
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
        Calc: Codeunit "LT NF Calc";
    begin
        Rec.Delete(true);
        Calc.CalculateAfterChange(Rec);
        RefreshUsage();
        exit(false);
    end;

    trigger OnOpenPage()
    var
        EnvironmentInformation: Codeunit "Environment Information";
    begin
        Log.Add('P.OPEN');
        Setup.Get();
        IsSaaS := EnvironmentInformation.IsSaaSInfrastructure();
    end;

    trigger OnAfterGetRecord()
    var
        Kind: Record "LT NF Kind";
    begin
        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');
        Setup.Get();
        IsOption := Rec."Kind Type" = Rec."Kind Type"::Option;
        IsGroupKind := false;
        if Kind.Get(Rec."Kind ID") then
            IsGroupKind := Kind.Blocked;
        ValueEditable := not Setup."Lock Values";
        VariantEditable := Rec."Kind ID" <> 0;
        if Setup."Custom 1 Caption" <> '' then begin
            CustomHeader[1] := Setup."Custom 1 Caption";
            CustomValue[1] := Format(Rec."Kind ID");
        end;
    end;

    local procedure RefreshUsage()
    begin
        CurrPage.Usage.Page.Invalidate();
        Log.Add('P.UPD');
        CurrPage.Update(false);
        Log.Add('P.UPD.end');
    end;

    local procedure CalculateAndRefresh()
    var
        Calc: Codeunit "LT NF Calc";
    begin
        Calc.CalculateAfterChange(Rec);
        RefreshUsage();
    end;

    local procedure SaveChangeAndCalculate()
    begin
        if Rec."Entry No." = 0 then
            exit;
        CurrPage.SaveRecord();
        CalculateAndRefresh();
    end;

    var
        Setup: Record "LT NF Setup";
        Log: Codeunit "LT NF Log";
        IsOption: Boolean;
        IsGroupKind: Boolean;
        IsSaaS: Boolean;
        ValueEditable: Boolean;
        VariantEditable: Boolean;
        CannotChangeErr: Label 'The kind cannot be changed.';
        UnknownErr: Label 'Unknown kind.';
        CustomHeader: array[30] of Text[30];
        CustomValue: array[30] of Text[30];
}

codeunit 97494 "LT NF Opener"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        Line: Record "LT NF Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}

codeunit 97495 "LT NF Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT NF Log";
        Expected: List of [Text];
        StepNo: Integer;
        Mode: Option Parts,NoPartReads,UnknownName,BlockedName,SameValue,ValueFirst,SecondValue;

    // Group G1 holds the line under test, on an empty view or after row Before (Sorting 0). Kind Station (7) has the values ST-07
    // and ST-08; kind Old (8) is blocked.
    local procedure Initialize(RowBefore: Boolean; NewMode: Option Parts,NoPartReads,UnknownName,BlockedName,SameValue,ValueFirst,SecondValue)
    var
        Line: Record "LT NF Line";
        Kind: Record "LT NF Kind";
        Setup: Record "LT NF Setup";
        Change: Record "LT NF Change";
        KindValue: Record "LT NF Value";
    begin
        Line.DeleteAll();
        Change.DeleteAll();
        KindValue.DeleteAll();
        Kind.DeleteAll();
        Setup.DeleteAll();
        Setup.Init();
        Setup."Custom 1 Caption" := 'Width';
        Setup.Insert();
        AddKind(7, 'Station', false);
        AddKind(8, 'Old', true);
        AddValue(7, 'ST-07');
        AddValue(7, 'ST-08');
        if RowBefore then begin
            Line.Init();
            Line."Group Code" := 'G1';
            Line.Name := 'Before';
            Line.Insert(true);
        end;
        Commit();
        Log.Reset();
        Clear(Expected);
        StepNo := 0;
        Mode := NewMode;
    end;

    local procedure AddKind(ID: Integer; Name: Text[30]; Blocked: Boolean)
    var
        Kind: Record "LT NF Kind";
    begin
        Kind.Init();
        Kind.ID := ID;
        Kind.Name := Name;
        Kind.Blocked := Blocked;
        Kind.Insert();
    end;

    local procedure AddValue(KindID: Integer; Value: Text[30])
    var
        KindValue: Record "LT NF Value";
    begin
        KindValue.Init();
        KindValue."Kind ID" := KindID;
        KindValue.Value := Value;
        KindValue.Insert();
    end;

    local procedure Expect(Steps: Text)
    var
        Step: Text;
    begin
        foreach Step in Steps.Split(' | ') do
            Expected.Add(Step);
    end;

    local procedure Check(Name: Text; Actual: Text)
    begin
        StepNo += 1;
        Assert.AreEqual(Expected.Get(StepNo), Name + '=' + Actual, 'step ' + Format(StepNo));
    end;

    local procedure Finish()
    begin
        Assert.AreEqual(Expected.Count(), StepNo, 'every expected step was read');
    end;

    local procedure ErrorAndLog(): Text
    begin
        exit(GetLastErrorText() + ' @' + Log.Take());
    end;

    local procedure Rows(): Text
    var
        Line: Record "LT NF Line";
        Setup: Record "LT NF Setup";
        Change: Record "LT NF Change";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Value + '/' + Format(Line."Kind ID") + '/' +
                    Format(Line."Sorting") + '/' + Format(Line."Calc State", 0, 2) + ',';
            until Line.Next() = 0;
        Setup.Get();
        exit(Result + ' calc=' + Format(Setup."Calc Count") + ' changes=' + Format(Change.Count()));
    end;

    local procedure DriveKey(var Lines: TestPage "LT NF Lines")
    var
        NewValue: Text;
    begin
        Check('open', Log.Take());
        Lines.New();
        Check('new', Log.Take());
        case Mode of
            Mode::ValueFirst:
                begin
                    Lines.Value.SetValue('ST-07');
                    Check('setvalue', Log.Take());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
            Mode::UnknownName:
                begin
                    asserterror Lines.Name.SetValue('Ghost');
                    Check('setname', ErrorAndLog());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
            Mode::BlockedName:
                begin
                    Lines.Name.SetValue('Old');
                    Check('setname', Log.Take());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
        end;
        Lines.Name.SetValue('Station');
        Check('setname', Log.Take());
        if Mode <> Mode::NoPartReads then begin
            Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value() + '/' + Lines."Sorting".Value());
            Check('parts', Lines.Rules.Lines.Value() + '/' + Lines.Usage.Uses.Value());
            Check('partslog', Log.Take());
            Check('rows1', Rows());
        end;
        NewValue := 'ST-07';
        if Mode = Mode::SameValue then
            NewValue := '';
        Lines.Value.SetValue(NewValue);
        Check('setvalue', Log.Take());
        if Mode <> Mode::NoPartReads then begin
            Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
            Check('parts2', Lines.Rules.Lines.Value() + '/' + Lines.Usage.Uses.Value());
            Check('parts2log', Log.Take());
            Check('rows2', Rows());
        end;
        if Mode = Mode::SecondValue then begin
            Lines.Value.SetValue('ST-08');
            Check('setvalue2', Log.Take());
            Check('shown3', Lines.Name.Value() + '/' + Lines.Value.Value());
            Check('rows2b', Rows());
        end;
        Lines.OK().Invoke();
        Check('ok', Log.Take());
    end;

    local procedure DrivePk(var Lines: TestPage "LT NF Lines Pk")
    var
        NewValue: Text;
    begin
        Check('open', Log.Take());
        Lines.New();
        Check('new', Log.Take());
        case Mode of
            Mode::ValueFirst:
                begin
                    Lines.Value.SetValue('ST-07');
                    Check('setvalue', Log.Take());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
            Mode::UnknownName:
                begin
                    asserterror Lines.Name.SetValue('Ghost');
                    Check('setname', ErrorAndLog());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
            Mode::BlockedName:
                begin
                    Lines.Name.SetValue('Old');
                    Check('setname', Log.Take());
                    Check('rows', Rows());
                    Lines.OK().Invoke();
                    Check('ok', Log.Take());
                    exit;
                end;
        end;
        Lines.Name.SetValue('Station');
        Check('setname', Log.Take());
        if Mode <> Mode::NoPartReads then begin
            Check('shown', Lines.Name.Value() + '/' + Lines.Value.Value() + '/' + Lines."Sorting".Value());
            Check('parts', Lines.Rules.Lines.Value() + '/' + Lines.Usage.Uses.Value());
            Check('partslog', Log.Take());
            Check('rows1', Rows());
        end;
        NewValue := 'ST-07';
        if Mode = Mode::SameValue then
            NewValue := '';
        Lines.Value.SetValue(NewValue);
        Check('setvalue', Log.Take());
        if Mode <> Mode::NoPartReads then begin
            Check('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
            Check('parts2', Lines.Rules.Lines.Value() + '/' + Lines.Usage.Uses.Value());
            Check('parts2log', Log.Take());
            Check('rows2', Rows());
        end;
        if Mode = Mode::SecondValue then begin
            Lines.Value.SetValue('ST-08');
            Check('setvalue2', Log.Take());
            Check('shown3', Lines.Name.Value() + '/' + Lines.Value.Value());
            Check('rows2b', Rows());
        end;
        Lines.OK().Invoke();
        Check('ok', Log.Take());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NF Lines")
    begin
        DriveKey(Lines);
    end;

    [ModalPageHandler]
    procedure PkHandler(var Lines: TestPage "LT NF Lines Pk")
    begin
        DrivePk(Lines);
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SecondaryKey_EmptyView_ReadsTheRowTwice()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::Parts);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//0 | parts=0/K2 | partslog= | rows1=G1:Station//7/0/0, calc=1 changes=0' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ST-07 | parts2=1/K2 | parts2log= | rows2=G1:Station/ST-07/7/0/2, calc=2 changes=1' +
            ' | ok= | after= | rows3=G1:Station/ST-07/7/0/2, calc=2 changes=1');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SecondaryKey_WithoutPartReads_ReadsTheRowTwice()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::NoPartReads);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | ok= | after= | rows3=G1:Station/ST-07/7/0/2, calc=2 changes=1');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('PkHandler')]
    procedure ListNewLine_FactboxesCode_PrimaryKey_EmptyView_ReadsTheRowOnce()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::Parts);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//0 | parts=0/K2 | partslog= | rows1=G1:Station//7/0/0, calc=1 changes=0' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ST-07 | parts2=1/K2 | parts2log= | rows2=G1:Station/ST-07/7/0/2, calc=2 changes=1' +
            ' | ok= | after= | rows3=G1:Station/ST-07/7/0/2, calc=2 changes=1');
        Opener.Open(Page::"LT NF Lines Pk", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('PkHandler')]
    procedure ListNewLine_FactboxesCode_PrimaryKey_WithoutPartReads_ReadsTheRowOnce()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::NoPartReads);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | ok= | after= | rows3=G1:Station/ST-07/7/0/2, calc=2 changes=1');
        Opener.Open(Page::"LT NF Lines Pk", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SecondaryKey_RowBefore_ReadsTheRowAgainFirst()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(true, Mode::Parts);
        Expect('open=P.OPEN;P.AGR(Before/);R.AGR(Before/0);W.AGCR(Before);W.DONE(K2); | new=' +
            ' | setname=W.AGCR();W.DONE(K2);T.VN(Station);T.I(Station/10);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);P.AGR(Before/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//10 | parts=0/K2 | partslog=' +
            ' | rows1=G1:Before//0/0/0,G1:Station//7/10/0, calc=1 changes=0' +
            ' | setvalue=P.AGR(Station/);T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);P.AGR(Station/ST-07);P.AGR(Before/);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ST-07 | parts2=1/K2 | parts2log=' +
            ' | rows2=G1:Before//0/0/0,G1:Station/ST-07/7/10/2, calc=2 changes=1 | ok=P.AGR(Station/ST-07);' +
            ' | after= | rows3=G1:Before//0/0/0,G1:Station/ST-07/7/10/2, calc=2 changes=1');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_UnknownName_ErrorUndoesTheInsert()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::UnknownName);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=Validation error for Field: Name,  Message = ''Unknown kind.'' @W.AGCR();T.VN(Ghost);T.I(Ghost/0);' +
            ' | rows= calc=0 changes=0 | ok= | after= | rows3= calc=0 changes=0');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_BlockedName_PageRelationNotChecked()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::BlockedName);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Old);T.I(Old/0);P.VN(Old);T.M(Old/);P.SR;S.CALC(Old//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Old/);P.AGR(Old/);R.AGR(Old/0);W.AGCR(Old/s);W.DONE(K2);' +
            ' | rows=G1:Old//8/0/0, calc=1 changes=0 | ok= | after= | rows3=G1:Old//8/0/0, calc=1 changes=0');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SameValue_SaveModifiesNothing()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::SameValue);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//0 | parts=0/K2 | partslog= | rows1=G1:Station//7/0/0, calc=1 changes=0' +
            ' | setvalue=T.VV(>);P.VV();T.M(Station/);P.SR;S.CALC(Station//1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ | parts2=0/K2 | parts2log= | rows2=G1:Station//7/0/2, calc=2 changes=0 | ok=' +
            ' | after= | rows3=G1:Station//7/0/2, calc=2 changes=0');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_ValueFirst_SaveRecordInsertsTheLine()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::ValueFirst);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setvalue=W.AGCR();T.VV(>ST-07);P.VV(ST-07);T.I(/0);P.SR;S.CALC(/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(/ST-07);P.AGR(/ST-07);R.AGR(/0);W.AGCR(/s);W.DONE(K2);' +
            ' | rows=G1:/ST-07/0/0/2, calc=1 changes=1 | ok= | after= | rows3=G1:/ST-07/0/0/2, calc=1 changes=1');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SecondValue_SameStepsAgain()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::SecondValue);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);T.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//0 | parts=0/K2 | partslog= | rows1=G1:Station//7/0/0, calc=1 changes=0' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ST-07 | parts2=1/K2 | parts2log= | rows2=G1:Station/ST-07/7/0/2, calc=2 changes=1' +
            ' | setvalue2=T.VV(ST-07>ST-08);P.VV(ST-08);T.M(Station/ST-08);P.SR;S.CALC(Station/ST-08/1);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-08);P.AGR(Station/ST-08);R.AGR(Station/2);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown3=Station/ST-08 | rows2b=G1:Station/ST-08/7/0/2, calc=3 changes=2 | ok= | after=' +
            ' | rows3=G1:Station/ST-08/7/0/2, calc=3 changes=2');
        Opener.Open(Page::"LT NF Lines", 'G1');
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure ListNewLine_FactboxesCode_SubscriberBound_RunsAfterEachModify()
    var
        Opener: Codeunit "LT NF Opener";
        Spy: Codeunit "LT NF Spy";
    begin
        Initialize(false, Mode::Parts);
        Expect('open=P.OPEN;W.AGCR(); | new=' +
            ' | setname=W.AGCR();T.VN(Station);T.I(Station/0);P.VN(Station);E.PM(Station/);T.M(Station/);E.M(Station/);P.SR;S.CALC(Station//0);P.UPD;P.UPD.end;P.VN.end;P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown=Station//0 | parts=0/K2 | partslog= | rows1=G1:Station//7/0/0, calc=1 changes=0' +
            ' | setvalue=T.VV(>ST-07);P.VV(ST-07);E.PM(Station/ST-07);T.M(Station/ST-07);E.M(Station/ST-07);P.SR;S.CALC(Station/ST-07/1);E.M(Station/ST-07);P.UPD;P.UPD.end;P.VV.end;P.AGR(Station/ST-07);P.AGR(Station/ST-07);R.AGR(Station/1);W.AGCR(Station/s);W.DONE(K2);' +
            ' | shown2=Station/ST-07 | parts2=1/K2 | parts2log= | rows2=G1:Station/ST-07/7/0/2, calc=2 changes=1' +
            ' | ok= | after= | rows3=G1:Station/ST-07/7/0/2, calc=2 changes=1');
        BindSubscription(Spy);
        Opener.Open(Page::"LT NF Lines", 'G1');
        UnbindSubscription(Spy);
        Check('after', Log.Take());
        Check('rows3', Rows());
        Finish();
    end;
}
