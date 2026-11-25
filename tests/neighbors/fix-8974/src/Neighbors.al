// Neighbors of the reporter's routes: one fact flipped per test. Each reads xRec.Series in a table procedure that an event
// subscriber of a Validate of Mode reaches; the row is stored with Series OLD, the field trigger sets TRG.

table 50415 "NB XRec Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Mode; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NB XRec Cfg";
            begin
                case Cfg.GetAssign() of
                    'direct':
                        Series := 'TRG';
                    'proc':
                        AssignSeries();
                    'modify':
                        begin
                            Series := 'TRG';
                            Modify();
                        end;
                    'codeunit':
                        Cfg.AssignSeries(Rec);
                end;
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; Note; Code[20]) { }
        field(5; Seen; Code[20]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    local procedure AssignSeries()
    begin
        Series := 'TRG';
    end;

    procedure TableProcRead()
    begin
        Seen := xRec.Series;
    end;
}

codeunit 50416 "NB XRec Cfg"
{
    SingleInstance = true;

    var
        CfgMode: Text;
        AssignMode: Text;

    procedure Set(NewMode: Text; NewAssign: Text)
    begin
        CfgMode := NewMode;
        AssignMode := NewAssign;
    end;

    procedure Get(): Text
    begin
        exit(CfgMode);
    end;

    procedure GetAssign(): Text
    begin
        exit(AssignMode);
    end;

    procedure AssignSeries(var Line: Record "NB XRec Line")
    begin
        Line.Series := 'TRG';
    end;
}

codeunit 50417 "NB XRec Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NB XRec Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnBeforeValidateMode(var Rec: Record "NB XRec Line"; var xRec: Record "NB XRec Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NB XRec Cfg";
    begin
        case Cfg.Get() of
            'before-assign':
                Rec.Note := 'B';
            'before-helper':
                Helper(Rec, xRec);
        end;
    end;

    [EventSubscriber(ObjectType::Table, Database::"NB XRec Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateMode(var Rec: Record "NB XRec Line"; var xRec: Record "NB XRec Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NB XRec Cfg";
        Other: Codeunit "NB XRec Other Helper";
    begin
        case Cfg.Get() of
            'table', 'before-assign':
                Rec.TableProcRead();
            'helper':
                Helper(Rec, xRec);
            'helper-flag':
                HelperWithFlag(Rec, xRec, true);
            'reconly':
                HelperRecOnly(Rec);
            'subassign':
                begin
                    Rec.Note := 'S';
                    Rec.TableProcRead();
                end;
            'helper2':
                HelperOuter(Rec, xRec);
            'other-cu':
                Other.Helper(Rec, xRec);
            'byval':
                HelperXRecByValue(Rec, xRec);
            'swapped':
                HelperSwapped(xRec, Rec);
        end;
    end;

    local procedure Helper(var Line: Record "NB XRec Line"; var xLine: Record "NB XRec Line")
    begin
        Line.TableProcRead();
    end;

    local procedure HelperWithFlag(var Line: Record "NB XRec Line"; var xLine: Record "NB XRec Line"; Flag: Boolean)
    begin
        if Flag then
            Line.TableProcRead();
    end;

    local procedure HelperRecOnly(var Line: Record "NB XRec Line")
    begin
        Line.TableProcRead();
    end;

    local procedure HelperOuter(var Line: Record "NB XRec Line"; var xLine: Record "NB XRec Line")
    begin
        Helper(Line, xLine);
    end;

    local procedure HelperXRecByValue(var Line: Record "NB XRec Line"; xLine: Record "NB XRec Line")
    begin
        Line.TableProcRead();
    end;

    local procedure HelperSwapped(var xLine: Record "NB XRec Line"; var Line: Record "NB XRec Line")
    begin
        Line.TableProcRead();
    end;
}

codeunit 50418 "NB XRec Other Helper"
{
    procedure Helper(var Line: Record "NB XRec Line"; var xLine: Record "NB XRec Line")
    begin
        Line.TableProcRead();
    end;
}

codeunit 50419 "NB XRec Route Tests"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NB XRec Cfg";

    local procedure Run(CfgMode: Text; Assign: Text)
    var
        Line: Record "NB XRec Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
        Cfg.Set(CfgMode, Assign);
        Line.Validate(Mode, 1);
        Cfg.Set('', '');
        if Line.Seen <> 'OLD' then
            Error('xRec.Series in the table procedure: ''%1''', Line.Seen);
    end;

    [Test]
    procedure Direct_TriggerAssignsInOwnProc()
    begin
        Run('table', 'proc');
    end;

    [Test]
    procedure Direct_TriggerAssignsThroughCodeunit()
    begin
        Run('table', 'codeunit');
    end;

    [Test]
    procedure Direct_TriggerAssignsThenModifies()
    begin
        Run('table', 'modify');
    end;

    [Test]
    procedure Direct_BeforeEventAssigns()
    begin
        Run('before-assign', 'direct');
    end;

    [Test]
    procedure Direct_SubscriberAssignsRec()
    begin
        Run('subassign', 'direct');
    end;

    [Test]
    procedure Helper_NoAssignment()
    begin
        Run('helper', '');
    end;

    [Test]
    procedure Helper_WithBooleanFlag()
    begin
        Run('helper-flag', 'direct');
    end;

    [Test]
    procedure Helper_RecOnly()
    begin
        Run('reconly', 'direct');
    end;

    [Test]
    procedure Helper_TwoLevels()
    begin
        Run('helper2', 'direct');
    end;

    [Test]
    procedure Helper_OtherCodeunit()
    begin
        Run('other-cu', 'direct');
    end;

    [Test]
    procedure Helper_XRecByValue()
    begin
        Run('byval', 'direct');
    end;

    [Test]
    procedure Helper_ParametersSwapped()
    begin
        Run('swapped', 'direct');
    end;

    [Test]
    procedure Helper_FromBeforeEvent()
    begin
        Run('before-helper', '');
    end;
}
