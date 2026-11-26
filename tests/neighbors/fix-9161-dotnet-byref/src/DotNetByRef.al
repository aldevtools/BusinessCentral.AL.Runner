// A DotNet member writes a field of Rec through a var argument in the field's OnValidate trigger, then a table
// procedure reads xRec: through a codeunit the trigger calls (field Plain) or through the one OnAfterValidateEvent
// subscriber (field Sub). The controls write a local variable instead of a field.
dotnet
{
    assembly("mscorlib")
    {
        type("System.Collections.Generic.Dictionary`2"; PXDnDict) { }
        type("System.Int32"; PXDnInt32) { }
    }
}

table 50450 "PXD Line"
{
    DataClassification = CustomerContent;
    InherentPermissions = RIMD;
    InherentEntitlements = RIMD;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Series; Code[20]) { }
        field(3; Note; Text[30]) { }
        field(4; Seen; Code[20]) { }
        field(5; Qty; Integer) { }
        field(6; Plain; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PXD Cfg";
                Worker: Codeunit "PXD Worker";
                D: DotNet PXDnDict;
                I: DotNet PXDnInt32;
                LocalNote: Text;
                LocalQty: Integer;
            begin
                D := D.Dictionary();
                D.Add('k', 'DV');
                case Cfg.Get() of
                    'trygetvalue-field':
                        D.TryGetValue('k', Note);
                    'trygetvalue-local':
                        D.TryGetValue('k', LocalNote);
                    'tryparse-field':
                        I.TryParse('7', Qty);
                    'tryparse-local':
                        I.TryParse('7', LocalQty);
                end;
                Worker.CallTableProc(Rec);
            end;
        }
        field(7; Sub; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PXD Cfg";
                D: DotNet PXDnDict;
                I: DotNet PXDnInt32;
                LocalNote: Text;
                LocalQty: Integer;
            begin
                D := D.Dictionary();
                D.Add('k', 'DV');
                case Cfg.Get() of
                    'trygetvalue-field':
                        D.TryGetValue('k', Note);
                    'trygetvalue-local':
                        D.TryGetValue('k', LocalNote);
                    'tryparse-field':
                        I.TryParse('7', Qty);
                    'tryparse-local':
                        I.TryParse('7', LocalQty);
                end;
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
}

codeunit 50454 "PXD Cfg"
{
    SingleInstance = true;

    var
        CfgMode: Text;

    procedure Set(NewMode: Text)
    begin
        CfgMode := NewMode;
    end;

    procedure Get(): Text
    begin
        exit(CfgMode);
    end;
}

codeunit 50451 "PXD Worker"
{
    procedure CallTableProc(var Line: Record "PXD Line")
    begin
        Line.TableProcRead();
    end;
}

codeunit 50452 "PXD Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"PXD Line", 'OnAfterValidateEvent', 'Sub', false, false)]
    local procedure OnAfterValidateSub(var Rec: Record "PXD Line"; var xRec: Record "PXD Line"; CurrFieldNo: Integer)
    begin
        Rec.TableProcRead();
    end;
}

codeunit 50453 "PXD Tests"
{
    Subtype = Test;

    var
        Cfg: Codeunit "PXD Cfg";

    local procedure Prepare(var Line: Record "PXD Line")
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
    end;

    local procedure Check(Line: Record "PXD Line")
    begin
        Cfg.Set('');
        if Line.Seen <> 'OLD' then
            Error('xRec.Series in the table procedure: ''%1''', Line.Seen);
    end;

    local procedure RunPlain(CfgMode: Text)
    var
        Line: Record "PXD Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Plain, 1);
        Check(Line);
    end;

    local procedure RunSub(CfgMode: Text)
    var
        Line: Record "PXD Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Sub, 1);
        Check(Line);
    end;

    [Test]
    procedure Plain_DotNetTryGetValueField()
    begin
        RunPlain('trygetvalue-field');
    end;

    [Test]
    procedure Plain_DotNetTryGetValueLocal()
    begin
        RunPlain('trygetvalue-local');
    end;

    [Test]
    procedure Plain_DotNetTryParseField()
    begin
        RunPlain('tryparse-field');
    end;

    [Test]
    procedure Plain_DotNetTryParseLocal()
    begin
        RunPlain('tryparse-local');
    end;

    [Test]
    procedure AfterEvent_DotNetTryGetValueField()
    begin
        RunSub('trygetvalue-field');
    end;

    [Test]
    procedure AfterEvent_DotNetTryParseField()
    begin
        RunSub('tryparse-field');
    end;

    [Test]
    procedure AfterEvent_DotNetTryGetValueLocal()
    begin
        RunSub('trygetvalue-local');
    end;
}
