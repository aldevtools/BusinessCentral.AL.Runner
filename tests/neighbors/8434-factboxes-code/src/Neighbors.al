// Neighbors of al-runner issue 8434 (corpus pull request 1454, ListNewLineFactboxesCode.al; its objects are in Probe.al).
// Each copy below differs from a probe object in one fact or one body; the test codeunit at the end drives them and, on the
// probe page itself, calls the probe does not make. BC has not been measured with any of them: the tests set no expected
// values, they only drive the steps; the runner keeps main's way or refuses.

// Page 97492 with DelayedInsert, a page property the probe pages do not declare.
page 97496 "LT NF Lines Delayed"
{
    AutoSplitKey = true;
    Caption = 'Lines';
    Editable = true;
    DelayedInsert = true;
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

// Page 97492 whose CaptionClass calls a procedure.
page 97497 "LT NF Lines Caption Proc"
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
                    CaptionClass = StyleOf();
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

    local procedure StyleOf(): Text
    begin
        exit(Rec.Style);
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

// Part 97489 with a page trigger the probe parts do not have, and page 97492 showing it.
page 97498 "LT NF Rules Open"
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

    trigger OnOpenPage()
    begin
    end;

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

page 97499 "LT NF Lines Part Trigger"
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
            part(Rules; "LT NF Rules Open")
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

// Codeunit 97487 that also inserts another row of the page's table between the save and the update.
codeunit 97500 "LT NF Calc Other Row"
{
    procedure CalculateAfterChange(var Line: Record "LT NF Line")
    var
        Setup: Record "LT NF Setup";
        Other: Record "LT NF Line";
        Log: Codeunit "LT NF Log";
    begin
        if Line.GetBySystemId(Line.SystemId) then;
        Log.Add('S.CALC(' + Line.Name + '/' + Line.Value + '/' + Format(Line."Calc State", 0, 2) + ')');
        Setup.Get();
        Setup."Calc Count" += 1;
        Setup.Modify();
        Other."Group Code" := 'G9';
        Other.Insert();
        // As a recalculation does: the line the Value marked is processed and written without its trigger, then read again.
        if Line."Calc State" = Line."Calc State"::Pending then begin
            Line."Calc State" := Line."Calc State"::Processed;
            Line.Modify(false);
        end;
        if Line.GetBySystemId(Line.SystemId) then;
    end;
}

page 97501 "LT NF Lines Other Row"
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
        Calc: Codeunit "LT NF Calc Other Row";
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
        Calc: Codeunit "LT NF Calc Other Row";
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

// Codeunit 97487 that writes the row through another record and does not read Rec again.
codeunit 97502 "LT NF Calc Other Rec"
{
    procedure CalculateAfterChange(var Line: Record "LT NF Line")
    var
        Setup: Record "LT NF Setup";
        Other: Record "LT NF Line";
        Log: Codeunit "LT NF Log";
    begin
        if Line.GetBySystemId(Line.SystemId) then;
        Log.Add('S.CALC(' + Line.Name + '/' + Line.Value + '/' + Format(Line."Calc State", 0, 2) + ')');
        Setup.Get();
        Setup."Calc Count" += 1;
        Setup.Modify();
        // As a recalculation does: the line the Value marked is processed and written without its trigger, then read again.
        if Line."Calc State" = Line."Calc State"::Pending then begin
            Other.GetBySystemId(Line.SystemId);
            Other."Calc State" := Other."Calc State"::Processed;
            Other.Modify(false);
        end;
    end;
}

page 97503 "LT NF Lines Other Rec"
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
        Calc: Codeunit "LT NF Calc Other Rec";
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
        Calc: Codeunit "LT NF Calc Other Rec";
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

// Part 97489 whose OnAfterGetRecord writes Rec, and page 97492 showing it.
page 97504 "LT NF Rules Writes"
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
        if Rec.Description = '' then begin
            Rec.Description := 'read';
            Rec.Modify();
        end;
        Log.Add('R.AGR(' + Rec.Name + '/' + RulesText + ')');
    end;

    var
        RulesText: Text;
}

page 97505 "LT NF Lines Part Writes"
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
            part(Rules; "LT NF Rules Writes")
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

