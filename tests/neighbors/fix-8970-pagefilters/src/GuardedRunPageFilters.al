// Guarded Codeunit.Run ends with Consistent(false) marks and a record argument with page-style filters (al-runner issue
// 8970). The twins of corpus PR 1600 (errors/ConsistentGuardedRunRecPage.al:2591-2813) run flagged on head; the other
// shapes are neighbors that still refuse on main and on head, so they have no row in differences.tsv.

enum 61460 "PF Kind" { Extensible = false; value(0; " ") { } value(1; Order) { } value(2; Invoice) { } }

table 61461 "PF X"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Doc Type"; Integer) { }
        field(4; Amount; Decimal) { }
        field(5; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(6; Kind; Enum "PF Kind") { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61462 "PF Y"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Doc Type"; Integer) { }
        field(4; Amount; Decimal) { }
        field(5; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(6; Kind; Enum "PF Kind") { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61463 "PF Z"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Doc Type"; Integer) { }
        field(4; Amount; Decimal) { }
        field(5; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(6; Kind; Enum "PF Kind") { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 61464 "PF Mode"
{
    SingleInstance = true;
    var
        Steps: Text;
        ChangeArg: Boolean;
        ReleaseArg: Boolean;

    procedure Set(NewSteps: Text; NewChangeArg: Boolean)
    begin
        Steps := NewSteps;
        ChangeArg := NewChangeArg;
        ReleaseArg := false;
    end;

    // argx: the argument itself calls Consistent(true) on GRP X between startx and the remaining steps.
    procedure SetArgRelease(NewSteps: Text)
    begin
        Steps := NewSteps;
        ChangeArg := false;
        ReleaseArg := true;
    end;

    procedure ReleasesArg(): Boolean
    begin
        exit(ReleaseArg);
    end;

    procedure ChangesArg(): Boolean
    begin
        exit(ChangeArg);
    end;

    // startx: X row 50 inserted, then X held; fya,fyc: Y marked, then marked again by another variable; txb: X released by another variable;
    // fxa,txb,fxc: X marked again after a release; err: Error('inner failed').
    procedure Run()
    begin
        RunSteps(Steps);
    end;

    procedure RunSteps(Which: Text)
    var
        XA: Record "PF X";
        XB: Record "PF X";
        XC: Record "PF X";
        YA: Record "PF Y";
        YC: Record "PF Y";
        Step: Text;
    begin
        foreach Step in Which.Split(',') do
            case Step of
                'startx':
                    begin
                        XA.Init(); XA.Id := 50; XA.Name := 'state'; XA.Insert();
                        XA.Consistent(false);
                    end;
                'fxa':
                    XA.Consistent(false);
                'txb':
                    XB.Consistent(true);
                'fxc':
                    XC.Consistent(false);
                'fya':
                    YA.Consistent(false);
                'fyc':
                    YC.Consistent(false);
                'err':
                    Error('inner failed');
            end;
    end;
}

codeunit 61465 "PF Inner X"
{
    TableNo = "PF X";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        if Mode.ChangesArg() then
            Rec.SetRange(Amount, 50);
        if Mode.ReleasesArg() then begin
            Mode.RunSteps('startx');
            Rec.Consistent(true);
            Mode.RunSteps('fya,err');
            exit;
        end;
        Mode.Run();
    end;
}

codeunit 61466 "PF Inner Y"
{
    TableNo = "PF Y";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        if Mode.ChangesArg() then
            Rec.SetRange(Amount, 50);
        Mode.Run();
    end;
}

codeunit 61467 "PF Inner Z"
{
    TableNo = "PF Z";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        if Mode.ChangesArg() then
            Rec.SetRange(Amount, 50);
        Mode.Run();
    end;
}

page 61468 "PF Page X"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "PF X";
    SourceTableView = where(Kind = const(Order));
    layout { area(Content) { repeater(Lines) { field(Id; Rec.Id) { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions
    {
        area(Processing)
        {
            action(RunInner)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    if Codeunit.Run(Codeunit::"PF Inner X", Rec) then
                        Error('the run returned true');
                    if GetLastErrorText() <> 'inner failed' then
                        Error('wrong error: %1', GetLastErrorText());
                    Error('END');
                end;
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange("Date Filter", 0D, WorkDate());
    end;
}

page 61469 "PF Page Z"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "PF Z";
    SourceTableView = where(Kind = const(Order));
    layout { area(Content) { repeater(Lines) { field(Id; Rec.Id) { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions
    {
        area(Processing)
        {
            action(RunInner)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    if Codeunit.Run(Codeunit::"PF Inner Z", Rec) then
                        Error('the run returned true');
                    if GetLastErrorText() <> 'inner failed' then
                        Error('wrong error: %1', GetLastErrorText());
                    Error('END');
                end;
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange("Date Filter", 0D, WorkDate());
    end;
}

codeunit 61470 "PF Tests"
{
    Subtype = Test;

    [Test]
    procedure RecZ_PageMix_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMix');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMix');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_PageMix_MarkedAgain()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'PageMix');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'PageMix');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecZ_PageMixKey_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixKey');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixKey');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_PageMixKey_MarkedAgain()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'PageMixKey');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'PageMixKey');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecZ_Three_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Three');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecZ_Three_AgainThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Three');
        Commit();
        Mode.Set('fxa,txb,fxc,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_Three_ReleasedThenHeld()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'Three');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_Three_AgainThenHeld()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'Three');
        Commit();
        Mode.Set('fxa,txb,fxc,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecZ_Key_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Key');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecZ_Key_AgainThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Key');
        Commit();
        Mode.Set('fxa,txb,fxc,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_Key_ReleasedThenHeld()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'Key');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure RecX_Key_AgainThenHeld()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'Key');
        Commit();
        Mode.Set('fxa,txb,fxc,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecY_Three_ReleasedThenHeld()
    var
        R: Record "PF Y";
    begin
        SeedY();
        R.Get(1);
        FilterY(R, 'Three');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Y", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckY(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_TempZ_Key_ReleasedThenHeld()
    var
        R: Record "PF Z" temporary;
    begin
        SeedZ();
        R.Init(); R.Id := 1; R.Name := 'caller'; R."Doc Type" := 1; R.Amount := 5; R.Kind := R.Kind::Order; R.Insert();
        FilterZ(R, 'Key');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMix_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMix');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMix');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_Three_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Three');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Three');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_Two_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Two');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Two');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMixOtherDate_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixOtherDate');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixOtherDate');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMixGroup4_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixGroup4');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixGroup4');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_Key_ReleasedThenHeld_ArgChanged()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Key');
        Commit();
        Mode.Set('startx,txb,fya,err', true);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_KeyOnName_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'KeyOnName');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'KeyOnName');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_ThreeWithKey_ReleasedThenHeld()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'ThreeWithKey');
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'ThreeWithKey');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMixViewDocType_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixViewDocType');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixViewDocType');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMixSameField_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixSameField');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixSameField');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_PageMixExtraName_MarkedAgain()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'PageMixExtraName');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckZ(R, 'PageMixExtraName');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure Neighbor_RecX_Key_ArgReleasesThenHeld()
    var
        R: Record "PF X";
    begin
        SeedX();
        R.Get(1);
        FilterX(R, 'Key');
        Commit();
        Mode.SetArgRelease('startx,fya,err');
        if Codeunit.Run(Codeunit::"PF Inner X", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        CheckX(R, 'Key');
        CheckRows();
        Commit();
    end;

    [Test]
    procedure PageX_EnumViewDateFilter_MarkedAgain()
    var
        R: Record "PF X";
        TP: TestPage "PF Page X";
    begin
        SeedX();
        R.Get(1);
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        TP.OpenView();
        TP.GoToRecord(R);
        asserterror TP.RunInner.Invoke();
        if StrPos(GetLastErrorText(), 'END') = 0 then
            Error('wrong error: %1', GetLastErrorText());
        TP.Close();
        CheckRows();
        Commit();
    end;

    [Test]
    procedure PageZ_EnumViewDateFilter_MarkedAgain()
    var
        R: Record "PF Z";
        TP: TestPage "PF Page Z";
    begin
        SeedZ();
        R.Get(1);
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        TP.OpenView();
        TP.GoToRecord(R);
        asserterror TP.RunInner.Invoke();
        if StrPos(GetLastErrorText(), 'END') = 0 then
            Error('wrong error: %1', GetLastErrorText());
        TP.Close();
        CheckRows();
        Commit();
    end;

    // The argument keeps the filters of every group and filter group 0 after the run (corpus cells, 'argstate').
    local procedure CheckX(var R: Record "PF X"; Shape: Text)
    var
        F: Record "PF X";
        D: Record "PF X";
        G: Integer;
    begin
        if D.Count() <> 2 then
            Error('PF X rows after the run: %1', D.Count());
        if R.FilterGroup() <> 0 then
            Error('filter group after the run: %1', R.FilterGroup());
        FilterX(F, Shape);
        for G := 0 to 6 do begin
            R.FilterGroup(G);
            F.FilterGroup(G);
            if R.GetFilters() <> F.GetFilters() then
                Error('group %1 filters after the run: %2', G, R.GetFilters());
        end;
        R.FilterGroup(0);
    end;

    // The argument keeps the filters of every group and filter group 0 after the run (corpus cells, 'argstate').
    local procedure CheckY(var R: Record "PF Y"; Shape: Text)
    var
        F: Record "PF Y";
        D: Record "PF Y";
        G: Integer;
    begin
        if D.Count() <> 2 then
            Error('PF Y rows after the run: %1', D.Count());
        if R.FilterGroup() <> 0 then
            Error('filter group after the run: %1', R.FilterGroup());
        FilterY(F, Shape);
        for G := 0 to 6 do begin
            R.FilterGroup(G);
            F.FilterGroup(G);
            if R.GetFilters() <> F.GetFilters() then
                Error('group %1 filters after the run: %2', G, R.GetFilters());
        end;
        R.FilterGroup(0);
    end;

    // The argument keeps the filters of every group and filter group 0 after the run (corpus cells, 'argstate').
    local procedure CheckZ(var R: Record "PF Z"; Shape: Text)
    var
        F: Record "PF Z";
        D: Record "PF Z";
        G: Integer;
    begin
        if D.Count() <> 2 then
            Error('PF Z rows after the run: %1', D.Count());
        if R.FilterGroup() <> 0 then
            Error('filter group after the run: %1', R.FilterGroup());
        FilterZ(F, Shape);
        for G := 0 to 6 do begin
            R.FilterGroup(G);
            F.FilterGroup(G);
            if R.GetFilters() <> F.GetFilters() then
                Error('group %1 filters after the run: %2', G, R.GetFilters());
        end;
        R.FilterGroup(0);
    end;

    // The inner insert of X row 50 is rolled back (corpus cells, 'durable rows').
    local procedure CheckRows()
    var
        X: Record "PF X";
    begin
        if X.Get(50) then
            Error('X row 50 kept');
    end;

    local procedure SeedX()
    var
        R: Record "PF X";
    begin
        R.DeleteAll();
        R.Init(); R.Id := 1; R.Name := 'caller'; R."Doc Type" := 1; R.Amount := 5; R.Kind := R.Kind::Order; R.Insert();
        R.Init(); R.Id := 2; R.Name := 'other'; R."Doc Type" := 2; R.Amount := 50; R.Kind := R.Kind::Order; R.Insert();
    end;

    local procedure FilterX(var R: Record "PF X"; Shape: Text)
    begin
        case Shape of
            'PageMix':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixKey':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Id, '1'); end;
            'PageMixOtherDate':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, CalcDate('<+1D>', WorkDate())); end;
            'PageMixGroup4':
                begin R.FilterGroup(4); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'Three':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); R.SetRange(Amount, 5); end;
            'Two':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); end;
            'KeyOnName':
                R.SetRange(Name, 'caller');
            'ThreeWithKey':
                begin R.SetRange(Id, 1); R.SetRange(Name, 'caller'); R.SetRange(Amount, 5); end;
            'PageMixViewDocType':
                begin R.FilterGroup(2); R.SetRange("Doc Type", 1); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixSameField':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetRange(Kind, R.Kind::Order); end;
            'PageMixExtraName':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Name, 'caller'); end;
            'Key':
                R.SetRange(Id, 1);
        end;
    end;

    local procedure SeedY()
    var
        R: Record "PF Y";
    begin
        R.DeleteAll();
        R.Init(); R.Id := 1; R.Name := 'caller'; R."Doc Type" := 1; R.Amount := 5; R.Kind := R.Kind::Order; R.Insert();
        R.Init(); R.Id := 2; R.Name := 'other'; R."Doc Type" := 2; R.Amount := 50; R.Kind := R.Kind::Order; R.Insert();
    end;

    local procedure FilterY(var R: Record "PF Y"; Shape: Text)
    begin
        case Shape of
            'PageMix':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixKey':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Id, '1'); end;
            'PageMixOtherDate':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, CalcDate('<+1D>', WorkDate())); end;
            'PageMixGroup4':
                begin R.FilterGroup(4); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'Three':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); R.SetRange(Amount, 5); end;
            'Two':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); end;
            'KeyOnName':
                R.SetRange(Name, 'caller');
            'ThreeWithKey':
                begin R.SetRange(Id, 1); R.SetRange(Name, 'caller'); R.SetRange(Amount, 5); end;
            'PageMixViewDocType':
                begin R.FilterGroup(2); R.SetRange("Doc Type", 1); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixSameField':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetRange(Kind, R.Kind::Order); end;
            'PageMixExtraName':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Name, 'caller'); end;
            'Key':
                R.SetRange(Id, 1);
        end;
    end;

    local procedure SeedZ()
    var
        R: Record "PF Z";
    begin
        R.DeleteAll();
        R.Init(); R.Id := 1; R.Name := 'caller'; R."Doc Type" := 1; R.Amount := 5; R.Kind := R.Kind::Order; R.Insert();
        R.Init(); R.Id := 2; R.Name := 'other'; R."Doc Type" := 2; R.Amount := 50; R.Kind := R.Kind::Order; R.Insert();
    end;

    local procedure FilterZ(var R: Record "PF Z"; Shape: Text)
    begin
        case Shape of
            'PageMix':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixKey':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Id, '1'); end;
            'PageMixOtherDate':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, CalcDate('<+1D>', WorkDate())); end;
            'PageMixGroup4':
                begin R.FilterGroup(4); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'Three':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); R.SetRange(Amount, 5); end;
            'Two':
                begin R.SetRange(Name, 'caller'); R.SetRange("Doc Type", 1); end;
            'KeyOnName':
                R.SetRange(Name, 'caller');
            'ThreeWithKey':
                begin R.SetRange(Id, 1); R.SetRange(Name, 'caller'); R.SetRange(Amount, 5); end;
            'PageMixViewDocType':
                begin R.FilterGroup(2); R.SetRange("Doc Type", 1); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); end;
            'PageMixSameField':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetRange(Kind, R.Kind::Order); end;
            'PageMixExtraName':
                begin R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter(Name, 'caller'); end;
            'Key':
                R.SetRange(Id, 1);
        end;
    end;

    var
        Mode: Codeunit "PF Mode";
}

