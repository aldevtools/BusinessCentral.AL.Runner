// Review round 3 of the ADR-070 step 4 reports: `--stability-replay isolation.singleInstance=carried` runs in BC's order
// (the order that variant had in the stability run, ADR-070 decision 7). The reader passes only after the setter ran in
// the same session, the P7 shape of corpus codeunits/TestCuP7BReader.al.

codeunit 50320 "N74R3 State"
{
    SingleInstance = true;

    procedure SetFlag(V: Text)
    begin
        Flag := V;
    end;

    procedure GetFlag(): Text
    begin
        exit(Flag);
    end;

    var
        Flag: Text;
}

codeunit 50321 "N74R3 Setter"
{
    Subtype = Test;

    [Test]
    procedure SetsTheFlag()
    var
        S: Codeunit "N74R3 State";
    begin
        S.SetFlag('left');
    end;
}

codeunit 50322 "N74R3 Reader"
{
    Subtype = Test;

    [Test]
    procedure ReadsTheFlag()
    var
        S: Codeunit "N74R3 State";
    begin
        if S.GetFlag() <> 'left' then
            Error('flag is empty');
    end;
}
