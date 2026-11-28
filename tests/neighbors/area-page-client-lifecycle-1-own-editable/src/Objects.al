// A page with its own Editable() without a result, beside the platform Page.Editable: which one `P.Editable()`
// binds is not measured (must-not-compile 3215 measures the name Caption).
page 50300 "NEdit Card"
{
    PageType = Card;
    Caption = 'Static';

    procedure Editable()
    begin
    end;
}

codeunit 50310 "NEdit Tests"
{
    Subtype = Test;

    [Test]
    procedure OwnEditable_ThroughPageVariable()
    var
        P: Page "NEdit Card";
        B: Boolean;
    begin
        B := P.Editable();
        if not B then
            Error('editable: %1', B);
    end;
}