// The inner run ends without an error ('err' is not in its steps), a shape the twins do not cover. It has its own
// codeunit so that its refusal does not skip the tests after it.
codeunit 61471 "PF Tests Ends"
{
    Subtype = Test;

    [Test]
    procedure Neighbor_RecZ_Key_ReleasedThenHeld_Ends()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        R.SetRange(Id, 1);
        Commit();
        Mode.Set('startx,txb,fya', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    local procedure SeedZ()
    var
        R: Record "PF Z";
    begin
        R.DeleteAll();
        R.Init(); R.Id := 1; R.Name := 'caller'; R."Doc Type" := 1; R.Amount := 5; R.Kind := R.Kind::Order; R.Insert();
        R.Init(); R.Id := 2; R.Name := 'other'; R."Doc Type" := 2; R.Amount := 50; R.Kind := R.Kind::Order; R.Insert();
    end;

    var
        Mode: Codeunit "PF Mode";
}

// Tables whose primary key has two fields: the plain PageMix arm must refuse them like main. W: key(PK; Id, Code), the
// view field Kind outside the key. V: key(PK; Kind, Id), the view field Kind inside the key.
table 61475 "PF W"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Code"; Code[10]) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(4; Kind; Enum "PF Kind") { }
    }
    keys { key(PK; Id, "Code") { Clustered = true; } }
}

