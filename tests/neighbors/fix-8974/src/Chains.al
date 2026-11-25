// Neighbors of the codeunit and integration-event chains with a record the field trigger changed: the trigger of Mode assigns
// Series, then hands Rec to a codeunit or raises an integration event whose subscriber calls a table procedure that reads
// xRec.Series. The row is stored with Series OLD; the Validate's before-image holds OLD.

table 50420 "NC XRec Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Mode; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NC XRec Cfg";
                Worker: Codeunit "NC XRec Worker";
            begin
                case Cfg.GetAssign() of
                    'direct':
                        Series := 'TRG';
                    'modify':
                        begin
                            Series := 'TRG';
                            Modify();
                        end;
                end;
                case Cfg.Get() of
                    'codeunit':
                        Worker.CallTableProc(Rec);
                    'codeunit-assign':
                        Worker.AssignThenTableProc(Rec);
                    'event', 'event-assign':
                        OnModeElse(Rec);
                    'event-two':
                        OnModeTwo(Rec);
                end;
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; Note; Code[20]) { }
        field(5; Seen; Code[20]) { }
        field(6; Outer; Integer)
        {
            // A Validate the table's own code nests, on a record the inner trigger changes: not measured.
            trigger OnValidate()
            begin
                Validate(Mode, 1);
            end;
        }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    procedure TableProcRead()
    begin
        Seen := xRec.Series;
    end;

    [IntegrationEvent(false, false)]
    local procedure OnModeElse(var Line: Record "NC XRec Line")
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnModeTwo(var Line: Record "NC XRec Line")
    begin
    end;
}

codeunit 50421 "NC XRec Cfg"
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
}

codeunit 50422 "NC XRec Worker"
{
    procedure CallTableProc(var Line: Record "NC XRec Line")
    begin
        Line.TableProcRead();
    end;

    procedure AssignThenTableProc(var Line: Record "NC XRec Line")
    begin
        Line.Note := 'CU';
        Line.TableProcRead();
    end;
}

codeunit 50423 "NC XRec Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NC XRec Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnBeforeValidateMode(var Rec: Record "NC XRec Line"; var xRec: Record "NC XRec Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NC XRec Cfg";
    begin
        if Cfg.GetAssign() = 'before' then
            Rec.Note := 'B';
    end;

    [EventSubscriber(ObjectType::Table, Database::"NC XRec Line", 'OnModeElse', '', false, false)]
    local procedure OnModeElse(var Line: Record "NC XRec Line")
    var
        Cfg: Codeunit "NC XRec Cfg";
    begin
        if Cfg.Get() = 'event-assign' then
            Line.Note := 'SUB';
        Line.TableProcRead();
    end;

    [EventSubscriber(ObjectType::Table, Database::"NC XRec Line", 'OnModeTwo', '', false, false)]
    local procedure OnModeTwoReads(var Line: Record "NC XRec Line")
    begin
        Line.TableProcRead();
    end;
}

codeunit 50424 "NC XRec Second Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NC XRec Line", 'OnModeTwo', '', false, false)]
    local procedure OnModeTwoIdle(var Line: Record "NC XRec Line")
    begin
    end;
}

codeunit 50425 "NC XRec Caller"
{
    procedure ValidateMode(var Line: Record "NC XRec Line")
    begin
        Line.Validate(Mode, 1);
    end;
}

codeunit 50426 "NC XRec Chain Tests"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NC XRec Cfg";

    local procedure Run(CfgMode: Text; Assign: Text; FromCodeunit: Boolean)
    var
        Line: Record "NC XRec Line";
        Caller: Codeunit "NC XRec Caller";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
        Cfg.Set(CfgMode, Assign);
        if FromCodeunit then
            Caller.ValidateMode(Line)
        else
            Line.Validate(Mode, 1);
        Cfg.Set('', '');
        if Line.Seen <> 'OLD' then
            Error('xRec.Series in the table procedure: ''%1''', Line.Seen);
    end;

    [Test]
    procedure Codeunit_TriggerAssigns()
    begin
        Run('codeunit', 'direct', false);
    end;

    [Test]
    procedure Nested_Codeunit_TriggerAssigns()
    var
        Line: Record "NC XRec Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
        Cfg.Set('codeunit', 'direct');
        Line.Validate(Outer, 1);
        Cfg.Set('', '');
        if Line.Seen <> 'OLD' then
            Error('xRec.Series in the table procedure: ''%1''', Line.Seen);
    end;

    [Test]
    procedure Codeunit_TriggerAssignsThenModifies()
    begin
        Run('codeunit', 'modify', false);
    end;

    [Test]
    procedure Codeunit_BeforeEventAssigns()
    begin
        Run('codeunit', 'before', false);
    end;

    [Test]
    procedure Codeunit_CodeunitAssigns()
    begin
        Run('codeunit-assign', '', false);
    end;

    [Test]
    procedure Codeunit_TriggerAssigns_ValidateFromCodeunit()
    begin
        Run('codeunit', 'direct', true);
    end;

    [Test]
    procedure Event_TriggerAssigns()
    begin
        Run('event', 'direct', false);
    end;

    [Test]
    procedure Event_SubscriberAssigns()
    begin
        Run('event-assign', '', false);
    end;

    [Test]
    procedure Event_TriggerAssigns_TwoSubscribers()
    begin
        Run('event-two', 'direct', false);
    end;

    [Test]
    procedure Event_TriggerAssigns_ValidateFromCodeunit()
    begin
        Run('event', 'direct', true);
    end;
}
