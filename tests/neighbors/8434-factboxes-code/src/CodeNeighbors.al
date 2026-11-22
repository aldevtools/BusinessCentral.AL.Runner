// Copies of the probe's objects (Probe.al) with one body changed each: the page with code runs only the probe's code line
// for line (code_inserted_pinned.rs coded_code), and any other code keeps main's way (review of #8463, round 1, finding 1).
// Page 97523 is page 97492 under another name, its code the probe's: the control.

// (a) The background task part's OnPageBackgroundTaskCompleted ends with CurrPage.Update(false).
page 97510 "LT NF Usage A"
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
        CurrPage.Update(false);
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

page 97511 "LT NF Lines A"
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
            part(Usage; "LT NF Usage A")
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

// (b) RefreshUsage calls CurrPage.Update(false) without CurrPage.Usage.Page.Invalidate().
page 97512 "LT NF Lines B"
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

// (c) The Value control's OnValidate inserts a second row after its CurrPage.Update(false).
page 97513 "LT NF Lines C"
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
                    var
                        Second: Record "LT NF Line";
                    begin
                        Log.Add('P.VV(' + Rec.Value + ')');
                        CurrPage.SaveRecord();
                        Log.Add('P.SR');
                        Rec.GetBySystemId(Rec.SystemId);
                        CalculateAndRefresh();
                        Second.Init();
                        Second."Group Code" := Rec."Group Code";
                        Second.Insert(true);
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

// (d) Table 97480 with another OnModify body; the parts, the calculation codeunit and the page over it.
table 97514 "LT NF Line D"
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
        Last: Record "LT NF Line D";
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
        Description := 'modified';
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

page 97515 "LT NF Rules D"
{
    Caption = ' ', Locked = true;
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NF Line D";
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

page 97516 "LT NF Picture D"
{
    Caption = 'Picture';
    Editable = false;
    PageType = CardPart;
    SourceTable = "LT NF Line D";
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

page 97517 "LT NF Usage D"
{
    Caption = 'Usage';
    Editable = false;
    Extensible = false;
    PageType = CardPart;
    SourceTable = "LT NF Line D";
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

codeunit 97518 "LT NF Calc D"
{
    procedure CalculateAfterChange(var Line: Record "LT NF Line D")
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

page 97519 "LT NF Lines D"
{
    AutoSplitKey = true;
    Caption = 'Lines';
    Editable = true;
    PageType = List;
    SourceTable = "LT NF Line D";
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
            part(Rules; "LT NF Rules D")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
                UpdatePropagation = SubPart;
            }
            part(Picture; "LT NF Picture D")
            {
                ApplicationArea = Basic, Suite;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NF Usage D")
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
        Calc: Codeunit "LT NF Calc D";
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
        Calc: Codeunit "LT NF Calc D";
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

codeunit 97520 "LT NF Opener D"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        Line: Record "LT NF Line D";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}

// (e) SourceTableView sorting(Name) order(ascending).
page 97521 "LT NF Lines E"
{
    AutoSplitKey = true;
    Caption = 'Lines';
    Editable = true;
    PageType = List;
    SourceTable = "LT NF Line";
    SourceTableView = sorting(Name) order(ascending);
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

// The control: page 97492 under another name, its code the probe's.
page 97523 "LT NF Lines Same"
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

codeunit 97522 "LT NF Code Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT NF Log";

    // The probe's data (Initialize of LT NF Tests): group G1 on an empty view, kinds Station (7) and Old (8, blocked).
    local procedure Initialize()
    var
        Line: Record "LT NF Line";
        LineD: Record "LT NF Line D";
        Kind: Record "LT NF Kind";
        Setup: Record "LT NF Setup";
        Change: Record "LT NF Change";
        KindValue: Record "LT NF Value";
    begin
        Line.DeleteAll();
        LineD.DeleteAll();
        Change.DeleteAll();
        KindValue.DeleteAll();
        Kind.DeleteAll();
        Setup.DeleteAll();
        Setup.Init();
        Setup."Custom 1 Caption" := 'Width';
        Setup.Insert();
        Kind.Init();
        Kind.ID := 7;
        Kind.Name := 'Station';
        Kind.Insert();
        Kind.Init();
        Kind.ID := 8;
        Kind.Name := 'Old';
        Kind.Blocked := true;
        Kind.Insert();
        KindValue.Init();
        KindValue."Kind ID" := 7;
        KindValue.Value := 'ST-07';
        KindValue.Insert();
        Commit();
        Log.Reset();
    end;

    local procedure Open(PageId: Integer)
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize();
        Opener.Open(PageId, 'G1');
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure CodeA_PartUpdatesInItsCompletionHandler(var Lines: TestPage "LT NF Lines A")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CodeA_PartUpdatesInItsCompletionHandler')]
    procedure CodeA_PartUpdatesInItsCompletion()
    begin
        Open(Page::"LT NF Lines A");
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure CodeB_UpdateWithoutInvalidateHandler(var Lines: TestPage "LT NF Lines B")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CodeB_UpdateWithoutInvalidateHandler')]
    procedure CodeB_UpdateWithoutInvalidate()
    begin
        Open(Page::"LT NF Lines B");
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure CodeC_SecondRowAfterTheUpdateHandler(var Lines: TestPage "LT NF Lines C")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CodeC_SecondRowAfterTheUpdateHandler')]
    procedure CodeC_SecondRowAfterTheUpdate()
    begin
        Open(Page::"LT NF Lines C");
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure CodeD_AnotherOnModifyHandler(var Lines: TestPage "LT NF Lines D")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CodeD_AnotherOnModifyHandler')]
    procedure CodeD_AnotherOnModify()
    var
        Opener: Codeunit "LT NF Opener D";
    begin
        Initialize();
        Opener.Open(Page::"LT NF Lines D", 'G1');
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure CodeE_SortedByNameHandler(var Lines: TestPage "LT NF Lines E")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CodeE_SortedByNameHandler')]
    procedure CodeE_SortedByName()
    begin
        Open(Page::"LT NF Lines E");
    end;

    // New(), the Name and the Value SetValue, OK(): the probe's steps.
    [ModalPageHandler]
    procedure SameCode_AnotherNameHandler(var Lines: TestPage "LT NF Lines Same")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('SameCode_AnotherNameHandler')]
    procedure SameCode_AnotherName()
    begin
        Open(Page::"LT NF Lines Same");
    end;
}
