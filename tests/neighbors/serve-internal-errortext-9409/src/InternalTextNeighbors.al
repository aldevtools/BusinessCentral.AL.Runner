// Neighbors of the Internal ErrorInfo text (al-runner issue 9409, corpus PR 1666). Each test changes one fact of the
// shape the probe measured. "Braced" means the id after "Correlation ID: " keeps main's braces; "bare" means the
// measured form (36 characters, lower case, no braces).
table 77411 "NB EIT Row"
{
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

codeunit 77410 "NB EIT Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Internal(): ErrorInfo
    var
        EI: ErrorInfo;
    begin
        EI.Message('neighbor internal');
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

    local procedure Check(Expected: Text)
    begin
        if IdForm(GetLastErrorText()) <> Expected then
            Error('id form %1, expected %2', IdForm(GetLastErrorText()), Expected);
    end;

    [Test]
    procedure Measured_Reference_Bare()
    begin
        asserterror Error(Internal());
        Check('bare');
    end;

    [Test]
    procedure Title_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.Title('a title');
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure CustomDimensions_Braced()
    var
        EI: ErrorInfo;
        D: Dictionary of [Text, Text];
    begin
        EI := Internal();
        D.Add('k', 'v');
        EI.CustomDimensions(D);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure CustomDimensionsRead_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        if EI.CustomDimensions().Count() <> 0 then
            Error('dimensions');
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure NavigationAction_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.AddNavigationAction('go');
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure Action_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.AddAction('fix', Codeunit::"NB EIT Neighbors", 'SomeMethod');
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure Verbosity_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.Verbosity(Verbosity::Warning);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure DataClassification_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.DataClassification(DataClassification::SystemMetadata);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure TableId_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.TableId(Database::"NB EIT Row");
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure FieldNo_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.FieldNo(2);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure PageNo_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.PageNo(22);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure ControlName_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.ControlName('Name');
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure RecordId_Braced()
    var
        EI: ErrorInfo;
        R: Record "NB EIT Row";
    begin
        R.Id := 1;
        R.Insert();
        EI := Internal();
        EI.RecordId(R.RecordId());
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure SystemId_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.SystemId(CreateGuid());
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure CollectibleOutsideScope_Braced()
    var
        EI: ErrorInfo;
    begin
        EI := Internal();
        EI.Collectible(true);
        asserterror Error(EI);
        Check('braced');
    end;

    [Test]
    procedure TestFieldErrorInfo_MainAnswer()
    var
        R: Record "NB EIT Row";
    begin
        R.Id := 2;
        asserterror R.TestField(Name, Internal());
        Check(IdForm(GetLastErrorText()));
    end;

    [Test]
    procedure FieldErrorErrorInfo_MainAnswer()
    var
        R: Record "NB EIT Row";
    begin
        R.Id := 3;
        asserterror R.FieldError(Name, Internal());
        Check(IdForm(GetLastErrorText()));
    end;

    [Test]
    procedure ClientErrorInfo_NoId()
    var
        EI: ErrorInfo;
    begin
        EI.Message('client');
        asserterror Error(EI);
        Check('no id: client');
    end;

    [Test]
    procedure Uncaught_FailureText()
    begin
        Error(Internal());
    end;
}
