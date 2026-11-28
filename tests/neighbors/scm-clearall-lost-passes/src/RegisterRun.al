// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86807 to 86810
// Probe: ClearAll in an owner codeunit whose TableNo register another variable shares, after the caller ran the register
// with Reg.Run(Row) and the run returned (ClearSharedCodeunitWhseTree.al measured registers that were never run). Observed
// after ClearAll: register globals, the globals of a plain codeunit the register holds, a copy of Rec that OnRun keeps in a
// register global (Rec exists only inside OnRun: AL0118 elsewhere), and the caller's Row variable. Also: a guarded run whose OnRun errors,
// ClearAll inside the register, the static Codeunit.Run form, and a second Reg.Run(Row) after ClearAll. al-runner issue 9292.
// BC versions: 27, 28, 29
// What BC did (observed on BC 28.5; the pull request CI run asserts the same values on 27.5, 28.5 and 29.0): after a returned
// Reg.Run(Row), ClearAll in the owner resets the register exactly as for a register that never ran: Integer, Text and the run
// counter go to their defaults, the kept copy of Rec goes blank, the plain codeunit the register holds goes back to defaults, and
// temporary records (in the register and in the held codeunit) keep their rows. The caller's Row keeps its field values, filter,
// temporariness and rows, including a field OnRun changed in Rec (passed back through the var record). A guarded Reg.Run(Row)
// whose OnRun errors returns false with the error text, keeps the global OnRun set before the error until ClearAll resets it.
// ClearAll inside the register gives the same result as ClearAll in the owner. Codeunit.Run(Codeunit::"CSR Reg", Row) runs a
// separate instance: read as a return value it returns true; read or not, the shared register's counter stays 0. A second
// Reg.Run(Row2) after ClearAll sees the reset globals (the temporary rows stay), and the caller's first Row is unchanged.
// Not measured: Reg.Run() without a record (does not compile for a TableNo codeunit: AL0135), Rec read from a procedure other
// than OnRun (Rec exists only inside OnRun: AL0118), a temporary record below the register that read a row into its buffer (Get or
// Find) or modified one, and a database Modify or Delete below the register. A deep register tree after a returned Reg.Run(Row)
// is in ClearSharedCodeunitDeepFields.al.
codeunit 86807 "CSR Node"
{
    var
        N: Integer;
        T: Text;
        Tmp: Record "CKK Row" temporary;

    procedure SetAll(V: Integer)
    begin
        N := V;
        T := 'n' + Format(V);
        Tmp."No." := V;
        Tmp.Insert();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('node[%1,%2,%3]', N, T, Tmp.Count()));
    end;
}

codeunit 86808 "CSR Reg"
{
    TableNo = "CKK Row";

    var
        Node: Codeunit "CSR Node";
        Tmp: Record "CKK Row" temporary;
        N: Integer;
        T: Text;
        Mode: Integer;
        Seen: Text;
        Runs: Integer;
        Kept: Record "CKK Row";

    trigger OnRun()
    var
        Cnt: Text;
    begin
        Runs += 1;
        Cnt := '-';
        if Rec.IsTemporary() then
            Cnt := Format(Rec.Count());
        Seen := StrSubstNo('start[%1,%2,%3,%4]', N, T, Tmp.Count(), Mode) + Node.State() + KeptState()
            + StrSubstNo('rec[%1,%2,%3,%4,%5]', Rec."No.", Rec.Txt, Rec.GetFilters(), Rec.IsTemporary(), Cnt);
        case Mode of
            1:
                begin
                    N := Rec."No.";
                    T := 'g' + Rec.Txt;
                    Tmp."No." := Rec."No.";
                    Tmp.Insert();
                    Node.SetAll(Rec."No." + 1);
                end;
            2:
                begin
                    N := 99;
                    Error('run failed');
                end;
            3:
                Rec.Txt := 'changed';
        end;
        Kept := Rec;
        Kept.CopyFilters(Rec);
    end;

    procedure SetMode(M: Integer)
    begin
        Mode := M;
    end;

    procedure KeptState(): Text
    begin
        exit(StrSubstNo('kept[%1,%2,%3]', Kept."No.", Kept.Txt, Kept.GetFilters()));
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('reg[%1,%2,%3,%4,%5]', N, T, Tmp.Count(), Mode, Runs) + Node.State());
    end;

    procedure SeenAtRun(): Text
    begin
        exit(Seen);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86809 "CSR Owner"
{
    var
        Reg: Codeunit "CSR Reg";

    procedure SetReg(var R: Codeunit "CSR Reg")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Text
    begin
        exit(Reg.State() + Reg.KeptState());
    end;
}

