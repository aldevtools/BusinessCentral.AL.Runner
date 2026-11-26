// Issue 9046: each install hashes an InStream the way Cryptography Management Impl. GenerateHash(InStream, ...) does:
// MemoryStream(), CopyStream into it, HashAlgorithm.ComputeHash(MemoryStream.ToArray()); each writes its own row.
codeunit 53090 "FP MemStream Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP MemStream TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP MemStream TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP MemStream TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure RowOfDExists()
    var
        R: Record "FP MemStream TD";
    begin
        R.Get('D');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    begin
        if 1 + 1 <> 2 then
            Error('arithmetic');
    end;
}
