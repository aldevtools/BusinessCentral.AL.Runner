// Where a runner 130450 codeunit runs against the TestRunnerHub's writers is not measured (RqIsolationModes.al:12 states the
// hub leg first in its header only). Ids 50120-50139. The hub writer is 50130 (RequiredTestIsolation = Disabled).

// 130450 leg (no TestPermissions = Disabled), before the hub writer, reads its table: refused (new in the walk).
codeunit 50120 "N70 Early Reader"
{
    Subtype = Test;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure EarlyReaderReadsHubTable() begin if Rows.Count() <> 0 then Error('early: %1', Rows.Count()); end;
}

// 130450 leg, before the hub writer, reads no table of it: runs.
codeunit 50121 "N70 Early Non Reader"
{
    Subtype = Test;
    [Test] procedure EarlyNonReaderRuns() begin end;
}

// 130450 leg, before the hub writer, reads another table only: runs.
codeunit 50122 "N70 Early Other Table"
{
    Subtype = Test;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure EarlyReadsOtherTable() begin if Rows.CountOther() <> 0 then Error('other: %1', Rows.CountOther()); end;
}

// Hub leg (TestPermissions = Disabled, no handlers), before the writer: the hub leg runs in id order, sees nothing.
codeunit 50123 "N70 Hub Early Reader"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure HubEarlyReaderSeesNone() begin if Rows.Count() <> 0 then Error('hub early: %1', Rows.Count()); end;
}

codeunit 50130 "N70 Hub Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure HubWriterWrites() begin Rows.Put('H1'); end;
}

// 130450 leg, after the writer, reads its table: sees the row (QuCommitIsolation.al D11).
codeunit 50131 "N70 Late Reader"
{
    Subtype = Test;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure LateReaderSeesRow() begin if Rows.Count() <> 1 then Error('late: %1', Rows.Count()); end;
}

// 130450 leg with handlers, after the writer, reads its table: refused, as main refuses it.
codeunit 50132 "N70 Late Handler Reader"
{
    Subtype = Test;
    var Rows: Codeunit "N70 Rows";
    [Test]
    [HandlerFunctions('N70Msg')]
    procedure LateHandlerReaderReads() begin Message('%1', Rows.Count()); end;
    [MessageHandler] procedure N70Msg(M: Text[1024]) begin end;
}

// 130450 leg with handlers, after the writer, reads nothing of it: runs.
codeunit 50133 "N70 Late Handler NonReader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('N70Msg2')]
    procedure LateHandlerNonReaderRuns() begin Message('x'); end;
    [MessageHandler] procedure N70Msg2(M: Text[1024]) begin end;
}

// Hub leg, after the writer: sees the row.
codeunit 50134 "N70 Hub Late Reader"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure HubLateReaderSeesRow() begin if Rows.Count() <> 1 then Error('hub late: %1', Rows.Count()); end;
}
