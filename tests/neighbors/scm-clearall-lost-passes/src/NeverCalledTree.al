// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86890 to 86900
// Probe: ClearAll in a TableNo owner codeunit ("Sales-Post" shape) that a plain holder codeunit ("Whse.-Activity-Post" shape)
// gave its register codeunit through a setter `SetReg(var R)` before `Owner.Run(Header)`, when NO routine of the register ran
// before the ClearAll. The owner calls ClearAll in a local procedure called from a procedure called from OnRun. The register tree
// has the global kinds of "Whse. Jnl.-Register Line" in BC 29: register (TableNo) with 4 non-temporary records, codeunits A and B,
// Integer, Labels and Boolean; A ("Item Tracking Management" shape) with 3 temporary and 2 non-temporary records, SingleInstance
// C and U, Integer, Date, 4 Booleans and Labels; B ("WMS Management" shape) with 5 non-temporary records, 1 temporary record, A,
// plain W (Labels only), U, 2 Booleans, Integer and Labels; U (SingleInstance) with 2 non-temporary records and Labels; C
// (SingleInstance) holding SingleInstance I (one Boolean). U and I are primed through other variables at the start of every test.
// Observed after the ClearAll: every global, whether the instance is still shared by holder and owner (writes both ways, also from
// inside OnRun), and registers run afterwards by the holder (once, twice) and by the owner (in and after OnRun), with a register
// entry row numbered on the first run and modified on later runs. Neighbor cells where the register was written or run before the
// ClearAll separate "reset" from "never set". al-runner issue 8964.
// singleinstance-shared-on-purpose: the subject is SingleInstance codeunits held below a register that ClearAll resets.
// BC versions: 27, 28, 29
// What BC did (local BC 27.5, 28.5 and 29.0 legs gave the same values; the pull request CI run asserts them on 27, 28, 29):
// - Register never called, owner ClearAll: every plain global of the register, A, B, the A below B and W reads its default
//   (records: no field values, no filters; temporary records: no rows; Integer 0, Date 0D, Booleans false). Labels keep their
//   text. The SingleInstance U (reached as B.U, A.U and B.A.U) keeps the record field value, filter and Txt primed through
//   another variable, and C/I keeps the primed Boolean (true). A write to U through another variable after the ClearAll is seen
//   through every path of the register. No register entry row is written.
// - Neighbor, register written before the ClearAll (SetAll through the holder): the same ClearAll resets every plain global that
//   was set (field values, filters, Integer, Date, Booleans), temporary records keep their rows (Count 1), Labels keep their
//   text, U and C/I keep their values.
// - The register stays shared after the ClearAll: a write through the owner's variable (after OnRun, or inside OnRun right after
//   the ClearAll) is read through the holder's variable, and a write through the holder's is read through the owner's.
// - Runs after the ClearAll use the one shared instance: the first run (by the holder, or by the owner inside OnRun) numbers a
//   new entry row after the last existing one (No. 5 after row 4) and inserts it; every later run, by the holder or the owner,
//   increments the run counter (2) and modifies that same row (Runs 2). Both variables read the same counter and entry No.
// - Neighbor, register run once before the ClearAll (entry 5 inserted, counter 1): the ClearAll resets the entry record and the
//   counter (No. 0, counter 0), so the next run inserts a new row No. 6 with counter 1.
table 86890 "CNC Reg Entry"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Runs; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 86891 "CNC I"
{
    SingleInstance = true;

    var
        Flag: Boolean;

    procedure SetFlag(V: Boolean)
    begin
        Flag := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('i[%1]', Flag));
    end;
}

codeunit 86892 "CNC C"
{
    SingleInstance = true;

    var
        Impl: Codeunit "CNC I";

    procedure State(): Text
    begin
        exit('c' + Impl.State());
    end;
}

