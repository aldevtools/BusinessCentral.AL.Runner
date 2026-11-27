// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86740 to 86751
// Probe: ClearAll in an owner codeunit over the whole tree of codeunit globals below a TableNo register, in the shape of
// "Whse. Jnl.-Register Line" (BC 29): a plain codeunit at depth 1 holding a SingleInstance codeunit with non-temporary records,
// Labels and a Boolean ("Item Tracking Management" holding "Unit of Measure Management"), a SingleInstance codeunit at depth 1
// ("Sequence No. Mgt."), SingleInstance under SingleInstance, plain under SingleInstance, plain under plain under SingleInstance.
// Every node holds the same global kinds: non-temporary and temporary record, Boolean, Integer, Decimal, BigInteger, Text,
// Code, Date, DateTime, Option, Enum, List, Dictionary, TextBuilder, Guid, RecordRef, FieldRef, KeyRef and Labels (plain, Comment,
// Locked, MaxLength). al-runner issue 9194.
// singleinstance-shared-on-purpose: the subject is SingleInstance codeunits held below a register that ClearAll resets.
// BC versions: 27, 28, 29
// What BC did: (local BC 27.5 and 29.0 identical) ClearAll in the owner resets the register and every plain codeunit reached only
// through plain codeunits (P1, P2): record filter and field, Boolean, Integer, Decimal, BigInteger, Text, Code, Date, DateTime,
// Option, Enum, List, Dictionary, TextBuilder and Guid go to their defaults, the RecordRef is closed (Number 0), and reading
// FieldRef.Number or KeyRef.FieldCount raises an error. Temporary record rows stay (Count 1) and Labels keep their text.
// Every SingleInstance codeunit (S1 to S4, at depth 1 and 2, and under another SingleInstance) and every plain codeunit below a
// SingleInstance (P3, P4) keeps all its state, FieldRef and KeyRef included. Writes made directly on S1 and S2 before or after the
// ClearAll are seen through the owner. ClearAll inside the register does the same as in the owner. ClearAll inside P1 resets P1
// and P2 only: the register keeps its state and S2 and S4 keep theirs.
enum 86740 "CWT Kind"
{
    Extensible = false;

    value(0; A) { }
    value(1; B) { }
}

