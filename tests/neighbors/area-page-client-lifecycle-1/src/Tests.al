codeunit 50310 "NCap Tests"
{
    Subtype = Test;

    // A local or parameter called Caption beside an own Caption procedure: not measured, refused on both
    // (al-runner issue 9505 keeps the probe cells).
    [Test]
    procedure LocalNamedCaption_Refused()
    var
        P: Page "NCap Text Card";
    begin
        if P.LocalNamedCaption() <> 'Loc' then
            Error('local: %1', P.LocalNamedCaption());
    end;

    [Test]
    procedure ParameterNamedCaption_Refused()
    var
        P: Page "NCap Text Card";
    begin
        if P.ParameterNamedCaption('A') <> 'AP' then
            Error('parameter: %1', P.ParameterNamedCaption('A'));
    end;

    [Test]
    procedure ExtensionLocalNamedCaption_Refused()
    var
        P: Page "NCap Text Card";
    begin
        if P.ExtLocalNamedCaption() <> 'Loc' then
            Error('extension local: %1', P.ExtLocalNamedCaption());
    end;

    [Test]
    procedure LocalNamedCaption_BesideFieldCaption()
    var
        P: Page "NCap Field Card";
    begin
        if P.LocalBesideField() <> 'Loc|' then
            Error('local beside field: %1', P.LocalBesideField());
    end;

    [Test]
    procedure LocalNamedCaption_BesideOwnInteger()
    var
        P: Page "NCap Integer Card";
    begin
        if P.LocalNamedCaption() <> 'Loc' then
            Error('beside integer: %1', P.LocalNamedCaption());
    end;

    [Test]
    procedure LocalNamedCaption_BesideOwnNoResult()
    var
        P: Page "NCap NoResult Card";
    begin
        if P.LocalNamedCaption() <> 'Loc' then
            Error('beside no result: %1', P.LocalNamedCaption());
    end;

    [Test]
    procedure LocalNamedCaption_BesideOwnTextParameter()
    var
        P: Page "NCap TextParam Card";
    begin
        if P.LocalNamedCaption() <> 'Loc' then
            Error('beside text parameter: %1', P.LocalNamedCaption());
    end;

    [Test]
    procedure ReturnNamedCaption_Refused()
    var
        P: Page "NCap Text Card";
    begin
        if P.ReturnNamedCaption() <> 'Ret' then
            Error('return: %1', P.ReturnNamedCaption());
    end;

    [Test]
    procedure FieldNamedCaption_Refused()
    var
        P: Page "NCap Field Card";
    begin
        if P.AssignField() <> 'Fld' then
            Error('field: %1', P.AssignField());
    end;

    // The bare call of the own parameterless procedure into a Text compiles and runs (main's answer).
    [Test]
    procedure OwnTextCall_IntoText()
    var
        P: Page "NCap Text Card";
    begin
        if P.OwnText() <> 'Own' then
            Error('own text: %1', P.OwnText());
    end;

    // Compile neighbors of must-not-compile 3211, 3212, 3214 and 3215 that alc accepts.
    [Test]
    procedure OwnIntegerCall_IntoInteger()
    var
        P: Page "NCap Integer Card";
    begin
        if P.OwnInteger() <> 5 then
            Error('own integer: %1', P.OwnInteger());
    end;

    [Test]
    procedure OtherIntegerCall_IntoInteger()
    var
        P: Page "NCap Integer Card";
    begin
        if P.OtherIntegerIntoInteger() <> 5 then
            Error('other integer: %1', P.OtherIntegerIntoInteger());
    end;

    [Test]
    procedure PageVariableTextCall_IntoText()
    var
        P: Page "NCap Integer Card";
        S: Text;
    begin
        S := P.Name();
        if S <> 'N' then
            Error('name: %1', S);
    end;

    [Test]
    procedure PageVariableTryFunction_IntoBoolean()
    var
        P: Page "NCap Integer Card";
        B: Boolean;
    begin
        B := P.TryIt();
        if not B then
            Error('try returned false');
    end;

    [Test]
    procedure PageVariableNoResult_AsStatement()
    var
        P: Page "NCap Integer Card";
    begin
        P.NoResult();
    end;

    [Test]
    procedure CurrPageCaptionIntoText_OnOpen()
    var
        T: TestPage "NCap Integer Card";
    begin
        T.OpenView();
        if T.Caption() <> 'Static' then
            Error('caption: %1', T.Caption());
        T.Close();
    end;
}