// Codeunit 97488 whose worker fails, part 97491 that enqueues it, and page 97492 showing that part.
codeunit 97506 "LT NF Usage Task Fails"
{
    trigger OnRun()
    var
        Kind: Record "LT NF Kind";
        Results: Dictionary of [Text, Text];
    begin
        if Kind.Count() >= 0 then
            Error('worker failed');
        Results.Add('Count', 'K' + Format(Kind.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

page 97507 "LT NF Usage Fails"
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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NF Usage Task Fails", Parameters);
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

page 97508 "LT NF Lines Worker Fails"
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
            part(Usage; "LT NF Usage Fails")
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

codeunit 97509 "LT NF Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT NF Log";
        Extra: Integer;

    // Group G1 holds the line under test, on an empty view or after row Before (Sorting 0). Kind Station (7) has the values ST-07
    // and ST-08; kind Old (8) is blocked.
    local procedure Initialize(RowBefore: Boolean; NewExtra: Integer)
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
        Extra := NewExtra;
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

    // The probe's steps on page 97492 (DriveKey of the probe, without its reads), with one call the probe does not make:
    // 1 a third Value SetValue, 2 Next() after the Name SetValue, 3 an action, 4 a SetValue of a control the probe does not
    // set, 5 a second Name SetValue, 6 a first SetValue longer than the field.
    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NF Lines")
    begin
        Lines.New();
        if Extra = 6 then
            asserterror Lines.Name.SetValue('Station Station Station Station')
        else
            Lines.Name.SetValue('Station');
        case Extra of
            2:
                Lines.Next();
            3:
                Lines.Recalculate.Invoke();
            4:
                Lines.Active.SetValue(true);
            5:
                Lines.Name.SetValue('Station');
        end;
        Lines.Value.SetValue('ST-07');
        if Extra = 1 then begin
            Lines.Value.SetValue('ST-08');
            Lines.Value.SetValue('ST-07');
        end;
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure ThirdValueSetValue()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 1);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NextAfterTheLanding()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 2);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure ActionAfterTheLanding()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 3);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure SetValueOfAnotherControl()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 4);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure SecondNameSetValue()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 5);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure OverlongFirstSetValue()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 6);
        Opener.Open(Page::"LT NF Lines", 'G1');
    end;

    // The probe's steps on a copy of page 97492: New(), the Name and the Value SetValue, OK().
    [ModalPageHandler]
    procedure DelayedInsertHandler(var Lines: TestPage "LT NF Lines Delayed")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('DelayedInsertHandler')]
    procedure DelayedInsert()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Delayed", 'G1');
    end;

    [ModalPageHandler]
    procedure CaptionClassCallsProcedureHandler(var Lines: TestPage "LT NF Lines Caption Proc")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CaptionClassCallsProcedureHandler')]
    procedure CaptionClassCallsProcedure()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Caption Proc", 'G1');
    end;

    [ModalPageHandler]
    procedure PartTriggerHandler(var Lines: TestPage "LT NF Lines Part Trigger")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('PartTriggerHandler')]
    procedure PartTrigger()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Part Trigger", 'G1');
    end;

    [ModalPageHandler]
    procedure WritesAnotherRowHandler(var Lines: TestPage "LT NF Lines Other Row")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('WritesAnotherRowHandler')]
    procedure WritesAnotherRow()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Other Row", 'G1');
    end;

    [ModalPageHandler]
    procedure WritesTheRowThroughAnotherRecordHandler(var Lines: TestPage "LT NF Lines Other Rec")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('WritesTheRowThroughAnotherRecordHandler')]
    procedure WritesTheRowThroughAnotherRecord()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Other Rec", 'G1');
    end;

    [ModalPageHandler]
    procedure PartWritesRecHandler(var Lines: TestPage "LT NF Lines Part Writes")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('PartWritesRecHandler')]
    procedure PartWritesRec()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Part Writes", 'G1');
    end;

    [ModalPageHandler]
    procedure FailingWorkerHandler(var Lines: TestPage "LT NF Lines Worker Fails")
    begin
        Lines.New();
        Lines.Name.SetValue('Station');
        Lines.Value.SetValue('ST-07');
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('FailingWorkerHandler')]
    procedure FailingWorker()
    var
        Opener: Codeunit "LT NF Opener";
    begin
        Initialize(false, 0);
        Opener.Open(Page::"LT NF Lines Worker Fails", 'G1');
    end;
}
