// The per-row sequence of the probe's "all row code" table without the GetGlobalTableTriggerMask subscriber (the probe's mask 4
// is refused by main for another reason): OnDelete, same-app table extension delete triggers, both delete events and OnDatabaseDelete.
// Expected values: corpus PR 1659, record/crud/DeleteAllRowCodeOrder.al:543-545, :566-567, :610-612.
table 94030 "DRN All Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Grp; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByGrp; Grp) { }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('T' + Format(Id));
    end;
}

tableextension 94031 "DRN All Row Ext" extends "DRN All Row"
{
    trigger OnBeforeDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('xB' + Format(Id));
    end;

    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('xT' + Format(Id));
    end;

    trigger OnAfterDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('xA' + Format(Id));
    end;
}

codeunit 94032 "DRN All Subs"
{
    var
        Ctl: Codeunit "DRN Ctl";

    [EventSubscriber(ObjectType::Table, Database::"DRN All Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure AllBefore(var Rec: Record "DRN All Row"; RunTrigger: Boolean)
    begin
        Ctl.Add('B' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"DRN All Row", 'OnAfterDeleteEvent', '', false, false)]
    local procedure AllAfter(var Rec: Record "DRN All Row"; RunTrigger: Boolean)
    begin
        Ctl.Add('A' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnGetSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"DRN All Row" then
            OnDatabaseDelete := true;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseDelete', '', false, false)]
    local procedure OnDatabaseDelete(RecRef: RecordRef)
    begin
        if RecRef.Number() = Database::"DRN All Row" then
            Ctl.Add('d' + Format(RecRef.Field(1).Value()));
    end;
}

codeunit 94033 "DRN All Row Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Ctl: Codeunit "DRN Ctl";
        Assert: Codeunit "DRN Assert";

    local procedure Seed(Rows: Integer)
    var
        Row: Record "DRN All Row";
        i: Integer;
    begin
        Row.DeleteAll(false);
        for i := 1 to Rows do begin
            Row.Id := i;
            case i of
                1, 3:
                    Row.Grp := 2;
                2, 4:
                    Row.Grp := 1;
                else
                    Row.Grp := 3;
            end;
            Row.Insert(false);
        end;
        Ctl.Take();
    end;

    [Test]
    procedure AllRowCode_True_Ascending()
    var
        Row: Record "DRN All Row";
    begin
        Seed(3);
        Row.DeleteAll(true);
        Assert.AreEqual('B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes', Ctl.Take(), 'order');
    end;

    [Test]
    procedure AllRowCode_True_Descending()
    var
        Row: Record "DRN All Row";
    begin
        Seed(3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes', Ctl.Take(), 'order');
    end;

    [Test]
    procedure AllRowCode_False_Descending()
    var
        Row: Record "DRN All Row";
    begin
        Seed(3);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Assert.AreEqual('B3/No,d3,A3/No,B2/No,d2,A2/No,B1/No,d1,A1/No', Ctl.Take(), 'order');
    end;

    [Test]
    procedure AllRowCode_SingleDelete()
    var
        Row: Record "DRN All Row";
    begin
        Seed(2);
        Row.Get(1);
        Row.Delete(true);
        Assert.AreEqual('B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes', Ctl.Take(), 'delete true');
        Row.Get(2);
        Row.Delete(false);
        Assert.AreEqual('B2/No,d2,A2/No', Ctl.Take(), 'delete false');
    end;

    [Test]
    procedure AllRowCode_Grp_Ascending()
    var
        Row: Record "DRN All Row";
    begin
        Seed(5);
        Row.SetCurrentKey(Grp);
        Row.DeleteAll(true);
        Assert.AreEqual('B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B4/Yes,xB4,T4,xT4,d4,xA4,A4/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B5/Yes,xB5,T5,xT5,d5,xA5,A5/Yes', Ctl.Take(), 'order');
    end;

    [Test]
    procedure AllRowCode_Temporary_Descending()
    var
        TempRow: Record "DRN All Row" temporary;
        i: Integer;
    begin
        for i := 1 to 3 do begin
            TempRow.Id := i;
            TempRow.Insert(false);
        end;
        Ctl.Take();
        TempRow.Ascending(false);
        TempRow.DeleteAll(true);
        Assert.AreEqual('B3/Yes,xB3,T3,xT3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,xA2,A2/Yes,B1/Yes,xB1,T1,xT1,xA1,A1/Yes', Ctl.Take(), 'order');
    end;
}
