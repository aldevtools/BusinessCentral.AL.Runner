// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86800 to 86806
// Probe: ClearAll in an owner codeunit whose TableNo register another variable shares, when the records below the register
// hold assigned field values (not only filters, as ClearSharedCodeunitWhseStore.al measured). Same tree as "CSW DReg" there:
// register -> plain depth 1 -> SingleInstance depth 2 -> plain depth 3, and plain depth 1 -> plain depth 2 -> plain depth 3.
// Every node holds a non-temporary record; depth 3 also holds a temporary record. One cell runs the register with R.Run(Row)
// before ClearAll. al-runner issue 9292.
// singleinstance-shared-on-purpose: the subject is a SingleInstance codeunit held below a register that ClearAll resets.
// BC versions: 27, 28, 29
// What BC did (observed on BC 28.5; the pull request CI run asserts the same values on 27.5, 28.5 and 29.0): ClearAll in the
// owner resets every plain codeunit reached through plain codeunits, at depth 1, 2 and 3, and the register's own record:
// assigned key and non-key field values go blank (No. 0, Txt empty) and filters go away, also after Init() plus assignment and
// for a record that Get loaded a row into and then changed in memory. A temporary record at depth 3 keeps its rows (Count 1)
// but its current field values go blank. The SingleInstance codeunit at depth 2 and the plain codeunit at depth 3 below it keep
// field values and filters. After ClearAll, a field written through the caller's register variable is read back through the
// owner: the register stays shared. After a returned R.Run(Row) on the same deep tree (every level assigned, a filter at depth 3,
// OnRun sets the register Integer from Rec), ClearAll gives the same result as for a register that never ran: the register
// Integer that OnRun set goes to 0, the plain branch goes blank, the SingleInstance branch keeps its field values and filter, and
// the caller's Row keeps its field values and filter.
// Not measured: a temporary record below the register that read a row into its buffer (Get or Find) or modified one, and a
// database Modify or Delete below the register.
codeunit 86800 "CSF D3"
{
    var
        Rows: Record "CKK Row";
        Tmp: Record "CKK Row" temporary;
        N: Integer;

    procedure Prime(V: Integer; Mode: Integer)
    begin
        Clear(Rows);
        Tmp.Reset();
        Tmp.DeleteAll();
        Clear(Tmp);
        N := V;
        case Mode of
            1:
                begin
                    Rows."No." := V;
                    Rows.Txt := 'd' + Format(V);
                end;
            2:
                begin
                    Rows."No." := V;
                    Rows.Txt := 'd' + Format(V);
                    Rows.SetRange("No.", V);
                end;
            3:
                begin
                    Rows.Init();
                    Rows."No." := V;
                    Rows.Txt := 'd' + Format(V);
                end;
            4:
                begin
                    Rows.Get(1);
                    Rows.Txt := 'g' + Format(V);
                end;
            5:
                begin
                    Tmp."No." := V;
                    Tmp.Txt := 't' + Format(V);
                    Tmp.Insert();
                    Tmp."No." := V + 1;
                    Tmp.Txt := 'u' + Format(V);
                end;
        end;
    end;

    procedure SetTxt(T: Text[30])
    begin
        Rows.Txt := T;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d3[%1,%2,%3,%4|tmp %5,%6,%7]', N, Rows."No.", Rows.Txt, Rows.GetFilters(), Tmp.Count(), Tmp."No.", Tmp.Txt));
    end;
}

codeunit 86801 "CSF D2Si"
{
    SingleInstance = true;

    var
        Rows: Record "CKK Row";
        D3: Codeunit "CSF D3";
        N: Integer;

    procedure Prime(V: Integer; Mode: Integer; AllLevels: Boolean)
    begin
        Clear(Rows);
        N := V;
        if AllLevels then begin
            Rows."No." := V;
            Rows.Txt := 's' + Format(V);
        end;
        D3.Prime(V + 1, Mode);
    end;

    procedure SetTxt(T: Text[30])
    begin
        D3.SetTxt(T);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d2si[%1,%2,%3]', N, Rows."No.", Rows.Txt) + D3.State());
    end;
}

codeunit 86802 "CSF D2P"
{
    var
        Rows: Record "CKK Row";
        D3: Codeunit "CSF D3";
        N: Integer;

    procedure Prime(V: Integer; Mode: Integer; AllLevels: Boolean)
    begin
        Clear(Rows);
        N := V;
        if AllLevels then begin
            Rows."No." := V;
            Rows.Txt := 'p' + Format(V);
        end;
        D3.Prime(V + 1, Mode);
    end;

    procedure SetTxt(T: Text[30])
    begin
        D3.SetTxt(T);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d2p[%1,%2,%3]', N, Rows."No.", Rows.Txt) + D3.State());
    end;
}

