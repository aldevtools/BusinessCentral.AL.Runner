// Each test names its shape: the argument (RecZ: GB Z, a table no mark names; RecX: GB X, the table marked first; RecY: GB Y,
// the table marked second; RecTemp: a temporary GB X), its filters (none, or the Prepare code after the kind), the steps, and
// the end (Fails: the run fails; Ends: it ends normally). Every expected value is the one main gives.
codeunit 98110 "GB Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "GB Mode";

    local procedure Initialize(Steps: Text)
    var
        X: Record "GB X";
        Y: Record "GB Y";
        Z: Record "GB Z";
    begin
        X.DeleteAll();
        Y.DeleteAll();
        Z.DeleteAll();
        X.Id := 1;
        X.Name := 'caller';
        X.Insert();
        Y.Id := 1;
        Y.Name := 'caller';
        Y.Insert();
        Z.Id := 1;
        Z.Name := 'caller';
        Z.Insert();
        Commit();
        ClearLastError();
        Mode.Set(Steps);
    end;

    // The argument's filters: '' none; key = SetRange(Id, 1) (measured by corpus PR 1598); two = SetRange(Id, 1) and
    // SetRange(Name, 'caller'); g2 = SetRange(Name, 'caller') in filter group 2, back in group 0; cur2 = the same, left in
    // group 2; range = SetRange(Id, 0, 5); expr = SetFilter(Name, '<>zzz'); flow = SetRange("Date Filter", 0D, WorkDate());
    // mark = one marked row without MarkedOnly.
    local procedure PrepareZ(var Z: Record "GB Z"; Filter: Text)
    begin
        case Filter of
            'key':
                Z.SetRange(Id, 1);
            'two':
                begin
                    Z.SetRange(Id, 1);
                    Z.SetRange(Name, 'caller');
                end;
            'g2', 'cur2':
                begin
                    Z.FilterGroup(2);
                    Z.SetRange(Name, 'caller');
                    if Filter = 'g2' then
                        Z.FilterGroup(0);
                end;
            'range':
                Z.SetRange(Id, 0, 5);
            'expr':
                Z.SetFilter(Name, '<>zzz');
            'flow':
                Z.SetRange("Date Filter", 0D, WorkDate());
            'mark':
                Z.Mark(true);
        end;
    end;

    local procedure RunRec(Kind: Text; Filter: Text) Ok: Boolean
    var
        X: Record "GB X";
        Y: Record "GB Y";
        Z: Record "GB Z";
        TempX: Record "GB X" temporary;
    begin
        case Kind of
            'z':
                begin
                    Z.Get(1);
                    PrepareZ(Z, Filter);
                    Ok := Codeunit.Run(Codeunit::"GB Inner Z", Z);
                end;
            'x':
                begin
                    X.Get(1);
                    if Filter = 'key' then
                        X.SetRange(Id, 1);
                    Ok := Codeunit.Run(Codeunit::"GB Inner X", X);
                end;
            'y':
                begin
                    Y.Get(1);
                    if Filter = 'key' then
                        Y.SetRange(Id, 1);
                    Ok := Codeunit.Run(Codeunit::"GB Inner Y", Y);
                end;
            'tmp':
                begin
                    TempX.Id := 1;
                    TempX.Name := 'temp';
                    TempX.Insert();
                    if Filter = 'key' then
                        TempX.SetRange(Id, 1);
                    Ok := Codeunit.Run(Codeunit::"GB Inner X", TempX);
                end;
        end;
    end;

    local procedure RunThenEnd(Kind: Text; Filter: Text; CallerMarks: Boolean)
    var
        CallerX: Record "GB X";
        Ok: Boolean;
    begin
        if CallerMarks then
            CallerX.Consistent(false);
        Ok := RunRec(Kind, Filter);
        Error('run=%1;code=%2', Ok, GetLastErrorCode());
    end;

    // The run returns false with the inner error of code `Code`.
    local procedure Returns(Kind: Text; Filter: Text; Steps: Text; Code: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind, Filter, false);
        if GetLastErrorText() <> StrSubstNo('run=No;code=%1', Code) then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;

    // The run raises the inconsistency error, which the guard does not trap.
    local procedure Raises(Kind: Text; Filter: Text; Steps: Text; CallerMarks: Boolean)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind, Filter, CallerMarks);
        if GetLastErrorCode() <> 'DB:TableInConsistent' then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;

    // ---- issue 9157: the argument sets the first mark, a database variable the second, without a Modify before ----

    [Test]
    procedure RecZ_ModifyRec_MarkRec_MarkYByA_Fails()
    begin
        // The measured cell (ConsistentGuardedRunRecFail.al RecZ_ModifyRec_MarkRec_MarkYByA_Fails).
        Returns('z', '', 'modrec,frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_MarkRec_MarkYByA_Fails()
    begin
        Returns('z', '', 'frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecX_MarkRec_MarkYByA_Fails()
    begin
        Returns('x', '', 'frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecTemp_MarkRec_MarkYByA_Fails()
    begin
        Returns('tmp', '', 'frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecTemp_ModifyRec_MarkRec_MarkYByA_Fails()
    begin
        Returns('tmp', '', 'modrec,frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_MarkRec_MarkYByA_Fails()
    begin
        Returns('z', 'key', 'frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_ModifyRec_MarkRec_MarkYByA_Fails()
    begin
        Returns('z', 'key', 'modrec,frec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecTempKey_MarkRec_MarkYByA_Fails()
    begin
        Returns('tmp', 'key', 'frec,fya,err', 'Dialog');
    end;

    // ---- issue 9184: a mark the argument itself released ----

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkYByA_Fails()
    begin
        // The measured cell (ConsistentGuardedRunRecFail.al RecX_MarkXByA_TrueXByB_MarkYByA_Fails).
        Returns('x', '', 'fxa,txb,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_MarkYByA_Fails()
    begin
        Returns('x', '', 'fxa,trec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_MarkXByC_MarkYByA_Fails()
    begin
        Returns('x', '', 'fxa,trec,fxc,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_MarkXByC_MarkYByA_Ends()
    begin
        Raises('x', '', 'fxa,trec,fxc,fya', false);
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_MarkXByC_TrueXByA_Ends()
    begin
        Raises('x', '', 'fxa,trec,fxc,txa', false);
    end;

    [Test]
    procedure RecY_MarkXByA_MarkYByA_TrueYByRec_MarkYByC_Fails()
    begin
        Returns('y', '', 'fxa,fya,trec,fyc,err', 'Dialog');
    end;

    [Test]
    procedure RecYKey_MarkXByA_MarkYByA_TrueYByRec_MarkYByC_Fails()
    begin
        Returns('y', 'key', 'fxa,fya,trec,fyc,err', 'Dialog');
    end;

    [Test]
    procedure RecXKey_MarkXByA_TrueXByRec_MarkYByA_Fails()
    begin
        Returns('x', 'key', 'fxa,trec,fya,err', 'Dialog');
    end;

    // ---- issue 9151: the end path (`guarded-run-consistent-end`) with a record argument of a table no mark names ----

    [Test]
    procedure RecZ_StartX_Fails()
    begin
        // The measured cell (ConsistentGuardedRunEnd.al GuardedRunIdRec_InnerStartsStateAndFails).
        Returns('z', '', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_StartX_MarkYByA_Fails()
    begin
        // The measured cell (ConsistentGuardedRunRecFiltered.al, the _SetRangeKey_ twin of _StartX_MarkYByA_Fails).
        Returns('z', 'key', 'startx,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_StartX_Fails()
    begin
        Returns('z', 'key', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZTwo_StartX_Fails()
    begin
        Returns('z', 'two', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZTwo_StartX_MarkYByA_Fails()
    begin
        Returns('z', 'two', 'startx,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZG2_StartX_Fails()
    begin
        Returns('z', 'g2', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZCur2_StartX_Fails()
    begin
        Returns('z', 'cur2', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZRange_StartX_Fails()
    begin
        Returns('z', 'range', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZExpr_StartX_Fails()
    begin
        Returns('z', 'expr', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZFlow_StartX_Fails()
    begin
        Returns('z', 'flow', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZMark_StartX_Fails()
    begin
        Returns('z', 'mark', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_StartX_GetFails()
    begin
        Returns('z', '', 'startx,getfail', 'DB:RecordNotFound');
    end;

    [Test]
    procedure RecZ_StartX_TestFieldFails()
    begin
        Returns('z', '', 'startx,testfield', 'TestField');
    end;

    [Test]
    procedure RecZTwo_StartX_GetFails()
    begin
        Returns('z', 'two', 'startx,getfail', 'DB:RecordNotFound');
    end;

    [Test]
    procedure RecZ_SetRec_StartX_Fails()
    begin
        Returns('z', '', 'setrec,startx,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_StartX_Ends()
    begin
        // The measured cell (ConsistentGuardedRunEnd.al GuardedRunIdRec_InnerStartsState_Succeeds).
        Raises('z', '', 'startx', false);
    end;

    [Test]
    procedure RecZTwo_StartX_Ends()
    begin
        Raises('z', 'two', 'startx', false);
    end;

    [Test]
    procedure RecZExpr_StartX_Ends()
    begin
        Raises('z', 'expr', 'startx', false);
    end;

    [Test]
    procedure RecZ_SetRec_StartX_Ends()
    begin
        Raises('z', '', 'setrec,startx', false);
    end;

    [Test]
    procedure RecZ_CallerMarks_Fails()
    begin
        // The measured cell (ConsistentGuardedRunEnd.al GuardedRunIdRec_CallerMarks_InnerFails).
        Raises('z', '', 'err', true);
    end;

    [Test]
    procedure RecZTwo_CallerMarks_Fails()
    begin
        Raises('z', 'two', 'err', true);
    end;

    [Test]
    procedure RecZTwo_CallerMarks_Ends()
    begin
        Raises('z', 'two', '', true);
    end;
}
