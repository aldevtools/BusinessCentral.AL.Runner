// Copy of corpus tests/al-language-isolation-probe/src/SqCommitFailFixture.al (corpus PRs 1465 and 1468), so the neighbor check
// compares main and head on every cell of those probes (the corpus pin does not reach them yet).
namespace ALLanguage.IsolationProbe;

// singleinstance-shared-on-purpose: "LT SQ Log" (76042) is written by the test codeunits 76045-76073 and read by "LT SQ Reader" (76089) in a later one; that is the measurement.

// Fixtures of the NumberSequence and manual-binding probes after a test that fails (SqCommitFail.al, SqBindFailed.al,
// SqReader.al): what a test codeunit's later tests, and the later test codeunits, see of a number sequence after a test
// that called Commit() (or caused a commit) and then failed. AL Runner issues 8152 and 8337.

table 76040 "LT SQ Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Key"; Code[30]) { }
    }

    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

// The reporter's table of AL Runner issue 8152 (comment of 2026-11-18): OnInsert takes the key from a company-wide
// number sequence it inserts when missing. The sequence name comes from "LT SQ Log".EntrySequence().
table 76041 "LT SQ Entry"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT SQ Log";
        SequenceName: Text;
    begin
        SequenceName := Log.EntrySequence();
        if not NumberSequence.Exists(SequenceName, true) then
            NumberSequence.Insert(SequenceName, 1, 1, true);
        "Entry No." := NumberSequence.Next(SequenceName, true);
    end;
}

// The probe log. SingleInstance: its state is not part of any transaction, so no rollback of a test or a test codeunit
// erases it, and "LT SQ Reader" (76089) reads it in a later test codeunit of the same session.
// Sequence names: 'LTSQ' + a base + a token made once per session, because a number sequence outlives the uninstall of the
// app with its data (IsolProbeAfterRollback.Codeunit.al, S3): a fixed name would find what an earlier run left.
codeunit 76042 "LT SQ Log"
{
    SingleInstance = true;

    procedure Seq(Base: Text): Text
    begin
        if RunToken = '' then
            RunToken := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 10);
        exit('LTSQ' + Base + RunToken);
    end;

    procedure SetEntrySequence(Base: Text)
    begin
        EntrySequenceName := Seq(Base);
    end;

    procedure EntrySequence(): Text
    begin
        exit(EntrySequenceName);
    end;

    procedure Put(EntryKey: Text; Value: Text)
    begin
        Entries.Set(EntryKey, Value);
    end;

    procedure Get(EntryKey: Text): Text
    begin
        if Entries.ContainsKey(EntryKey) then
            exit(Entries.Get(EntryKey));
        exit('<no entry>');
    end;

    var
        Entries: Dictionary of [Text, Text];
        RunToken: Text;
        EntrySequenceName: Text;
}

codeunit 76043 "LT SQ Helper"
{
    // Run guarded by C6 (Codeunit.Run commits its own transaction).
    trigger OnRun()
    begin
        InsertRow('C6-RUN');
    end;

    // 'missing', or 'cur=<Current>'.
    procedure State(Name: Text): Text
    begin
        if NumberSequence.Exists(Name) then
            exit('cur=' + Format(NumberSequence.Current(Name)));
        exit('missing');
    end;

    // 'missing', or what Next returns.
    procedure TakeNext(Name: Text): Text
    begin
        if NumberSequence.Exists(Name) then
            exit(Format(NumberSequence.Next(Name)));
        exit('missing');
    end;

    // Logs '<Cell> state' and then '<Cell> next' of the sequence.
    procedure Read(Cell: Text; Name: Text)
    var
        Log: Codeunit "LT SQ Log";
    begin
        Log.Put(Cell + ' state', State(Name));
        Log.Put(Cell + ' next', TakeNext(Name));
    end;

    procedure InsertRow(RowKey: Code[30])
    var
        Row: Record "LT SQ Row";
    begin
        Row."Key" := RowKey;
        Row.Insert();
    end;

    // "K1=Yes K2=No" for the keys of a |-separated list, in that order.
    procedure Rows(Keys: Text): Text
    var
        Row: Record "LT SQ Row";
        RowKey: Text;
        Result: Text;
    begin
        foreach RowKey in Keys.Split('|') do begin
            if Result <> '' then
                Result += ' ';
            Result += RowKey + '=' + Format(Row.Get(RowKey));
        end;
        exit(Result);
    end;

    procedure RaiseIsolated()
    begin
        OnIsolated();
    end;

    [CommitBehavior(CommitBehavior::Ignore)]
    procedure IgnoredCommit()
    begin
        InsertRow('C12-IGNORED');
        Commit();
    end;

    // Isolated = true: BC runs each subscriber in a transaction of its own.
    [IntegrationEvent(false, false, true)]
    local procedure OnIsolated()
    begin
    end;
}

codeunit 76044 "LT SQ Isolated Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LT SQ Helper", 'OnIsolated', '', false, false)]
    local procedure WriteRow()
    var
        Helper: Codeunit "LT SQ Helper";
    begin
        Helper.InsertRow('C7-SUB');
    end;
}
