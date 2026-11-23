// Al-runner issues 8695 and 8696: three codeunits in one session, each reads what the one before it left
// (codeunits/TestCuP7BReader.al:24-37 against TestCuP7ASetter.al:24-58). Ids 50110-50116.
codeunit 50110 "N70 SI First"
{
    Subtype = Test;
    [Test] procedure SiFirstSetsSet() var S: Codeunit "N70 State"; begin S.SetFlag('set'); end;
}

// 8695: reads what First left, then writes its own value.
codeunit 50111 "N70 SI Second"
{
    Subtype = Test;
    [Test] procedure SiSecondReadsThenWrites() var S: Codeunit "N70 State";
    begin
        if S.GetFlag() <> 'set' then Error('second saw flag=%1', S.GetFlag());
        S.SetFlag('second');
    end;
}

codeunit 50112 "N70 SI Third"
{
    Subtype = Test;
    [Test] procedure SiThirdSeesSecond() var S: Codeunit "N70 State";
    begin if S.GetFlag() <> 'second' then Error('third saw flag=%1', S.GetFlag()); end;
}

// 8696: keeps the value in a local, overwrites the flag, then decides on the local.
codeunit 50113 "N70 SI Local Overwrite"
{
    Subtype = Test;
    [Test] procedure SiLocalThenOverwrite() var S: Codeunit "N70 State"; Seen: Text;
    begin
        Seen := S.GetFlag();
        S.SetFlag('fourth');
        if Seen <> 'second' then Error('fourth saw flag=%1', Seen);
    end;
}

// A codeunit that does not read the flag: same outcome in every order.
codeunit 50114 "N70 SI Does Not Read"
{
    Subtype = Test;
    [Test] procedure SiDoesNotRead() begin end;
}
