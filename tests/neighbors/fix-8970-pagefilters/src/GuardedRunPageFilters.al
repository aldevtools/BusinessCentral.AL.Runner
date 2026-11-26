// Guarded Codeunit.Run ends with Consistent(false) marks and a record argument with page-style filters (al-runner issue
// 8970). The twins of corpus PR 1600 (errors/ConsistentGuardedRunRecPage.al:2591-2799) run flagged on head; the other
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

    procedure Set(NewSteps: Text; NewChangeArg: Boolean)
    begin
        Steps := NewSteps;
        ChangeArg := NewChangeArg;
    end;

    procedure ChangesArg(): Boolean
    begin
        exit(ChangeArg);
    end;

    // startx: X held; fya,fyc: Y marked, then marked again by another variable; txb: X released by another variable;
    // fxa,txb,fxc: X marked again after a release; err: Error('inner failed').
    procedure Run()
    var
        XA: Record "PF X";
        XB: Record "PF X";
        XC: Record "PF X";
        YA: Record "PF Y";
        YC: Record "PF Y";
        Step: Text;
    begin
        foreach Step in Steps.Split(',') do
            case Step of
                'startx', 'fxa':
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
        Commit();
    end;

    [Test]
    procedure Neighbor_RecZ_Key_ReleasedThenHeld_Ends()
    var
        R: Record "PF Z";
    begin
        SeedZ();
        R.Get(1);
        FilterZ(R, 'Key');
        Commit();
        Mode.Set('startx,txb,fya', false);
        if Codeunit.Run(Codeunit::"PF Inner Z", R) then
            Error('the run returned true');
        if GetLastErrorText() <> 'inner failed' then
            Error('wrong error: %1', GetLastErrorText());
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
        Commit();
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
            'Key':
                R.SetRange(Id, 1);
        end;
    end;

    var
        Mode: Codeunit "PF Mode";
}
