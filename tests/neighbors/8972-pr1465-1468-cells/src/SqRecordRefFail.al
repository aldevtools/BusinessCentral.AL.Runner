// Copy of corpus tests/al-language-isolation-probe/src/SqRecordRefFail.al (corpus PRs 1465 and 1468), so the neighbor check
// compares main and head on every cell of those probes (the corpus pin does not reach them yet).
namespace ALLanguage.IsolationProbe;

// Probe: the C14 and C2 cells of SqCommitFail.al with RecordRef access in the failed test (AL Runner: ERM Intercompany III,
// Tests-ERM codeunit 134154, whose failed test VerifyShipmentFieldsInOutboxExportFile reaches the IC tables through
// Library - Utility.GetNewRecNo). Does a failed test that touches a table only through a RecordRef lose a number sequence
// an earlier test inserted, as a failed test with Record access does (C2, C14)?
//
// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/recordref/recordref-data-type
// Scope: NumberSequence after a failed test; RecordRef.Open / GetTable / FindLast / Insert, FieldRef.SetRange / Value, a
// Record passed to a RecordRef parameter.
// Fixtures used: SqCommitFailFixture.al ("LT SQ Log", "LT SQ Helper"); "LT SQ Line" and "LT SQ Ref Access" below.
// "LT SQ Ref Reader" (76239, SqRecordRefReader.al) asserts every reading.
// Probe: written against BC 27.5, 28.5 and 29.0 (Windows sandbox containers), runner 61301 (TestIsolation =
// Codeunit), and 28.5 under 130450 (Microsoft's test runner): the same readings on every version and both runners. Each
// cell codeunit below fails on purpose; the reader is the assertion.
//
// What BC does under TestIsolation = Codeunit (C14 cells R14*, C2 cells R2*):
// - A RecordRef access in the failed test counts as table access, the same as a Record access: the sequence an earlier
//   test inserted is gone in the next test of the codeunit (C14 shape: GetNextNo inserts it again and gives 1; C2 shape:
//   missing). That holds for a RecordRef read that finds a row (R14O), one that finds no row (R14E, R2O), the
//   Library - Utility.GetNewRecNo copy through a Variant (R14V, R2V), a Record passed to a RecordRef parameter (R14P,
//   R2P), RecordRef.Insert (R14W, R2W) and the ERM Intercompany III mock, GetNewRecNo then Record.Insert (R14M, R2M).
//   The Record control (R14R, R2R) reads the same.
// - RecordRef.Open and Close with no read or write is not table access: the sequence stays (R14N, R2N: cur=1, Next 2).
// - Rows the failed test inserted (R14W, R14M, R2W, R2M) are gone; the row Initialize committed stays (R14*: 10000).
// - A later test codeunit never finds any of these sequences.

table 76240 "LT SQ Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Doc; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }

    keys
    {
        key(PK; Doc, "Line No.") { Clustered = true; }
    }
}

