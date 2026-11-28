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

codeunit 98103 "GB Mode"
{
    SingleInstance = true;

    var
        Steps: Text;

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