codeunit 86803 "CSF D1"
{
    var
        Rows: Record "CKK Row";
        Si: Codeunit "CSF D2Si";
        P: Codeunit "CSF D2P";
        N: Integer;

    procedure Prime(V: Integer; Mode: Integer; AllLevels: Boolean)
    begin
        Clear(Rows);
        N := V;
        if AllLevels then begin
            Rows."No." := V;
            Rows.Txt := 'o' + Format(V);
        end;
        Si.Prime(V + 10, Mode, AllLevels);
        P.Prime(V + 20, Mode, AllLevels);
    end;

    procedure SetTxt(T: Text[30])
    begin
        Si.SetTxt(T + 's');
        P.SetTxt(T + 'p');
    end;

    procedure StateSi(): Text
    begin
        exit(Si.State());
    end;

    procedure StateP(): Text
    begin
        exit(P.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d1[%1,%2,%3]', N, Rows."No.", Rows.Txt));
    end;
}

codeunit 86804 "CSF DReg"
{
    TableNo = "CKK Row";

    var
        Loc: Record "CKK Row";
        D1: Codeunit "CSF D1";
        N: Integer;

    trigger OnRun()
    begin
        N := Rec."No.";
    end;

    procedure Prime(V: Integer; Mode: Integer; AllLevels: Boolean)
    begin
        Clear(Loc);
        N := V;
        if AllLevels then begin
            Loc."No." := V;
            Loc.Txt := 'r' + Format(V);
        end;
        D1.Prime(V + 100, Mode, AllLevels);
    end;

    procedure SetTxt(T: Text[30])
    begin
        D1.SetTxt(T);
    end;

    procedure StateReg(): Text
    begin
        exit(StrSubstNo('dreg[%1,%2,%3]', N, Loc."No.", Loc.Txt));
    end;

    procedure StateD1(): Text
    begin
        exit(D1.State());
    end;

    procedure StateSi(): Text
    begin
        exit(D1.StateSi());
    end;

    procedure StateP(): Text
    begin
        exit(D1.StateP());
    end;
}

codeunit 86805 "CSF DOwner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CSF DReg";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "CSF DReg")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure StateP(): Text
    begin
        exit(Reg.StateP());
    end;

    procedure StateSi(): Text
    begin
        exit(Reg.StateSi());
    end;
}