codeunit 76240 "LT SQ Ref Access"
{
    // The sequence code of IC Inbox Outbox Mgt. (GetNextICTransactionNo), as in C14: insert at seed 0 when missing, burn
    // the seed, then take the next number.
    procedure GetNextNo(Cell: Text): BigInteger
    var
        Log: Codeunit "LT SQ Log";
        Name: Text;
    begin
        Name := Log.Seq(Cell);
        if not NumberSequence.Exists(Name) then begin
            NumberSequence.Insert(Name, 0, 1);
            if NumberSequence.Next(Name) = 0 then;
        end;
        exit(NumberSequence.Next(Name));
    end;

    // Library - Utility.GetNewRecNo (Tests-TestLibraries), unchanged.
    procedure GetNewRecNo(RecVariant: Variant; FieldNo: Integer): Integer
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(RecVariant);
        exit(GetNewLineNo(RecRef, FieldNo));
    end;

    // Library - Utility.GetNewLineNo, unchanged except the key-not-found error.
    procedure GetNewLineNo(RecRef: RecordRef; FieldNo: Integer): Integer
    var
        RecRef2: RecordRef;
        FieldRef: FieldRef;
        FieldRef2: FieldRef;
        KeyRef: KeyRef;
        FieldCount: Integer;
        LineNumberFound: Boolean;
    begin
        RecRef2.Open(RecRef.Number, false, CompanyName);
        KeyRef := RecRef.KeyIndex(1);
        for FieldCount := 1 to KeyRef.FieldCount do begin
            FieldRef := KeyRef.FieldIndex(FieldCount);
            if FieldRef.Number <> FieldNo then begin
                FieldRef2 := RecRef2.Field(FieldRef.Number);
                FieldRef2.SetRange(FieldRef.Value);
            end else
                LineNumberFound := true;
        end;
        if not LineNumberFound then
            Error('LT SQ: field %1 is not in the primary key', FieldNo);
        if RecRef2.FindLast() then begin
            FieldRef := RecRef2.Field(FieldNo);
            FieldCount := FieldRef.Value();
        end else
            FieldCount := 0;
        exit(FieldCount + 10000);
    end;

    // 'last=<Line No.>' of the last row of the document, read through a RecordRef, or 'none'.
    procedure LastLineByRef(Doc: Text): Text
    var
        RecRef: RecordRef;
        FieldRef: FieldRef;
        LineNo: Integer;
    begin
        RecRef.Open(Database::"LT SQ Line");
        FieldRef := RecRef.Field(1);
        FieldRef.SetRange(Doc);
        if not RecRef.FindLast() then
            exit('none');
        FieldRef := RecRef.Field(2);
        LineNo := FieldRef.Value();
        exit('last=' + Format(LineNo));
    end;

    // The same read through a Record.
    procedure LastLineByRecord(Doc: Text): Text
    var
        Line: Record "LT SQ Line";
    begin
        Line.SetRange(Doc, Doc);
        if not Line.FindLast() then
            exit('none');
        exit('last=' + Format(Line."Line No."));
    end;

    procedure InsertByRef(Doc: Text; LineNo: Integer): Text
    var
        RecRef: RecordRef;
        FieldRef: FieldRef;
    begin
        RecRef.Open(Database::"LT SQ Line");
        FieldRef := RecRef.Field(1);
        FieldRef.Value := Doc;
        FieldRef := RecRef.Field(2);
        FieldRef.Value := LineNo;
        exit('inserted=' + Format(RecRef.Insert()));
    end;

    procedure OpenOnly(): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::"LT SQ Line");
        RecRef.Close();
        exit('opened');
    end;

    // ERM Intercompany III's MockICOutboxTrans: the key from GetNewRecNo, then Record.Insert.
    procedure MockInsert(Doc: Text): Text
    var
        Line: Record "LT SQ Line";
    begin
        Line.Init();
        Line.Doc := CopyStr(Doc, 1, MaxStrLen(Line.Doc));
        Line."Line No." := GetNewRecNo(Line, Line.FieldNo("Line No."));
        Line.Insert();
        exit('inserted ' + Format(Line."Line No."));
    end;

    procedure InsertLine(Doc: Text; LineNo: Integer)
    var
        Line: Record "LT SQ Line";
    begin
        Line.Doc := CopyStr(Doc, 1, MaxStrLen(Line.Doc));
        Line."Line No." := LineNo;
        Line.Insert();
    end;

    // The Line Nos. of the document, comma-separated, or 'none'.
    procedure Lines(Doc: Text): Text
    var
        Line: Record "LT SQ Line";
        Result: Text;
    begin
        Line.SetRange(Doc, Doc);
        if Line.FindSet() then
            repeat
                if Result <> '' then
                    Result += ',';
                Result += Format(Line."Line No.");
            until Line.Next() = 0;
        if Result = '' then
            exit('none');
        exit(Result);
    end;
}

// R14O (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through RecordRef.Open, FindLast and a field read (a row is there) and fails; the test after it reads the sequence.
codeunit 76241 "LT SQ R14O RecRef Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Oa_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Ob_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Oc_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.LastLineByRef(Cell));
        Error('R14Oc fails on purpose');
    end;

    [Test]
    procedure R14Od_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14O';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14V (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through Library - Utility.GetNewRecNo: the Record as a Variant, GetTable, then GetNewLineNo (Open with CompanyName, KeyIndex, SetRange, FindLast, field read) and fails; the test after it reads the sequence.