codeunit 86810 "CSR Test"
{
    Subtype = Test;

    var
        Observer: Codeunit "Probe Observer";

    local procedure RowState(var Row: Record "CKK Row"): Text
    var
        Cnt: Text;
    begin
        Cnt := '-';
        if Row.IsTemporary() then
            Cnt := Format(Row.Count());
        exit(StrSubstNo('row[%1,%2,%3,%4,%5]', Row."No.", Row.Txt, Row.GetFilters(), Row.IsTemporary(), Cnt));
    end;

    local procedure MakeRow(var Row: Record "CKK Row")
    begin
        Row."No." := 5;
        Row.Txt := 'row';
        Row.SetRange("No.", 5);
    end;

    local procedure ObserveAfter(var R: Codeunit "CSR Reg"; var Owner: Codeunit "CSR Owner"; var Row: Record "CKK Row")
    begin
        Observer.Observe('after.reg', R.State());
        Observer.Observe('after.kept', R.KeptState());
        Observer.Observe('after.owner', Owner.State());
        Observer.Observe('after.row', RowState(Row));
    end;

    local procedure RunAndClear(Mode: Integer; var R: Codeunit "CSR Reg"; var Owner: Codeunit "CSR Owner"; var Row: Record "CKK Row")
    begin
        Owner.SetReg(R);
        R.SetMode(Mode);
        R.Run(Row);
        Observer.Observe('run.seen', R.SeenAtRun());
        Observer.Observe('before.reg', R.State());
        Observer.Observe('before.kept', R.KeptState());
        Observer.Observe('before.row', RowState(Row));
        Owner.DoClearAll();
        ObserveAfter(R, Owner, Row);
    end;

    [Test]
    procedure RegRun_ReadOnly_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        RunAndClear(0, R, Owner, Row);
        Observer.Expect('run.seen', 'start[0,,0,0]node[0,,0]kept[0,,]rec[5,row,No.: 5,No,-]');
        Observer.Expect('before.reg', 'reg[0,,0,0,1]node[0,,0]');
        Observer.Expect('before.kept', 'kept[5,row,No.: 5]');
        Observer.Expect('before.row', 'row[5,row,No.: 5,No,-]');
        Observer.Expect('after.reg', 'reg[0,,0,0,0]node[0,,0]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,0,0,0]node[0,,0]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_SetsGlobals_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        RunAndClear(1, R, Owner, Row);
        Observer.Expect('run.seen', 'start[0,,0,1]node[0,,0]kept[0,,]rec[5,row,No.: 5,No,-]');
        Observer.Expect('before.reg', 'reg[5,grow,1,1,1]node[6,n6,1]');
        Observer.Expect('before.kept', 'kept[5,row,No.: 5]');
        Observer.Expect('before.row', 'row[5,row,No.: 5,No,-]');
        Observer.Expect('after.reg', 'reg[0,,1,0,0]node[0,,1]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,1,0,0]node[0,,1]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_ChangesRec_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        RunAndClear(3, R, Owner, Row);
        Observer.Expect('run.seen', 'start[0,,0,3]node[0,,0]kept[0,,]rec[5,row,No.: 5,No,-]');
        Observer.Expect('before.reg', 'reg[0,,0,3,1]node[0,,0]');
        Observer.Expect('before.kept', 'kept[5,changed,No.: 5]');
        Observer.Expect('before.row', 'row[5,changed,No.: 5,No,-]');
        Observer.Expect('after.reg', 'reg[0,,0,0,0]node[0,,0]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,0,0,0]node[0,,0]kept[0,,]');
        Observer.Expect('after.row', 'row[5,changed,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_TempRow_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 4;
        Row.Insert();
        Row."No." := 5;
        Row.Txt := 'row';
        Row.Insert();
        Row.SetRange("No.", 4, 5);
        RunAndClear(1, R, Owner, Row);
        Observer.Expect('run.seen', 'start[0,,0,1]node[0,,0]kept[0,,]rec[5,row,No.: 4..5,Yes,2]');
        Observer.Expect('before.reg', 'reg[5,grow,1,1,1]node[6,n6,1]');
        Observer.Expect('before.kept', 'kept[5,row,No.: 4..5]');
        Observer.Expect('before.row', 'row[5,row,No.: 4..5,Yes,2]');
        Observer.Expect('after.reg', 'reg[0,,1,0,0]node[0,,1]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,1,0,0]node[0,,1]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 4..5,Yes,2]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_GuardedError_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        Owner.SetReg(R);
        R.SetMode(2);
        Observer.Observe('run.result', Format(R.Run(Row)));
        Observer.Observe('run.error', GetLastErrorText());
        Observer.Observe('before.reg', R.State());
        Observer.Observe('before.kept', R.KeptState());
        Owner.DoClearAll();
        ObserveAfter(R, Owner, Row);
        Observer.Expect('run.result', 'No');
        Observer.Expect('run.error', 'run failed');
        Observer.Expect('before.reg', 'reg[99,,0,2,1]node[0,,0]');
        Observer.Expect('before.kept', 'kept[0,,]');
        Observer.Expect('after.reg', 'reg[0,,0,0,0]node[0,,0]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,0,0,0]node[0,,0]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_RegisterClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        Owner.SetReg(R);
        R.SetMode(1);
        R.Run(Row);
        R.DoClearAll();
        ObserveAfter(R, Owner, Row);
        Observer.Expect('after.reg', 'reg[0,,1,0,0]node[0,,1]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,1,0,0]node[0,,1]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_StaticRun_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        Owner.SetReg(R);
        R.SetMode(1);
        Observer.Observe('static.ok', Format(Codeunit.Run(Codeunit::"CSR Reg", Row)));
        Observer.Observe('before.reg', R.State());
        Observer.Observe('before.kept', R.KeptState());
        Owner.DoClearAll();
        ObserveAfter(R, Owner, Row);
        Observer.Expect('static.ok', 'Yes');
        Observer.Expect('before.reg', 'reg[0,,0,1,0]node[0,,0]');
        Observer.Expect('before.kept', 'kept[0,,]');
        Observer.Expect('after.reg', 'reg[0,,0,0,0]node[0,,0]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,0,0,0]node[0,,0]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_StaticRunUnguarded_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
    begin
        MakeRow(Row);
        Owner.SetReg(R);
        R.SetMode(1);
        Codeunit.Run(Codeunit::"CSR Reg", Row);
        Observer.Observe('before.reg', R.State());
        Observer.Observe('before.kept', R.KeptState());
        Owner.DoClearAll();
        ObserveAfter(R, Owner, Row);
        Observer.Expect('before.reg', 'reg[0,,0,1,0]node[0,,0]');
        Observer.Expect('before.kept', 'kept[0,,]');
        Observer.Expect('after.reg', 'reg[0,,0,0,0]node[0,,0]');
        Observer.Expect('after.kept', 'kept[0,,]');
        Observer.Expect('after.owner', 'reg[0,,0,0,0]node[0,,0]kept[0,,]');
        Observer.Expect('after.row', 'row[5,row,No.: 5,No,-]');
        Observer.Verify();
    end;

    [Test]
    procedure RegRun_RunAgainAfter_OwnerClearAll()
    var
        R: Codeunit "CSR Reg";
        Owner: Codeunit "CSR Owner";
        Row: Record "CKK Row";
        Row2: Record "CKK Row";
    begin
        MakeRow(Row);
        Owner.SetReg(R);
        R.SetMode(1);
        R.Run(Row);
        Owner.DoClearAll();
        Row2."No." := 8;
        Row2.Txt := 'two';
        R.Run(Row2);
        Observer.Observe('again.seen', R.SeenAtRun());
        Observer.Observe('again.reg', R.State());
        Observer.Observe('again.kept', R.KeptState());
        Observer.Observe('again.owner', Owner.State());
        Observer.Observe('again.row', RowState(Row));
        Observer.Observe('again.row2', RowState(Row2));
        Observer.Expect('again.seen', 'start[0,,1,0]node[0,,1]kept[0,,]rec[8,two,,No,-]');
        Observer.Expect('again.reg', 'reg[0,,1,0,1]node[0,,1]');
        Observer.Expect('again.kept', 'kept[8,two,]');
        Observer.Expect('again.owner', 'reg[0,,1,0,1]node[0,,1]kept[8,two,]');
        Observer.Expect('again.row', 'row[5,row,No.: 5,No,-]');
        Observer.Expect('again.row2', 'row[8,two,,No,-]');
        Observer.Verify();
    end;
}
