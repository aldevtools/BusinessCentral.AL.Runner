// GetLastErrorObject after an Internal ErrorInfo raise (al-runner issue 9409, review round 1 of the runtime PR). The probe
// (corpus PR 1666) did not read GetLastErrorObject; its Message keeps main's braced id, also where GetLastErrorText answers
// the measured bare id. GetLastErrorObject has scope OnPrem, so this is an app of its own.
dotnet
{
    assembly("mscorlib")
    {
        type("System.Exception"; NbEitExc) { }
        type("System.Type"; NbEitType) { }
    }
}

codeunit 77420 "NB EIT Last Error Object"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Internal(): ErrorInfo
    var
        EI: ErrorInfo;
    begin
        EI.Message('object internal');
        EI.ErrorType(ErrorType::Internal);
        exit(EI);
    end;

    local procedure IdForm(T: Text): Text
    var
        P: Integer;
        Id: Text;
    begin
        P := StrPos(T, 'Correlation ID: ');
        if P = 0 then
            exit('no id: ' + T);
        Id := CopyStr(T, P + 16);
        if CopyStr(Id, 1, 1) = '{' then
            exit('braced');
        if (StrLen(Id) = 36) and (StrPos(Id, '{') = 0) and (LowerCase(Id) = Id) then
            exit('bare');
        exit('other');
    end;

    local procedure CheckObject(Expected: Text)
    var
        Exc: DotNet NbEitExc;
        TypeOfExc: DotNet NbEitType;
    begin
        Exc := GetLastErrorObject();
        TypeOfExc := Exc.GetType();
        if TypeOfExc.FullName <> 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLDialogException' then
            Error('type %1', TypeOfExc.FullName);
        if IdForm(Exc.Message) <> Expected then
            Error('Message id form %1, expected %2', IdForm(Exc.Message), Expected);
        if IdForm(Exc.GetBaseException().Message) <> Expected then
            Error('base Message id form %1, expected %2', IdForm(Exc.GetBaseException().Message), Expected);
    end;

    // The measured shape: GetLastErrorText answers bare, GetLastErrorObject keeps the braced id.
    [Test]
    procedure Measured_ObjectMessage_Braced()
    begin
        asserterror Error(Internal());
        if IdForm(GetLastErrorText()) <> 'bare' then
            Error('text id form %1', IdForm(GetLastErrorText()));
        CheckObject('braced');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectAndLeave(EI: ErrorInfo)
    begin
        Error(EI);
    end;

    // The measured end of a collecting procedure that holds one collectible Internal error: GetLastErrorText answers bare,
    // GetLastErrorObject keeps the braced id.
    [Test]
    procedure CollectedLeave_ObjectMessage_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.Collectible(true);
        asserterror CollectAndLeave(EI);
        if IdForm(GetLastErrorText()) <> 'bare' then
            Error('text id form %1', IdForm(GetLastErrorText()));
        CheckObject('braced');
    end;

    // An unmeasured shape (a Title): braced everywhere, as on main.
    [Test]
    procedure Title_ObjectMessage_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.Title('a title');
        asserterror Error(EI);
        CheckObject('braced');
    end;
}
