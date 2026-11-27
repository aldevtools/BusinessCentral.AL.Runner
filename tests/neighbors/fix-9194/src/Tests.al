codeunit 64940 "N94 Tests"
{
    Subtype = Test;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;

    // Corpus PR 1619 ClearSharedCodeunitWhseStore.al Whse_OwnerClearAll_EveryLevel :356.
    [Test]
    procedure Whse_EveryLevel()
    var
        R: Codeunit "N94 Reg";
        Owner: Codeunit "N94 Owner";
        Confirm: Codeunit "N94 Confirm";
        Impl: Codeunit "N94 Impl";
        Mid: Codeunit "N94 Mid";
    begin
        R.Prime(1000);
        Owner.SetReg(R);
        Expect(Owner.State(), 'owner[7]reg[1000,Yes,1000,Qty 1 short,Serial 2 found]mid[1100,1100]confirm[1110]impl[Yes,1111]');
        Owner.DoClearAll();
        Expect(R.State(), 'reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[1110]impl[Yes,1111]');
        Expect(Owner.State(), 'owner[0]reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[1110]impl[Yes,1111]');
        Expect(Confirm.State(), 'confirm[1110]impl[Yes,1111]');
        Expect(Impl.State(), 'impl[Yes,1111]');
        Expect(Mid.State(), 'mid[0,]confirm[1110]impl[Yes,1111]');
    end;

    // Whse_OwnerPrimes_OwnerClearAll :386.
    [Test]
    procedure Whse_OwnerPrimes()
    var
        R: Codeunit "N94 Reg";
        Owner: Codeunit "N94 Owner";
        Confirm: Codeunit "N94 Confirm";
    begin
        Owner.SetReg(R);
        Owner.PrimeOwn(2000);
        Owner.DoClearAll();
        Expect(R.State(), 'reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[2110]impl[Yes,2111]');
        Confirm.Prime(2500);
        Expect(R.State(), 'reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[2500]impl[Yes,2501]');
        R.Prime(2600);
        Expect(Owner.State(), 'owner[0]reg[2600,Yes,2600,Qty 1 short,Serial 2 found]mid[2700,2700]confirm[2710]impl[Yes,2711]');
    end;

    // Whse_DirectSiWrite_OwnerClearAll :412.
    [Test]
    procedure Whse_DirectSiWrite()
    var
        R: Codeunit "N94 Reg";
        Owner: Codeunit "N94 Owner";
        Impl: Codeunit "N94 Impl";
    begin
        R.Prime(3000);
        Owner.SetReg(R);
        Impl.Prime(3333);
        Owner.DoClearAll();
        Expect(R.State(), 'reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[3110]impl[Yes,3333]');
        Expect(Impl.State(), 'impl[Yes,3333]');
    end;

    // Kinds_OwnerClearAll :431.
    [Test]
    procedure Kinds()
    var
        R: Codeunit "N94 KReg";
        Owner: Codeunit "N94 KOwner";
        Si: Codeunit "N94 KSi";
    begin
        R.Prime(40);
        Owner.SetReg(R);
        Expect(R.State(), 'kreg[40,t40,u40,C40,40.5,2020-02-01,40000,C,A,Yes,comment 1,plain 2,locked]ksi[41]');
        Owner.DoClearAll();
        Expect(R.State(), 'kreg[0,,,,0,,0,A,' + Format(Ch0()) + ',No,comment 1,plain 2,locked]ksi[41]');
        Expect(Si.State(), 'ksi[41]');
    end;

    local procedure Ch0(): Char
    var
        C: Char;
    begin
        exit(C);
    end;

    // Deep_OwnerClearAll :451.
    [Test]
    procedure Deep()
    var
        R: Codeunit "N94 DReg";
        Owner: Codeunit "N94 DOwner";
        D2: Codeunit "N94 D2Si";
    begin
        R.Prime(5000);
        Owner.SetReg(R);
        Expect(R.State(), 'dreg[5000]d1[5100]d2si[5110]d3[5111,5111,deep 3]d2p[5120]d3[5121,5121,deep 3]');
        Owner.DoClearAll();
        Expect(R.State(), 'dreg[0]d1[0]d2si[5110]d3[5111,5111,deep 3]d2p[0]d3[0,,deep 3]');
        Expect(D2.State(), 'd2si[5110]d3[5111,5111,deep 3]');
        D2.Prime(5500);
        Expect(R.State(), 'dreg[0]d1[0]d2si[5500]d3[5501,5501,deep 3]d2p[0]d3[0,,deep 3]');
    end;

    // Not measured: the plain owner.
    [Test]
    procedure Refused_PlainOwner()
    var
        R: Codeunit "N94 Reg";
        Owner: Codeunit "N94 PlainOwner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(R.State(), 'reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[11]impl[Yes,12]');
    end;

    // Not measured: the register never called before the ClearAll.
    [Test]
    procedure Refused_NeverCalled()
    var
        R: Codeunit "N94 Reg";
        Owner: Codeunit "N94 Owner";
    begin
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(Owner.State(), 'owner[0]reg[0,No,,Qty 1 short,Serial 2 found]mid[0,]confirm[0]impl[No,0]');
    end;

    // Not measured: XTime.
    [Test]
    procedure Refused_XTime()
    var
        R: Codeunit "N94 XTime";
        Owner: Codeunit "N94 XTime Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;

    // Not measured: XTemp.
    [Test]
    procedure Refused_XTemp()
    var
        R: Codeunit "N94 XTemp";
        Owner: Codeunit "N94 XTemp Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;

    // Not measured: XList.
    [Test]
    procedure Refused_XList()
    var
        R: Codeunit "N94 XList";
        Owner: Codeunit "N94 XList Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;

    // Not measured: XSiSi.
    [Test]
    procedure Refused_XSiSi()
    var
        R: Codeunit "N94 XSiSi";
        Owner: Codeunit "N94 XSiSi Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;

    // Not measured: XText.
    [Test]
    procedure Refused_XText()
    var
        R: Codeunit "N94 XText";
        Owner: Codeunit "N94 XText Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;

    // Not measured: XD3Temp.
    [Test]
    procedure Refused_XD3Temp()
    var
        R: Codeunit "N94 XD3Temp";
        Owner: Codeunit "N94 XD3Temp Owner";
    begin
        R.Prime(1);
        Owner.SetReg(R);
        Owner.DoClearAll();
        Expect(CopyStr(R.State(), 1, 4), 'x[0]');
    end;
}
