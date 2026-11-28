// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-deleteall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own tables 90660 to 90663, table extension 90664, SingleInstance log codeunit 90665
//   (used only by test codeunit 90667), static subscriber codeunit 90666 (table events and "Global Triggers" events of the own tables only).
// local object: the OnDelete triggers, the table extension delete triggers and the subscribers log the order of the row code of DeleteAll;
//   that order is what is under test.
// Probe (al-runner issue 9401): the order in which DeleteAll runs its row code over two or more rows, by the record's sort order:
//   Ascending(false) and Ascending(true) on the primary key, a filter, a secondary key with ties (ascending, descending, SetAscending),
//   MarkedOnly, a temporary record, a RecordRef; and the row code that runs: OnDelete alone, OnBeforeDeleteEvent and OnAfterDeleteEvent
//   subscribers alone, the "Global Triggers" OnDatabaseDelete alone, and all of them together with OnGlobalDelete and the table extension
//   OnBeforeDelete, OnDelete and OnAfterDelete triggers; DeleteAll(false) and DeleteAll() as controls.
// Probe: measured on BC 27.5, 28.5 and 29.0 (BC on Linux, the corpus CI leg setup); the three gave the same values.
// Facts:
//   - DeleteAll runs its row code row by row in the record's sort order: Ascending(false) on the primary key gives 3,2,1; a filter
//     keeps that order over the filtered rows; MarkedOnly, a temporary record and a RecordRef with Ascending(false) do the same.
//   - Ascending(false) on a secondary key reverses the whole order, the primary key inside equal key values too (Grp 2,1,2,1,3 for
//     rows 1..5: ascending 2,4,1,3,5, descending 5,3,1,4,2). SetAscending(Grp, false) reverses only Grp: 5,1,3,2,4.
//   - Per row, DeleteAll(true) runs OnBeforeDeleteEvent, the table extension OnBeforeDelete, OnDelete, the table extension OnDelete,
//     the "Global Triggers" OnDatabaseDelete, the table extension OnAfterDelete, OnAfterDeleteEvent; then the next row. A temporary
//     record runs the same without OnDatabaseDelete.
//   - DeleteAll(false) runs no table or table extension trigger (DeleteAll() neither, measured on the OnDelete-only table), but runs OnBeforeDeleteEvent,
//     OnDatabaseDelete and OnAfterDeleteEvent per row (RunTrigger = No), in the same sort order.
//   - OnGlobalDelete (trigger mask 4) is not raised by DeleteAll, nor by a single Delete.
// Every test records what BC produced with Observer.Expect and ends in Verify().
// BC versions: 27, 28, 29

// OnDelete only.
table 90660 "DRO Row"
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
        Log: Codeunit "DRO Log";
    begin
        Log.Add('T' + Format(Id));
    end;
}

// No OnDelete; OnBeforeDeleteEvent and OnAfterDeleteEvent subscribers only.
table 90661 "DRO Ev Row"
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
}

// No OnDelete; "Global Triggers" OnDatabaseDelete only.
table 90662 "DRO Db Row"
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
}

// OnDelete, table extension triggers, table event subscribers, OnGlobalDelete (mask 4) and OnDatabaseDelete.
table 90663 "DRO All Row"
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
        Log: Codeunit "DRO Log";
    begin
        Log.Add('T' + Format(Id));
    end;
}

tableextension 90664 "DRO All Row Ext" extends "DRO All Row"
{
    trigger OnBeforeDelete()
    var
        Log: Codeunit "DRO Log";
    begin
        Log.Add('xB' + Format(Id));
    end;

    trigger OnDelete()
    var
        Log: Codeunit "DRO Log";
    begin
        Log.Add('xT' + Format(Id));
    end;

    trigger OnAfterDelete()
    var
        Log: Codeunit "DRO Log";
    begin
        Log.Add('xA' + Format(Id));
    end;
}

