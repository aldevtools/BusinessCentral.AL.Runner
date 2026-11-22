// Neighbors of a codeunit that comes after a carry-chain member (al-runner issues 8593 and 8594). The codeunits run in
// id order; each group has its own table and its own SingleInstance codeunit. A test passes when it sees what Business
// Central gives in one session (QuCommitIsolation.al:711-760 QU_D11_NextCodeunitSees), else it fails with what it saw.
// The groups whose members leave a carry that is not known or not followed come last: the walk hands that on.

table 62000 "LC Chain Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62001 "LC Outside Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62002 "LC Hub Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62003 "LC OnRun Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62004 "LC Storage Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62005 "LC Refused Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62006 "LC OnRun Refused Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
table 62007 "LC OnRun After Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }

codeunit 62090 "LC Flag"
{
    SingleInstance = true;
    // One global per group, so a group's readers depend only on its own writers.
    procedure Set(Group: Text; V: Text)
    begin
        case Group of
            'chain': ChainFlag := V;
            'outside': OutsideFlag := V;
            'hub': HubFlag := V;
            'onrun': OnRunFlag := V;
            'refused': RefusedFlag := V;
        end;
    end;
    procedure Get(Group: Text): Text
    begin
        case Group of
            'chain': exit(ChainFlag);
            'outside': exit(OutsideFlag);
            'hub': exit(HubFlag);
            'onrun': exit(OnRunFlag);
            'refused': exit(RefusedFlag);
        end;
    end;
    var
        ChainFlag: Text;
        OutsideFlag: Text;
        HubFlag: Text;
        OnRunFlag: Text;
        RefusedFlag: Text;
}

// A SingleInstance writer, then a member that reads it (a chain of 2), then one more (a chain of 3), then an unrelated
// codeunit.
codeunit 62010 "LC Chain Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure WritesFlagAndRow() var F: Codeunit "LC Flag"; R: Record "LC Chain Row";
    begin F.Set('chain', 'set'); R.Name := 'One'; R.Insert(); end;
}
codeunit 62011 "LC Chain Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SecondMemberReadsFlagAndRow() var F: Codeunit "LC Flag"; R: Record "LC Chain Row";
    begin
        if (R.Count() <> 1) or (F.Get('chain') <> 'set') then Error('rows=%1 flag=%2', R.Count(), F.Get('chain'));
        F.Set('chain', 'second');
    end;
}
codeunit 62012 "LC Chain Third"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure ThirdMemberReadsFlag() var F: Codeunit "LC Flag";
    begin if F.Get('chain') <> 'second' then Error('flag=%1', F.Get('chain')); end;
}
codeunit 62013 "LC Chain Unrelated"
{
    Subtype = Test;
    [Test] procedure UnrelatedAfterChain() var N: Integer;
    begin N := 2 + 3; if N <> 5 then Error('n=%1', N); end;
}

// Issue 8594 shape: a member writes rows; outside the chain a codeunit sets a flag, one reads the rows and the flag,
// one reads only the flag, and one before the member reads the rows.
codeunit 62020 "LC Outside Early Reader"
{
    Subtype = Test;
    [Test] procedure ReaderBeforeTheMember() var R: Record "LC Outside Row";
    begin if R.Count() <> 0 then Error('count=%1', R.Count()); end;
}
codeunit 62021 "LC Outside Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberWritesThreeRows() var R: Record "LC Outside Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); R.Name := 'Charlie'; R.Insert(); end;
}
codeunit 62022 "LC Outside Setter"
{
    Subtype = Test;
    [Test] procedure OutsideSetsFlag() var F: Codeunit "LC Flag"; begin F.Set('outside', 'left'); end;
}
codeunit 62023 "LC Outside Reader"
{
    Subtype = Test;
    [Test] procedure OutsideReadsRowsAndFlag() var F: Codeunit "LC Flag"; R: Record "LC Outside Row";
    begin
        if (R.Count() <> 3) or (F.Get('outside') <> 'left') then
            Error('count=%1 flag=%2', R.Count(), F.Get('outside'));
    end;
}
codeunit 62024 "LC Outside Flag Reader"
{
    Subtype = Test;
    [Test] procedure OutsideReadsOnlyFlag() var F: Codeunit "LC Flag";
    begin if F.Get('outside') <> 'left' then Error('flag=%1', F.Get('outside')); end;
}

// A reader with handlers outside the chain after a member on the hub (RequiredTestIsolation = Disabled), and after a
// member whose OnRun writes (the other leg); and one without handlers there.
codeunit 62030 "LC Hub Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure HubMemberWritesTwoRows() var R: Record "LC Hub Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
}
codeunit 62031 "LC Hub Setter"
{
    Subtype = Test;
    [Test] procedure HubSetsFlag() var F: Codeunit "LC Flag"; begin F.Set('hub', 'left'); end;
}
codeunit 62032 "LC Hub Handler Reader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('LcHubMessage')]
    procedure HandlerReaderAfterHubMember() var F: Codeunit "LC Flag"; R: Record "LC Hub Row";
    begin
        Message('counting');
        if (R.Count() <> 2) or (F.Get('hub') <> 'left') then Error('count=%1 flag=%2', R.Count(), F.Get('hub'));
    end;
    [MessageHandler] procedure LcHubMessage(Msg: Text[1024]) begin end;
}
codeunit 62040 "LC OnRun Writer"
{
    Subtype = Test;
    trigger OnRun() var R: Record "LC OnRun Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
    [Test] procedure OnRunMemberTest() var N: Integer; begin N := 1; if N <> 1 then Error('n=%1', N); end;
}
codeunit 62041 "LC OnRun Setter"
{
    Subtype = Test;
    [Test] procedure OnRunLegSetsFlag() var F: Codeunit "LC Flag"; begin F.Set('onrun', 'left'); end;
}
codeunit 62042 "LC OnRun Handler Reader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('LcOnRunMessage')]
    procedure HandlerReaderOnOtherLeg() var F: Codeunit "LC Flag"; R: Record "LC OnRun Row";
    begin
        Message('counting');
        if (R.Count() <> 2) or (F.Get('onrun') <> 'left') then Error('count=%1 flag=%2', R.Count(), F.Get('onrun'));
    end;
    [MessageHandler] procedure LcOnRunMessage(Msg: Text[1024]) begin end;
}
codeunit 62043 "LC OnRun Reader"
{
    Subtype = Test;
    [Test] procedure ReaderOnOtherLeg() var F: Codeunit "LC Flag"; R: Record "LC OnRun Row";
    begin
        if (R.Count() <> 2) or (F.Get('onrun') <> 'left') then Error('count=%1 flag=%2', R.Count(), F.Get('onrun'));
    end;
}