table 61476 "PF V"
{
    fields
    {
        field(1; Kind; Enum "PF Kind") { }
        field(2; Id; Integer) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Kind, Id) { Clustered = true; } }
}

codeunit 61477 "PF Inner W"
{
    TableNo = "PF W";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        Mode.Run();
    end;
}

codeunit 61478 "PF Inner V"
{
    TableNo = "PF V";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        Mode.Run();
    end;
}

// Corpus PR 1616's table shape (ConsistentGuardedRunRecTwoKey.al:50-67, GRK X/Z): key(PK; "Doc Kind", "No."), an Enum
// and a Code field, like Purchase Header. Kind is an Enum outside the key.
table 61480 "PF K"
{
    fields
    {
        field(1; "Doc Kind"; Enum "PF Kind") { }
        field(2; "No."; Code[20]) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(4; Kind; Enum "PF Kind") { }
    }
    keys { key(PK; "Doc Kind", "No.") { Clustered = true; } }
}

codeunit 61481 "PF Inner K"
{
    TableNo = "PF K";
    trigger OnRun()
    var
        Mode: Codeunit "PF Mode";
    begin
        Mode.Run();
    end;
}

codeunit 61479 "PF Tests Key"
{
    Subtype = Test;

    [Test]
    procedure Neighbor_RecW_PageMix_TwoFieldKey_MarkedAgain()
    var
        R: Record "PF W";
    begin
        R.DeleteAll();
        R.Init(); R.Id := 1; R."Code" := 'a'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(1, 'a');
        R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner W", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecV_PageMix_ViewInKey_MarkedAgain()
    var
        R: Record "PF V";
    begin
        R.DeleteAll();
        R.Init(); R.Kind := R.Kind::Order; R.Id := 1; R.Insert();
        R.Get(R.Kind::Order, 1);
        R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner V", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure RecK_PageMix_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure RecK_PageMixKey_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetFilter("No.", '1');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMixKeySetRange_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate()); R.SetRange("No.", '1');
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMixOut_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange(Kind, R.Kind::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMixViewSetFilter_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetFilter("Doc Kind", '%1', R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMix_TwoKey_Fails()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMix_TwoKey_ReleasedThenHeld()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, WorkDate());
        Commit();
        Mode.Set('startx,txb,fya,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure Neighbor_RecK_PageMixOtherDate_TwoKey_MarkedAgain()
    var
        R: Record "PF K";
    begin
        R.DeleteAll();
        R.Init(); R."Doc Kind" := R."Doc Kind"::Order; R."No." := '1'; R.Kind := R.Kind::Order; R.Insert();
        R.Get(R."Doc Kind"::Order, '1');
        R.FilterGroup(2); R.SetRange("Doc Kind", R."Doc Kind"::Order); R.FilterGroup(0); R.SetRange("Date Filter", 0D, CalcDate('<-1D>', WorkDate()));
        Commit();
        Mode.Set('startx,fya,fyc,err', false);
        if Codeunit.Run(Codeunit::"PF Inner K", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
        Commit();
    end;

    var
        Mode: Codeunit "PF Mode";
}
