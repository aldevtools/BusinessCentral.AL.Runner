table 52870 "CTC Own Row"
{
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52871 "CTC Task Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallTookTheTaskBranch()
    var
        Row: Record "CTC Row A";
    begin
        Row.Get('BRANCH');
        if Row.Value <> 'task' then
            Error('branch: %1', Row.Value);
    end;

    [Test]
    procedure WriterRowIsNotThere()
    var
        Row: Record "CTC Row A";
    begin
        if Row.Get('WRITER') then
            Error('writer row: %1', Row.Value);
    end;

    [Test]
    procedure TaskExistsOfTheInstallTask()
    var
        Row: Record "CTC Row A";
    begin
        Row.Get('BRANCH');
        if TaskScheduler.TaskExists(Row.Task) then
            Error('the install task exists');
    end;

    [Test]
    procedure ScheduledTaskRowOfTheInstallTask()
    var
        Row: Record "CTC Row A";
        Task: Record "Scheduled Task";
    begin
        Row.Get('BRANCH');
        Task.SetRange(ID, Row.Task);
        if Task.IsEmpty() then
            Error('no Scheduled Task row');
    end;

    [Test]
    procedure TaskCreatedWithoutCheck()
    var
        Row: Record "CTC Row B";
    begin
        Row.Get('CREATED');
        if IsNullGuid(Row.Task) then
            Error('no task id');
    end;

    [Test]
    procedure RecordRefReadOfTheBranchRow()
    var
        Ref: RecordRef;
    begin
        Ref.Open(Database::"CTC Row A");
        if Ref.IsEmpty() then
            Error('no rows');
    end;

    [Test]
    procedure OwnTableIsClean()
    var
        Row: Record "CTC Own Row";
    begin
        Row.Code := 'X';
        Row.Insert();
        Row.Get('X');
    end;

    [Test]
    procedure WriterTableOfItsOwnIsEmpty()
    var
        Row: Record "CTC Writer Row";
    begin
        if not Row.IsEmpty() then
            Error('the fallback writer ran');
    end;

    [Test]
    procedure IsolatedStorageOfTheBranch()
    var
        Storage: Codeunit "CTC Storage A";
    begin
        if Storage.Branch() <> 'task' then
            Error('isolated storage: %1', Storage.Branch());
    end;

    [Test]
    procedure MirrorInALaterInstall()
    var
        Mirror: Record "CTC Mirror C";
    begin
        Mirror.Get('MIRROR');
        if Mirror.Value <> 'task' then
            Error('mirror: %1', Mirror.Value);
    end;
}
