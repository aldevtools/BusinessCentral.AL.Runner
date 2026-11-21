//! A record handed by value to a procedure of another table, `Buffer.Proc(Entry)` (al-runner issue 8385): the copy has the
//! view of a fresh record, flagged by `by-value-table-proc-other-table` (corpus PR 1451,
//! record/filters/ByValueTableProcedureView.al). The callers the probe did not measure keep the refusal.

use crate::record_copy_reports::{PASS, VIEW_REFUSED, check, run};

const OTHER_TABLE: &str = r##"
table 50170 "BO Entry"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Item Code"; Code[20]) { }
        field(3; "Location Code"; Code[10]) { }
        field(4; Quantity; Decimal) { }
        field(5; "Location Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(6; "Item Qty"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BO Entry".Quantity where("Item Code" = field("Item Code"), "Location Code" = field("Location Filter")));
        }
    }
    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
        key(ByItem; "Item Code", "Location Code") { SumIndexFields = Quantity; }
    }

    procedure ViaBuffer(): Text
    var
        Buffer: Record "BO Buffer";
    begin
        exit(Buffer.Describe(Rec));
    end;
}

table 50171 "BO Buffer"
{
    TableType = Temporary;
    fields { field(1; "Line No."; Integer) { } field(2; Info; Text[250]) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }

    procedure Describe(E: Record "BO Entry"): Text
    begin
        exit('id=' + Format(E."Entry No.") + ' f={' + E.GetFilters() + '} n=' + Format(E.Count()) + ' key=' + E.CurrentKey()
            + ' asc=' + Format(E.Ascending()) + ' grp=' + Format(E.FilterGroup()));
    end;

    procedure RecRefOf(E: Record "BO Entry"): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(E);
        exit('rid=' + Format(RecRef.RecordId) + ' f={' + RecRef.GetFilters() + '} n=' + Format(RecRef.Count()) + ' view=' + RecRef.GetView(false));
    end;

    procedure FromOwnCode(): Text
    var
        Entry: Record "BO Entry";
    begin
        Entry.SetRange("Item Code", 'ITEM-A');
        exit(Describe(Entry));
    end;
}

table 50172 "BO Holder"
{
    fields { field(1; "Line No."; Integer) { } field(2; Info; Text[250]) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }

    trigger OnInsert()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Entry.SetRange("Item Code", 'ITEM-A');
        Info := CopyStr(Buffer.Describe(Entry), 1, MaxStrLen(Info));
    end;

    procedure Describe(E: Record "BO Entry"): Text
    begin
        exit('id=' + Format(E."Entry No.") + ' f={' + E.GetFilters() + '} n=' + Format(E.Count()) + ' key=' + E.CurrentKey());
    end;

    procedure FromOtherTable(): Text
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Entry.SetCurrentKey("Item Code", "Location Code");
        Entry.SetRange("Item Code", 'ITEM-A');
        Entry.FindFirst();
        exit(Buffer.Describe(Entry));
    end;
}

tableextension 50173 "BO Holder Ext" extends "BO Holder"
{
    procedure ExtDescribe(E: Record "BO Entry"): Text
    begin
        exit('f={' + E.GetFilters() + '}');
    end;

    procedure FromExtension(): Text
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Entry.SetRange("Item Code", 'ITEM-A');
        exit(Buffer.Describe(Entry));
    end;
}

codeunit 50174 "BO Availability"
{
    // The reporter's shape (issue 8385): Base Application's codeunit 5530 "Calc. Item Availability".TryGetQtyOnInventory
    // hands a filtered Item Ledger Entry to table 5530 "Inventory Event Buffer".TransferInventoryQty.
    procedure Collect(ItemCode: Code[20]; var Buffer: Record "BO Buffer"): Text
    var
        Entry: Record "BO Entry";
    begin
        Entry.SetCurrentKey("Item Code", "Location Code");
        Entry.SetRange("Item Code", ItemCode);
        Entry.FindFirst();
        Entry.SetRange("Location Code", Entry."Location Code");
        Entry.CalcSums(Quantity);
        exit(Buffer.RecRefOf(Entry));
    end;
}

