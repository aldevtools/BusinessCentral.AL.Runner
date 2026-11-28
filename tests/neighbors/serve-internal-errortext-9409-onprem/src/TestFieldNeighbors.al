// TestField with a collectible Internal ErrorInfo in a collecting scope (review round 2 of the runtime PR for al-runner issue
// 9409). TestField collects the ErrorInfo's own Message, so a held Internal Message has a second writer next to Error. Only
// a Message that Error collected for a measured shape, still unchanged, gets the bare raised text and the braced object
// Message at the end of the scope. GetLastErrorObject has scope OnPrem, so these tests live in this app.
dotnet
{
    assembly("mscorlib")
    {
        type("System.Exception"; NbEitTfExc) { }
    }
}

table 77421 "NB EIT TestField Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 77421 "NB EIT TestField"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure CrLf(): Text
    var
        T: Text[2];
    begin
        T[1] := 13;
        T[2] := 10;
        exit(T);
    end;

    local procedure Generic(): Text
    begin
        exit('An error has occurred. If the error persists, please contact your system administrator.');
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

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectTestField(EI: ErrorInfo)
    var
        Row: Record "NB EIT TestField Row";
    begin
        Row.TestField(Name, EI);
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

    // A Message the test wrote, shaped like a measured collected one (36 characters after the label): the object Message
    // is the raised text, as on main.
    [Test]
    procedure UserMessage_LooksMeasured_ObjectIsText()
    var
        EI: ErrorInfo;
        Exc: DotNet NbEitTfExc;
    begin
        EI.Message(Generic() + CrLf() + 'Correlation ID: ' + 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
        EI.ErrorType(ErrorType::Internal);
        EI.Collectible(true);
        asserterror CollectTestField(EI);
        Exc := GetLastErrorObject();
        if Exc.Message <> GetLastErrorText() then
            Error('object Message [%1], text [%2]', Exc.Message, GetLastErrorText());
    end;

    // An ErrorInfo that GetCollectedErrors returned, given to TestField in a second collecting scope: the scope end passes
    // its collected Message through (bare id), the object Message keeps the braced id.
    [Test]
    procedure CollectedErrorInfo_TextBare_ObjectBraced()
    var
        EI: ErrorInfo;
        Collected: ErrorInfo;
        Exc: DotNet NbEitTfExc;
    begin
        EI.Message('x');
        EI.ErrorType(ErrorType::Internal);
        EI.Collectible(true);
        CollectOne(EI, Collected);
        asserterror CollectTestField(Collected);
        if IdForm(GetLastErrorText()) <> 'bare' then
            Error('text id form %1', IdForm(GetLastErrorText()));
        Exc := GetLastErrorObject();
        if IdForm(Exc.Message) <> 'braced' then
            Error('object Message id form %1', IdForm(Exc.Message));
    end;

    // The same returned ErrorInfo, its Message set by the test to another one of the measured form before TestField:
    // main's answer.
    [Test]
    procedure CollectedErrorInfo_MessageSet_ObjectIsText()
    var
        EI: ErrorInfo;
        Collected: ErrorInfo;
        Exc: DotNet NbEitTfExc;
    begin
        EI.Message('x');
        EI.ErrorType(ErrorType::Internal);
        EI.Collectible(true);
        CollectOne(EI, Collected);
        Collected.Message(Generic() + CrLf() + 'Correlation ID: ' + 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb');
        asserterror CollectTestField(Collected);
        Exc := GetLastErrorObject();
        if Exc.Message <> GetLastErrorText() then
            Error('object Message [%1], text [%2]', Exc.Message, GetLastErrorText());
    end;
}