codeunit 76242 "LT SQ R14V GetNewRecNo"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Va_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Vb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Vc_AccessesFails()
    var
        Line: Record "LT SQ Line";
    begin
        Initialize();
        Line.Doc := Cell;
        Log.Put(Cell + ' access', Format(Ref.GetNewRecNo(Line, Line.FieldNo("Line No."))));
        Error('R14Vc fails on purpose');
    end;

    [Test]
    procedure R14Vd_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14V';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14P (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through the Record passed to a procedure that takes a RecordRef (Library - Utility.GetNewLineNo) and fails; the test after it reads the sequence.
codeunit 76243 "LT SQ R14P Record As RecRef"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Pa_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Pb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Pc_AccessesFails()
    var
        Line: Record "LT SQ Line";
    begin
        Initialize();
        Line.Doc := Cell;
        Log.Put(Cell + ' access', Format(Ref.GetNewLineNo(Line, Line.FieldNo("Line No."))));
        Error('R14Pc fails on purpose');
    end;

    [Test]
    procedure R14Pd_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14P';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14W (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through RecordRef.Open, field values and RecordRef.Insert and fails; the test after it reads the sequence.
codeunit 76244 "LT SQ R14W RecRef Insert"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Wa_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Wb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Wc_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.InsertByRef(Cell, 20000));
        Error('R14Wc fails on purpose');
    end;

    [Test]
    procedure R14Wd_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14W';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14N (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through RecordRef.Open and Close with no read and fails; the test after it reads the sequence.
codeunit 76245 "LT SQ R14N RecRef Open Only"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Na_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Nb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Nc_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.OpenOnly());
        Error('R14Nc fails on purpose');
    end;

    [Test]
    procedure R14Nd_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14N';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14E (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through RecordRef.Open, SetRange to a key with no row, FindLast and fails; the test after it reads the sequence.
codeunit 76246 "LT SQ R14E RecRef Read No Row"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Ea_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Eb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Ec_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.LastLineByRef(Cell + 'X'));
        Error('R14Ec fails on purpose');
    end;

    [Test]
    procedure R14Ed_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14E';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14M (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through the ERM Intercompany III mock (MockICOutboxTrans): GetNewRecNo, then Record.Insert and fails; the test after it reads the sequence.
codeunit 76247 "LT SQ R14M IC Outbox Mock"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Ma_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Mb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Mc_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.MockInsert(Cell));
        Error('R14Mc fails on purpose');
    end;

    [Test]
    procedure R14Md_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14M';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R14R (C14 shape): the first test's Initialize writes a row and commits; a later passing test inserts the sequence; a later
// test accesses the table through control: Record.FindLast of the same table (no RecordRef) and fails; the test after it reads the sequence.
codeunit 76248 "LT SQ R14R Record Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        IsInitialized: Boolean;
        Cell: Text[20];

    [Test]
    procedure R14Ra_InitializeCommits()
    begin
        Initialize();
    end;

    [Test]
    procedure R14Rb_TakesNumber()
    begin
        Initialize();
        Log.Put(Cell + ' first no', Format(Ref.GetNextNo(Cell)));
    end;

    [Test]
    procedure R14Rc_AccessesFails()
    begin
        Initialize();
        Log.Put(Cell + ' access', Ref.LastLineByRecord(Cell));
        Error('R14Rc fails on purpose');
    end;

    [Test]
    procedure R14Rd_ReadsSequence()
    begin
        Initialize();
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Log.Put(Cell + ' after state', Helper.State(Log.Seq(Cell)));
        Log.Put(Cell + ' after no', Format(Ref.GetNextNo(Cell)));
    end;

    // ERM Intercompany III's Initialize (Tests-ERM codeunit 134154): every test calls it first; only the first call writes
    // and commits.
    local procedure Initialize()
    begin
        Cell := 'R14R';
        if IsInitialized then
            exit;
        Ref.InsertLine(Cell, 10000);
        IsInitialized := true;
        Commit();
    end;
}

