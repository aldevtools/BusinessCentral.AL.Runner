// One codeunit per test: a test refused while it ran makes the later tests of its codeunit untrusted.

codeunit 50360 "N T01"
{
    Subtype = Test;

    // OnBeforeInsertRecord skips the suite that exists, no trigger assigns a field, the line skips itself.
    [Test]
    procedure SkipBefore_Existing_NoAssign_LinesSkipped()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50310, true);
        if R <> 'A/da/;' then
            Error('rows %1', R);
    end;
}

codeunit 50361 "N T02"
{
    Subtype = Test;

    // The same with a key attribute trigger that assigns a field (the shape of Suite_SameAppSameHash_Skips).
    [Test]
    procedure SkipBefore_Existing_Assign_LinesSkipped()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50311, true);
        if R <> 'A/da/;' then
            Error('rows %1', R);
    end;
}

codeunit 50362 "N T03"
{
    Subtype = Test;

    // Skip in the key attribute trigger and in OnBeforeInsertRecord: the measured shape.
    [Test]
    procedure SkipBoth_Existing_Assign_LinesSkipped()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50315, true);
        if R <> 'A/da/;' then
            Error('rows %1', R);
    end;
}

codeunit 50363 "N T04"
{
    Subtype = Test;

    // The line under the skipped suite is stored (not measured).
    [Test]
    procedure SkipBefore_Existing_NoAssign_LineStored()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50310, false);
        if R <> 'A/da/;LA/6;' then
            Error('rows %1', R);
    end;
}

codeunit 50364 "N T05"
{
    Subtype = Test;

    // BreakUnbound in OnBeforeInsertRecord of the suite that exists (not measured).
    [Test]
    procedure UnboundBefore_Existing()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50312, true);
        if R <> 'A/da/;' then
            Error('rows %1', R);
    end;
}

codeunit 50365 "N T06"
{
    Subtype = Test;

    // Skip in OnBeforeInsertRecord of a new suite: main's refusal.
    [Test]
    procedure SkipBefore_New()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        L.Run(50313, '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        if L.Rows() <> '' then
            Error('rows %1', L.Rows());
    end;
}

codeunit 50366 "N T07"
{
    Subtype = Test;

    // Skip in the key attribute trigger only: the first save stores the row (request dropped, P112XpXml.al:748-818).
    [Test]
    procedure AttrSkip_Existing_NoSkipBefore()
    var
        L: Codeunit "N Lib";
        R: Text;
    begin
        R := L.Again(50314, true);
        if R <> 'A/db/;' then
            Error('rows %1', R);
    end;
}
