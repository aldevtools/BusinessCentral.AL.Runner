// Neighbors of al-runner issue 8969: a CurrentTransactionType read while the type a test method before left is not known,
// whose value a local saves and one set back after it uses. Each shape is a test codeunit of its own: a first test that
// is refused while it runs and could set the type, then the shape.
table 50000 "NB TV Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

report 50001 "NB TV Update"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    TransactionType = Update;

    dataset
    {
        dataitem(Loop; "NB TV Row") { }
    }
}

codeunit 50002 "NB TV Lib"
{
    var
        GlobalSaved: TransactionType;

    procedure SaveSetRestore()
    var
        Saved: TransactionType;
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        UpdateReport.UseRequestPage := false;
        UpdateReport.RunModal();
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveCompared()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if Saved = TransactionType::Update then;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveToGlobal()
    begin
        Commit();
        GlobalSaved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(GlobalSaved);
    end;

    procedure SaveReassigned()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Saved := TransactionType::Update;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SavePassedOn()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        SetBack(Saved);
    end;

    procedure SaveInIf()
    var
        Saved: TransactionType;
    begin
        Commit();
        if true then
            Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoredTwice()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveNoSetBack()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
    end;

    procedure SaveRestoreAfterWrite()
    var
        Saved: TransactionType;
        Row: Record "NB TV Row";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 7;
        Row.Insert();
        CurrentTransactionType(Saved);
    end;

    procedure SaveParam(Saved: TransactionType)
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveReturned() Saved: TransactionType
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveAfterReadNoCommit()
    var
        Saved: TransactionType;
        Row: Record "NB TV Row";
    begin
        if Row.FindFirst() then;
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveNoSetBetween()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveNoCommit()
    var
        Saved: TransactionType;
    begin
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoreAfterRead()
    var
        Saved: TransactionType;
        Row: Record "NB TV Row";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if Row.FindFirst() then;
        CurrentTransactionType(Saved);
    end;

    procedure SaveErrorRestore(Fail: Boolean)
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if Fail then
            Error('Before the set back.');
        Commit();
        CurrentTransactionType(Saved);
    end;

    local procedure SetBack(Saved: TransactionType)
    begin
        CurrentTransactionType(Saved);
    end;
}

codeunit 50003 "NB TV Run Restore"
{
    trigger OnRun()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;
}
codeunit 50010 "NB TV Saved Restored"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveSetRestore_Runs()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveSetRestore();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50011 "NB TV Saved Restored Known"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure B_SaveSetRestoreTypeKnown_Runs()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveSetRestore();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50012 "NB TV Saved Restored Then Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_ReadAfterSetBack_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveSetRestore();
        if CurrentTransactionType = TransactionType::Update then;
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50013 "NB TV Saved Restored Then Fail"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SetBackThenError_Fails()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveSetRestore();
        Error('After the set back.');
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50014 "NB TV Saved Compared"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedCompared_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveCompared();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50015 "NB TV Saved To Global"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedToGlobal_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveToGlobal();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50016 "NB TV Saved Reassigned"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedReassigned_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveReassigned();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50017 "NB TV Saved Passed On"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedPassedOn_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SavePassedOn();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50018 "NB TV Saved In If"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedInIf_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveInIf();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50019 "NB TV Saved Restored Twice"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedRestoredTwice_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveRestoredTwice();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50020 "NB TV Saved No Set Back"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedNoSetBack_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveNoSetBack();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50021 "NB TV Saved Param"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedToParameter_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveParam(TransactionType::Update);
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50022 "NB TV Saved Returned"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SavedToReturnValue_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveReturned();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50023 "NB TV Restore After Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SetBackAfterWrite_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveRestoreAfterWrite();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50024 "NB TV Codeunit Run Restore"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveSetRestoreInCodeunitRun()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Codeunit.Run(Codeunit::"NB TV Run Restore");
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50025 "NB TV Saved In Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveSetRestoreInTestMethod()
    var
        Row: Record "NB TV Row";
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50026 "NB TV Save After Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveAfterReadNoCommit_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveAfterReadNoCommit();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50027 "NB TV Save No Set Between"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveNoSetBetween()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveNoSetBetween();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50028 "NB TV Save No Commit"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveSetBackBeforeAnyCall()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveNoCommit();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50029 "NB TV Restore After Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SetBackAfterRead_Refused()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveRestoreAfterRead();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50030 "NB TV Save Error Then Again"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_ErrorBeforeSetBackThenAgain()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        asserterror Lib.SaveErrorRestore(true);
        Lib.SaveErrorRestore(false);
        Row."Entry No." := 1;
        Row.Insert();
    end;
}

codeunit 50031 "NB TV Restored Called Twice"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV Row";
        UpdateReport: Report "NB TV Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveSetRestoreCalledTwice()
    var
        Lib: Codeunit "NB TV Lib";
        Row: Record "NB TV Row";
    begin
        Lib.SaveSetRestore();
        Lib.SaveSetRestore();
        Row."Entry No." := 1;
        Row.Insert();
    end;
}
