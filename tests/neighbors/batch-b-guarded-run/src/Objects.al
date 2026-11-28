// Guarded Codeunit.Run with Consistent(false) marks and a record argument (al-runner issues 9151, 9157, 9184): the shapes
// next to the cells of corpus PRs 1568, 1590 and 1598 that the argument check of `consistent_marks.rs` reads. Same steps as
// "GRR Steps" of errors/ConsistentGuardedRunRecFail.al (corpus master), plus a record of the table marked second (y), the
// argument's filters (see "GB Tests".Prepare), a failing Get (getfail), a failing TestField (testfield) and a filter the run
// sets on its argument (setrec).

table 98100 "GB X"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 98101 "GB Y"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 98102 "GB Z"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 98111 "GB W"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

enum 98112 "GB Kind"
{
    Extensible = false;

    value(0; Quote) { }
    value(1; Order) { }
}

// The table of a page view: a key (Enum, Code), like GPC Z of errors/ConsistentGuardedRunPageConst.al (corpus PR 1622).
table 98113 "GB V"
{
    fields
    {
        field(1; "Doc Kind"; Enum "GB Kind") { }
        field(2; "No."; Code[20]) { }
        field(3; Name; Text[30]) { }
    }
    keys { key(PK; "Doc Kind", "No.") { Clustered = true; } }
}

codeunit 98103 "GB Mode"
{
    SingleInstance = true;

    var
        Steps: Text;
        Extra: Text;
        Result: Text;

    procedure SetExtra(NewExtra: Text)
    begin
        Extra := NewExtra;
    end;

    procedure GetExtra(): Text
    begin
        exit(Extra);
    end;

    procedure SetResult(NewResult: Text)
    begin
        Result := NewResult;
    end;

    procedure GetResult(): Text
    begin
        exit(Result);
    end;

    procedure Set(NewSteps: Text)
    begin
        Steps := NewSteps;
    end;

    procedure Get(): Text
    begin
        exit(Steps);
    end;
}

// x / y: table GB X / GB Y. f = Consistent(false), t = Consistent(true). a, b, c: three local variables.
// fwa: GB W by a third local. ftmp: a temporary GB X marks.
// rec: the record argument (frec, trec: Consistent on it; modrec: Name := 'modified' and Modify; setrec: SetRange(Name, 'other')).
codeunit 98104 "GB Steps"
{
    procedure DoSteps(var RecX: Record "GB X"; var RecY: Record "GB Y"; var RecZ: Record "GB Z"; Kind: Text)
    var
        Mode: Codeunit "GB Mode";
        XA: Record "GB X";
        XB: Record "GB X";
        XC: Record "GB X";
        YA: Record "GB Y";
        YC: Record "GB Y";
        WA: Record "GB W";
        TempX: Record "GB X" temporary;
        Step: Text;
    begin
        foreach Step in Mode.Get().Split(',') do
            case Step of
                'startx':
                    StartX();
                'fxa':
                    XA.Consistent(false);
                'fxc':
                    XC.Consistent(false);
                'txa':
                    XA.Consistent(true);
                'txb':
                    XB.Consistent(true);
                'fya':
                    YA.Consistent(false);
                'fyc':
                    YC.Consistent(false);
                'fwa':
                    WA.Consistent(false);
                'ftmp':
                    TempX.Consistent(false);
                'frec':
                    case Kind of
                        'x':
                            RecX.Consistent(false);
                        'y':
                            RecY.Consistent(false);
                        'z':
                            RecZ.Consistent(false);
                    end;
                'trec':
                    case Kind of
                        'x':
                            RecX.Consistent(true);
                        'y':
                            RecY.Consistent(true);
                        'z':
                            RecZ.Consistent(true);
                    end;
                'modrec':
                    case Kind of
                        'x':
                            begin
                                RecX.Name := 'modified';
                                RecX.Modify();
                            end;
                        'z':
                            begin
                                RecZ.Name := 'modified';
                                RecZ.Modify();
                            end;
                    end;
                'setrec':
                    RecZ.SetRange(Name, 'other');
                'getfail':
                    YA.Get(999);
                'testfield':
                    TestFieldFails();
                'err':
                    Error('inner failed');
            end;
    end;

    local procedure StartX()
    var
        X: Record "GB X";
    begin
        X.Init();
        X.Id := 100;
        X.Name := 'state';
        X.Insert();
        X.Consistent(false);
    end;

    local procedure TestFieldFails()
    var
        Y: Record "GB Y";
    begin
        Y.Init();
        Y.TestField(Name);
    end;
}

codeunit 98105 "GB Inner Z"
{
    TableNo = "GB Z";

    trigger OnRun()
    var
        Steps: Codeunit "GB Steps";
        NoX: Record "GB X";
        NoY: Record "GB Y";
    begin
        Steps.DoSteps(NoX, NoY, Rec, 'z');
    end;
}

codeunit 98106 "GB Inner X"
{
    TableNo = "GB X";

    trigger OnRun()
    var
        Steps: Codeunit "GB Steps";
        NoY: Record "GB Y";
        NoZ: Record "GB Z";
    begin
        Steps.DoSteps(Rec, NoY, NoZ, 'x');
    end;
}

codeunit 98107 "GB Inner Y"
{
    TableNo = "GB Y";

    trigger OnRun()
    var
        Steps: Codeunit "GB Steps";
        NoX: Record "GB X";
        NoZ: Record "GB Z";
    begin
        Steps.DoSteps(NoX, Rec, NoZ, 'y');
    end;
}

codeunit 98114 "GB Inner V"
{
    TableNo = "GB V";

    trigger OnRun()
    var
        Steps: Codeunit "GB Steps";
        NoX: Record "GB X";
        NoY: Record "GB Y";
        NoZ: Record "GB Z";
    begin
        Steps.DoSteps(NoX, NoY, NoZ, 'v');
    end;
}

// What the page action calls, like "GPC Run".RunCopy of corpus PR 1622: runs the inner codeunit guarded on the page's Rec
// (extra 'direct') or on a local copy of it, and keeps the outcome.
codeunit 98115 "GB Run"
{
    procedure RunPageRec(var PageRec: Record "GB V")
    var
        Mode: Codeunit "GB Mode";
        R: Record "GB V";
        Ok: Boolean;
    begin
        R.Copy(PageRec);
        if Mode.GetExtra() = 'direct' then
            Ok := Codeunit.Run(Codeunit::"GB Inner V", PageRec)
        else
            Ok := Codeunit.Run(Codeunit::"GB Inner V", R);
        Mode.SetResult(StrSubstNo('run=%1;code=%2', Ok, GetLastErrorCode()));
    end;
}

page 98116 "GB V List"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GB V";
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
                    Runner: Codeunit "GB Run";
                begin
                    Runner.RunPageRec(Rec);
                end;
            }
        }
    }
}

page 98117 "GB V Worksheet"
{
    PageType = Worksheet;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GB V";
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
                    Runner: Codeunit "GB Run";
                begin
                    Runner.RunPageRec(Rec);
                end;
            }
        }
    }
}