codeunit 50175 "BO Test"
{
    Subtype = Test;

    local procedure Seed()
    var
        Entry: Record "BO Entry";
        Holder: Record "BO Holder";
    begin
        Entry.DeleteAll();
        Holder.DeleteAll();
        Add(1, 'ITEM-A', 'BLUE');
        Add(2, 'ITEM-A', 'BLUE');
        Add(3, 'ITEM-A', 'RED');
        Add(4, 'ITEM-B', 'BLUE');
    end;

    local procedure Add(No: Integer; Item: Code[20]; Location: Code[10])
    var
        Entry: Record "BO Entry";
    begin
        Entry."Entry No." := No;
        Entry."Item Code" := Item;
        Entry."Location Code" := Location;
        Entry.Quantity := No;
        Entry.Insert();
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    [Test]
    procedure Reporter_FromCodeunit_Flagged()
    var
        Buffer: Record "BO Buffer";
        Availability: Codeunit "BO Availability";
    begin
        Seed();
        Expect(Availability.Collect('ITEM-A', Buffer), 'rid=BO Entry: 1 f={} n=4 view=VERSION(1) SORTING(Field1)');
    end;

    [Test]
    procedure RichView_FromTestCodeunit_Flagged()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.SetCurrentKey("Item Code", "Location Code");
        Entry.Ascending(false);
        Entry.SetRange("Item Code", 'ITEM-A');
        Entry.FilterGroup(2);
        Entry.SetRange("Location Code", 'BLUE');
        Entry.FilterGroup(0);
        Entry.FindFirst();
        Entry.Mark(true);
        Expect(Buffer.Describe(Entry), 'id=2 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    [Test]
    procedure SecondaryKey_NoFilter_Flagged()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.SetCurrentKey("Item Code", "Location Code");
        Expect(Buffer.Describe(Entry), 'id=0 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    [Test]
    procedure NormalTable_TemporaryVariable_Flagged()
    var
        Entry: Record "BO Entry";
        Holder: Record "BO Holder" temporary;
    begin
        Seed();
        Entry.SetRange("Item Code", 'ITEM-B');
        Expect(Holder.Describe(Entry), 'id=0 f={} n=4 key=Entry No.');
    end;

    [Test]
    procedure FromOtherTableProcedure_Flagged()
    var
        Holder: Record "BO Holder";
    begin
        Seed();
        Expect(Holder.FromOtherTable(), 'id=1 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    [Test]
    procedure FromOtherTableTrigger_Flagged()
    var
        Holder: Record "BO Holder";
    begin
        Seed();
        Holder."Line No." := 1;
        Holder.Insert(true);
        Expect(Holder.Info, 'id=0 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    [Test]
    procedure NoFilter_KeepsMain()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.Get(3);
        Expect(Buffer.Describe(Entry), 'id=3 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    [Test]
    procedure FlowFieldFilter_Flagged()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.SetRange("Location Filter", 'BLUE');
        Entry.SetFilter("Item Qty", '>%1', 3);
        Expect(Buffer.Describe(Entry), 'id=0 f={} n=4 key=Entry No. asc=Yes grp=0');
    end;

    // ReadIsolation on the caller's record is not measured: with no filter and the plain view the copy keeps main's
    // view, the declared secondary key included.
    [Test]
    procedure ReadIsolation_SecondaryKey_NoFilter_KeepsMain()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.SetCurrentKey("Item Code", "Location Code");
        Entry.ReadIsolation := IsolationLevel::ReadCommitted;
        Expect(Buffer.Describe(Entry), 'id=0 f={} n=4 key=Item Code,Location Code asc=Yes grp=0');
    end;

    [Test]
    procedure FromParameterTableCode_Refused()
    var
        Entry: Record "BO Entry";
    begin
        Seed();
        Entry.SetRange("Item Code", 'ITEM-A');
        Entry.FindFirst();
        Expect(Entry.ViaBuffer(), '');
    end;

    [Test]
    procedure FromOwnTableCode_Refused()
    var
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Expect(Buffer.FromOwnCode(), '');
    end;

    [Test]
    procedure FromTableExtension_Refused()
    var
        Holder: Record "BO Holder";
    begin
        Seed();
        Expect(Holder.FromExtension(), '');
    end;

    [Test]
    procedure TableExtensionProcedure_Refused()
    var
        Entry: Record "BO Entry";
        Holder: Record "BO Holder";
    begin
        Seed();
        Entry.SetRange("Item Code", 'ITEM-A');
        Expect(Holder.ExtDescribe(Entry), '');
    end;

    [Test]
    procedure Group3Filter_Refused()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.FilterGroup(3);
        Entry.SetRange("Item Code", 'ITEM-A');
        Entry.FilterGroup(0);
        Expect(Buffer.Describe(Entry), '');
    end;

    [Test]
    procedure FieldDescending_Refused()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.SetCurrentKey("Item Code", "Location Code");
        Entry.SetAscending("Location Code", false);
        Entry.SetRange("Item Code", 'ITEM-A');
        Expect(Buffer.Describe(Entry), '');
    end;

    // Last: a filter in filter group 1 is seen by every record of the table in the session.
    [Test]
    procedure Group1Filter_Refused()
    var
        Entry: Record "BO Entry";
        Buffer: Record "BO Buffer";
    begin
        Seed();
        Entry.FilterGroup(1);
        Entry.SetRange("Item Code", 'ITEM-A');
        Entry.FilterGroup(0);
        Expect(Buffer.Describe(Entry), '');
    end;
}
"##;

#[test]
fn by_value_record_of_another_table_gets_a_fresh_view_from_measured_callers() {
    let o = run(OTHER_TABLE, "BO Test");
    let want = [
        ("Reporter_FromCodeunit_Flagged", PASS, true),
        ("RichView_FromTestCodeunit_Flagged", PASS, true),
        // Main kept the declared secondary key here (no filter, plain view); BC gives the primary key
        // (ByValueTableProcedureView.al:528-544).
        ("SecondaryKey_NoFilter_Flagged", PASS, true),
        ("NormalTable_TemporaryVariable_Flagged", PASS, true),
        ("FromOtherTableProcedure_Flagged", PASS, true),
        ("FromOtherTableTrigger_Flagged", PASS, true),
        ("NoFilter_KeepsMain", PASS, false),
        ("FlowFieldFilter_Flagged", PASS, true),
        ("ReadIsolation_SecondaryKey_NoFilter_KeepsMain", PASS, false),
        ("FromParameterTableCode_Refused", VIEW_REFUSED, false),
        ("FromOwnTableCode_Refused", VIEW_REFUSED, false),
        ("FromTableExtension_Refused", VIEW_REFUSED, false),
        ("TableExtensionProcedure_Refused", VIEW_REFUSED, false),
        ("Group1Filter_Refused", VIEW_REFUSED, false),
        ("Group3Filter_Refused", VIEW_REFUSED, false),
        ("FieldDescending_Refused", VIEW_REFUSED, false),
    ];
    check(&o, &want);
}
