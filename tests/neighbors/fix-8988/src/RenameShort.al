// Rename to a key that does not convert to a related field (issues 8988, 9178), in the shapes of corpus PR 1610
// (plain TableRelation, Integer key first, related field second) and one fact away from them.
table 50210 "N8988 M Pend" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50211 "N8988 C Pend"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Text[4]) { TableRelation = "N8988 M Pend"; ObsoleteState = Pending; ObsoleteReason = 'Short.'; }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50212 "N8988 M Live" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50213 "N8988 C Live"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Ref; Text[4]) { TableRelation = "N8988 M Live"; } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50214 "N8988 M Code4" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50215 "N8988 C Code4"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Ref; Code[4]) { TableRelation = "N8988 M Code4"; } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50216 "N8988 M Trig"
{
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnRename() begin end;
}
table 50217 "N8988 C Trig"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Ref; Text[4]) { TableRelation = "N8988 M Trig"; } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50218 "N8988 M Where" { fields { field(1; "Code"; Code[20]) { } field(2; Kind; Integer) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50219 "N8988 C Where"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Text[4]) { TableRelation = "N8988 M Where" where(Kind = const(1)); ObsoleteState = Removed; ObsoleteReason = 'Short.'; }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50220 "N8988 M Int" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50221 "N8988 C Int"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Ref; Integer) { TableRelation = "N8988 M Int"; } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 50211 "N8988 Short Tests"
{
    Subtype = Test;

    [Test]
    procedure PendingPlain_RaisesMeasuredError()
    var
        M: Record "N8988 M Pend";
    begin
        M.Code := 'AB';
        M.Insert();
        asserterror M.Rename('ABCDEFGHIJ');
        if GetLastErrorText() <> 'The record could not be renamed. A type conversion error occurred while updating reference field N8988 C Pend.Ref. The record has data type Code and the referencing field has data type Text.' then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> 'InvalidType:StringLengthExceeded' then
            Error('code: %1', GetLastErrorCode());
    end;

    [Test]
    procedure LivePlain_RecordRef_RaisesMeasuredError()
    var
        M: Record "N8988 M Live";
        R: RecordRef;
    begin
        M.Code := 'AB';
        M.Insert();
        R.Open(Database::"N8988 M Live");
        R.FindFirst();
        asserterror R.Rename('ABCDEFGHIJ');
        if GetLastErrorCode() <> 'InvalidType:StringLengthExceeded' then
            Error('code: %1', GetLastErrorCode());
    end;

    [Test]
    procedure LivePlain_NineCharKey_Refused()
    var
        M: Record "N8988 M Live";
    begin
        M.Code := 'AB';
        M.Insert();
        asserterror M.Rename('ABCDEFGHI');
    end;

    [Test]
    procedure LiveCode4_Refused()
    var
        M: Record "N8988 M Code4";
    begin
        M.Code := 'AB';
        M.Insert();
        asserterror M.Rename('ABCDEFGHIJ');
    end;

    [Test]
    procedure LiveWithOnRename_Refused()
    var
        M: Record "N8988 M Trig";
    begin
        M.Code := 'AB';
        M.Insert();
        asserterror M.Rename('ABCDEFGHIJ');
    end;

    [Test]
    procedure RemovedWithWhere_Refused()
    var
        M: Record "N8988 M Where";
    begin
        M.Code := 'AB';
        M.Kind := 1;
        M.Insert();
        M.Rename('ABCDEFGHIJ');
    end;

    [Test]
    procedure LiveInteger_Refused()
    var
        M: Record "N8988 M Int";
    begin
        M.Code := 'AB';
        M.Insert();
        asserterror M.Rename('ABCDEFGHIJ');
    end;

    [Test]
    procedure RemovedReturnValue_Refused()
    var
        M: Record "N8988 Parent";
    begin
        M.Code := 'AB';
        M.Insert();
        if not M.Rename('GU00000039') then
            Error('rename failed');
    end;

    [Test]
    procedure LivePlain_OldKeyLongerThanField_Refused()
    var
        M: Record "N8988 M Live";
    begin
        M.Code := 'ABCDEFG';
        M.Insert();
        asserterror M.Rename('ABCDEFGHIJ');
    end;

    [Test]
    procedure RemovedText4_OldKeyLongerThanField_Refused()
    var
        M: Record "N8988 Parent";
    begin
        M.Code := 'ABCDEFG';
        M.Insert();
        M.Rename('GU00000040');
        M.Get('GU00000040');
    end;

    [Test]
    procedure RemovedInteger_NumericOldKey_Refused()
    var
        M: Record "N8988 Parent2";
    begin
        M.Code := '12';
        M.Insert();
        M.Rename('GU00000041');
        M.Get('GU00000041');
    end;
}
