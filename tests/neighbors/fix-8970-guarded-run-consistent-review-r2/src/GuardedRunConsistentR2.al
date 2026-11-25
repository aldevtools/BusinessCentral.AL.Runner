// Review round 2 of the guarded Codeunit.Run end-of-run check (al-runner issue 8970): a failed run nested in another
// guarded run, where the [CommitBehavior(CommitBehavior::Error)] scope opens above the outer run, not inside it.
// The measured cell opens the scope in a procedure the outer run calls (GRC Outer.PreviewStart).
// The two ScopeAbove tests are refused on main and on head, so they have no row in differences.tsv.

table 61440 "GS Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

// Marks the row table and fails: the run nested in the outer one.
codeunit 61441 "GS Fail Marks"
{
    trigger OnRun()
    var
        Row: Record "GS Row";
    begin
        Row.Consistent(false);
        Error('nested failed');
    end;
}

// The outer run, nested run directly in OnRun (no scope of its own), then an empty Error.
codeunit 61442 "GS Outer Direct"
{
    trigger OnRun()
    begin
        if Codeunit.Run(Codeunit::"GS Fail Marks") then;
        Error('');
    end;
}

// The outer run with the scope opened inside it (the measured cell).
codeunit 61443 "GS Outer Scoped"
{
    trigger OnRun()
    begin
        Nested();
        Error('');
    end;

    [CommitBehavior(CommitBehavior::Error)]
    local procedure Nested()
    begin
        if Codeunit.Run(Codeunit::"GS Fail Marks") then;
    end;
}

// An unguarded run between the scope and the outer guarded run.
codeunit 61444 "GS Wrap"
{
    trigger OnRun()
    var
        Ok: Boolean;
    begin
        Ok := Codeunit.Run(Codeunit::"GS Outer Direct");
    end;
}

codeunit 61445 "GS Tests"
{
    Subtype = Test;

    [Test]
    procedure ScopeInsideOuter_OuterFails()
    var
        Ok: Boolean;
    begin
        Commit();
        Ok := Codeunit.Run(Codeunit::"GS Outer Scoped");
        if Ok then
            Error('the run returned true');
        Commit();
    end;

    [Test]
    [CommitBehavior(CommitBehavior::Error)]
    procedure ScopeAboveOuter_OuterFails()
    var
        Ok: Boolean;
    begin
        Ok := Codeunit.Run(Codeunit::"GS Outer Direct");
        if Ok then
            Error('the run returned true');
    end;

    [Test]
    [CommitBehavior(CommitBehavior::Error)]
    procedure ScopeAboveUnguardedRun_OuterFails()
    begin
        Codeunit.Run(Codeunit::"GS Wrap");
    end;
}
