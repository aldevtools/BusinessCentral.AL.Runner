dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream MS B") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Hash B") { }
        type("System.Array"; "FP MemStream Arr B") { }
    }
}

table 53010 "FP MemStream TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53011 "FP MemStream B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream TB" temporary;
        Row: Record "FP MemStream TB";
        MemoryStream: DotNet "FP MemStream MS B";
        HashAlgorithm: DotNet "FP MemStream Hash B";
        HashBytes: DotNet "FP MemStream Arr B";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('B');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream();
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'B';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