codeunit 86893 "CNC U"
{
    SingleInstance = true;

    var
        Uom: Record "CKK Row";
        Item: Record "CKK Row";
        ULbl: Label 'u label', Comment = 'a comment';
        UPlain: Label 'u plain';

    procedure Set(V: Integer)
    begin
        Uom."No." := V;
        Uom.SetRange("No.", V);
        Item.Txt := 'u' + Format(V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('u[%1,%2,%3,%4,%5]', Uom."No.", Uom.GetFilters(), Item.Txt, ULbl, UPlain));
    end;
}

codeunit 86894 "CNC W"
{
    var
        WLbl: Label 'w label', Comment = 'a comment';
        WPlain: Label 'w plain';

    procedure State(): Text
    begin
        exit(StrSubstNo('w[%1,%2]', WLbl, WPlain));
    end;
}

codeunit 86895 "CNC A"
{
    var
        TempA1: Record "CKK Row" temporary;
        TempA2: Record "CKK Row" temporary;
        TempA3: Record "CKK Row" temporary;
        N1: Record "CKK Row";
        N2: Record "CKK Row";
        Confirm: Codeunit "CNC C";
        UomMgt: Codeunit "CNC U";
        Cnt: Integer;
        DueDate: Date;
        B1: Boolean;
        B2: Boolean;
        B3: Boolean;
        B4: Boolean;
        ALbl: Label 'a label', Comment = 'a comment';
        APlain: Label 'a plain';

    procedure SetAll(V: Integer)
    begin
        TempA1."No." := V;
        TempA1.Insert();
        TempA2."No." := V;
        TempA2.SetRange("No.", V);
        TempA3.Txt := 't' + Format(V);
        N1."No." := V;
        N1.SetRange("No.", V);
        N2.Txt := 'n' + Format(V);
        Cnt := V;
        DueDate := DMY2Date(1, 1, 2020) + V;
        B1 := true;
        B2 := true;
        B3 := true;
        B4 := true;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('a[%1,%2,%3,%4,%5,%6,', TempA1.Count(), TempA2."No.", TempA2.GetFilters(), TempA3.Txt, N1."No.", N1.GetFilters())
            + StrSubstNo('%1,%2,%3,%4%5%6%7,%8,%9]', N2.Txt, Cnt, Format(DueDate, 0, 9), Format(B1, 0, 9), Format(B2, 0, 9), Format(B3, 0, 9), Format(B4, 0, 9), ALbl, APlain));
    end;

    procedure StateSI(): Text
    begin
        exit(Confirm.State() + UomMgt.State());
    end;
}

codeunit 86896 "CNC B"
{
    var
        R1: Record "CKK Row";
        R2: Record "CKK Row";
        R3: Record "CKK Row";
        R4: Record "CKK Row";
        R5: Record "CKK Row";
        TempB: Record "CKK Row" temporary;
        ItemTracking: Codeunit "CNC A";
        WhseMgt: Codeunit "CNC W";
        UomMgt: Codeunit "CNC U";
        B1: Boolean;
        B2: Boolean;
        Cnt: Integer;
        BLbl: Label 'b label', Comment = 'a comment';
        BPlain: Label 'b plain';

    procedure SetAll(V: Integer)
    begin
        R1."No." := V;
        R2.SetRange("No.", V);
        R3.Txt := 'r' + Format(V);
        R4."No." := V + 1;
        R5.SetRange(Txt, 'x');
        TempB."No." := V;
        TempB.Insert();
        ItemTracking.SetAll(V + 1);
        B1 := true;
        B2 := true;
        Cnt := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('b[%1,%2,%3,%4,%5,%6,', R1."No.", R2.GetFilters(), R3.Txt, R4."No.", R5.GetFilters(), TempB.Count())
            + StrSubstNo('%1%2,%3,%4,%5]', Format(B1, 0, 9), Format(B2, 0, 9), Cnt, BLbl, BPlain) + WhseMgt.State());
    end;

    procedure StateA(): Text
    begin
        exit(ItemTracking.State());
    end;

    procedure StateSI(): Text
    begin
        exit(UomMgt.State() + ItemTracking.StateSI());
    end;
}

