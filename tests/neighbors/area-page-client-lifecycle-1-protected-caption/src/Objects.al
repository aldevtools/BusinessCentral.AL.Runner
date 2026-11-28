// A page whose own Caption() without a result is protected: a codeunit cannot call it, so which method
// `P.Caption()` binds there is not measured (must-not-compile 3215 has a procedure without an access modifier).
page 50300 "NProt Card"
{
    PageType = Card;
    Caption = 'Static';

    protected procedure Caption()
    begin
    end;
}

codeunit 50310 "NProt Tests"
{
    Subtype = Test;

    [Test]
    procedure ProtectedCaption_ThroughPageVariable()
    var
        P: Page "NProt Card";
        S: Text;
    begin
        S := P.Caption();
        if S <> 'Static' then
            Error('caption: %1', S);
    end;

    // Passes when the app compiles: a wrong compile error would stop it too.
    [Test]
    procedure OtherTestOfTheApp_Runs()
    begin
    end;
}
