// Neighbors of al-runner issue 8696, the NP Retail shape: a partner dependency app's login subscriber changes, through an
// event, a SingleInstance codeunit of an app it does not depend on, and a test reads it. No probe measures this shape.
codeunit 60980 "XS Login Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsMicrosoftLibraryMemo()
    var
        M: Codeunit "XS Library Memo";
    begin
        if M.Get() <> 1 then
            Error('library memo: %1', M.Get());
    end;

    [Test]
    procedure ReadsOtherPublisherMemo()
    var
        M: Codeunit "XS Other Memo";
    begin
        if M.Get() <> 1 then
            Error('other memo: %1', M.Get());
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}