codeunit 86897 "CNC Reg"
{
    TableNo = "CKK Row";

    var
        WhseReg: Record "CNC Reg Entry";
        R2: Record "CKK Row";
        R3: Record "CKK Row";
        R4: Record "CKK Row";
        ItemTracking: Codeunit "CNC A";
        WmsMgt: Codeunit "CNC B";
        Cnt: Integer;
        RLbl: Label 'reg label', Comment = 'a comment';
        RPlain: Label 'reg plain';
        Flag: Boolean;

    trigger OnRun()
    begin
        Cnt += 1;
        if WhseReg."No." = 0 then begin
            WhseReg.Reset();
            if WhseReg.FindLast() then
                WhseReg."No." := WhseReg."No." + 1
            else
                WhseReg."No." := 1;
            WhseReg.Runs := 1;
            WhseReg.Insert();
        end else begin
            WhseReg.Runs += 1;
            WhseReg.Modify();
        end;
        R2."No." := Rec."No.";
        Flag := true;
    end;

    procedure SetAll(V: Integer)
    begin
        R2."No." := V;
        R3.SetRange("No.", V);
        R4.Txt := 'q' + Format(V);
        ItemTracking.SetAll(V);
        WmsMgt.SetAll(V + 10);
        Cnt := V;
        Flag := true;
    end;

    procedure SetCnt(V: Integer)
    begin
        Cnt := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('reg[%1,%2,%3,%4,%5,%6,%7,%8,%9]', WhseReg."No.", WhseReg.Runs, R2."No.", R3.GetFilters(), R4.Txt, Cnt, RLbl, RPlain, Format(Flag, 0, 9)));
    end;

    procedure StateA(): Text
    begin
        exit(ItemTracking.State());
    end;

    procedure StateB(): Text
    begin
        exit(WmsMgt.State() + WmsMgt.StateA());
    end;

    procedure StateSI(): Text
    begin
        exit(ItemTracking.StateSI() + '|' + WmsMgt.StateSI());
    end;
}

codeunit 86898 "CNC Owner"
{
    TableNo = "CKK Row";

    var
        WhseJnlPostLine: Codeunit "CNC Reg";
        Mode: Integer;

    trigger OnRun()
    var
        M: Integer;
    begin
        M := Mode;
        RunWithCheck(M);
    end;

    local procedure RunWithCheck(M: Integer)
    begin
        ClearAllVariables();
        case M of
            1:
                WhseJnlPostLine.SetCnt(21);
            2:
                RunReg();
        end;
    end;

    local procedure ClearAllVariables()
    begin
        ClearAll();
    end;

    procedure SetWhseJnlRegisterCU(var R: Codeunit "CNC Reg")
    begin
        WhseJnlPostLine := R;
    end;

    procedure SetMode(M: Integer)
    begin
        Mode := M;
    end;

    procedure SetCnt(V: Integer)
    begin
        WhseJnlPostLine.SetCnt(V);
    end;

    procedure State(): Text
    begin
        exit(WhseJnlPostLine.State());
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 8;
        TempRow.Insert();
        WhseJnlPostLine.Run(TempRow);
    end;
}

codeunit 86899 "CNC Holder"
{
    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        SalesPost: Codeunit "CNC Owner";

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure Post(M: Integer)
    var
        TempHeader: Record "CKK Row" temporary;
    begin
        SalesPost.SetWhseJnlRegisterCU(WhseJnlRegisterLine);
        SalesPost.SetMode(M);
        TempHeader."No." := 1;
        TempHeader.Insert();
        SalesPost.Run(TempHeader);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
    end;

    procedure SetCnt(V: Integer)
    begin
        WhseJnlRegisterLine.SetCnt(V);
    end;

    procedure OwnerSetCnt(V: Integer)
    begin
        SalesPost.SetCnt(V);
    end;

    procedure OwnerRunReg()
    begin
        SalesPost.RunReg();
    end;

    procedure State(): Text
    begin
        exit(WhseJnlRegisterLine.State());
    end;

    procedure OwnerState(): Text
    begin
        exit(SalesPost.State());
    end;

    procedure StateA(): Text
    begin
        exit(WhseJnlRegisterLine.StateA());
    end;

    procedure StateB(): Text
    begin
        exit(WhseJnlRegisterLine.StateB());
    end;

    procedure StateSI(): Text
    begin
        exit(WhseJnlRegisterLine.StateSI());
    end;
}

