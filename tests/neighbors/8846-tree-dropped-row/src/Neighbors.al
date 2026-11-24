// Neighbors of al-runner issue 8846 just past the cells of corpus pull request 1539 (TreeValidateFilterRefresh.al): the
// repro's tree with CurrPage.Update(true), with TreeInitialState = CollapseAll, with OnAfterGetCurrRecord, and the repro's page
// over rows where the filter leaves no row in the view.

// The repro's page with CurrPage.Update(true).
page 97670 "LT TDN Drop Tree True"
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
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }
}

// The repro's page with TreeInitialState = CollapseAll.
page 97671 "LT TDN Drop Tree Collapsed"
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
                TreeInitialState = CollapseAll;

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

// The repro's page with OnAfterGetCurrRecord.
page 97672 "LT TDN Drop Tree Curr"
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

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TDN Log";
}

codeunit 97673 "LT TDN Log"
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

codeunit 97674 "LT TDN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT TDN Log";

    local procedure Initialize(SecondGroup: Code[10])
    var
        Row: Record "Repro Tree Drop Row";
    begin
        Row.DeleteAll();
        AddRow('G0', 1, 'A', 0);
        AddRow(SecondGroup, 2, 'B', 0);
        AddRow(SecondGroup, 3, 'C', 1);
        Commit();
        Log.Take();
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Row: Record "Repro Tree Drop Row";
    begin
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Indentation := Indentation;
        Row.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Row: Record "Repro Tree Drop Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + ',';
            until Row.Next() = 0;
        exit(Result);
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected:<%1>. Actual:<%2>.', Expected, Actual);
    end;

    [TryFunction]
    local procedure TrySetTrue(var Rows: TestPage "LT TDN Drop Tree True"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    [TryFunction]
    local procedure TrySetCollapsed(var Rows: TestPage "LT TDN Drop Tree Collapsed"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    [TryFunction]
    local procedure TrySetCurr(var Rows: TestPage "LT TDN Drop Tree Curr"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    [TryFunction]
    local procedure TrySetRepro(var Rows: TestPage "Repro Tree Drop List"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    local procedure Outcome(Ok: Boolean): Text
    begin
        if Ok then
            exit('none');
        exit(GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    // Update(true) from the first row A: the reading is the error (or none), then the stored rows.
    [Test]
    procedure DropTreeTrue_FirstRow()
    var
        Rows: TestPage "LT TDN Drop Tree True";
        Ok: Boolean;
    begin
        Initialize('G1');
        Rows.OpenEdit();
        ClearLastError();
        Ok := TrySetTrue(Rows, 'A2');
        Check(Outcome(Ok) + '|' + StoredRows(), 'none|A2,B,C,');
    end;

    // CollapseAll, from the first row A.
    [Test]
    procedure DropTreeCollapsed_FirstRow()
    var
        Rows: TestPage "LT TDN Drop Tree Collapsed";
        Ok: Boolean;
    begin
        Initialize('G1');
        Rows.OpenEdit();
        ClearLastError();
        Ok := TrySetCollapsed(Rows, 'A2');
        Check(Outcome(Ok) + '|' + StoredRows(), 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''|A2,B,C,');
    end;

    // OnAfterGetCurrRecord, from the first row A: also the page's log of the SetValue.
    [Test]
    procedure DropTreeCurr_FirstRow()
    var
        Rows: TestPage "LT TDN Drop Tree Curr";
        Ok: Boolean;
    begin
        Initialize('G1');
        Rows.OpenEdit();
        Log.Take();
        ClearLastError();
        Ok := TrySetCurr(Rows, 'A2');
        Check(Outcome(Ok) + '|' + Log.Take() + '|' + StoredRows(), 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''|AGCR(B);|A2,B,C,');
    end;

    // The repro's page over A (G0), B and C (G2): SetRange on G1 leaves no row in the view.
    [Test]
    procedure DropTree_NoRowLeft()
    var
        Rows: TestPage "Repro Tree Drop List";
        Ok: Boolean;
    begin
        Initialize('G2');
        Rows.OpenEdit();
        ClearLastError();
        Ok := TrySetRepro(Rows, 'A2');
        Check(Outcome(Ok) + '|' + StoredRows(), 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''|A2,B,C,');
    end;
}
