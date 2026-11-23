// al-runner issue 8696, the reporter's repro, unchanged: a SingleInstance global read into a local, then overwritten.
codeunit 63090 "TC State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 63010 "TC First"
{
    Subtype = Test;
    [Test] procedure FirstSetsSet() var S: Codeunit "TC State"; begin S.SetFlag('set'); end;
}
codeunit 63011 "TC Second"
{
    Subtype = Test;
    [Test] procedure SecondSeesSetAndSetsSecond() var S: Codeunit "TC State"; Seen: Text;
    begin
        Seen := S.GetFlag();
        S.SetFlag('second');
        if Seen <> 'set' then Error('second saw flag=%1', Seen);
    end;
}
codeunit 63012 "TC Third"
{
    Subtype = Test;
    [Test] procedure ThirdSeesSecond() var S: Codeunit "TC State";
    begin if S.GetFlag() <> 'second' then Error('third saw flag=%1', S.GetFlag()); end;
}
