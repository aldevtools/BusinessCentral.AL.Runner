// Copy of open corpus PR 1650 at 03b9f669, tests/al-language/codeunits/clear-instance/ClearSharedCodeunitNeverCalledShapes.al,
// its objects 86901 to 86907 renumbered 86965 to 86971. Line numbers below are the corpus file's plus 2.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; "CNC Reg Entry",
// "CNC U", "CNC I", "CNC Reg" and "CNC Owner" (86890 to 86898) of ClearSharedCodeunitNeverCalledTree.al; own objects 86965 to 86971
// Probe: the call shapes around the ClearAll of ClearSharedCodeunitNeverCalledTree.al. There a plain holder ("CNC Holder") holds
// the register and the owner in globals and runs the owner from a public procedure; the owner's OnRun reaches ClearAll through
// a local RunWithCheck. "Whse.-Activity-Post" running "Sales-Post" (BC 29) differs in four facts, and each cell here changes one
// of them against that shape:
// - TableNo: the holder has a TableNo (called through its public procedure, and entered through its own OnRun by Run).
// - LocalProc: the holder runs the owner from a local procedure (PostSourceDocument).
// - LocalOwner: the holder holds the owner in a local variable of the procedure that runs it.
// - Internal: the owner's RunWithCheck is `internal`, not local.
// - Activity: all four together, entered by Run on the holder; OnRun -> local Code -> local PostSourceDoc -> local
//   PostSourceDocument (owner in a local) -> owner Run -> OnRun -> internal RunWithCheck -> local ClearAllVariables -> ClearAll.
// Each cell observes what the holder (the later reader of the shared register) sees after the owner's ClearAll:
// a register never called before, with a write by the owner right after its ClearAll (Cnt := 21); a register written before
// (every global set through the holder), with the same write; and a register run by the owner right after its ClearAll, then by
// the holder. "ClearAll clears the shared instance" and "ClearAll only drops the owner's reference" predict different values
// in every one of these cells. al-runner issue 8964.
// BC versions: 27, 28, 29
// What BC did (the same in all six shapes: TableNo, TableNoRun, LocalProc, LocalOwner, Internal, Activity):
// - The ClearAll clears the one shared register instance; the owner's variable stays bound to it. Never called before: the
//   owner's write right after its ClearAll (Cnt 21) is read through the holder's variable, every other global reads its default.
// - Written before through the holder (every global set): after the owner's ClearAll the holder reads every plain global reset
//   (field values, filters, Integer, Date, Booleans) except the owner's write (Cnt 21); temporary records keep their rows
//   (Count 1); Labels keep their text; the SingleInstance U and C/I keep the values primed through other variables.
// - The owner runs the register right after its ClearAll (inserts entry row 5, counter 1); the holder's later run (by the test
//   after the holder returns, or by the holder itself inside its own OnRun after PostSourceDocument) uses the same instance:
//   counter 2, row 5 modified (Runs 2), no second row. With the owner in a local, its view read right after Run is the same.
codeunit 86965 "CNC Owner Internal"
{
    TableNo = "CKK Row";

    var
        WhseJnlPostLine: Codeunit "CNC Reg";
        Mode: Integer;

    trigger OnRun()
    begin
        RunWithCheck(Rec);
    end;

    internal procedure RunWithCheck(var Header: Record "CKK Row")
    var
        M: Integer;
    begin
        M := Mode;
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

codeunit 86966 "CNC Holder TableNo"
{
    TableNo = "CKK Row";

    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        SalesPost: Codeunit "CNC Owner";
        Mode: Integer;

    trigger OnRun()
    begin
        Post(Mode);
    end;

    procedure SetMode(M: Integer)
    begin
        Mode := M;
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

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
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

codeunit 86967 "CNC Holder LocalProc"
{
    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        SalesPost: Codeunit "CNC Owner";

    procedure Post(M: Integer)
    begin
        PostSourceDocument(M);
    end;

    local procedure PostSourceDocument(M: Integer)
    var
        TempHeader: Record "CKK Row" temporary;
    begin
        SalesPost.SetWhseJnlRegisterCU(WhseJnlRegisterLine);
        SalesPost.SetMode(M);
        TempHeader."No." := 1;
        TempHeader.Insert();
        SalesPost.Run(TempHeader);
    end;

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
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

codeunit 86968 "CNC Holder LocalOwner"
{
    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        OwnerView: Text;

    procedure Post(M: Integer)
    var
        TempHeader: Record "CKK Row" temporary;
        SalesPost: Codeunit "CNC Owner";
    begin
        SalesPost.SetWhseJnlRegisterCU(WhseJnlRegisterLine);
        SalesPost.SetMode(M);
        TempHeader."No." := 1;
        TempHeader.Insert();
        SalesPost.Run(TempHeader);
        OwnerView := SalesPost.State();
    end;

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
    end;

    procedure State(): Text
    begin
        exit(WhseJnlRegisterLine.State());
    end;

    procedure OwnerState(): Text
    begin
        exit(OwnerView);
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

codeunit 86969 "CNC Holder Internal"
{
    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        SalesPost: Codeunit "CNC Owner Internal";

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

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
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

codeunit 86970 "CNC Holder Activity"
{
    TableNo = "CKK Row";

    var
        WhseJnlRegisterLine: Codeunit "CNC Reg";
        Mode: Integer;
        RegisterAfter: Boolean;
        OwnerView: Text;
        InsideView: Text;

    trigger OnRun()
    begin
        Code();
    end;

    local procedure Code()
    begin
        PostSourceDoc();
        if RegisterAfter then begin
            RunReg();
            InsideView := WhseJnlRegisterLine.State();
        end;
    end;

    local procedure PostSourceDoc()
    begin
        PostSourceDocument();
    end;

    local procedure PostSourceDocument()
    var
        SalesHeader: Record "CKK Row" temporary;
        SalesPost: Codeunit "CNC Owner Internal";
    begin
        SalesHeader."No." := 1;
        SalesHeader.Insert();
        SalesPost.SetWhseJnlRegisterCU(WhseJnlRegisterLine);
        SalesPost.SetMode(Mode);
        SalesPost.Run(SalesHeader);
        OwnerView := SalesPost.State();
    end;

    procedure SetMode(M: Integer; RunAfter: Boolean)
    begin
        Mode := M;
        RegisterAfter := RunAfter;
    end;

    procedure WriteRegister(V: Integer)
    begin
        WhseJnlRegisterLine.SetAll(V);
    end;

    procedure RunReg()
    var
        TempRow: Record "CKK Row" temporary;
    begin
        TempRow."No." := 7;
        TempRow.Insert();
        WhseJnlRegisterLine.Run(TempRow);
    end;

    procedure State(): Text
    begin
        exit(WhseJnlRegisterLine.State());
    end;

    procedure OwnerState(): Text
    begin
        exit(OwnerView);
    end;

    procedure InsideState(): Text
    begin
        exit(InsideView);
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

codeunit 86971 "CNC Shape Test"
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

    local procedure Header(var Row: Record "CKK Row" temporary)
    begin
        Row."No." := 1;
        Row.Insert();
    end;

    // TableNo holder, called through its public procedure

    [Test]
    procedure TableNo_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder TableNo";
    begin
        ResetEntries(false);
        Prime(71);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[71,No.: 71,u71,u label,u plain]|u[71,No.: 71,u71,u label,u plain]ci[Yes]u[71,No.: 71,u71,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure TableNo_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder TableNo";
    begin
        ResetEntries(false);
        Prime(72);
        H.WriteRegister(5);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[72,No.: 72,u72,u label,u plain]|u[72,No.: 72,u72,u label,u plain]ci[Yes]u[72,No.: 72,u72,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure TableNo_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder TableNo";
    begin
        ResetEntries(true);
        Prime(73);
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

    // TableNo holder, entered through its own OnRun by Run

    [Test]
    procedure TableNoRun_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder TableNo";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(false);
        Prime(74);
        H.SetMode(1);
        Header(Row);
        H.Run(Row);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[74,No.: 74,u74,u label,u plain]|u[74,No.: 74,u74,u label,u plain]ci[Yes]u[74,No.: 74,u74,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoRun_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder TableNo";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(false);
        Prime(75);
        H.WriteRegister(5);
        H.SetMode(1);
        Header(Row);
        H.Run(Row);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[75,No.: 75,u75,u label,u plain]|u[75,No.: 75,u75,u label,u plain]ci[Yes]u[75,No.: 75,u75,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoRun_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder TableNo";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(true);
        Prime(76);
        H.SetMode(2);
        Header(Row);
        H.Run(Row);
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

    // Holder runs the owner from a local procedure

    [Test]
    procedure LocalProc_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder LocalProc";
    begin
        ResetEntries(false);
        Prime(77);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[77,No.: 77,u77,u label,u plain]|u[77,No.: 77,u77,u label,u plain]ci[Yes]u[77,No.: 77,u77,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure LocalProc_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder LocalProc";
    begin
        ResetEntries(false);
        Prime(78);
        H.WriteRegister(5);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[78,No.: 78,u78,u label,u plain]|u[78,No.: 78,u78,u label,u plain]ci[Yes]u[78,No.: 78,u78,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure LocalProc_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder LocalProc";
    begin
        ResetEntries(true);
        Prime(79);
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

    // Holder holds the owner in a local variable (owner view read right after Run, inside that procedure)

    [Test]
    procedure LocalOwner_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder LocalOwner";
    begin
        ResetEntries(false);
        Prime(81);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[81,No.: 81,u81,u label,u plain]|u[81,No.: 81,u81,u label,u plain]ci[Yes]u[81,No.: 81,u81,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure LocalOwner_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder LocalOwner";
    begin
        ResetEntries(false);
        Prime(82);
        H.WriteRegister(5);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[82,No.: 82,u82,u label,u plain]|u[82,No.: 82,u82,u label,u plain]ci[Yes]u[82,No.: 82,u82,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure LocalOwner_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder LocalOwner";
    begin
        ResetEntries(true);
        Prime(83);
        H.Post(2);
        Observer.Observe('first.holder', H.State());
        Observer.Observe('first.owner', H.OwnerState());
        Observer.Observe('first.entries', Entries());
        H.RunReg();
        Observer.Observe('second.holder', H.State());
        Observer.Observe('second.entries', Entries());
        Observer.Expect('first.holder', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('first.owner', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('first.entries', 'entries[4:0;5:1;]');
        Observer.Expect('second.holder', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('second.entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;

    // Owner's RunWithCheck is internal

    [Test]
    procedure Internal_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder Internal";
    begin
        ResetEntries(false);
        Prime(84);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[84,No.: 84,u84,u label,u plain]|u[84,No.: 84,u84,u label,u plain]ci[Yes]u[84,No.: 84,u84,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure Internal_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder Internal";
    begin
        ResetEntries(false);
        Prime(85);
        H.WriteRegister(5);
        H.Post(1);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[85,No.: 85,u85,u label,u plain]|u[85,No.: 85,u85,u label,u plain]ci[Yes]u[85,No.: 85,u85,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure Internal_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder Internal";
    begin
        ResetEntries(true);
        Prime(86);
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

    // All four facts: the "Whse.-Activity-Post" -> "Sales-Post" shape

    [Test]
    procedure Activity_NeverCalled_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder Activity";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(false);
        Prime(87);
        H.SetMode(1, false);
        Header(Row);
        H.Run(Row);
        Observer.Observe('holder', H.State());
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('si', 'ci[Yes]u[87,No.: 87,u87,u label,u plain]|u[87,No.: 87,u87,u label,u plain]ci[Yes]u[87,No.: 87,u87,u label,u plain]');
        Observer.Expect('entries', 'entries[]');
        Observer.Verify();
    end;

    [Test]
    procedure Activity_WrittenBefore_OwnerWritesInOnRun()
    var
        H: Codeunit "CNC Holder Activity";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(false);
        Prime(88);
        H.WriteRegister(5);
        H.SetMode(1, false);
        Header(Row);
        H.Run(Row);
        Observer.Observe('holder', H.State());
        Observer.Observe('a', H.StateA());
        Observer.Observe('b', H.StateB());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('owner', H.OwnerState());
        Observer.Expect('holder', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Expect('a', 'a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('b', 'b[0,,,0,,1,falsefalse,0,b label,b plain]w[w label,w plain]a[1,0,,,0,,,0,,falsefalsefalsefalse,a label,a plain]');
        Observer.Expect('si', 'ci[Yes]u[88,No.: 88,u88,u label,u plain]|u[88,No.: 88,u88,u label,u plain]ci[Yes]u[88,No.: 88,u88,u label,u plain]');
        Observer.Expect('owner', 'reg[0,0,0,,,21,reg label,reg plain,false]');
        Observer.Verify();
    end;

    [Test]
    procedure Activity_NeverCalled_OwnerRunsInOnRun_ThenHolderRuns()
    var
        H: Codeunit "CNC Holder Activity";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(true);
        Prime(89);
        H.SetMode(2, false);
        Header(Row);
        H.Run(Row);
        Observer.Observe('first.holder', H.State());
        Observer.Observe('first.owner', H.OwnerState());
        Observer.Observe('first.entries', Entries());
        H.RunReg();
        Observer.Observe('second.holder', H.State());
        Observer.Observe('second.entries', Entries());
        Observer.Expect('first.holder', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('first.owner', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('first.entries', 'entries[4:0;5:1;]');
        Observer.Expect('second.holder', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('second.entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;

    [Test]
    procedure Activity_NeverCalled_OwnerRunsInOnRun_HolderRunsInOnRun()
    var
        H: Codeunit "CNC Holder Activity";
        Row: Record "CKK Row" temporary;
    begin
        ResetEntries(true);
        Prime(90);
        H.SetMode(2, true);
        Header(Row);
        H.Run(Row);
        Observer.Observe('owner', H.OwnerState());
        Observer.Observe('inside', H.InsideState());
        Observer.Observe('holder', H.State());
        Observer.Observe('si', H.StateSI());
        Observer.Observe('entries', Entries());
        Observer.Expect('owner', 'reg[5,1,8,,,1,reg label,reg plain,true]');
        Observer.Expect('inside', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('holder', 'reg[5,2,7,,,2,reg label,reg plain,true]');
        Observer.Expect('si', 'ci[Yes]u[90,No.: 90,u90,u label,u plain]|u[90,No.: 90,u90,u label,u plain]ci[Yes]u[90,No.: 90,u90,u label,u plain]');
        Observer.Expect('entries', 'entries[4:0;5:2;]');
        Observer.Verify();
    end;
}