// A member whose OnRun changes isolated storage: a member and an outside codeunit whose reach touches the storage, and
// an outside codeunit that reads only the member's rows.
codeunit 62050 "LC Storage Member"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    trigger OnRun() begin IsolatedStorage.Set('lc-key', 'kept', DataScope::Module); end;
    [Test] procedure StorageMemberWritesRow() var R: Record "LC Storage Row"; begin R.Name := 'One'; R.Insert(); end;
}
codeunit 62051 "LC Storage Member Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberReadsStorage() var V: Text;
    begin
        if not IsolatedStorage.Get('lc-key', DataScope::Module, V) then V := '';
        if V <> 'kept' then Error('storage=%1', V);
    end;
}
codeunit 62052 "LC Storage Outside Rows"
{
    Subtype = Test;
    [Test] procedure OutsideReadsRowsNotStorage() var R: Record "LC Storage Row";
    begin if R.Count() <> 1 then Error('count=%1', R.Count()); end;
}
codeunit 62053 "LC Storage Outside Storage"
{
    Subtype = Test;
    [Test] procedure OutsideReadsStorage() var V: Text;
    begin
        if not IsolatedStorage.Get('lc-key', DataScope::Module, V) then V := '';
        if V <> 'kept' then Error('storage=%1', V);
    end;
}

// Issue 8593 shape: a member is refused while it runs (the order of æ is not measured, issue 8195); after it a member
// that reads its table, a member that reads only the flag, and an outside codeunit that reads its table.
codeunit 62060 "LC Refused Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure WritesNamesWithAe() var R: Record "LC Refused Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'ærø'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
}
codeunit 62061 "LC Refused Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberSetsFlag() var F: Codeunit "LC Flag"; begin F.Set('refused', 'left'); end;
}
codeunit 62062 "LC Refused Member"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure RefusedLastName() var F: Codeunit "LC Flag"; R: Record "LC Refused Row";
    begin
        if F.Get('refused') = '' then;
        R.FindLast();
        if R.Name <> 'ærø' then Error('last=%1', R.Name);
    end;
}
codeunit 62063 "LC Refused Counter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberReadsRefusedTable() var F: Codeunit "LC Flag"; R: Record "LC Refused Row";
    begin
        if (R.Count() <> 3) or (F.Get('refused') <> 'left') then
            Error('count=%1 flag=%2', R.Count(), F.Get('refused'));
    end;
}
codeunit 62064 "LC Refused Flag Only"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberReadsOnlyFlag() var F: Codeunit "LC Flag";
    begin if F.Get('refused') <> 'left' then Error('flag=%1', F.Get('refused')); end;
}
codeunit 62065 "LC Refused Outside Reader"
{
    Subtype = Test;
    [Test] procedure OutsideReadsRefusedTable() var R: Record "LC Refused Row";
    begin if R.Count() <> 3 then Error('count=%1', R.Count()); end;
}

// The same under runner 130450 without RequiredTestIsolation: members whose OnRun writes, one of them refused while it
// runs; after it a member that reads its table, a member that does not, and an outside codeunit that reads it.
codeunit 62070 "LC OnRun Refused Member"
{
    Subtype = Test;
    trigger OnRun() var R: Record "LC OnRun Refused Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'ærø'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
    [Test] procedure OnRunRefusedLastName() var R: Record "LC OnRun Refused Row";
    begin R.FindLast(); if R.Name <> 'ærø' then Error('last=%1', R.Name); end;
}
codeunit 62071 "LC OnRun After Reader"
{
    Subtype = Test;
    trigger OnRun() var R: Record "LC OnRun After Row"; begin R.Name := 'One'; R.Insert(); end;
    [Test] procedure OnRunMemberReadsRefusedTable() var R: Record "LC OnRun Refused Row";
    begin if R.Count() <> 3 then Error('count=%1', R.Count()); end;
}
codeunit 62072 "LC OnRun After Other"
{
    Subtype = Test;
    trigger OnRun() var R: Record "LC OnRun After Row"; begin R.Name := 'Two'; R.Insert(); end;
    [Test] procedure OnRunMemberReadsNothingCarried() var N: Integer;
    begin N := 4; if N <> 4 then Error('n=%1', N); end;
}
codeunit 62073 "LC OnRun Outside Reader"
{
    Subtype = Test;
    [Test] procedure OutsideReadsOnRunRefusedTable() var R: Record "LC OnRun Refused Row";
    begin if R.Count() <> 3 then Error('count=%1', R.Count()); end;
}
