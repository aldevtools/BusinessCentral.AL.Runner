codeunit 50310 "NCap Tests"
{
    Subtype = Test;

    // Head keeps the variable (the gate takes the checker's resolution); main refused it.
    [Test]
    procedure LocalNamedCaption_IsTheVariable()
    var
        P: Page "NCap Text Card";
    begin
        if P.LocalNamedCaption() <> 'Loc' then
            Error('local: %1', P.LocalNamedCaption());
    end;

    [Test]
    procedure ParameterNamedCaption_IsTheVariable()
    var
        P: Page "NCap Text Card";
    begin
        if P.ParameterNamedCaption('A') <> 'AP' then
            Error('parameter: %1', P.ParameterNamedCaption('A'));
    end;

    // Not measured: stays refused on both.
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