codeunit 86806 "CSF Test"
{
    Subtype = Test;

    var
        Observer: Codeunit "Probe Observer";

    local procedure ObserveTree(Prefix: Text; var R: Codeunit "CSF DReg")
    begin
        Observer.Observe(Prefix + 'reg', R.StateReg());
        Observer.Observe(Prefix + 'd1', R.StateD1());
        Observer.Observe(Prefix + 'si', R.StateSi());
        Observer.Observe(Prefix + 'p', R.StateP());
    end;

    local procedure RowState(var Row: Record "CKK Row"): Text
    begin
        exit(StrSubstNo('row[%1,%2,%3]', Row."No.", Row.Txt, Row.GetFilters()));
    end;

    local procedure Run(Mode: Integer; AllLevels: Boolean)
    var
        R: Codeunit "CSF DReg";
        Owner: Codeunit "CSF DOwner";
        D3: Codeunit "CSF D3";
    begin
        R.Prime(5000, Mode, AllLevels);
        Owner.SetReg(R);
        ObserveTree('before.', R);
        Owner.DoClearAll();
        ObserveTree('after.', R);
        Observer.Observe('after.owner.p', Owner.StateP());
        Observer.Observe('after.d3', D3.State());
    end;

    [Test]
    procedure DeepFields_OnlyDepth3_OwnerClearAll()
    begin
        Run(1, false);
        Observer.Expect('before.reg', 'dreg[5000,0,]');
        Observer.Expect('before.d1', 'd1[5100,0,]');
        Observer.Expect('before.si', 'd2si[5110,0,]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,0,]d3[5121,5121,d5121,|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,0,]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_OnlyDepth3Filter_OwnerClearAll()
    begin
        Run(2, false);
        Observer.Expect('before.reg', 'dreg[5000,0,]');
        Observer.Expect('before.d1', 'd1[5100,0,]');
        Observer.Expect('before.si', 'd2si[5110,0,]d3[5111,5111,d5111,No.: 5111|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,0,]d3[5121,5121,d5121,No.: 5121|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,0,]d3[5111,5111,d5111,No.: 5111|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_OnlyDepth3Init_OwnerClearAll()
    begin
        Run(3, false);
        Observer.Expect('before.reg', 'dreg[5000,0,]');
        Observer.Expect('before.d1', 'd1[5100,0,]');
        Observer.Expect('before.si', 'd2si[5110,0,]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,0,]d3[5121,5121,d5121,|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,0,]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_OnlyDepth3Get_OwnerClearAll()
    var
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 1;
        Row.Txt := 'db';
        Row.Insert();
        Run(4, false);
        Observer.Expect('before.reg', 'dreg[5000,0,]');
        Observer.Expect('before.d1', 'd1[5100,0,]');
        Observer.Expect('before.si', 'd2si[5110,0,]d3[5111,1,g5111,|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,0,]d3[5121,1,g5121,|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,0,]d3[5111,1,g5111,|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_OnlyDepth3Temp_OwnerClearAll()
    begin
        Run(5, false);
        Observer.Expect('before.reg', 'dreg[5000,0,]');
        Observer.Expect('before.d1', 'd1[5100,0,]');
        Observer.Expect('before.si', 'd2si[5110,0,]d3[5111,0,,|tmp 1,5112,u5111]');
        Observer.Expect('before.p', 'd2p[5120,0,]d3[5121,0,,|tmp 1,5122,u5121]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,0,]d3[5111,0,,|tmp 1,5112,u5111]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 1,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 1,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_EveryLevel_OwnerClearAll()
    begin
        Run(1, true);
        Observer.Expect('before.reg', 'dreg[5000,5000,r5000]');
        Observer.Expect('before.d1', 'd1[5100,5100,o5100]');
        Observer.Expect('before.si', 'd2si[5110,5110,s5110]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,5120,p5120]d3[5121,5121,d5121,|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,5110,s5110]d3[5111,5111,d5111,|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_EveryLevelFilter_OwnerClearAll()
    begin
        Run(2, true);
        Observer.Expect('before.reg', 'dreg[5000,5000,r5000]');
        Observer.Expect('before.d1', 'd1[5100,5100,o5100]');
        Observer.Expect('before.si', 'd2si[5110,5110,s5110]d3[5111,5111,d5111,No.: 5111|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[5120,5120,p5120]d3[5121,5121,d5121,No.: 5121|tmp 0,0,]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[5110,5110,s5110]d3[5111,5111,d5111,No.: 5111|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.d3', 'd3[0,0,,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_WriteAfter_OwnerClearAll()
    var
        R: Codeunit "CSF DReg";
        Owner: Codeunit "CSF DOwner";
    begin
        R.Prime(6000, 1, true);
        Owner.SetReg(R);
        Owner.DoClearAll();
        R.SetTxt('w');
        Observer.Observe('write.owner.p', Owner.StateP());
        Observer.Observe('write.owner.si', Owner.StateSi());
        Observer.Observe('write.r.p', R.StateP());
        Observer.Expect('write.owner.p', 'd2p[0,0,]d3[0,0,wp,|tmp 0,0,]');
        Observer.Expect('write.owner.si', 'd2si[6110,6110,s6110]d3[6111,6111,ws,|tmp 0,0,]');
        Observer.Expect('write.r.p', 'd2p[0,0,]d3[0,0,wp,|tmp 0,0,]');
        Observer.Verify();
    end;

    [Test]
    procedure DeepFields_AfterRegRun_OwnerClearAll()
    var
        R: Codeunit "CSF DReg";
        Owner: Codeunit "CSF DOwner";
        Row: Record "CKK Row";
    begin
        R.Prime(7000, 2, true);
        Owner.SetReg(R);
        Row."No." := 7;
        Row.Txt := 'row';
        Row.SetRange("No.", 7);
        R.Run(Row);
        ObserveTree('before.', R);
        Observer.Observe('before.row', RowState(Row));
        Owner.DoClearAll();
        ObserveTree('after.', R);
        Observer.Observe('after.owner.p', Owner.StateP());
        Observer.Observe('after.owner.si', Owner.StateSi());
        Observer.Observe('after.row', RowState(Row));
        Observer.Expect('before.reg', 'dreg[7,7000,r7000]');
        Observer.Expect('before.d1', 'd1[7100,7100,o7100]');
        Observer.Expect('before.si', 'd2si[7110,7110,s7110]d3[7111,7111,d7111,No.: 7111|tmp 0,0,]');
        Observer.Expect('before.p', 'd2p[7120,7120,p7120]d3[7121,7121,d7121,No.: 7121|tmp 0,0,]');
        Observer.Expect('before.row', 'row[7,row,No.: 7]');
        Observer.Expect('after.reg', 'dreg[0,0,]');
        Observer.Expect('after.d1', 'd1[0,0,]');
        Observer.Expect('after.si', 'd2si[7110,7110,s7110]d3[7111,7111,d7111,No.: 7111|tmp 0,0,]');
        Observer.Expect('after.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.p', 'd2p[0,0,]d3[0,0,,|tmp 0,0,]');
        Observer.Expect('after.owner.si', 'd2si[7110,7110,s7110]d3[7111,7111,d7111,No.: 7111|tmp 0,0,]');
        Observer.Expect('after.row', 'row[7,row,No.: 7]');
        Observer.Verify();
    end;
}
