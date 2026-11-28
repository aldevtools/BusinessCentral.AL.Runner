// Neighbors of the Internal ErrorInfo text in collecting scopes (al-runner issue 9409, review round 1 of the runtime PR).
// The probe (corpus PR 1666) measured one collecting scope that held no error before the Internal one, and an ErrorInfo
// that was never collected. Each test changes one of those facts; every such shape keeps main's braced id.
codeunit 77412 "NB EIT Scope Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Internal(Collectible: Boolean): ErrorInfo
    var
        EI: ErrorInfo;
    begin
        EI.Message('scope internal');
        EI.ErrorType(ErrorType::Internal);
        EI.Collectible(Collectible);
        exit(EI);
    end;

    local procedure Client(): ErrorInfo
    var
        EI: ErrorInfo;
    begin
        EI.Message('scope client');
        EI.Collectible(true);
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

    local procedure CheckText(What: Text; T: Text; Expected: Text)
    begin
        if IdForm(T) <> Expected then
            Error('%1: id form %2, expected %3', What, IdForm(T), Expected);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectOne(EI: ErrorInfo; var Collected: ErrorInfo)
    var
        Errors: List of [ErrorInfo];
    begin
        Error(EI);
        Errors := GetCollectedErrors(true);
        Errors.Get(1, Collected);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Outer(EI: ErrorInfo; var InnerMessage: Text)
    begin
        Inner(EI, InnerMessage);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Inner(EI: ErrorInfo; var InnerMessage: Text)
    var
        Errors: List of [ErrorInfo];
        Collected: ErrorInfo;
    begin
        Error(EI);
        Errors := GetCollectedErrors(false);
        Errors.Get(Errors.Count(), Collected);
        InnerMessage := Collected.Message;
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectSecond(EI: ErrorInfo; var SecondMessage: Text)
    var
        Errors: List of [ErrorInfo];
        Collected: ErrorInfo;
    begin
        Error(Client());
        Error(EI);
        Errors := GetCollectedErrors(true);
        Errors.Get(2, Collected);
        SecondMessage := Collected.Message;
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RaiseAfterCollected(EI: ErrorInfo)
    begin
        Error(Client());
        Error(EI);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RaiseInEmptyScope(EI: ErrorInfo)
    begin
        Error(EI);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectAgain(EI: ErrorInfo; var AgainMessage: Text)
    var
        Errors: List of [ErrorInfo];
        Collected: ErrorInfo;
    begin
        Error(EI);
        Errors := GetCollectedErrors(true);
        Errors.Get(1, Collected);
        AgainMessage := Collected.Message;
    end;

    // Nesting depth 2: the collected Message.
    [Test]
    procedure Nested_CollectedMessage_Braced()
    var
        InnerMessage: Text;
    begin
        asserterror Outer(Internal(true), InnerMessage);
        CheckText('collected message', InnerMessage, 'braced');
    end;

    // Nesting depth 2: the text when the outer procedure ends.
    [Test]
    procedure Nested_LeaveText_Braced()
    var
        InnerMessage: Text;
    begin
        asserterror Outer(Internal(true), InnerMessage);
        CheckText('leave text', GetLastErrorText(), 'braced');
    end;

    // One scope, but an error is held already: the Message of the second collected error.
    [Test]
    procedure SecondCollected_Message_Braced()
    var
        SecondMessage: Text;
    begin
        CollectSecond(Internal(true), SecondMessage);
        CheckText('second collected message', SecondMessage, 'braced');
    end;

    // A collected Internal ErrorInfo raised again with Collectible(false).
    [Test]
    procedure Reraise_CollectedNotCollectible_Braced()
    var
        Collected: ErrorInfo;
    begin
        CollectOne(Internal(true), Collected);
        Collected.Collectible(false);
        asserterror Error(Collected);
        CheckText('re-raised text', GetLastErrorText(), 'braced');
    end;

    // A collected Internal ErrorInfo collected again in another collecting scope.
    [Test]
    procedure Reraise_CollectedInScope_Braced()
    var
        Collected: ErrorInfo;
        AgainMessage: Text;
    begin
        CollectOne(Internal(true), Collected);
        CollectAgain(Collected, AgainMessage);
        CheckText('collected again message', AgainMessage, 'braced');
    end;

    // A non-collectible Internal raised in a scope that holds a collected error.
    [Test]
    procedure NonCollectible_AfterCollected_Braced()
    begin
        asserterror RaiseAfterCollected(Internal(false));
        CheckText('raised text', GetLastErrorText(), 'braced');
    end;

    // A non-collectible Internal raised in a scope that holds no error: the measured raise text, passed through.
    [Test]
    procedure NonCollectible_EmptyScope_Bare()
    begin
        asserterror RaiseInEmptyScope(Internal(false));
        CheckText('raised text', GetLastErrorText(), 'bare');
    end;

    // Properties set explicitly to the value a fresh ErrorInfo has: the state is the measured one.
    [Test]
    procedure ExplicitDefaults_Bare()
    var
        EI: ErrorInfo;
    begin
        EI := Internal(false);
        EI.Title('');
        EI.Verbosity(Verbosity::Error);
        EI.DataClassification(DataClassification::CustomerContent);
        EI.ControlName('');
        asserterror Error(EI);
        CheckText('raised text', GetLastErrorText(), 'bare');
    end;
}
