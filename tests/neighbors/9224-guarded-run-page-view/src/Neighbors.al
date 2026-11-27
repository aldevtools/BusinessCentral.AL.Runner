// Neighbor shapes of the page-view guarded run (runner issue 9224): each runs the Ends ('mark') and the PlainCommit
// ('mark,commit') steps of "GPC Inner" in a shape the probe did not measure. The runtime keeps main's result for each.

page 69170 "GPN Worksheet"
{
    PageType = Worksheet;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 69171 "GPN ListPart"
{
    PageType = ListPart;
    ApplicationArea = All;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

// A List page that a page extension extends: its own action and the extension's action.
page 69172 "GPN Extended"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

pageextension 69173 "GPN Extension" extends "GPN Extended"
{
    actions
    {
        addlast(Processing)
        {
            action(RunGuardedExt)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

// The run starts in OnOpenPage, after the page applied its view.
page 69174 "GPN Open Run"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    var
        Runner: Codeunit "GPC Run";
    begin
        Runner.RunCopy(Rec);
    end;
}

// A List page over the probe's table whose action builds the argument another way, by GPC Log's extra:
// 'codeview' a fresh record with the group-2 filter set in code, 'assign' a := copy of Rec, 'reset' Rec.Reset() and the
// same group-2 filter set again in code, 'clear' the same after Clear(Rec) (the global Clear, not Rec.Reset).
page 69175 "GPN Variants"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    LogCu: Codeunit "GPC Log";
                    R: Record "GPC Z";
                    Ok: Boolean;
                begin
                    case LogCu.GetExtra() of
                        'codeview':
                            begin
                                R.FilterGroup(2);
                                R.SetRange("Doc Kind", R."Doc Kind"::Order);
                                R.FilterGroup(0);
                            end;
                        'assign':
                            R := Rec;
                        'clear':
                            begin
                                Clear(Rec);
                                Rec.FilterGroup(2);
                                Rec.SetRange("Doc Kind", Rec."Doc Kind"::Order);
                                Rec.FilterGroup(0);
                                R.Copy(Rec);
                            end;
                        'reset':
                            begin
                                Rec.Reset();
                                Rec.FilterGroup(2);
                                Rec.SetRange("Doc Kind", Rec."Doc Kind"::Order);
                                Rec.FilterGroup(0);
                                R.Copy(Rec);
                            end;
                    end;
                    ClearLastError();
                    Ok := Codeunit.Run(Codeunit::"GPC Inner", R);
                    LogCu.Log('run=' + Format(Ok));
                    LogCu.Log('code=' + GetLastErrorCode());
                end;
            }
        }
    }
}

// A table whose primary key is two Code fields (no Enum field), with a list page over a const view of the first one.
table 69177 "GPN Code Key"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Doc Type"; Code[10]) { }
        field(2; "No."; Code[20]) { }
    }

    keys
    {
        key(PK; "Doc Type", "No.") { Clustered = true; }
    }
}

codeunit 69178 "GPN Code Key Inner"
{
    TableNo = "GPN Code Key";

    trigger OnRun()
    var
        LogCu: Codeunit "GPC Log";
        M: Record "GPC M";
        Step: Text;
    begin
        foreach Step in LogCu.GetSteps().Split(',') do
            case Step of
                'mark':
                    begin
                        M.Init();
                        M.Id := 50;
                        M.Insert();
                        M.Consistent(false);
                    end;
                'commit':
                    Commit();
            end;
    end;
}

page 69179 "GPN Code Key List"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPN Code Key";
    SourceTableView = where("Doc Type" = const('ORDER'));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    LogCu: Codeunit "GPC Log";
                    R: Record "GPN Code Key";
                    Ok: Boolean;
                begin
                    R.Copy(Rec);
                    ClearLastError();
                    Ok := Codeunit.Run(Codeunit::"GPN Code Key Inner", R);
                    LogCu.Log('run=' + Format(Ok));
                    LogCu.Log('code=' + GetLastErrorCode());
                end;
            }
        }
    }
}

