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
        V: Record "GB V";
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
        V.DeleteAll();
        V."Doc Kind" := V."Doc Kind"::Order;
        V."No." := '1';
        V.Name := 'caller';
        V.Insert();
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
        V: Record "GB V";
    begin
        case Kind of
            'v':
                begin
                    // The page view of GB V List, set in code: SetRange on the Enum key field in filter group 2.
                    V.Get(V."Doc Kind"::Order, '1');
                    V.FilterGroup(2);
                    V.SetRange("Doc Kind", V."Doc Kind"::Order);
                    V.FilterGroup(0);
                    Ok := Codeunit.Run(Codeunit::"GB Inner V", V);
                end;
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

    // Caller: '' nothing; 'marks' the caller marks GB X; 'released' another variable then releases that mark.
    local procedure RunThenEnd(Kind: Text; Filter: Text; Caller: Text)
    var
        CallerX: Record "GB X";
        OtherX: Record "GB X";
        Ok: Boolean;
    begin
        if Caller in ['marks', 'released'] then
            CallerX.Consistent(false);
        if Caller = 'released' then
            OtherX.Consistent(true);
        Ok := RunRec(Kind, Filter);
        Error('run=%1;code=%2', Ok, GetLastErrorCode());
    end;

    // The run returns false with the inner error of code `Code`.
    local procedure Returns(Kind: Text; Filter: Text; Steps: Text; Code: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind, Filter, '');
        if GetLastErrorText() <> StrSubstNo('run=No;code=%1', Code) then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;

    // The run raises the inconsistency error, which the guard does not trap.
    local procedure Raises(Kind: Text; Filter: Text; Steps: Text; Caller: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind, Filter, Caller);
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
        Raises('x', '', 'fxa,trec,fxc,fya', '');
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_MarkXByC_TrueXByA_Ends()
    begin
        Raises('x', '', 'fxa,trec,fxc,txa', '');
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
        Raises('z', '', 'startx', '');
    end;

    [Test]
    procedure RecZTwo_StartX_Ends()
    begin
        Raises('z', 'two', 'startx', '');
    end;

    [Test]
    procedure RecZExpr_StartX_Ends()
    begin
        Raises('z', 'expr', 'startx', '');
    end;

    [Test]
    procedure RecZ_SetRec_StartX_Ends()
    begin
        Raises('z', '', 'setrec,startx', '');
    end;

    [Test]
    procedure RecZ_CallerMarks_Fails()
    begin
        // The measured cell (ConsistentGuardedRunEnd.al GuardedRunIdRec_CallerMarks_InnerFails).
        Raises('z', '', 'err', 'marks');
    end;

    [Test]
    procedure RecZTwo_CallerMarks_Fails()
    begin
        Raises('z', 'two', 'err', 'marks');
    end;

    [Test]
    procedure RecZTwo_CallerMarks_Ends()
    begin
        Raises('z', 'two', '', 'marks');
    end;

    // ---- round 1 of review: the end arms read every mark and the page view ----

    // Opens the page `PageName` with OpenView, invokes its action on the first row (the run on the page's Rec with extra
    // 'direct', else on a local copy), then raises the outcome.
    local procedure PageThenEnd(PageName: Text)
    var
        List: TestPage "GB V List";
        Worksheet: TestPage "GB V Worksheet";
    begin
        case PageName of
            'list':
                begin
                    List.OpenView();
                    List.First();
                    List.RunGuarded.Invoke();
                    List.Close();
                end;
            'worksheet':
                begin
                    Worksheet.OpenView();
                    Worksheet.First();
                    Worksheet.RunGuarded.Invoke();
                    Worksheet.Close();
                end;
        end;
        Error(Mode.GetResult());
    end;

    local procedure PageReturns(PageName: Text; Extra: Text; Steps: Text; Code: Text)
    begin
        Initialize(Steps);
        Mode.SetExtra(Extra);
        Mode.SetResult('');
        asserterror PageThenEnd(PageName);
        if GetLastErrorText() <> StrSubstNo('run=No;code=%1', Code) then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;

    [Test]
    procedure RecZView_StartX_Fails()
    begin
        // The page view of corpus PR 1622 set in code, with no page.
        Returns('v', '', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure PageList_Direct_StartX_Fails()
    begin
        // The measured shape (ConsistentGuardedRunPageConst.al, OrderDirect Fails): no difference.
        PageReturns('list', 'direct', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure PageList_Copy_StartX_Fails()
    begin
        PageReturns('list', 'copy', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure PageWorksheet_Direct_StartX_Fails()
    begin
        PageReturns('worksheet', 'direct', 'startx,err', 'Dialog');
    end;

    [Test]
    procedure PageList_Direct_TempMarksX_Fails()
    begin
        PageReturns('list', 'direct', 'ftmp,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_StartX_MarkYByA_MarkWByA_Fails()
    begin
        Returns('z', '', 'startx,fya,fwa,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_StartX_MarkYByA_MarkWByA_Ends()
    begin
        Raises('z', '', 'startx,fya,fwa', '');
    end;

    [Test]
    procedure RecZ_CallerMarksReleased_Fails()
    begin
        Raises('z', '', 'err', 'released');
    end;

    [Test]
    procedure RecZ_TempMarksX_Fails()
    begin
        Returns('z', '', 'ftmp,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_TempMarksX_MarkYByA_Fails()
    begin
        Returns('z', 'key', 'ftmp,fya,err', 'Dialog');
    end;

    // ---- round 2 of review: a mark the argument set or released, then lifted during the run ----

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_TrueXByA_MarkYByA_Fails()
    begin
        // The end arm: X released by the argument, then lifted by its setter; only Y is left.
        Returns('x', '', 'fxa,trec,txa,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecZ_MarkRec_TrueRec_MarkYByA_Fails()
    begin
        // The end arm: the argument marks Z and lifts it again; only Y is left.
        Returns('z', '', 'frec,trec,fya,err', 'Dialog');
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByRec_TrueXByA_MarkWByA_MarkYByA_MarkYByC_Fails()
    begin
        // The fail arm (held, then marked again by another variable) after the lifted release.
        Returns('x', '', 'fxa,trec,txa,fwa,fya,fyc,err', 'Dialog');
    end;

    [Test]
    procedure RecZKey_MarkRec_TrueRec_StartX_TrueXByB_MarkYByA_Fails()
    begin
        // The page arm (Key, released then held) after the argument marked and lifted Z.
        Returns('z', 'key', 'frec,trec,startx,txb,fya,err', 'Dialog');
    end;

    [Test]
    procedure PageList_DirectLift_StartX_Ends()
    begin
        // The page view arm: the page's Rec marks and lifts its table, then the run marks X and ends.
        Initialize('startx');
        Mode.SetExtra('direct-lift');
        Mode.SetResult('');
        asserterror PageThenEnd('list');
        if GetLastErrorCode() <> 'TestWrapped:CSide' then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;
}