codeunit 86900 "CNC Test"
{
    Subtype = Test;

    var
        Observer: Codeunit "Probe Observer";

    local procedure Prime(Seed: Integer)
    var
        UomMgt: Codeunit "CNC U";
        Impl: Codeunit "CNC I";
    begin
        UomMgt.Set(Seed);
        Impl.SetFlag(true);
    end;

    local procedure ResetEntries(WithRow: Boolean)
    var
        Entry: Record "CNC Reg Entry";
    begin
        Entry.DeleteAll();
        if WithRow then begin
            Entry."No." := 4;
            Entry.Runs := 0;
            Entry.Insert();
        end;
    end;

    local procedure Entries(): Text
    var
        Entry: Record "CNC Reg Entry";
        Result: Text;
    begin
        if Entry.FindSet() then
            repeat
                Result += StrSubstNo('%1:%2;', Entry."No.", Entry.Runs);
            until Entry.Next() = 0;
        exit('entries[' + Result + ']');
    end;

    local procedure ObserveTree(Prefix: Text; var H: Codeunit "CNC Holder")
    begin
        Observer.Observe(Prefix + '.reg', H.State());
        Observer.Observe(Prefix + '.a', H.StateA());
        Observer.Observe(Prefix + '.b', H.StateB());
        Observer.Observe(Prefix + '.si', H.StateSI());
    end;

    [Test]
    procedure NeverCalled_OwnerClearAll_EveryGlobal()
    var
        H: Codeunit "CNC Holder";
        UomMgt: Codeunit "CNC U";
    begin
        ResetEntries(false);
        Prime(31);
        H.Post(0);
        ObserveTree('after', H);
        Observer.Observe('after.owner', H.OwnerState());
        UomMgt.Set(32);
        Observer.Observe('siwrite.si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('after.reg', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Expect('after.a', 'a[0,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('after.b', 'b[0,,,0,,0,falsefalse,0,b label,b plain]w[w label,w plain]a[0,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('after.si', 'ci[Yes]u[31,No.: 31,u31,u label,u plain]|u[31,No.: 31,u31,u label,u plain]ci[Yes]u[31,No.: 31,u31,u label,u plain]');
        Observer.Expect('after.owner', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Expect('siwrite.si', 'ci[Yes]u[32,No.: 32,u32,u label,u plain]|u[32,No.: 32,u32,u label,u plain]ci[Yes]u[32,No.: 32,u32,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure WrittenBefore_OwnerClearAll_EveryGlobal()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(false);
        Prime(41);
        H.WriteRegister(5);
        ObserveTree('before', H);
        H.Post(0);
        ObserveTree('after', H);
        Observer.Observe('after.owner', H.OwnerState());
        Observer.Expect('before.reg', 'reg[0,0,5,No.: 5,q5,5,reg label,reg plain,true]');
        Observer.Expect('before.a', 'a[1,5,No.: 5,t5,5,No.: 5,n5,5,2020-01-06,truetruetruetrue,a label,a plain]');
        Observer.Expect('before.b', 'b[15,No.: 15,r15,16,Txt: x,1,truetrue,15,b label,b plain]w[w label,w plain]a[1,16,No.: 16,t16,16,No.: 16,n16,16,2020-01-17,truetruetruetrue,a label,a plain]');
        Observer.Expect('before.si', 'ci[Yes]u[41,No.: 41,u41,u label,u plain]|u[41,No.: 41,u41,u label,u plain]ci[Yes]u[41,No.: 41,u41,u label,u plain]');
        Observer.Expect('after.reg', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Expect('after.a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('after.b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('after.si', 'ci[Yes]u[41,No.: 41,u41,u label,u plain]|u[41,No.: 41,u41,u label,u plain]ci[Yes]u[41,No.: 41,u41,u label,u plain]');
        Observer.Expect('after.owner', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_OwnerWritesAfter_HolderReads()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(false);
        Prime(51);
        H.Post(0);
        H.OwnerSetCnt(11);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,11,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,11,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_HolderWritesAfter_OwnerReads()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(false);
        Prime(52);
        H.Post(0);
        H.SetCnt(12);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,12,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,12,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_OwnerWritesInOnRun_HolderReads()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(false);
        Prime(53);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_HolderRunsOnce_EntryAndCounter()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(true);
        Prime(61);
        H.Post(0);
        H.RunReg();
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('entries', Entries());
        Observer.Observe('si', H.StateSI());
        Observer.Expect('holder', 'reg[5,1,7,,,1,reg label,reg plain,true]');
        Observer.Expect('owner', 'reg[5,1,7,,,1,reg label,reg plain,true]');
        Observer.Expect('entries', 'entries[4:0;5:1;]');
        Observer.Expect('si', 'ci[Yes]u[61,No.: 61,u61,u label,u plain]|u[61,No.: 61,u61,u label,u plain]ci[Yes]u[61,No.: 61,u61,u label,u plain]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_HolderRunsTwice_EntryAndCounter()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(true);
        Prime(62);
        H.Post(0);
        H.RunReg();
        H.RunReg();
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('owner', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_HolderRunsThenOwnerRuns_EntryAndCounter()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(true);
        Prime(63);
        H.Post(0);
        H.RunReg();
        H.OwnerRunReg();
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[5,2,8,,,2,reg label,reg plain,true]');
        Observer.Expect('owner', 'reg[5,2,8,,,2,reg label,reg plain,true]');
        Observer.Expect('entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;

    [Test]
    procedure NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(true);
        Prime(64);
        H.Post(2);
        Observer.Observe('first.holder', H.State());
        Observer.Observe('first.entries', Entries());
        H.RunReg();
        Observer.Observe('second.holder', H.State());
        Observer.Observe('second.owner', H.OwnerState());
        Observer.Observe('second.entries', Entries());
        Observer.Expect('first.holder', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('first.entries', 'entries[4:0;5:1;]');
        Observer.Expect('second.holder', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('second.owner', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('second.entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;

    [Test]
    procedure RunBefore_OwnerClearAll_HolderRunsAgain()
    var
        H: Codeunit "CNC Holder";
    begin
        ResetEntries(true);
        Prime(65);
        H.RunReg();
        Observer.Observe('before.holder', H.State());
        Observer.Observe('before.entries', Entries());
        H.Post(0);
        Observer.Observe('after.holder', H.State());
        Observer.Observe('after.owner', H.OwnerState());
        H.RunReg();
        Observer.Observe('again.holder', H.State());
        Observer.Observe('again.owner', H.OwnerState());
        Observer.Observe('again.entries', Entries());
        Observer.Expect('before.holder', 'reg[5,1,7,,,1,reg label,reg plain,true]');
        Observer.Expect('before.entries', 'entries[4:0;5:1;]');
        Observer.Expect('after.holder', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Expect('after.owner', 'reg[0,0,0,,,0,reg label,reg plain,false]');
        Observer.Expect('again.holder', 'reg[6,1,7,,,1,reg label,reg plain,true]');
        Observer.Expect('again.owner', 'reg[6,1,7,,,1,reg label,reg plain,true]');
        Observer.Expect('again.entries', 'entries[4:0;5:1;6:1;]');
        Observer.Verify();
    end;
}