codeunit 69180 "GPN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        LogCu: Codeunit "GPC Log";

    local procedure Initialize(Steps: Text; Extra: Text)
    var
        Z: Record "GPC Z";
        M: Record "GPC M";
        C: Record "GPN Code Key";
    begin
        Z.Consistent(true);
        M.Consistent(true);
        Z.DeleteAll();
        M.DeleteAll();
        C.DeleteAll();
        Z.Init();
        Z."Doc Kind" := Z."Doc Kind"::Order;
        Z."No." := '1';
        Z.Insert();
        C.Init();
        C."Doc Type" := 'ORDER';
        C."No." := '1';
        C.Insert();
        Commit();
        ClearLastError();
        LogCu.Set(Steps, Extra);
    end;

    // Not local: GPN Param Tests runs its cells through it too.
    procedure Cell(var Observer: Codeunit "Probe Observer"; Shape: Text; Steps: Text; Extra: Text)
    begin
        Initialize(Steps, Extra);
        asserterror RunShape(Shape);
        Observer.ObserveError('run end');
        Observer.Observe('log', LogCu.GetLog());
    end;

    local procedure RunShape(Shape: Text)
    var
        Worksheet: TestPage "GPN Worksheet";
        ListPart: TestPage "GPN ListPart";
        Extended: TestPage "GPN Extended";
        OpenRun: TestPage "GPN Open Run";
        Variants: TestPage "GPN Variants";
        CodeKey: TestPage "GPN Code Key List";
        CallShapes: TestPage "GPN Call Shapes";
    begin
        case Shape of
            'Worksheet':
                begin
                    Worksheet.OpenView();
                    Worksheet.First();
                    Worksheet.RunGuarded.Invoke();
                end;
            'ListPart':
                begin
                    ListPart.OpenView();
                    ListPart.First();
                    ListPart.RunGuarded.Invoke();
                end;
            'Extended':
                begin
                    Extended.OpenView();
                    Extended.First();
                    Extended.RunGuarded.Invoke();
                end;
            'ExtensionAction':
                begin
                    Extended.OpenView();
                    Extended.First();
                    Extended.RunGuardedExt.Invoke();
                end;
            'OpenRun':
                OpenRun.OpenView();
            'Variants':
                begin
                    Variants.OpenView();
                    Variants.First();
                    Variants.RunGuarded.Invoke();
                end;
            'CodeKey':
                begin
                    CodeKey.OpenView();
                    CodeKey.First();
                    CodeKey.RunGuarded.Invoke();
                end;
            'CallShapes':
                begin
                    CallShapes.OpenView();
                    CallShapes.First();
                    CallShapes.RunGuarded.Invoke();
                end;
        end;
        Error('END');
    end;

    [Test]
    procedure Worksheet_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Worksheet', 'mark', '');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure Worksheet_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Worksheet', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ListPart_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'ListPart', 'mark', '');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ListPart_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'ListPart', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ExtendedPageOwnAction_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Extended', 'mark', '');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ExtendedPageOwnAction_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Extended', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ExtensionAction_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'ExtensionAction', 'mark', '');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ExtensionAction_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'ExtensionAction', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure RunFromOnOpenPage_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'OpenRun', 'mark', '');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Verify();
    end;

    [Test]
    procedure RunFromOnOpenPage_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'OpenRun', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure CodeKey_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CodeKey', 'mark', '');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure CodeKey_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CodeKey', 'mark,commit', '');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure GroupTwoFilterInCode_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark', 'codeview');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure GroupTwoFilterInCode_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark,commit', 'codeview');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure AssignedCopy_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark', 'assign');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure AssignedCopy_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark,commit', 'assign');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ResetThenSameView_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark', 'reset');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ResetThenSameView_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark,commit', 'reset');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ClearThenSameView_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark', 'clear');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ClearThenSameView_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'Variants', 'mark,commit', 'clear');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure DirectInAction_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark', 'direct');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure DirectInAction_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark,commit', 'direct');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure LocalProcedure_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark', 'local');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure LocalProcedure_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark,commit', 'local');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure OtherCodeunitMethod_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark', 'method');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure OtherCodeunitMethod_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark,commit', 'method');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure EventSubscriber_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark', 'event');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure EventSubscriber_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Cell(Observer, 'CallShapes', 'mark,commit', 'event');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;
}

// Review round 2 of runner PR 9252: the CallShapes cells whose codeunit method has one var parameter of another type. They
// run in a test codeunit of their own, so a refusal of an earlier GPN Tests cell cannot hide their own result.
codeunit 69176 "GPN Param Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ParamInteger_Ends()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark', 'param-int');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ParamInteger_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark,commit', 'param-int');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;

    [Test]
    procedure ParamOtherTable_Ends()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark', 'param-other');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ParamOtherTable_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark,commit', 'param-other');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;
}

// Fix round of runner PR 9252: a method whose one var parameter is the page's Rec runs the guarded run on a global copy
// made in an earlier call. A codeunit of its own: GPN Param Tests' first cell is refused, which hides its later cells.
codeunit 69173 "GPN Same Param Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ParamSameGlobal_Ends()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark', 'param-same-global');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Verify();
    end;

    [Test]
    procedure ParamSameGlobal_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
        Tests: Codeunit "GPN Tests";
    begin
        Tests.Cell(Observer, 'CallShapes', 'mark,commit', 'param-same-global');
        Observer.Expect('run end|text', 'END');
        Observer.Verify();
    end;
}