codeunit 90665 "DRO Log"
{
    SingleInstance = true;

    var
        Seen: Text;

    procedure Add(Entry: Text)
    begin
        if Seen <> '' then
            Seen += ',';
        Seen += Entry;
    end;

    procedure Take() Result: Text
    begin
        Result := Seen;
        Seen := '';
    end;
}

codeunit 90666 "DRO Subs"
{
    var
        Log: Codeunit "DRO Log";

    [EventSubscriber(ObjectType::Table, Database::"DRO Ev Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure EvBefore(var Rec: Record "DRO Ev Row"; RunTrigger: Boolean)
    begin
        Log.Add('B' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"DRO Ev Row", 'OnAfterDeleteEvent', '', false, false)]
    local procedure EvAfter(var Rec: Record "DRO Ev Row"; RunTrigger: Boolean)
    begin
        Log.Add('A' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"DRO All Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure AllBefore(var Rec: Record "DRO All Row"; RunTrigger: Boolean)
    begin
        Log.Add('B' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"DRO All Row", 'OnAfterDeleteEvent', '', false, false)]
    local procedure AllAfter(var Rec: Record "DRO All Row"; RunTrigger: Boolean)
    begin
        Log.Add('A' + Format(Rec.Id) + '/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetGlobalTableTriggerMask', '', false, false)]
    local procedure OnGetMask(TableId: Integer; var TableTriggerMask: Integer)
    begin
        if TableId = Database::"DRO All Row" then
            TableTriggerMask := 4;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnGetSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId in [Database::"DRO Db Row", Database::"DRO All Row"] then
            OnDatabaseDelete := true;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnGlobalDelete', '', false, false)]
    local procedure OnGlobalDelete(RecRef: RecordRef)
    begin
        if RecRef.Number() in [Database::"DRO Db Row", Database::"DRO All Row"] then
            Log.Add('g' + Format(RecRef.Field(1).Value()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseDelete', '', false, false)]
    local procedure OnDatabaseDelete(RecRef: RecordRef)
    begin
        if RecRef.Number() in [Database::"DRO Db Row", Database::"DRO All Row"] then
            Log.Add('d' + Format(RecRef.Field(1).Value()));
    end;
}

codeunit 90667 "DRO Probe"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "DRO Log";

    // Rows 1..Rows; with five rows Grp is 2,1,2,1,3 (ties on Grp, so the primary key decides inside a Grp value).
    local procedure Seed(TableNo: Integer; Rows: Integer)
    var
        RecRef: RecordRef;
        i: Integer;
    begin
        RecRef.Open(TableNo);
        RecRef.DeleteAll(false);
        for i := 1 to Rows do begin
            RecRef.Init();
            RecRef.Field(1).Value := i;
            RecRef.Field(2).Value := GrpOf(i);
            RecRef.Insert(false);
        end;
        RecRef.Close();
        Log.Take();
    end;

    local procedure GrpOf(i: Integer): Integer
    begin
        case i of
            1, 3:
                exit(2);
            2, 4:
                exit(1);
        end;
        exit(3);
    end;

    local procedure Left(TableNo: Integer) Result: Text
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(TableNo);
        if RecRef.FindSet() then
            repeat
                if Result <> '' then
                    Result += ',';
                Result += Format(RecRef.Field(1).Value());
            until RecRef.Next() = 0;
        if Result = '' then
            Result := '<none>';
    end;

    // The reporter's repro: OnDelete only, three rows, primary key descending and ascending.
    [Test]
    procedure OnDeleteOnly_PrimaryKey_Order()
    var
        Row: Record "DRO Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Row", 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('pk_descending', Log.Take());
        Observer.Observe('pk_descending|left', Left(Database::"DRO Row"));

        Seed(Database::"DRO Row", 3);
        Clear(Row);
        Row.Ascending(true);
        Row.DeleteAll(true);
        Observer.Observe('pk_ascending', Log.Take());
        Observer.Observe('pk_ascending|left', Left(Database::"DRO Row"));

        Seed(Database::"DRO Row", 5);
        Clear(Row);
        Row.SetRange(Id, 2, 4);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('pk_descending_filter', Log.Take());
        Observer.Observe('pk_descending_filter|left', Left(Database::"DRO Row"));

        Seed(Database::"DRO Row", 5);
        Clear(Row);
        Row.SetRange(Id, 2, 4);
        Row.DeleteAll(true);
        Observer.Observe('pk_ascending_filter', Log.Take());
        Observer.Expect('pk_descending', 'T3,T2,T1');
        Observer.Expect('pk_descending|left', '<none>');
        Observer.Expect('pk_ascending', 'T1,T2,T3');
        Observer.Expect('pk_ascending|left', '<none>');
        Observer.Expect('pk_descending_filter', 'T4,T3,T2');
        Observer.Expect('pk_descending_filter|left', '1,5');
        Observer.Expect('pk_ascending_filter', 'T2,T3,T4');
        Observer.Verify();
    end;

    // OnDelete only, secondary key with ties.
    [Test]
    procedure OnDeleteOnly_SecondaryKey_Order()
    var
        Row: Record "DRO Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Row", 5);
        Row.SetCurrentKey(Grp);
        Row.DeleteAll(true);
        Observer.Observe('grp_ascending', Log.Take());

        Seed(Database::"DRO Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('grp_descending', Log.Take());

        Seed(Database::"DRO Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.SetAscending(Grp, false);
        Row.DeleteAll(true);
        Observer.Observe('grp_setascending_false', Log.Take());

        Seed(Database::"DRO Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.SetRange(Grp, 1, 2);
        Row.DeleteAll(true);
        Observer.Observe('grp_descending_filter', Log.Take());
        Observer.Observe('grp_descending_filter|left', Left(Database::"DRO Row"));
        Observer.Expect('grp_ascending', 'T2,T4,T1,T3,T5');
        Observer.Expect('grp_descending', 'T5,T3,T1,T4,T2');
        Observer.Expect('grp_setascending_false', 'T5,T1,T3,T2,T4');
        Observer.Expect('grp_descending_filter', 'T3,T1,T4,T2');
        Observer.Expect('grp_descending_filter|left', '5');
        Observer.Verify();
    end;

    // OnDelete only: MarkedOnly, a temporary record and a RecordRef, all descending.
    [Test]
    procedure OnDeleteOnly_OtherForms_Descending()
    var
        Row: Record "DRO Row";
        TempRow: Record "DRO Row" temporary;
        RecRef: RecordRef;
        i: Integer;
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Row", 5);
        Row.Get(1);
        Row.Mark(true);
        Row.Get(3);
        Row.Mark(true);
        Row.Get(4);
        Row.Mark(true);
        Row.MarkedOnly(true);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('markedonly_descending', Log.Take());
        Observer.Observe('markedonly_descending|left', Left(Database::"DRO Row"));

        for i := 1 to 3 do begin
            TempRow.Init();
            TempRow.Id := i;
            TempRow.Grp := GrpOf(i);
            TempRow.Insert(false);
        end;
        Log.Take();
        TempRow.Ascending(false);
        TempRow.DeleteAll(true);
        Observer.Observe('temporary_descending', Log.Take());
        Observer.Observe('temporary_descending|count', Format(TempRow.Count()));

        Seed(Database::"DRO Row", 3);
        RecRef.Open(Database::"DRO Row");
        RecRef.Ascending(false);
        RecRef.DeleteAll(true);
        Observer.Observe('recordref_descending', Log.Take());
        Observer.Observe('recordref_descending|left', Left(Database::"DRO Row"));
        Observer.Expect('markedonly_descending', 'T4,T3,T1');
        Observer.Expect('markedonly_descending|left', '2,5');
        Observer.Expect('temporary_descending', 'T3,T2,T1');
        Observer.Expect('temporary_descending|count', '0');
        Observer.Expect('recordref_descending', 'T3,T2,T1');
        Observer.Expect('recordref_descending|left', '<none>');
        Observer.Verify();
    end;

    // OnDelete only: DeleteAll(false) and DeleteAll() run no row code.
    [Test]
    procedure OnDeleteOnly_NoRunTrigger_Descending()
    var
        Row: Record "DRO Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Row", 3);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Observer.Observe('false_descending', Log.Take());
        Observer.Observe('false_descending|left', Left(Database::"DRO Row"));

        Seed(Database::"DRO Row", 3);
        Clear(Row);
        Row.Ascending(false);
        Row.DeleteAll();
        Observer.Observe('default_descending', Log.Take());
        Observer.Observe('default_descending|left', Left(Database::"DRO Row"));
        Observer.Expect('false_descending', '');
        Observer.Expect('false_descending|left', '<none>');
        Observer.Expect('default_descending', '');
        Observer.Expect('default_descending|left', '<none>');
        Observer.Verify();
    end;

    // Table event subscribers only, no OnDelete.
    [Test]
    procedure EventsOnly_Order()
    var
        Row: Record "DRO Ev Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Ev Row", 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_descending', Log.Take());

        Seed(Database::"DRO Ev Row", 3);
        Clear(Row);
        Row.DeleteAll(true);
        Observer.Observe('true_ascending', Log.Take());

        Seed(Database::"DRO Ev Row", 3);
        Clear(Row);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Observer.Observe('false_descending', Log.Take());

        Seed(Database::"DRO Ev Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_grp_descending', Log.Take());

        Seed(Database::"DRO Ev Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Observer.Observe('false_grp_descending', Log.Take());
        Observer.Observe('left', Left(Database::"DRO Ev Row"));
        Observer.Expect('true_descending', 'B3/Yes,A3/Yes,B2/Yes,A2/Yes,B1/Yes,A1/Yes');
        Observer.Expect('true_ascending', 'B1/Yes,A1/Yes,B2/Yes,A2/Yes,B3/Yes,A3/Yes');
        Observer.Expect('false_descending', 'B3/No,A3/No,B2/No,A2/No,B1/No,A1/No');
        Observer.Expect('true_grp_descending', 'B5/Yes,A5/Yes,B3/Yes,A3/Yes,B1/Yes,A1/Yes,B4/Yes,A4/Yes,B2/Yes,A2/Yes');
        Observer.Expect('false_grp_descending', 'B5/No,A5/No,B3/No,A3/No,B1/No,A1/No,B4/No,A4/No,B2/No,A2/No');
        Observer.Expect('left', '<none>');
        Observer.Verify();
    end;

    // "Global Triggers" OnDatabaseDelete only, no OnDelete.
    [Test]
    procedure DatabaseTriggerOnly_Order()
    var
        Row: Record "DRO Db Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO Db Row", 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_descending', Log.Take());

        Seed(Database::"DRO Db Row", 3);
        Clear(Row);
        Row.DeleteAll(true);
        Observer.Observe('true_ascending', Log.Take());

        Seed(Database::"DRO Db Row", 3);
        Clear(Row);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Observer.Observe('false_descending', Log.Take());

        Seed(Database::"DRO Db Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_grp_descending', Log.Take());
        Observer.Observe('left', Left(Database::"DRO Db Row"));
        Observer.Expect('true_descending', 'd3,d2,d1');
        Observer.Expect('true_ascending', 'd1,d2,d3');
        Observer.Expect('false_descending', 'd3,d2,d1');
        Observer.Expect('true_grp_descending', 'd5,d3,d1,d4,d2');
        Observer.Expect('left', '<none>');
        Observer.Verify();
    end;

    // OnDelete, table extension triggers, subscribers, OnGlobalDelete and OnDatabaseDelete together: primary key.
    [Test]
    procedure AllRowCode_PrimaryKey_Order()
    var
        Row: Record "DRO All Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO All Row", 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_descending', Log.Take());

        Seed(Database::"DRO All Row", 3);
        Clear(Row);
        Row.DeleteAll(true);
        Observer.Observe('true_ascending', Log.Take());

        Seed(Database::"DRO All Row", 3);
        Clear(Row);
        Row.Ascending(false);
        Row.DeleteAll(false);
        Observer.Observe('false_descending', Log.Take());

        Seed(Database::"DRO All Row", 5);
        Clear(Row);
        Row.SetRange(Id, 2, 4);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_descending_filter', Log.Take());
        Observer.Observe('true_descending_filter|left', Left(Database::"DRO All Row"));
        Observer.Expect('true_descending', 'B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes');
        Observer.Expect('true_ascending', 'B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes');
        Observer.Expect('false_descending', 'B3/No,d3,A3/No,B2/No,d2,A2/No,B1/No,d1,A1/No');
        Observer.Expect('true_descending_filter', 'B4/Yes,xB4,T4,xT4,d4,xA4,A4/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes');
        Observer.Expect('true_descending_filter|left', '1,5');
        Observer.Verify();
    end;

    // Control for the row code of the table with all of it: one Delete(true) and one Delete(false) of a single row
    // (does OnGlobalDelete run for a single Delete where DeleteAll does not call it).
    [Test]
    procedure AllRowCode_SingleDelete_Control()
    var
        Row: Record "DRO All Row";
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO All Row", 2);
        Row.Get(1);
        Row.Delete(true);
        Observer.Observe('delete_true', Log.Take());
        Row.Get(2);
        Row.Delete(false);
        Observer.Observe('delete_false', Log.Take());
        Observer.Expect('delete_true', 'B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes');
        Observer.Expect('delete_false', 'B2/No,d2,A2/No');
        Observer.Verify();
    end;

    // All row code together: secondary key, temporary record, RecordRef.
    [Test]
    procedure AllRowCode_OtherForms_Order()
    var
        Row: Record "DRO All Row";
        TempRow: Record "DRO All Row" temporary;
        RecRef: RecordRef;
        i: Integer;
        Observer: Codeunit "Probe Observer";
    begin
        Seed(Database::"DRO All Row", 5);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Observer.Observe('true_grp_descending', Log.Take());

        Seed(Database::"DRO All Row", 5);
        Clear(Row);
        Row.SetCurrentKey(Grp);
        Row.DeleteAll(true);
        Observer.Observe('true_grp_ascending', Log.Take());

        for i := 1 to 3 do begin
            TempRow.Init();
            TempRow.Id := i;
            TempRow.Grp := GrpOf(i);
            TempRow.Insert(false);
        end;
        Log.Take();
        TempRow.Ascending(false);
        TempRow.DeleteAll(true);
        Observer.Observe('temporary_true_descending', Log.Take());

        Seed(Database::"DRO All Row", 3);
        RecRef.Open(Database::"DRO All Row");
        RecRef.Ascending(false);
        RecRef.DeleteAll(true);
        Observer.Observe('recordref_true_descending', Log.Take());
        Observer.Observe('left', Left(Database::"DRO All Row"));
        Observer.Expect('true_grp_descending', 'B5/Yes,xB5,T5,xT5,d5,xA5,A5/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes,B4/Yes,xB4,T4,xT4,d4,xA4,A4/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes');
        Observer.Expect('true_grp_ascending', 'B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B4/Yes,xB4,T4,xT4,d4,xA4,A4/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes,B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B5/Yes,xB5,T5,xT5,d5,xA5,A5/Yes');
        Observer.Expect('temporary_true_descending', 'B3/Yes,xB3,T3,xT3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,xA2,A2/Yes,B1/Yes,xB1,T1,xT1,xA1,A1/Yes');
        Observer.Expect('recordref_true_descending', 'B3/Yes,xB3,T3,xT3,d3,xA3,A3/Yes,B2/Yes,xB2,T2,xT2,d2,xA2,A2/Yes,B1/Yes,xB1,T1,xT1,d1,xA1,A1/Yes');
        Observer.Expect('left', '<none>');
        Observer.Verify();
    end;
}