// R2O (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through RecordRef.Open, FindLast and a field read (no row of the document exists, so it finds none) and fails;
// the test after it reads the sequence.
codeunit 76249 "LT SQ R2O RecRef Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Oa_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2O';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Ob_AccessesFails()
    begin
        Cell := 'R2O';
        Log.Put(Cell + ' access', Ref.LastLineByRef(Cell));
        Error('R2Ob fails on purpose');
    end;

    [Test]
    procedure R2Oc_ReadsSequence()
    begin
        Cell := 'R2O';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2V (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through Library - Utility.GetNewRecNo: the Record as a Variant, GetTable, then GetNewLineNo (Open with CompanyName, KeyIndex, SetRange, FindLast, field read) and fails; the test after it reads the sequence.
codeunit 76250 "LT SQ R2V GetNewRecNo"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Va_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2V';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Vb_AccessesFails()
    var
        Line: Record "LT SQ Line";
    begin
        Cell := 'R2V';
        Line.Doc := Cell;
        Log.Put(Cell + ' access', Format(Ref.GetNewRecNo(Line, Line.FieldNo("Line No."))));
        Error('R2Vb fails on purpose');
    end;

    [Test]
    procedure R2Vc_ReadsSequence()
    begin
        Cell := 'R2V';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2P (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through the Record passed to a procedure that takes a RecordRef (Library - Utility.GetNewLineNo) and fails; the test after it reads the sequence.
codeunit 76251 "LT SQ R2P Record As RecRef"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Pa_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2P';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Pb_AccessesFails()
    var
        Line: Record "LT SQ Line";
    begin
        Cell := 'R2P';
        Line.Doc := Cell;
        Log.Put(Cell + ' access', Format(Ref.GetNewLineNo(Line, Line.FieldNo("Line No."))));
        Error('R2Pb fails on purpose');
    end;

    [Test]
    procedure R2Pc_ReadsSequence()
    begin
        Cell := 'R2P';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2W (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through RecordRef.Open, field values and RecordRef.Insert and fails; the test after it reads the sequence.
codeunit 76252 "LT SQ R2W RecRef Insert"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Wa_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2W';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Wb_AccessesFails()
    begin
        Cell := 'R2W';
        Log.Put(Cell + ' access', Ref.InsertByRef(Cell, 20000));
        Error('R2Wb fails on purpose');
    end;

    [Test]
    procedure R2Wc_ReadsSequence()
    begin
        Cell := 'R2W';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2N (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through RecordRef.Open and Close with no read and fails; the test after it reads the sequence.
codeunit 76253 "LT SQ R2N RecRef Open Only"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Na_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2N';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Nb_AccessesFails()
    begin
        Cell := 'R2N';
        Log.Put(Cell + ' access', Ref.OpenOnly());
        Error('R2Nb fails on purpose');
    end;

    [Test]
    procedure R2Nc_ReadsSequence()
    begin
        Cell := 'R2N';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2M (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through the ERM Intercompany III mock (MockICOutboxTrans): GetNewRecNo, then Record.Insert and fails; the test after it reads the sequence.
codeunit 76254 "LT SQ R2M IC Outbox Mock"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Ma_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2M';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Mb_AccessesFails()
    begin
        Cell := 'R2M';
        Log.Put(Cell + ' access', Ref.MockInsert(Cell));
        Error('R2Mb fails on purpose');
    end;

    [Test]
    procedure R2Mc_ReadsSequence()
    begin
        Cell := 'R2M';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}

// R2R (C2 shape): a passing test inserts the sequence, takes a number and calls Commit(); a later test accesses the
// table through control: Record.FindLast of the same table (no RecordRef) and fails; the test after it reads the sequence.
codeunit 76255 "LT SQ R2R Record Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";
        Ref: Codeunit "LT SQ Ref Access";
        Cell: Text[20];

    [Test]
    procedure R2Ra_InsertsCommitsPasses()
    var
        Name: Text;
    begin
        Cell := 'R2R';
        Name := Log.Seq(Cell);
        NumberSequence.Insert(Name, 1, 1);
        Log.Put(Cell + ' first next', Format(NumberSequence.Next(Name)));
        Commit();
    end;

    [Test]
    procedure R2Rb_AccessesFails()
    begin
        Cell := 'R2R';
        Log.Put(Cell + ' access', Ref.LastLineByRecord(Cell));
        Error('R2Rb fails on purpose');
    end;

    [Test]
    procedure R2Rc_ReadsSequence()
    begin
        Cell := 'R2R';
        Log.Put(Cell + ' lines', Ref.Lines(Cell));
        Helper.Read(Cell + ' after', Log.Seq(Cell));
    end;
}
