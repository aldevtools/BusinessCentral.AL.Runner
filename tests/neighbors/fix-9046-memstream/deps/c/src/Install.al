dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream MS C") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Hash C") { }
        type("System.Array"; "FP MemStream Arr C") { }
    }
}

table 53020 "FP MemStream TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53021 "FP MemStream C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream TC" temporary;
        Row: Record "FP MemStream TC";
        MemoryStream: DotNet "FP MemStream MS C";
        HashAlgorithm: DotNet "FP MemStream Hash C";
        HashBytes: DotNet "FP MemStream Arr C";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('C');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream();
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'C';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