codeunit 86741 "CWT S4"
{
    SingleInstance = true;

    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('S4.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('S4.tmp=' + Format(T.Count()));
        Out.Add('S4.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('S4.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('S4.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('S4.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('S4.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('S4.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86742 "CWT P4"
{
    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('P4.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('P4.tmp=' + Format(T.Count()));
        Out.Add('P4.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('P4.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('P4.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('P4.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('P4.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('P4.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86743 "CWT S3"
{
    SingleInstance = true;

    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('S3.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('S3.tmp=' + Format(T.Count()));
        Out.Add('S3.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('S3.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('S3.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('S3.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('S3.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('S3.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86744 "CWT P3"
{
    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;
        P4: Codeunit "CWT P4";

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
        P4.SetAll(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('P3.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('P3.tmp=' + Format(T.Count()));
        Out.Add('P3.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('P3.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('P3.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('P3.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('P3.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('P3.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
        P4.AddStates(Out);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86745 "CWT S2"
{
    SingleInstance = true;

    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;
        S4: Codeunit "CWT S4";

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
        S4.SetAll(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('S2.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('S2.tmp=' + Format(T.Count()));
        Out.Add('S2.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('S2.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('S2.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('S2.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('S2.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('S2.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
        S4.AddStates(Out);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86746 "CWT P2"
{
    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('P2.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('P2.tmp=' + Format(T.Count()));
        Out.Add('P2.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('P2.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('P2.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('P2.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('P2.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('P2.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86747 "CWT P1"
{
    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;
        S2: Codeunit "CWT S2";
        P2: Codeunit "CWT P2";

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
        S2.SetAll(Seed);
        P2.SetAll(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('P1.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('P1.tmp=' + Format(T.Count()));
        Out.Add('P1.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('P1.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('P1.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('P1.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('P1.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('P1.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
        S2.AddStates(Out);
        P2.AddStates(Out);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86748 "CWT S1"
{
    SingleInstance = true;

    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;
        S3: Codeunit "CWT S3";
        P3: Codeunit "CWT P3";

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
        S3.SetAll(Seed);
        P3.SetAll(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Out.Add('S1.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('S1.tmp=' + Format(T.Count()));
        Out.Add('S1.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('S1.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('S1.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('S1.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('S1.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('S1.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
        S3.AddStates(Out);
        P3.AddStates(Out);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86749 "CWT Reg"
{
    TableNo = "CKK Row";

    trigger OnRun()
    begin
    end;

    var
        R: Record "CKK Row";
        T: Record "CKK Row" temporary;
        B: Boolean;
        I: Integer;
        Dc: Decimal;
        Bi: BigInteger;
        Tx: Text;
        Cd: Code[20];
        D: Date;
        Dt: DateTime;
        Op: Option A,B,C;
        En: Enum "CWT Kind";
        L: List of [Integer];
        Dict: Dictionary of [Integer, Text];
        Tb: TextBuilder;
        G: Guid;
        RR: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Lp: Label 'lp';
        Lc: Label 'lc', Comment = 'c';
        Ll: Label 'll', Locked = true;
        Lm: Label 'lm', MaxLength = 10;
        P1: Codeunit "CWT P1";
        S1: Codeunit "CWT S1";

    procedure SetAll(Seed: Integer)
    begin
        SetOwn(Seed);
        P1.SetAll(Seed);
        S1.SetAll(Seed);
    end;

    procedure SetOwn(Seed: Integer)
    begin
        Clear(R);
        Clear(L);
        Clear(Dict);
        Clear(Tb);
        Clear(G);
        T.Reset();
        T.DeleteAll();
        if RR.Number <> 0 then
            RR.Close();
        B := Seed <> 0;
        I := Seed;
        Dc := Seed / 2;
        Bi := Seed * 1000000000L;
        Tx := Format(Seed);
        Cd := 'C' + Format(Seed);
        D := 0D;
        Dt := 0DT;
        Op := Op::A;
        En := En::A;
        if Seed = 0 then
            exit;
        R.SetRange("No.", Seed);
        R.Txt := 'r' + Format(Seed);
        T.Init();
        T."No." := Seed;
        T.Insert();
        D := DMY2Date(1, 1, 2000 + Seed);
        Dt := CreateDateTime(D, 120000T);
        Op := Op::C;
        En := En::B;
        L.Add(Seed);
        Dict.Add(Seed, Format(Seed));
        Tb.Append('t' + Format(Seed));
        G := CreateGuid();
        RR.Open(Database::"CKK Row");
        FR := RR.Field(1);
        KR := RR.KeyIndex(1);
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        AddStatesP1(Out);
        S1.AddStates(Out);
    end;

    procedure AddStatesP1(var Out: List of [Text])
    begin
        Out.Add('Reg.rec=' + R.GetFilters() + '/' + R.Txt);
        Out.Add('Reg.tmp=' + Format(T.Count()));
        Out.Add('Reg.scalars=' + Format(B) + '/' + Format(I) + '/' + Format(Dc, 0, 9) + '/' + Format(Bi) + '/' + Tx + '/' + Cd);
        Out.Add('Reg.time=' + Format(D, 0, 9) + '/' + Format(Dt <> 0DT));
        Out.Add('Reg.enum=' + Format(Op) + '/' + Format(En));
        Out.Add('Reg.coll=' + Format(L.Count()) + '/' + Format(Dict.Count()) + '/' + Tb.ToText());
        Out.Add('Reg.ref=' + Format(IsNullGuid(G)) + '/' + Format(RR.Number) + '/' + FrState() + '/' + KrState());
        Out.Add('Reg.labels=' + Lp + '/' + Lc + '/' + Ll + '/' + Lm);
        P1.AddStates(Out);
    end;

    procedure AddStatesS1(var Out: List of [Text])
    begin
        S1.AddStates(Out);
    end;

    local procedure FrState(): Text
    var
        N: Integer;
    begin
        if TryFieldNo(N) then
            exit(Format(N));
        exit('error');
    end;

    local procedure KrState(): Text
    var
        N: Integer;
    begin
        if TryKeyFieldCount(N) then
            exit(Format(N));
        exit('error');
    end;

    [TryFunction]
    local procedure TryFieldNo(var N: Integer)
    begin
        N := FR.Number;
    end;

    [TryFunction]
    local procedure TryKeyFieldCount(var N: Integer)
    begin
        N := KR.FieldCount;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure ClearInP1()
    begin
        P1.DoClearAll();
    end;
}

codeunit 86750 "CWT Owner"
{
    var
        Reg: Codeunit "CWT Reg";

    procedure Prime(Seed: Integer)
    begin
        Reg.SetAll(Seed);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure ClearInReg()
    begin
        Reg.DoClearAll();
    end;

    procedure ClearInP1()
    begin
        Reg.ClearInP1();
    end;

    procedure AddStates(var Out: List of [Text])
    begin
        Reg.AddStates(Out);
    end;

    procedure AddStatesP1(var Out: List of [Text])
    begin
        Reg.AddStatesP1(Out);
    end;

    procedure AddStatesS1(var Out: List of [Text])
    begin
        Reg.AddStatesS1(Out);
    end;
}

codeunit 86751 "CWT Test"
{
    Subtype = Test;

    var
        Observer: Codeunit "Probe Observer";

    local procedure ResetSingleInstances()
    var
        S1: Codeunit "CWT S1";
        S2: Codeunit "CWT S2";
    begin
        // S1 also resets S3, P3 and P4; S2 also resets S4.
        S1.SetAll(0);
        S2.SetAll(0);
    end;

    local procedure ObserveList(Prefix: Text; Out: List of [Text])
    var
        Line: Text;
    begin
        foreach Line in Out do
            Observer.Observe(Prefix + CopyStr(Line, 1, StrPos(Line, '=') - 1), CopyStr(Line, StrPos(Line, '=') + 1));
    end;

    local procedure ObserveOwner(Prefix: Text; var Owner: Codeunit "CWT Owner")
    var
        Out: List of [Text];
    begin
        Owner.AddStates(Out);
        ObserveList(Prefix, Out);
    end;

    local procedure ObserveOwnerSubtree(Prefix: Text; var Owner: Codeunit "CWT Owner"; Subtree: Text)
    var
        Out: List of [Text];
    begin
        if Subtree = 'P1' then
            Owner.AddStatesP1(Out)
        else
            Owner.AddStatesS1(Out);
        ObserveList(Prefix, Out);
    end;

    local procedure ObserveDirect(Prefix: Text)
    var
        S1: Codeunit "CWT S1";
        S2: Codeunit "CWT S2";
        Out: List of [Text];
    begin
        S1.AddStates(Out);
        S2.AddStates(Out);
        ObserveList(Prefix, Out);
    end;

    [Test]
    procedure Tree_OwnerClearAll_Before_P1()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        ObserveOwnerSubtree('before.', Owner, 'P1');
        Observer.Expect('before.Reg.rec', 'No.: 1/r1');
        Observer.Expect('before.Reg.tmp', '1');
        Observer.Expect('before.Reg.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.Reg.time', '2001-01-01/Yes');
        Observer.Expect('before.Reg.enum', 'C/B');
        Observer.Expect('before.Reg.coll', '1/1/t1');
        Observer.Expect('before.Reg.ref', 'No/86270/1/1');
        Observer.Expect('before.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.P1.rec', 'No.: 1/r1');
        Observer.Expect('before.P1.tmp', '1');
        Observer.Expect('before.P1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.P1.time', '2001-01-01/Yes');
        Observer.Expect('before.P1.enum', 'C/B');
        Observer.Expect('before.P1.coll', '1/1/t1');
        Observer.Expect('before.P1.ref', 'No/86270/1/1');
        Observer.Expect('before.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.S2.rec', 'No.: 1/r1');
        Observer.Expect('before.S2.tmp', '1');
        Observer.Expect('before.S2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.S2.time', '2001-01-01/Yes');
        Observer.Expect('before.S2.enum', 'C/B');
        Observer.Expect('before.S2.coll', '1/1/t1');
        Observer.Expect('before.S2.ref', 'No/86270/1/1');
        Observer.Expect('before.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.S4.rec', 'No.: 1/r1');
        Observer.Expect('before.S4.tmp', '1');
        Observer.Expect('before.S4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.S4.time', '2001-01-01/Yes');
        Observer.Expect('before.S4.enum', 'C/B');
        Observer.Expect('before.S4.coll', '1/1/t1');
        Observer.Expect('before.S4.ref', 'No/86270/1/1');
        Observer.Expect('before.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.P2.rec', 'No.: 1/r1');
        Observer.Expect('before.P2.tmp', '1');
        Observer.Expect('before.P2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.P2.time', '2001-01-01/Yes');
        Observer.Expect('before.P2.enum', 'C/B');
        Observer.Expect('before.P2.coll', '1/1/t1');
        Observer.Expect('before.P2.ref', 'No/86270/1/1');
        Observer.Expect('before.P2.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_OwnerClearAll_Before_S1()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        ObserveOwnerSubtree('before.', Owner, 'S1');
        Observer.Expect('before.S1.rec', 'No.: 1/r1');
        Observer.Expect('before.S1.tmp', '1');
        Observer.Expect('before.S1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.S1.time', '2001-01-01/Yes');
        Observer.Expect('before.S1.enum', 'C/B');
        Observer.Expect('before.S1.coll', '1/1/t1');
        Observer.Expect('before.S1.ref', 'No/86270/1/1');
        Observer.Expect('before.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.S3.rec', 'No.: 1/r1');
        Observer.Expect('before.S3.tmp', '1');
        Observer.Expect('before.S3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.S3.time', '2001-01-01/Yes');
        Observer.Expect('before.S3.enum', 'C/B');
        Observer.Expect('before.S3.coll', '1/1/t1');
        Observer.Expect('before.S3.ref', 'No/86270/1/1');
        Observer.Expect('before.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.P3.rec', 'No.: 1/r1');
        Observer.Expect('before.P3.tmp', '1');
        Observer.Expect('before.P3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.P3.time', '2001-01-01/Yes');
        Observer.Expect('before.P3.enum', 'C/B');
        Observer.Expect('before.P3.coll', '1/1/t1');
        Observer.Expect('before.P3.ref', 'No/86270/1/1');
        Observer.Expect('before.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('before.P4.rec', 'No.: 1/r1');
        Observer.Expect('before.P4.tmp', '1');
        Observer.Expect('before.P4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('before.P4.time', '2001-01-01/Yes');
        Observer.Expect('before.P4.enum', 'C/B');
        Observer.Expect('before.P4.coll', '1/1/t1');
        Observer.Expect('before.P4.ref', 'No/86270/1/1');
        Observer.Expect('before.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_OwnerClearAll_After()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.DoClearAll();
        ObserveOwner('after.', Owner);
        Observer.Expect('after.Reg.rec', '/');
        Observer.Expect('after.Reg.tmp', '1');
        Observer.Expect('after.Reg.scalars', 'No/0/0/0//');
        Observer.Expect('after.Reg.time', '/No');
        Observer.Expect('after.Reg.enum', 'A/A');
        Observer.Expect('after.Reg.coll', '0/0/');
        Observer.Expect('after.Reg.ref', 'Yes/0/error/error');
        Observer.Expect('after.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P1.rec', '/');
        Observer.Expect('after.P1.tmp', '1');
        Observer.Expect('after.P1.scalars', 'No/0/0/0//');
        Observer.Expect('after.P1.time', '/No');
        Observer.Expect('after.P1.enum', 'A/A');
        Observer.Expect('after.P1.coll', '0/0/');
        Observer.Expect('after.P1.ref', 'Yes/0/error/error');
        Observer.Expect('after.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S2.rec', 'No.: 1/r1');
        Observer.Expect('after.S2.tmp', '1');
        Observer.Expect('after.S2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S2.time', '2001-01-01/Yes');
        Observer.Expect('after.S2.enum', 'C/B');
        Observer.Expect('after.S2.coll', '1/1/t1');
        Observer.Expect('after.S2.ref', 'No/86270/1/1');
        Observer.Expect('after.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S4.rec', 'No.: 1/r1');
        Observer.Expect('after.S4.tmp', '1');
        Observer.Expect('after.S4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S4.time', '2001-01-01/Yes');
        Observer.Expect('after.S4.enum', 'C/B');
        Observer.Expect('after.S4.coll', '1/1/t1');
        Observer.Expect('after.S4.ref', 'No/86270/1/1');
        Observer.Expect('after.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P2.rec', '/');
        Observer.Expect('after.P2.tmp', '1');
        Observer.Expect('after.P2.scalars', 'No/0/0/0//');
        Observer.Expect('after.P2.time', '/No');
        Observer.Expect('after.P2.enum', 'A/A');
        Observer.Expect('after.P2.coll', '0/0/');
        Observer.Expect('after.P2.ref', 'Yes/0/error/error');
        Observer.Expect('after.P2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S1.rec', 'No.: 1/r1');
        Observer.Expect('after.S1.tmp', '1');
        Observer.Expect('after.S1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S1.time', '2001-01-01/Yes');
        Observer.Expect('after.S1.enum', 'C/B');
        Observer.Expect('after.S1.coll', '1/1/t1');
        Observer.Expect('after.S1.ref', 'No/86270/1/1');
        Observer.Expect('after.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S3.rec', 'No.: 1/r1');
        Observer.Expect('after.S3.tmp', '1');
        Observer.Expect('after.S3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S3.time', '2001-01-01/Yes');
        Observer.Expect('after.S3.enum', 'C/B');
        Observer.Expect('after.S3.coll', '1/1/t1');
        Observer.Expect('after.S3.ref', 'No/86270/1/1');
        Observer.Expect('after.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P3.rec', 'No.: 1/r1');
        Observer.Expect('after.P3.tmp', '1');
        Observer.Expect('after.P3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P3.time', '2001-01-01/Yes');
        Observer.Expect('after.P3.enum', 'C/B');
        Observer.Expect('after.P3.coll', '1/1/t1');
        Observer.Expect('after.P3.ref', 'No/86270/1/1');
        Observer.Expect('after.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P4.rec', 'No.: 1/r1');
        Observer.Expect('after.P4.tmp', '1');
        Observer.Expect('after.P4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P4.time', '2001-01-01/Yes');
        Observer.Expect('after.P4.enum', 'C/B');
        Observer.Expect('after.P4.coll', '1/1/t1');
        Observer.Expect('after.P4.ref', 'No/86270/1/1');
        Observer.Expect('after.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_OwnerClearAll_DirectSi()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.DoClearAll();
        ObserveDirect('direct.');
        Observer.Expect('direct.S1.rec', 'No.: 1/r1');
        Observer.Expect('direct.S1.tmp', '1');
        Observer.Expect('direct.S1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.S1.time', '2001-01-01/Yes');
        Observer.Expect('direct.S1.enum', 'C/B');
        Observer.Expect('direct.S1.coll', '1/1/t1');
        Observer.Expect('direct.S1.ref', 'No/86270/1/1');
        Observer.Expect('direct.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('direct.S3.rec', 'No.: 1/r1');
        Observer.Expect('direct.S3.tmp', '1');
        Observer.Expect('direct.S3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.S3.time', '2001-01-01/Yes');
        Observer.Expect('direct.S3.enum', 'C/B');
        Observer.Expect('direct.S3.coll', '1/1/t1');
        Observer.Expect('direct.S3.ref', 'No/86270/1/1');
        Observer.Expect('direct.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('direct.P3.rec', 'No.: 1/r1');
        Observer.Expect('direct.P3.tmp', '1');
        Observer.Expect('direct.P3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.P3.time', '2001-01-01/Yes');
        Observer.Expect('direct.P3.enum', 'C/B');
        Observer.Expect('direct.P3.coll', '1/1/t1');
        Observer.Expect('direct.P3.ref', 'No/86270/1/1');
        Observer.Expect('direct.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('direct.P4.rec', 'No.: 1/r1');
        Observer.Expect('direct.P4.tmp', '1');
        Observer.Expect('direct.P4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.P4.time', '2001-01-01/Yes');
        Observer.Expect('direct.P4.enum', 'C/B');
        Observer.Expect('direct.P4.coll', '1/1/t1');
        Observer.Expect('direct.P4.ref', 'No/86270/1/1');
        Observer.Expect('direct.P4.labels', 'lp/lc/ll/lm');
        Observer.Expect('direct.S2.rec', 'No.: 1/r1');
        Observer.Expect('direct.S2.tmp', '1');
        Observer.Expect('direct.S2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.S2.time', '2001-01-01/Yes');
        Observer.Expect('direct.S2.enum', 'C/B');
        Observer.Expect('direct.S2.coll', '1/1/t1');
        Observer.Expect('direct.S2.ref', 'No/86270/1/1');
        Observer.Expect('direct.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('direct.S4.rec', 'No.: 1/r1');
        Observer.Expect('direct.S4.tmp', '1');
        Observer.Expect('direct.S4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('direct.S4.time', '2001-01-01/Yes');
        Observer.Expect('direct.S4.enum', 'C/B');
        Observer.Expect('direct.S4.coll', '1/1/t1');
        Observer.Expect('direct.S4.ref', 'No/86270/1/1');
        Observer.Expect('direct.S4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_OwnerClearAll_Reprimed_P1()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.DoClearAll();
        Owner.Prime(2);
        ObserveOwnerSubtree('reprimed.', Owner, 'P1');
        Observer.Expect('reprimed.Reg.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.Reg.tmp', '1');
        Observer.Expect('reprimed.Reg.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.Reg.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.Reg.enum', 'C/B');
        Observer.Expect('reprimed.Reg.coll', '1/1/t2');
        Observer.Expect('reprimed.Reg.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.P1.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.P1.tmp', '1');
        Observer.Expect('reprimed.P1.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.P1.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.P1.enum', 'C/B');
        Observer.Expect('reprimed.P1.coll', '1/1/t2');
        Observer.Expect('reprimed.P1.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.S2.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.S2.tmp', '1');
        Observer.Expect('reprimed.S2.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.S2.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.S2.enum', 'C/B');
        Observer.Expect('reprimed.S2.coll', '1/1/t2');
        Observer.Expect('reprimed.S2.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.S4.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.S4.tmp', '1');
        Observer.Expect('reprimed.S4.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.S4.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.S4.enum', 'C/B');
        Observer.Expect('reprimed.S4.coll', '1/1/t2');
        Observer.Expect('reprimed.S4.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.P2.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.P2.tmp', '1');
        Observer.Expect('reprimed.P2.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.P2.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.P2.enum', 'C/B');
        Observer.Expect('reprimed.P2.coll', '1/1/t2');
        Observer.Expect('reprimed.P2.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.P2.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_OwnerClearAll_Reprimed_S1()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.DoClearAll();
        Owner.Prime(2);
        ObserveOwnerSubtree('reprimed.', Owner, 'S1');
        Observer.Expect('reprimed.S1.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.S1.tmp', '1');
        Observer.Expect('reprimed.S1.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.S1.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.S1.enum', 'C/B');
        Observer.Expect('reprimed.S1.coll', '1/1/t2');
        Observer.Expect('reprimed.S1.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.S3.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.S3.tmp', '1');
        Observer.Expect('reprimed.S3.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.S3.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.S3.enum', 'C/B');
        Observer.Expect('reprimed.S3.coll', '1/1/t2');
        Observer.Expect('reprimed.S3.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.P3.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.P3.tmp', '1');
        Observer.Expect('reprimed.P3.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.P3.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.P3.enum', 'C/B');
        Observer.Expect('reprimed.P3.coll', '1/1/t2');
        Observer.Expect('reprimed.P3.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('reprimed.P4.rec', 'No.: 2/r2');
        Observer.Expect('reprimed.P4.tmp', '1');
        Observer.Expect('reprimed.P4.scalars', 'Yes/2/1/2000000000/2/C2');
        Observer.Expect('reprimed.P4.time', '2002-01-01/Yes');
        Observer.Expect('reprimed.P4.enum', 'C/B');
        Observer.Expect('reprimed.P4.coll', '1/1/t2');
        Observer.Expect('reprimed.P4.ref', 'No/86270/1/1');
        Observer.Expect('reprimed.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_DirectSiWriteBefore_OwnerClearAll()
    var
        Owner: Codeunit "CWT Owner";
        S1: Codeunit "CWT S1";
        S2: Codeunit "CWT S2";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        S1.SetAll(3);
        S2.SetAll(4);
        Owner.DoClearAll();
        ObserveOwner('after.', Owner);
        Observer.Expect('after.Reg.rec', '/');
        Observer.Expect('after.Reg.tmp', '1');
        Observer.Expect('after.Reg.scalars', 'No/0/0/0//');
        Observer.Expect('after.Reg.time', '/No');
        Observer.Expect('after.Reg.enum', 'A/A');
        Observer.Expect('after.Reg.coll', '0/0/');
        Observer.Expect('after.Reg.ref', 'Yes/0/error/error');
        Observer.Expect('after.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P1.rec', '/');
        Observer.Expect('after.P1.tmp', '1');
        Observer.Expect('after.P1.scalars', 'No/0/0/0//');
        Observer.Expect('after.P1.time', '/No');
        Observer.Expect('after.P1.enum', 'A/A');
        Observer.Expect('after.P1.coll', '0/0/');
        Observer.Expect('after.P1.ref', 'Yes/0/error/error');
        Observer.Expect('after.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S2.rec', 'No.: 4/r4');
        Observer.Expect('after.S2.tmp', '1');
        Observer.Expect('after.S2.scalars', 'Yes/4/2/4000000000/4/C4');
        Observer.Expect('after.S2.time', '2004-01-01/Yes');
        Observer.Expect('after.S2.enum', 'C/B');
        Observer.Expect('after.S2.coll', '1/1/t4');
        Observer.Expect('after.S2.ref', 'No/86270/1/1');
        Observer.Expect('after.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S4.rec', 'No.: 4/r4');
        Observer.Expect('after.S4.tmp', '1');
        Observer.Expect('after.S4.scalars', 'Yes/4/2/4000000000/4/C4');
        Observer.Expect('after.S4.time', '2004-01-01/Yes');
        Observer.Expect('after.S4.enum', 'C/B');
        Observer.Expect('after.S4.coll', '1/1/t4');
        Observer.Expect('after.S4.ref', 'No/86270/1/1');
        Observer.Expect('after.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P2.rec', '/');
        Observer.Expect('after.P2.tmp', '1');
        Observer.Expect('after.P2.scalars', 'No/0/0/0//');
        Observer.Expect('after.P2.time', '/No');
        Observer.Expect('after.P2.enum', 'A/A');
        Observer.Expect('after.P2.coll', '0/0/');
        Observer.Expect('after.P2.ref', 'Yes/0/error/error');
        Observer.Expect('after.P2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S1.rec', 'No.: 3/r3');
        Observer.Expect('after.S1.tmp', '1');
        Observer.Expect('after.S1.scalars', 'Yes/3/1.5/3000000000/3/C3');
        Observer.Expect('after.S1.time', '2003-01-01/Yes');
        Observer.Expect('after.S1.enum', 'C/B');
        Observer.Expect('after.S1.coll', '1/1/t3');
        Observer.Expect('after.S1.ref', 'No/86270/1/1');
        Observer.Expect('after.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S3.rec', 'No.: 3/r3');
        Observer.Expect('after.S3.tmp', '1');
        Observer.Expect('after.S3.scalars', 'Yes/3/1.5/3000000000/3/C3');
        Observer.Expect('after.S3.time', '2003-01-01/Yes');
        Observer.Expect('after.S3.enum', 'C/B');
        Observer.Expect('after.S3.coll', '1/1/t3');
        Observer.Expect('after.S3.ref', 'No/86270/1/1');
        Observer.Expect('after.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P3.rec', 'No.: 3/r3');
        Observer.Expect('after.P3.tmp', '1');
        Observer.Expect('after.P3.scalars', 'Yes/3/1.5/3000000000/3/C3');
        Observer.Expect('after.P3.time', '2003-01-01/Yes');
        Observer.Expect('after.P3.enum', 'C/B');
        Observer.Expect('after.P3.coll', '1/1/t3');
        Observer.Expect('after.P3.ref', 'No/86270/1/1');
        Observer.Expect('after.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P4.rec', 'No.: 3/r3');
        Observer.Expect('after.P4.tmp', '1');
        Observer.Expect('after.P4.scalars', 'Yes/3/1.5/3000000000/3/C3');
        Observer.Expect('after.P4.time', '2003-01-01/Yes');
        Observer.Expect('after.P4.enum', 'C/B');
        Observer.Expect('after.P4.coll', '1/1/t3');
        Observer.Expect('after.P4.ref', 'No/86270/1/1');
        Observer.Expect('after.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_DirectSiWriteAfter_OwnerClearAll()
    var
        Owner: Codeunit "CWT Owner";
        S1: Codeunit "CWT S1";
        S2: Codeunit "CWT S2";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.DoClearAll();
        S1.SetAll(5);
        S2.SetAll(6);
        ObserveOwner('after.', Owner);
        Observer.Expect('after.Reg.rec', '/');
        Observer.Expect('after.Reg.tmp', '1');
        Observer.Expect('after.Reg.scalars', 'No/0/0/0//');
        Observer.Expect('after.Reg.time', '/No');
        Observer.Expect('after.Reg.enum', 'A/A');
        Observer.Expect('after.Reg.coll', '0/0/');
        Observer.Expect('after.Reg.ref', 'Yes/0/error/error');
        Observer.Expect('after.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P1.rec', '/');
        Observer.Expect('after.P1.tmp', '1');
        Observer.Expect('after.P1.scalars', 'No/0/0/0//');
        Observer.Expect('after.P1.time', '/No');
        Observer.Expect('after.P1.enum', 'A/A');
        Observer.Expect('after.P1.coll', '0/0/');
        Observer.Expect('after.P1.ref', 'Yes/0/error/error');
        Observer.Expect('after.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S2.rec', 'No.: 6/r6');
        Observer.Expect('after.S2.tmp', '1');
        Observer.Expect('after.S2.scalars', 'Yes/6/3/6000000000/6/C6');
        Observer.Expect('after.S2.time', '2006-01-01/Yes');
        Observer.Expect('after.S2.enum', 'C/B');
        Observer.Expect('after.S2.coll', '1/1/t6');
        Observer.Expect('after.S2.ref', 'No/86270/1/1');
        Observer.Expect('after.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S4.rec', 'No.: 6/r6');
        Observer.Expect('after.S4.tmp', '1');
        Observer.Expect('after.S4.scalars', 'Yes/6/3/6000000000/6/C6');
        Observer.Expect('after.S4.time', '2006-01-01/Yes');
        Observer.Expect('after.S4.enum', 'C/B');
        Observer.Expect('after.S4.coll', '1/1/t6');
        Observer.Expect('after.S4.ref', 'No/86270/1/1');
        Observer.Expect('after.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P2.rec', '/');
        Observer.Expect('after.P2.tmp', '1');
        Observer.Expect('after.P2.scalars', 'No/0/0/0//');
        Observer.Expect('after.P2.time', '/No');
        Observer.Expect('after.P2.enum', 'A/A');
        Observer.Expect('after.P2.coll', '0/0/');
        Observer.Expect('after.P2.ref', 'Yes/0/error/error');
        Observer.Expect('after.P2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S1.rec', 'No.: 5/r5');
        Observer.Expect('after.S1.tmp', '1');
        Observer.Expect('after.S1.scalars', 'Yes/5/2.5/5000000000/5/C5');
        Observer.Expect('after.S1.time', '2005-01-01/Yes');
        Observer.Expect('after.S1.enum', 'C/B');
        Observer.Expect('after.S1.coll', '1/1/t5');
        Observer.Expect('after.S1.ref', 'No/86270/1/1');
        Observer.Expect('after.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S3.rec', 'No.: 5/r5');
        Observer.Expect('after.S3.tmp', '1');
        Observer.Expect('after.S3.scalars', 'Yes/5/2.5/5000000000/5/C5');
        Observer.Expect('after.S3.time', '2005-01-01/Yes');
        Observer.Expect('after.S3.enum', 'C/B');
        Observer.Expect('after.S3.coll', '1/1/t5');
        Observer.Expect('after.S3.ref', 'No/86270/1/1');
        Observer.Expect('after.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P3.rec', 'No.: 5/r5');
        Observer.Expect('after.P3.tmp', '1');
        Observer.Expect('after.P3.scalars', 'Yes/5/2.5/5000000000/5/C5');
        Observer.Expect('after.P3.time', '2005-01-01/Yes');
        Observer.Expect('after.P3.enum', 'C/B');
        Observer.Expect('after.P3.coll', '1/1/t5');
        Observer.Expect('after.P3.ref', 'No/86270/1/1');
        Observer.Expect('after.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P4.rec', 'No.: 5/r5');
        Observer.Expect('after.P4.tmp', '1');
        Observer.Expect('after.P4.scalars', 'Yes/5/2.5/5000000000/5/C5');
        Observer.Expect('after.P4.time', '2005-01-01/Yes');
        Observer.Expect('after.P4.enum', 'C/B');
        Observer.Expect('after.P4.coll', '1/1/t5');
        Observer.Expect('after.P4.ref', 'No/86270/1/1');
        Observer.Expect('after.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_RegisterClearAll()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.ClearInReg();
        ObserveOwner('after.', Owner);
        Observer.Expect('after.Reg.rec', '/');
        Observer.Expect('after.Reg.tmp', '1');
        Observer.Expect('after.Reg.scalars', 'No/0/0/0//');
        Observer.Expect('after.Reg.time', '/No');
        Observer.Expect('after.Reg.enum', 'A/A');
        Observer.Expect('after.Reg.coll', '0/0/');
        Observer.Expect('after.Reg.ref', 'Yes/0/error/error');
        Observer.Expect('after.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P1.rec', '/');
        Observer.Expect('after.P1.tmp', '1');
        Observer.Expect('after.P1.scalars', 'No/0/0/0//');
        Observer.Expect('after.P1.time', '/No');
        Observer.Expect('after.P1.enum', 'A/A');
        Observer.Expect('after.P1.coll', '0/0/');
        Observer.Expect('after.P1.ref', 'Yes/0/error/error');
        Observer.Expect('after.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S2.rec', 'No.: 1/r1');
        Observer.Expect('after.S2.tmp', '1');
        Observer.Expect('after.S2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S2.time', '2001-01-01/Yes');
        Observer.Expect('after.S2.enum', 'C/B');
        Observer.Expect('after.S2.coll', '1/1/t1');
        Observer.Expect('after.S2.ref', 'No/86270/1/1');
        Observer.Expect('after.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S4.rec', 'No.: 1/r1');
        Observer.Expect('after.S4.tmp', '1');
        Observer.Expect('after.S4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S4.time', '2001-01-01/Yes');
        Observer.Expect('after.S4.enum', 'C/B');
        Observer.Expect('after.S4.coll', '1/1/t1');
        Observer.Expect('after.S4.ref', 'No/86270/1/1');
        Observer.Expect('after.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P2.rec', '/');
        Observer.Expect('after.P2.tmp', '1');
        Observer.Expect('after.P2.scalars', 'No/0/0/0//');
        Observer.Expect('after.P2.time', '/No');
        Observer.Expect('after.P2.enum', 'A/A');
        Observer.Expect('after.P2.coll', '0/0/');
        Observer.Expect('after.P2.ref', 'Yes/0/error/error');
        Observer.Expect('after.P2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S1.rec', 'No.: 1/r1');
        Observer.Expect('after.S1.tmp', '1');
        Observer.Expect('after.S1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S1.time', '2001-01-01/Yes');
        Observer.Expect('after.S1.enum', 'C/B');
        Observer.Expect('after.S1.coll', '1/1/t1');
        Observer.Expect('after.S1.ref', 'No/86270/1/1');
        Observer.Expect('after.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S3.rec', 'No.: 1/r1');
        Observer.Expect('after.S3.tmp', '1');
        Observer.Expect('after.S3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S3.time', '2001-01-01/Yes');
        Observer.Expect('after.S3.enum', 'C/B');
        Observer.Expect('after.S3.coll', '1/1/t1');
        Observer.Expect('after.S3.ref', 'No/86270/1/1');
        Observer.Expect('after.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P3.rec', 'No.: 1/r1');
        Observer.Expect('after.P3.tmp', '1');
        Observer.Expect('after.P3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P3.time', '2001-01-01/Yes');
        Observer.Expect('after.P3.enum', 'C/B');
        Observer.Expect('after.P3.coll', '1/1/t1');
        Observer.Expect('after.P3.ref', 'No/86270/1/1');
        Observer.Expect('after.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P4.rec', 'No.: 1/r1');
        Observer.Expect('after.P4.tmp', '1');
        Observer.Expect('after.P4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P4.time', '2001-01-01/Yes');
        Observer.Expect('after.P4.enum', 'C/B');
        Observer.Expect('after.P4.coll', '1/1/t1');
        Observer.Expect('after.P4.ref', 'No/86270/1/1');
        Observer.Expect('after.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;

    [Test]
    procedure Tree_PlainHolderOfSiClearAll()
    var
        Owner: Codeunit "CWT Owner";
    begin
        ResetSingleInstances();
        Owner.Prime(1);
        Owner.ClearInP1();
        ObserveOwner('after.', Owner);
        Observer.Expect('after.Reg.rec', 'No.: 1/r1');
        Observer.Expect('after.Reg.tmp', '1');
        Observer.Expect('after.Reg.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.Reg.time', '2001-01-01/Yes');
        Observer.Expect('after.Reg.enum', 'C/B');
        Observer.Expect('after.Reg.coll', '1/1/t1');
        Observer.Expect('after.Reg.ref', 'No/86270/1/1');
        Observer.Expect('after.Reg.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P1.rec', '/');
        Observer.Expect('after.P1.tmp', '1');
        Observer.Expect('after.P1.scalars', 'No/0/0/0//');
        Observer.Expect('after.P1.time', '/No');
        Observer.Expect('after.P1.enum', 'A/A');
        Observer.Expect('after.P1.coll', '0/0/');
        Observer.Expect('after.P1.ref', 'Yes/0/error/error');
        Observer.Expect('after.P1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S2.rec', 'No.: 1/r1');
        Observer.Expect('after.S2.tmp', '1');
        Observer.Expect('after.S2.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S2.time', '2001-01-01/Yes');
        Observer.Expect('after.S2.enum', 'C/B');
        Observer.Expect('after.S2.coll', '1/1/t1');
        Observer.Expect('after.S2.ref', 'No/86270/1/1');
        Observer.Expect('after.S2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S4.rec', 'No.: 1/r1');
        Observer.Expect('after.S4.tmp', '1');
        Observer.Expect('after.S4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S4.time', '2001-01-01/Yes');
        Observer.Expect('after.S4.enum', 'C/B');
        Observer.Expect('after.S4.coll', '1/1/t1');
        Observer.Expect('after.S4.ref', 'No/86270/1/1');
        Observer.Expect('after.S4.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P2.rec', '/');
        Observer.Expect('after.P2.tmp', '1');
        Observer.Expect('after.P2.scalars', 'No/0/0/0//');
        Observer.Expect('after.P2.time', '/No');
        Observer.Expect('after.P2.enum', 'A/A');
        Observer.Expect('after.P2.coll', '0/0/');
        Observer.Expect('after.P2.ref', 'Yes/0/error/error');
        Observer.Expect('after.P2.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S1.rec', 'No.: 1/r1');
        Observer.Expect('after.S1.tmp', '1');
        Observer.Expect('after.S1.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S1.time', '2001-01-01/Yes');
        Observer.Expect('after.S1.enum', 'C/B');
        Observer.Expect('after.S1.coll', '1/1/t1');
        Observer.Expect('after.S1.ref', 'No/86270/1/1');
        Observer.Expect('after.S1.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.S3.rec', 'No.: 1/r1');
        Observer.Expect('after.S3.tmp', '1');
        Observer.Expect('after.S3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.S3.time', '2001-01-01/Yes');
        Observer.Expect('after.S3.enum', 'C/B');
        Observer.Expect('after.S3.coll', '1/1/t1');
        Observer.Expect('after.S3.ref', 'No/86270/1/1');
        Observer.Expect('after.S3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P3.rec', 'No.: 1/r1');
        Observer.Expect('after.P3.tmp', '1');
        Observer.Expect('after.P3.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P3.time', '2001-01-01/Yes');
        Observer.Expect('after.P3.enum', 'C/B');
        Observer.Expect('after.P3.coll', '1/1/t1');
        Observer.Expect('after.P3.ref', 'No/86270/1/1');
        Observer.Expect('after.P3.labels', 'lp/lc/ll/lm');
        Observer.Expect('after.P4.rec', 'No.: 1/r1');
        Observer.Expect('after.P4.tmp', '1');
        Observer.Expect('after.P4.scalars', 'Yes/1/0.5/1000000000/1/C1');
        Observer.Expect('after.P4.time', '2001-01-01/Yes');
        Observer.Expect('after.P4.enum', 'C/B');
        Observer.Expect('after.P4.coll', '1/1/t1');
        Observer.Expect('after.P4.ref', 'No/86270/1/1');
        Observer.Expect('after.P4.labels', 'lp/lc/ll/lm');
        Observer.Verify();
    end;
}
