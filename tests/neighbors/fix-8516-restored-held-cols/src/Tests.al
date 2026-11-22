// al-runner issue 8516 and the review of PR 8517: a store made from exported rows (the install baseline as a worker
// thread or the baseline cache reads it back) notes the held columns of its rows, as the writes that stored them noted
// them. One codeunit per shape, so the shapes run on different threads. Each test reports what it read, so a run that
// answers shows the answer.

// Baseline rows with no held column: ASCII text only. First, so the thread that holds the captured baseline (not one
// read back from its bytes) likely runs a shape whose answer does not rest on the notes.
codeunit 61720 "HN AsciiKey FindLast"
{
    Subtype = Test;

    [Test]
    procedure AsciiKeyFromBaseline_FindLast()
    var
        Row: Record "HN Ascii Key";
    begin
        Row.FindLast();
        if Row.Name <> 'Bravo' then
            Error('last=%1', Row.Name);
    end;
}

// The pinned shape: an order by a key column whose baseline rows hold Æ/æ.
codeunit 61723 "HN AeKey FindLast"
{
    Subtype = Test;

    [Test]
    procedure AeKeyFromBaseline_FindLast()
    var
        Row: Record "HN Ae Key";
    begin
        Row.FindLast();
        if Row.Name <> 'Bravo' then
            Error('last=%1', Row.Name);
    end;
}

// The other held kind: a text outside ASCII without Æ/æ in an ordered key column.
codeunit 61721 "HN AccentKey FindLast"
{
    Subtype = Test;

    [Test]
    procedure AccentKeyFromBaseline_FindLast()
    var
        Row: Record "HN Accent Key";
    begin
        Row.FindLast();
        if Row.Name <> 'Émile' then
            Error('last=%1', Row.Name);
    end;
}

// A read of the held table that neither filters on nor orders by a column: no comparison of the held value.
codeunit 61722 "HN AeKey Count"
{
    Subtype = Test;

    [Test]
    procedure AeKeyFromBaseline_CountUnfiltered()
    var
        Row: Record "HN Ae Key";
    begin
        if Row.Count() <> 3 then
            Error('count=%1', Row.Count());
    end;
}

// The held column is not the one the read orders by: the order is by an ASCII Code key.
codeunit 61724 "HN AeDescription ByKey"
{
    Subtype = Test;

    [Test]
    procedure AeInOtherColumnFromBaseline_FindLastByKey()
    var
        Row: Record "HN Ae Description";
    begin
        Row.FindLast();
        if Row.Description <> 'ærø' then
            Error('last=%1', Row.Description);
    end;
}

// The read orders by the held column, though it is not in the primary key.
codeunit 61725 "HN AeDescription Sorted"
{
    Subtype = Test;

    [Test]
    procedure AeInOtherColumnFromBaseline_FindLastByThatColumn()
    var
        Row: Record "HN Ae Description";
    begin
        Row.SetCurrentKey(Description);
        Row.FindLast();
        if Row.Description <> 'ærø' then
            Error('last=%1', Row.Description);
    end;
}

// A store made by writes in the test itself: it noted its rows before this change too.
codeunit 61726 "HN AeKey Written"
{
    Subtype = Test;

    [Test]
    procedure AeKeyWrittenInTest_FindLast()
    var
        Row: Record "HN Ascii Key";
    begin
        Row.Name := 'ærø';
        Row.Insert();
        Row.FindLast();
        if Row.Name <> 'Bravo' then
            Error('last=%1', Row.Name);
    end;
}

// A Code key holding the capital Æ.
codeunit 61727 "HN AeCode FindLast"
{
    Subtype = Test;

    [Test]
    procedure AeCodeKeyFromBaseline_FindLast()
    var
        Row: Record "HN Ae Code";
    begin
        Row.FindLast();
        if Row."No." <> 'BRAVO' then
            Error('last=%1', Row."No.");
    end;
}

// An @ filter on a column whose baseline rows hold an accent (no Æ/æ).
codeunit 61728 "HN AccentKey AtFilter"
{
    Subtype = Test;

    [Test]
    procedure AccentKeyFromBaseline_AtFilter()
    var
        Row: Record "HN Accent Key";
    begin
        Row.SetFilter(Name, '@emile');
        if Row.Count() <> 0 then
            Error('count=%1', Row.Count());
    end;
}

// An equality filter on the Æ/æ key column, to a text no held row ties with.
codeunit 61729 "HN AeKey Equality"
{
    Subtype = Test;

    [Test]
    procedure AeKeyFromBaseline_EqualityFilter()
    var
        Row: Record "HN Ae Key";
    begin
        Row.SetRange(Name, 'Bravo');
        if Row.Count() <> 1 then
            Error('count=%1', Row.Count());
    end;
}
