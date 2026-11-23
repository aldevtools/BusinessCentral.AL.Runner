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

// 130450 leg, before the hub storage writer 50137, reads the isolated storage it sets: refused by the carry rule, as the
// walk runs the hub leg first and who reads that storage is not followed (review round 1).
codeunit 50126 "N70 Early Storage Reader"
{
    Subtype = Test;
    [Test] procedure EarlyStorageReaderReads() begin if IsolatedStorage.Contains('n70-key', DataScope::Module) then Error('storage set'); end;
}

// Hub leg (TestPermissions = Disabled, no handlers), before the writer: the hub leg runs in id order, sees nothing.
codeunit 50123 "N70 Hub Early Reader"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "N70 Rows";
    [Test] procedure HubEarlyReaderSeesNone() begin if Rows.Count() <> 0 then Error('hub early: %1', Rows.Count()); end;
}

// Hub leg, a writer of its own table before the reader below (review round 1).
codeunit 50124 "N70 Hub Mid Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure HubMidWriterWrites() var R: Record "N70 Mid Row"; begin R.K := 'M1'; R.Insert(); end;
}

// 130450 leg, between the hub writers 50124 and 50130, reads only 50124's table: hub leg first or id order, it sees the
// row; only 50130's place against it is not pinned. Runs (review round 1).
codeunit 50125 "N70 Mid Reader"
{
    Subtype = Test;
    [Test] procedure MidReaderSeesEarlierHubRow() var R: Record "N70 Mid Row";
    begin if R.Count() <> 1 then Error('mid: %1', R.Count()); end;
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

// 130450 leg, after the hub writers: its OnRun writes a table no hub codeunit writes.
codeunit 50135 "N70 OnRun Writer"
{
    Subtype = Test;
    trigger OnRun() var R: Record "N70 Third Row"; begin R.K := 'T1'; R.Insert(); end;
    [Test] procedure OnRunWriterRuns() begin end;
}

// 130450 leg with handlers, after 50135, reads only 50135's table: both run under runner 130450 in id order, so the hub
// leg's place does not matter. Runs (review round 1).
codeunit 50136 "N70 Handler Third Reader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('N70Msg3')]
    procedure HandlerReaderSeesOnRunRow() var R: Record "N70 Third Row";
    begin Message('x'); if R.Count() <> 1 then Error('third: %1', R.Count()); end;
    [MessageHandler] procedure N70Msg3(M: Text[1024]) begin end;
}

// Hub leg, the last hub writer: sets isolated storage only.
codeunit 50137 "N70 Hub Storage Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure HubStorageWriterSets() begin IsolatedStorage.Set('n70-key', 'v', DataScope::Module); end;
}
