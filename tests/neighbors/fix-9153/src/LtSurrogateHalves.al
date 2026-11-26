// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/text/text-data-type
// Scope: in-scope
// Fixtures used: codeunit 69160 "Probe Observer"
// Facts: Text and Code hold one half of a UTF-16 surrogate pair (CopyStr of a pair, or a Char assigned 55357 / 56832). Values are shown as
// lists of UTF-16 codes. In memory the lone half stays as it is through text functions, TextBuilder (except Insert at 0 of a low half, which
// raises), comparisons (ordinal, no pairing), Error and Message text, and Text variables. A write to a stream with an encoding, a Base64
// round trip, and a read back from a table field turn the lone half into 65533. Stream writes without an encoding turn it into 63.
// The three BC versions agree on every observed value.
// BC versions: 27, 28, 29

table 75790 "LT SH Rec"
{
    fields
    {
        field(1; Name; Text[20]) { }
        field(2; T50; Text[50]) { }
        field(3; T1; Text[1]) { }
        field(4; C10; Code[10]) { }
        field(5; Data; Blob) { }
    }
    keys { key(PK; Name) { Clustered = true; } }
}

codeunit 75791 "LT SH Ops"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Res: Text;
        MsgText: Text;

    local procedure Codes(Value: Text): Text
    var
        i: Integer;
        n: Integer;
        R: Text;
    begin
        for i := 1 to StrLen(Value) do begin
            if R <> '' then
                R += ',';
            n := Value[i];
            R += Format(n);
        end;
        exit(Format(StrLen(Value)) + ':' + R);
    end;

    local procedure SplitRes(L: List of [Text]): Text
    var
        R: Text;
        S: Text;
    begin
        R := Format(L.Count());
        foreach S in L do
            R += '|' + Codes(S);
        exit(R);
    end;

    local procedure PairText(): Text
    var
        T: Text;
    begin
        T[1] := 55357;
        T[2] := 56832;
        exit(T);
    end;

    local procedure Source(Src: Integer; Which: Integer): Text
    var
        T: Text;
        Ch: Char;
    begin
        if Src = 1 then
            exit(CopyStr(PairText(), Which, 1));
        if Which = 1 then
            Ch := 55357
        else
            Ch := 56832;
        T[1] := Ch;
        exit(T);
    end;

    [TryFunction]
    local procedure Op(Id: Integer; Hi: Text; Lo: Text)
    var
        Pair: Text;
        Rep: Text;
        T: Text;
        B: TextBuilder;
        L: List of [Text];
        LC: List of [Char];
        D: Dictionary of [Text, Integer];
        JV: JsonValue;
        JO: JsonObject;
        JO2: JsonObject;
        JA: JsonArray;
        JT: JsonToken;
        T1: Text[1];
        C10: Code[10];
        I: Integer;
        Bo: Boolean;
        Bo2: Boolean;
        Ch: Char;
        HiCh: Char;
    begin
        Pair := Hi + Lo;
        Rep[1] := 65533;
        HiCh := 55357;
        Res := '';
        case Id of
            1: begin B.Append(Hi); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            2: begin B.Append(Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            3: begin B.Append(Hi); B.Append(Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            4: begin B.Append(Lo); B.Append(Hi); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            5: begin B.Append('a'); B.Append(Hi); B.Append('b'); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            6: begin B.AppendLine(Hi); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            7: begin B.AppendLine(Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            8: begin B.Append(Pair); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            9: begin B.Append('ab'); B.Insert(1, Hi); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            10: begin B.Append('ab'); B.Insert(0, Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            11: begin B.Append(Hi); B.Insert(1, Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            12: begin B.Append('abc'); B.Replace('b', Hi); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            13: begin B.Append('a' + Hi + 'b'); B.Replace(Hi, 'x'); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            14: begin B.Append(Pair); B.Replace(Lo, 'x'); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            15: begin B.Append(Pair); B.Replace(Hi, 'x'); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            16: begin B.Append(Hi); B.Replace(Hi, Lo); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            17: begin B.Append(Hi); B.Replace('a', 'b'); Res := Codes(B.ToText()) + '|' + Format(B.Length()); end;
            18: begin B.Append('a' + Hi + 'b'); Res := Codes(B.ToText(2, 1)); end;
            19: begin B.Append('a' + Hi + 'b'); Res := Codes(B.ToText(2, 2)); end;
            20: begin Res := Format(Hi = 'a'); end;
            21: begin Res := Format(Hi <> 'a'); end;
            22: begin Res := Format(Hi < 'a'); end;
            23: begin Res := Format(Hi > 'a'); end;
            24: begin Res := Format('a' = Hi); end;
            25: begin Res := Format('a' <> Hi); end;
            26: begin Res := Format('a' < Hi); end;
            27: begin Res := Format('a' > Hi); end;
            28: begin Res := Format(Hi = 'z'); end;
            29: begin Res := Format(Hi <> 'z'); end;
            30: begin Res := Format(Hi < 'z'); end;
            31: begin Res := Format(Hi > 'z'); end;
            32: begin Res := Format(Hi = ''); end;
            33: begin Res := Format(Hi <> ''); end;
            34: begin Res := Format(Hi < ''); end;
            35: begin Res := Format(Hi > ''); end;
            36: begin Res := Format(Hi = Hi); end;
            37: begin Res := Format(Hi <> Hi); end;
            38: begin Res := Format(Hi < Hi); end;
            39: begin Res := Format(Hi > Hi); end;
            40: begin Res := Format(Hi = Lo); end;
            41: begin Res := Format(Hi <> Lo); end;
            42: begin Res := Format(Hi < Lo); end;
            43: begin Res := Format(Hi > Lo); end;
            44: begin Res := Format(Lo = Hi); end;
            45: begin Res := Format(Lo <> Hi); end;
            46: begin Res := Format(Lo < Hi); end;
            47: begin Res := Format(Lo > Hi); end;
            48: begin Res := Format(Hi = Pair); end;
            49: begin Res := Format(Hi <> Pair); end;
            50: begin Res := Format(Hi < Pair); end;
            51: begin Res := Format(Hi > Pair); end;
            52: begin Res := Format(Pair = Hi); end;
            53: begin Res := Format(Pair <> Hi); end;
            54: begin Res := Format(Pair < Hi); end;
            55: begin Res := Format(Pair > Hi); end;
            56: begin Res := Format(Lo = Pair); end;
            57: begin Res := Format(Lo <> Pair); end;
            58: begin Res := Format(Lo < Pair); end;
            59: begin Res := Format(Lo > Pair); end;
            60: begin Res := Format(Pair = Lo); end;
            61: begin Res := Format(Pair <> Lo); end;
            62: begin Res := Format(Pair < Lo); end;
            63: begin Res := Format(Pair > Lo); end;
            64: begin Res := Format(Hi = Rep); end;
            65: begin Res := Format(Hi <> Rep); end;
            66: begin Res := Format(Hi < Rep); end;
            67: begin Res := Format(Hi > Rep); end;
            68: begin Res := Format(Rep = Hi); end;
            69: begin Res := Format(Rep <> Hi); end;
            70: begin Res := Format(Rep < Hi); end;
            71: begin Res := Format(Rep > Hi); end;
            72: begin Res := Format(Lo = 'a'); end;
            73: begin Res := Format(Lo <> 'a'); end;
            74: begin Res := Format(Lo < 'a'); end;
            75: begin Res := Format(Lo > 'a'); end;
            76: begin Res := Format(Lo = Rep); end;
            77: begin Res := Format(Lo <> Rep); end;
            78: begin Res := Format(Lo < Rep); end;
            79: begin Res := Format(Lo > Rep); end;
            80: begin Res := Format(Hi = 'abc'); end;
            81: begin Res := Format(Hi <> 'abc'); end;
            82: begin Res := Format(Hi < 'abc'); end;
            83: begin Res := Format(Hi > 'abc'); end;
            84: begin Res := Format(Lo = 'abc'); end;
            85: begin Res := Format(Lo <> 'abc'); end;
            86: begin Res := Format(Lo < 'abc'); end;
            87: begin Res := Format(Lo > 'abc'); end;
            88: begin Res := Format(Hi = CopyStr(Pair, 1, 1)); end;
            89: begin Res := Format(Hi <> CopyStr(Pair, 1, 1)); end;
            90: begin Res := Format(Hi < CopyStr(Pair, 1, 1)); end;
            91: begin Res := Format(Hi > CopyStr(Pair, 1, 1)); end;
            92: begin Res := Format(Lo = '~'); end;
            93: begin Res := Format(Lo <> '~'); end;
            94: begin Res := Format(Lo < '~'); end;
            95: begin Res := Format(Lo > '~'); end;
            96: begin Res := Format(Hi = 'a' + Hi); end;
            97: begin Res := Format(Hi <> 'a' + Hi); end;
            98: begin Res := Format(Hi < 'a' + Hi); end;
            99: begin Res := Format(Hi > 'a' + Hi); end;
            100: begin Res := Format(StrLen(Hi)); end;
            101: begin Res := Codes(CopyStr(Hi, 1, 1)); end;
            102: begin Res := Codes(CopyStr('a' + Hi + 'b', 2, 1)); end;
            103: begin Res := Format(StrPos(Pair, Hi)); end;
            104: begin Res := Format(StrPos(Pair, Lo)); end;
            105: begin Res := Format(StrPos('a' + Hi + 'b', Hi)); end;
            106: begin Res := Format(StrPos(Hi, 'a')); end;
            107: begin Res := Format(StrPos(Hi, Hi)); end;
            108: begin Res := Format(StrPos(Hi, Lo)); end;
            109: begin Res := Format(Pair.IndexOf(Hi)); end;
            110: begin Res := Format(Pair.IndexOf(Lo)); end;
            111: begin Res := Format(('a' + Hi).IndexOf(Hi)); end;
            112: begin Res := Format(Hi.IndexOf('a')); end;
            113: begin Res := Format(Pair.IndexOf(Lo, 2)); end;
            114: begin Res := Format(Pair.Contains(Hi)); end;
            115: begin Res := Format(Pair.Contains(Lo)); end;
            116: begin Res := Format(Hi.Contains(Hi)); end;
            117: begin Res := Format(Hi.Contains(Lo)); end;
            118: begin Res := Format(('a' + Hi).Contains('a')); end;
            119: begin Res := Format(Pair.StartsWith(Hi)); end;
            120: begin Res := Format(Pair.StartsWith(Lo)); end;
            121: begin Res := Format(Hi.StartsWith(Hi)); end;
            122: begin Res := Format((Hi + 'a').StartsWith(Hi)); end;
            123: begin Res := Format(Pair.EndsWith(Lo)); end;
            124: begin Res := Format(Pair.EndsWith(Hi)); end;
            125: begin Res := Format(('a' + Hi).EndsWith(Hi)); end;
            126: begin Res := Format(Hi.EndsWith('a')); end;
            127: begin Res := Codes(UpperCase(Hi)); end;
            128: begin Res := Codes(LowerCase(Hi)); end;
            129: begin Res := Codes(Hi.ToUpper()); end;
            130: begin Res := Codes(Lo.ToLower()); end;
            131: begin Res := Codes(UpperCase('a' + Lo + 'b')); end;
            132: begin Res := Codes(ConvertStr(Hi, 'a', 'b')); end;
            133: begin Res := Codes(ConvertStr(Hi, Hi, 'x')); end;
            134: begin Res := Codes(ConvertStr('abc', 'b', Hi)); end;
            135: begin Res := Codes(ConvertStr('a' + Hi, Hi, 'x')); end;
            136: begin Res := Codes(ConvertStr(Pair, Hi, 'x')); end;
            137: begin Res := Codes(DelChr(Hi, '=', Hi)); end;
            138: begin Res := Codes(DelChr(Pair, '=', Hi)); end;
            139: begin Res := Codes(DelChr(Pair, '=', Lo)); end;
            140: begin Res := Codes(DelChr(Hi, '=', 'a')); end;
            141: begin Res := Codes(DelChr('a' + Hi + 'a', '<>', 'a')); end;
            142: begin Res := Codes(DelChr(Hi + 'a', '>', 'a')); end;
            143: begin Res := Codes(DelChr(Hi + 'a', '<', Hi)); end;
            144: begin Res := Codes(PadStr(Hi, 3, 'x')); end;
            145: begin Res := Codes(PadStr('a', 3, Hi)); end;
            146: begin Res := Codes(PadStr(Hi, 1)); end;
            147: begin Res := Codes(PadStr(Pair, 5, 'x')); end;
            148: begin Res := Codes(SelectStr(1, Hi + ',' + Lo)); end;
            149: begin Res := Codes(SelectStr(2, Hi + ',' + Lo)); end;
            150: begin Res := Codes(SelectStr(2, 'a,' + Hi)); end;
            151: begin Res := Codes(SelectStr(1, Hi)); end;
            152: begin Res := Codes(Format(Hi)); end;
            153: begin Res := Codes(Format(Lo)); end;
            154: begin Res := Codes(Format(Hi) + Format(Lo)); end;
            155: begin Res := Codes(Format(Hi, 5)); end;
            156: begin Res := Codes(DelStr(Hi, 1, 1)); end;
            157: begin Res := Codes(InsStr('ab', Hi, 2)); end;
            158: begin Res := Codes(InsStr(Hi, 'x', 1)); end;
            159: begin Res := Codes(IncStr(Hi)); end;
            160: begin Res := Codes(IncStr('a' + Hi)); end;
            161: begin Res := Codes('abc'.Replace('b', Hi)); end;
            162: begin Res := Codes(('a' + Hi + 'b').Replace(Hi, 'x')); end;
            163: begin Res := Codes(Pair.Replace(Hi, 'x')); end;
            164: begin Res := Codes(Pair.Replace(Lo, 'x')); end;
            165: begin Res := Codes(Hi.Replace(Hi, Lo)); end;
            166: begin Res := Codes(Hi.Replace('a', 'b')); end;
            167: begin Res := Codes(Hi.Trim()); end;
            168: begin Res := Codes((' ' + Hi + ' ').Trim()); end;
            169: begin Res := Codes(('a' + Hi).TrimEnd(Hi)); end;
            170: begin Res := Codes((Hi + 'a').TrimStart(Hi)); end;
            171: begin Res := Codes(Hi.Trim()); end;
            172: begin Res := Codes(('a' + Hi).Substring(2)); end;
            173: begin Res := Codes(Hi.Substring(1, 1)); end;
            174: begin Res := Codes(Hi.PadLeft(3, 'x')); end;
            175: begin Res := Codes(Hi.PadRight(3, HiCh)); end;
            176: begin Res := Codes(Hi.PadLeft(3, HiCh)); end;
            177: begin Res := Codes(Hi + 'a'); end;
            178: begin Res := Codes('a' + Hi); end;
            179: begin Res := Codes(Hi + Lo); end;
            180: begin Res := Codes(Lo + Hi); end;
            181: begin Res := Codes(Hi + Hi); end;
            182: begin Res := Codes(StrSubstNo('%1', Hi)); end;
            183: begin Res := Codes(StrSubstNo('%1%2', Hi, Lo)); end;
            184: begin Res := Codes(StrSubstNo('x%1y', Lo)); end;
            185: begin Res := Codes(StrSubstNo(Hi, 'x')); end;
            186: begin Res := Codes(StrSubstNo('%1', Pair)); end;
            187: begin T1 := Hi; Res := Codes(T1); end;
            188: begin T1 := Lo; Res := Codes(T1); end;
            189: begin T1 := Pair; Res := Codes(T1); end;
            190: begin T1[1] := HiCh; Res := Codes(T1); end;
            191: begin C10 := Hi; Res := Codes(C10); end;
            192: begin C10 := Lo; Res := Codes(C10); end;
            193: begin C10 := Hi; Res := Format(C10 = Hi); end;
            194: begin C10 := CopyStr(Pair, 1, 1); Res := Codes(C10); end;
            195: begin C10 := 'a' + Hi; Res := Codes(C10); end;
            196: begin I := Hi[1]; Res := Format(I); end;
            197: begin I := Lo[1]; Res := Format(I); end;
            198: begin Bo := Evaluate(I, Hi); Res := Format(Bo) + '|' + Format(I); end;
            199: begin Bo := Evaluate(T, Hi); Res := Format(Bo) + '|' + Codes(T); end;
            200: begin Bo := Evaluate(Ch, Hi); I := Ch; Res := Format(Bo) + '|' + Format(I); end;
            201: begin Bo := Evaluate(C10, Hi); Res := Format(Bo) + '|' + Codes(C10); end;
            202: begin Bo := Evaluate(Bo2, Hi); Res := Format(Bo) + '|' + Format(Bo2); end;
            203: begin Bo := Evaluate(T, Lo); Res := Format(Bo) + '|' + Codes(T); end;
            204: begin LC.Add(HiCh); Res := Format(('a' + 'b').IndexOfAny(LC)) + '|' + Format(('a' + Hi + 'b').IndexOfAny(LC)); end;
            205: begin L := Hi.Split(Hi); Res := SplitRes(L); end;
            206: begin L := ('a' + Hi + 'b').Split(Hi); Res := SplitRes(L); end;
            207: begin L := Pair.Split(Hi); Res := SplitRes(L); end;
            208: begin L := ('a' + Lo + 'b').Split(Lo); Res := SplitRes(L); end;
            209: begin L := ('a,' + Hi).Split(','); Res := SplitRes(L); end;
            210: begin L := Hi.Split(','); Res := SplitRes(L); end;
            211: begin L.Add(Hi); Res := Format(L.Contains(Hi)) + Format(L.Contains(Lo)) + Format(L.Contains('a')) + Format(L.IndexOf(Hi)); end;
            212: begin L.Add(Hi); L.Add(Lo); Res := Format(L.Count()) + Format(L.IndexOf(Lo)) + Format(L.Contains(Hi + Lo)); end;
            213: begin D.Add(Hi, 1); D.Add(Lo, 2); Res := Format(D.Count()) + Format(D.ContainsKey(Hi)) + Format(D.ContainsKey(Lo)) + Format(D.ContainsKey(Pair)) + Format(D.Get(Lo)); end;
            214: begin D.Add(Hi, 1); L := D.Keys(); Res := SplitRes(L); end;
            215: begin D.Add(Hi, 1); Res := Format(D.Add(Hi, 2)); end;
            216: begin JV.SetValue(Hi); JV.WriteTo(T); Res := Codes(T) + '|' + Codes(JV.AsText()); end;
            217: begin JV.SetValue(Lo); JV.WriteTo(T); Res := Codes(T) + '|' + Codes(JV.AsText()); end;
            218: begin JO.Add('k', Hi); JO.WriteTo(T); Res := Codes(T); end;
            219: begin JO.Add('k', Hi); JO.Get('k', JT); Res := Codes(JT.AsValue().AsText()); end;
            220: begin JA.Add(Lo); JA.WriteTo(T); Res := Codes(T); end;
            221: begin JO.Add(Hi, 'x'); JO.WriteTo(T); Res := Codes(T) + Format(JO.Contains(Hi)); end;
            222: begin Bo := JO.ReadFrom('{"k":"\uD83D"}'); JO.Get('k', JT); Res := Format(Bo) + '|' + Codes(JT.AsValue().AsText()); end;
            223: begin Bo := JO.ReadFrom('{"k":"\uDE00"}'); JO.Get('k', JT); Res := Format(Bo) + '|' + Codes(JT.AsValue().AsText()); end;
            224: begin JO.Add('k', Hi); JO.WriteTo(T); Bo := JO2.ReadFrom(T); JO2.Get('k', JT); Res := Format(Bo) + '|' + Codes(JT.AsValue().AsText()); end;
        end;
    end;

    local procedure Run(var Observer: Codeunit "Probe Observer"; Name: Text; Id: Integer)
    begin
        if Op(Id, Source(1, 1), Source(1, 2)) then
            Observer.Observe(Name, Res)
        else
            Observer.Observe(Name, 'ERR:' + GetLastErrorText());
    end;

    [Test]
    procedure Ops_tb_1()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'tb_append_hi', 1);
        Run(Observer, 'tb_append_lo', 2);
        Run(Observer, 'tb_append_hi_lo', 3);
        Run(Observer, 'tb_append_lo_hi', 4);
        Run(Observer, 'tb_append_a_hi_b', 5);
        Run(Observer, 'tb_appendline_hi', 6);
        Run(Observer, 'tb_appendline_lo', 7);
        Run(Observer, 'tb_append_pair', 8);
        Observer.Expect('tb_append_hi', '1:55357|1');
        Observer.Expect('tb_append_lo', '1:56832|1');
        Observer.Expect('tb_append_hi_lo', '2:55357,56832|2');
        Observer.Expect('tb_append_lo_hi', '2:56832,55357|2');
        Observer.Expect('tb_append_a_hi_b', '3:97,55357,98|3');
        Observer.Expect('tb_appendline_hi', '3:55357,13,10|3');
        Observer.Expect('tb_appendline_lo', '3:56832,13,10|3');
        Observer.Expect('tb_append_pair', '2:55357,56832|2');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_tb_2()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'tb_insert_hi_in_ab', 9);
        Run(Observer, 'tb_insert_lo_at0', 10);
        Run(Observer, 'tb_insert_lo_after_hi', 11);
        Run(Observer, 'tb_replace_b_with_hi', 12);
        Run(Observer, 'tb_replace_hi_with_x', 13);
        Run(Observer, 'tb_replace_lo_in_pair', 14);
        Run(Observer, 'tb_replace_hi_in_pair', 15);
        Run(Observer, 'tb_replace_hi_with_lo', 16);
        Observer.Expect('tb_insert_hi_in_ab', '3:55357,97,98|3');
        Observer.Expect('tb_insert_lo_at0', 'ERR:An invalid argument was passed to a ''TextBuilder'' data type method.');
        Observer.Expect('tb_insert_lo_after_hi', '2:56832,55357|2');
        Observer.Expect('tb_replace_b_with_hi', '3:97,55357,99|3');
        Observer.Expect('tb_replace_hi_with_x', '3:97,120,98|3');
        Observer.Expect('tb_replace_lo_in_pair', '2:55357,120|2');
        Observer.Expect('tb_replace_hi_in_pair', '2:120,56832|2');
        Observer.Expect('tb_replace_hi_with_lo', '1:56832|1');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_tb_3()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'tb_replace_a_in_hi', 17);
        Run(Observer, 'tb_totext_range', 18);
        Run(Observer, 'tb_totext_range2', 19);
        Observer.Expect('tb_replace_a_in_hi', '1:55357|1');
        Observer.Expect('tb_totext_range', '1:55357');
        Observer.Expect('tb_totext_range2', '2:55357,98');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_1()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Hi_eq_a', 20);
        Run(Observer, 'cmp_Hi_ne_a', 21);
        Run(Observer, 'cmp_Hi_lt_a', 22);
        Run(Observer, 'cmp_Hi_gt_a', 23);
        Run(Observer, 'cmp_a_eq_Hi', 24);
        Run(Observer, 'cmp_a_ne_Hi', 25);
        Run(Observer, 'cmp_a_lt_Hi', 26);
        Run(Observer, 'cmp_a_gt_Hi', 27);
        Observer.Expect('cmp_Hi_eq_a', 'No');
        Observer.Expect('cmp_Hi_ne_a', 'Yes');
        Observer.Expect('cmp_Hi_lt_a', 'No');
        Observer.Expect('cmp_Hi_gt_a', 'Yes');
        Observer.Expect('cmp_a_eq_Hi', 'No');
        Observer.Expect('cmp_a_ne_Hi', 'Yes');
        Observer.Expect('cmp_a_lt_Hi', 'Yes');
        Observer.Expect('cmp_a_gt_Hi', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_2()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Hi_eq_z', 28);
        Run(Observer, 'cmp_Hi_ne_z', 29);
        Run(Observer, 'cmp_Hi_lt_z', 30);
        Run(Observer, 'cmp_Hi_gt_z', 31);
        Run(Observer, 'cmp_Hi_eq_empty', 32);
        Run(Observer, 'cmp_Hi_ne_empty', 33);
        Run(Observer, 'cmp_Hi_lt_empty', 34);
        Run(Observer, 'cmp_Hi_gt_empty', 35);
        Observer.Expect('cmp_Hi_eq_z', 'No');
        Observer.Expect('cmp_Hi_ne_z', 'Yes');
        Observer.Expect('cmp_Hi_lt_z', 'No');
        Observer.Expect('cmp_Hi_gt_z', 'Yes');
        Observer.Expect('cmp_Hi_eq_empty', 'No');
        Observer.Expect('cmp_Hi_ne_empty', 'Yes');
        Observer.Expect('cmp_Hi_lt_empty', 'No');
        Observer.Expect('cmp_Hi_gt_empty', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_3()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Hi_eq_Hi', 36);
        Run(Observer, 'cmp_Hi_ne_Hi', 37);
        Run(Observer, 'cmp_Hi_lt_Hi', 38);
        Run(Observer, 'cmp_Hi_gt_Hi', 39);
        Run(Observer, 'cmp_Hi_eq_Lo', 40);
        Run(Observer, 'cmp_Hi_ne_Lo', 41);
        Run(Observer, 'cmp_Hi_lt_Lo', 42);
        Run(Observer, 'cmp_Hi_gt_Lo', 43);
        Observer.Expect('cmp_Hi_eq_Hi', 'Yes');
        Observer.Expect('cmp_Hi_ne_Hi', 'No');
        Observer.Expect('cmp_Hi_lt_Hi', 'No');
        Observer.Expect('cmp_Hi_gt_Hi', 'No');
        Observer.Expect('cmp_Hi_eq_Lo', 'No');
        Observer.Expect('cmp_Hi_ne_Lo', 'Yes');
        Observer.Expect('cmp_Hi_lt_Lo', 'Yes');
        Observer.Expect('cmp_Hi_gt_Lo', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_4()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Lo_eq_Hi', 44);
        Run(Observer, 'cmp_Lo_ne_Hi', 45);
        Run(Observer, 'cmp_Lo_lt_Hi', 46);
        Run(Observer, 'cmp_Lo_gt_Hi', 47);
        Run(Observer, 'cmp_Hi_eq_Pair', 48);
        Run(Observer, 'cmp_Hi_ne_Pair', 49);
        Run(Observer, 'cmp_Hi_lt_Pair', 50);
        Run(Observer, 'cmp_Hi_gt_Pair', 51);
        Observer.Expect('cmp_Lo_eq_Hi', 'No');
        Observer.Expect('cmp_Lo_ne_Hi', 'Yes');
        Observer.Expect('cmp_Lo_lt_Hi', 'No');
        Observer.Expect('cmp_Lo_gt_Hi', 'Yes');
        Observer.Expect('cmp_Hi_eq_Pair', 'No');
        Observer.Expect('cmp_Hi_ne_Pair', 'Yes');
        Observer.Expect('cmp_Hi_lt_Pair', 'No');
        Observer.Expect('cmp_Hi_gt_Pair', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_5()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Pair_eq_Hi', 52);
        Run(Observer, 'cmp_Pair_ne_Hi', 53);
        Run(Observer, 'cmp_Pair_lt_Hi', 54);
        Run(Observer, 'cmp_Pair_gt_Hi', 55);
        Run(Observer, 'cmp_Lo_eq_Pair', 56);
        Run(Observer, 'cmp_Lo_ne_Pair', 57);
        Run(Observer, 'cmp_Lo_lt_Pair', 58);
        Run(Observer, 'cmp_Lo_gt_Pair', 59);
        Observer.Expect('cmp_Pair_eq_Hi', 'No');
        Observer.Expect('cmp_Pair_ne_Hi', 'Yes');
        Observer.Expect('cmp_Pair_lt_Hi', 'Yes');
        Observer.Expect('cmp_Pair_gt_Hi', 'No');
        Observer.Expect('cmp_Lo_eq_Pair', 'No');
        Observer.Expect('cmp_Lo_ne_Pair', 'Yes');
        Observer.Expect('cmp_Lo_lt_Pair', 'No');
        Observer.Expect('cmp_Lo_gt_Pair', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_6()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Pair_eq_Lo', 60);
        Run(Observer, 'cmp_Pair_ne_Lo', 61);
        Run(Observer, 'cmp_Pair_lt_Lo', 62);
        Run(Observer, 'cmp_Pair_gt_Lo', 63);
        Run(Observer, 'cmp_Hi_eq_Rep', 64);
        Run(Observer, 'cmp_Hi_ne_Rep', 65);
        Run(Observer, 'cmp_Hi_lt_Rep', 66);
        Run(Observer, 'cmp_Hi_gt_Rep', 67);
        Observer.Expect('cmp_Pair_eq_Lo', 'No');
        Observer.Expect('cmp_Pair_ne_Lo', 'Yes');
        Observer.Expect('cmp_Pair_lt_Lo', 'Yes');
        Observer.Expect('cmp_Pair_gt_Lo', 'No');
        Observer.Expect('cmp_Hi_eq_Rep', 'No');
        Observer.Expect('cmp_Hi_ne_Rep', 'Yes');
        Observer.Expect('cmp_Hi_lt_Rep', 'Yes');
        Observer.Expect('cmp_Hi_gt_Rep', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_7()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Rep_eq_Hi', 68);
        Run(Observer, 'cmp_Rep_ne_Hi', 69);
        Run(Observer, 'cmp_Rep_lt_Hi', 70);
        Run(Observer, 'cmp_Rep_gt_Hi', 71);
        Run(Observer, 'cmp_Lo_eq_a', 72);
        Run(Observer, 'cmp_Lo_ne_a', 73);
        Run(Observer, 'cmp_Lo_lt_a', 74);
        Run(Observer, 'cmp_Lo_gt_a', 75);
        Observer.Expect('cmp_Rep_eq_Hi', 'No');
        Observer.Expect('cmp_Rep_ne_Hi', 'Yes');
        Observer.Expect('cmp_Rep_lt_Hi', 'No');
        Observer.Expect('cmp_Rep_gt_Hi', 'Yes');
        Observer.Expect('cmp_Lo_eq_a', 'No');
        Observer.Expect('cmp_Lo_ne_a', 'Yes');
        Observer.Expect('cmp_Lo_lt_a', 'No');
        Observer.Expect('cmp_Lo_gt_a', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_8()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Lo_eq_Rep', 76);
        Run(Observer, 'cmp_Lo_ne_Rep', 77);
        Run(Observer, 'cmp_Lo_lt_Rep', 78);
        Run(Observer, 'cmp_Lo_gt_Rep', 79);
        Run(Observer, 'cmp_Hi_eq_abc', 80);
        Run(Observer, 'cmp_Hi_ne_abc', 81);
        Run(Observer, 'cmp_Hi_lt_abc', 82);
        Run(Observer, 'cmp_Hi_gt_abc', 83);
        Observer.Expect('cmp_Lo_eq_Rep', 'No');
        Observer.Expect('cmp_Lo_ne_Rep', 'Yes');
        Observer.Expect('cmp_Lo_lt_Rep', 'Yes');
        Observer.Expect('cmp_Lo_gt_Rep', 'No');
        Observer.Expect('cmp_Hi_eq_abc', 'No');
        Observer.Expect('cmp_Hi_ne_abc', 'Yes');
        Observer.Expect('cmp_Hi_lt_abc', 'No');
        Observer.Expect('cmp_Hi_gt_abc', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_9()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Lo_eq_abc', 84);
        Run(Observer, 'cmp_Lo_ne_abc', 85);
        Run(Observer, 'cmp_Lo_lt_abc', 86);
        Run(Observer, 'cmp_Lo_gt_abc', 87);
        Run(Observer, 'cmp_Hi_eq_Copy', 88);
        Run(Observer, 'cmp_Hi_ne_Copy', 89);
        Run(Observer, 'cmp_Hi_lt_Copy', 90);
        Run(Observer, 'cmp_Hi_gt_Copy', 91);
        Observer.Expect('cmp_Lo_eq_abc', 'No');
        Observer.Expect('cmp_Lo_ne_abc', 'Yes');
        Observer.Expect('cmp_Lo_lt_abc', 'No');
        Observer.Expect('cmp_Lo_gt_abc', 'Yes');
        Observer.Expect('cmp_Hi_eq_Copy', 'Yes');
        Observer.Expect('cmp_Hi_ne_Copy', 'No');
        Observer.Expect('cmp_Hi_lt_Copy', 'No');
        Observer.Expect('cmp_Hi_gt_Copy', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_cmp_10()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'cmp_Lo_eq_~', 92);
        Run(Observer, 'cmp_Lo_ne_~', 93);
        Run(Observer, 'cmp_Lo_lt_~', 94);
        Run(Observer, 'cmp_Lo_gt_~', 95);
        Run(Observer, 'cmp_Hi_eq_aHi', 96);
        Run(Observer, 'cmp_Hi_ne_aHi', 97);
        Run(Observer, 'cmp_Hi_lt_aHi', 98);
        Run(Observer, 'cmp_Hi_gt_aHi', 99);
        Observer.Expect('cmp_Lo_eq_~', 'No');
        Observer.Expect('cmp_Lo_ne_~', 'Yes');
        Observer.Expect('cmp_Lo_lt_~', 'No');
        Observer.Expect('cmp_Lo_gt_~', 'Yes');
        Observer.Expect('cmp_Hi_eq_aHi', 'No');
        Observer.Expect('cmp_Hi_ne_aHi', 'Yes');
        Observer.Expect('cmp_Hi_lt_aHi', 'No');
        Observer.Expect('cmp_Hi_gt_aHi', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_1()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'strlen', 100);
        Run(Observer, 'copystr_hi', 101);
        Run(Observer, 'copystr_a_hi_b', 102);
        Run(Observer, 'strpos_pair_hi', 103);
        Run(Observer, 'strpos_pair_lo', 104);
        Run(Observer, 'strpos_ahib_hi', 105);
        Run(Observer, 'strpos_hi_a', 106);
        Run(Observer, 'strpos_hi_hi', 107);
        Observer.Expect('strlen', '1');
        Observer.Expect('copystr_hi', '1:55357');
        Observer.Expect('copystr_a_hi_b', '1:55357');
        Observer.Expect('strpos_pair_hi', '1');
        Observer.Expect('strpos_pair_lo', '2');
        Observer.Expect('strpos_ahib_hi', '2');
        Observer.Expect('strpos_hi_a', '0');
        Observer.Expect('strpos_hi_hi', '1');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_2()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'strpos_hi_lo', 108);
        Run(Observer, 'indexof_pair_hi', 109);
        Run(Observer, 'indexof_pair_lo', 110);
        Run(Observer, 'indexof_ahi_hi', 111);
        Run(Observer, 'indexof_hi_a', 112);
        Run(Observer, 'indexof_pair_lo_start2', 113);
        Run(Observer, 'contains_pair_hi', 114);
        Run(Observer, 'contains_pair_lo', 115);
        Observer.Expect('strpos_hi_lo', '0');
        Observer.Expect('indexof_pair_hi', '1');
        Observer.Expect('indexof_pair_lo', '2');
        Observer.Expect('indexof_ahi_hi', '2');
        Observer.Expect('indexof_hi_a', '0');
        Observer.Expect('indexof_pair_lo_start2', '2');
        Observer.Expect('contains_pair_hi', 'Yes');
        Observer.Expect('contains_pair_lo', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_3()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'contains_hi_hi', 116);
        Run(Observer, 'contains_hi_lo', 117);
        Run(Observer, 'contains_ahi_a', 118);
        Run(Observer, 'startswith_pair_hi', 119);
        Run(Observer, 'startswith_pair_lo', 120);
        Run(Observer, 'startswith_hi_hi', 121);
        Run(Observer, 'startswith_hia_hi', 122);
        Run(Observer, 'endswith_pair_lo', 123);
        Observer.Expect('contains_hi_hi', 'Yes');
        Observer.Expect('contains_hi_lo', 'No');
        Observer.Expect('contains_ahi_a', 'Yes');
        Observer.Expect('startswith_pair_hi', 'Yes');
        Observer.Expect('startswith_pair_lo', 'No');
        Observer.Expect('startswith_hi_hi', 'Yes');
        Observer.Expect('startswith_hia_hi', 'Yes');
        Observer.Expect('endswith_pair_lo', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_4()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'endswith_pair_hi', 124);
        Run(Observer, 'endswith_ahi_hi', 125);
        Run(Observer, 'endswith_hi_a', 126);
        Run(Observer, 'uppercase_hi', 127);
        Run(Observer, 'lowercase_hi', 128);
        Run(Observer, 'toupper_hi', 129);
        Run(Observer, 'tolower_lo', 130);
        Run(Observer, 'uppercase_alob', 131);
        Observer.Expect('endswith_pair_hi', 'No');
        Observer.Expect('endswith_ahi_hi', 'Yes');
        Observer.Expect('endswith_hi_a', 'No');
        Observer.Expect('uppercase_hi', '1:55357');
        Observer.Expect('lowercase_hi', '1:55357');
        Observer.Expect('toupper_hi', '1:55357');
        Observer.Expect('tolower_lo', '1:56832');
        Observer.Expect('uppercase_alob', '3:65,56832,66');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_5()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'convertstr_hi_ab', 132);
        Run(Observer, 'convertstr_hi_hi_x', 133);
        Run(Observer, 'convertstr_abc_b_hi', 134);
        Run(Observer, 'convertstr_ahi_hi_x', 135);
        Run(Observer, 'convertstr_pair_hi_x', 136);
        Run(Observer, 'delchr_hi_hi', 137);
        Run(Observer, 'delchr_pair_hi', 138);
        Run(Observer, 'delchr_pair_lo', 139);
        Observer.Expect('convertstr_hi_ab', '1:55357');
        Observer.Expect('convertstr_hi_hi_x', '1:120');
        Observer.Expect('convertstr_abc_b_hi', '3:97,55357,99');
        Observer.Expect('convertstr_ahi_hi_x', '2:97,120');
        Observer.Expect('convertstr_pair_hi_x', '2:120,56832');
        Observer.Expect('delchr_hi_hi', '0:');
        Observer.Expect('delchr_pair_hi', '1:56832');
        Observer.Expect('delchr_pair_lo', '1:55357');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_6()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'delchr_hi_a', 140);
        Run(Observer, 'delchr_ahia_trim_a', 141);
        Run(Observer, 'delchr_hia_trail_a', 142);
        Run(Observer, 'delchr_hi_lead_hi', 143);
        Run(Observer, 'padstr_hi_3', 144);
        Run(Observer, 'padstr_a_3_hi', 145);
        Run(Observer, 'padstr_hi_1', 146);
        Run(Observer, 'padstr_pair_5', 147);
        Observer.Expect('delchr_hi_a', '1:55357');
        Observer.Expect('delchr_ahia_trim_a', '1:55357');
        Observer.Expect('delchr_hia_trail_a', '1:55357');
        Observer.Expect('delchr_hi_lead_hi', '1:97');
        Observer.Expect('padstr_hi_3', '3:55357,120,120');
        Observer.Expect('padstr_a_3_hi', '3:97,55357,55357');
        Observer.Expect('padstr_hi_1', '1:55357');
        Observer.Expect('padstr_pair_5', '5:55357,56832,120,120,120');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_7()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'selectstr_1', 148);
        Run(Observer, 'selectstr_2', 149);
        Run(Observer, 'selectstr_2_ahi', 150);
        Run(Observer, 'selectstr_1_hi', 151);
        Run(Observer, 'format_hi', 152);
        Run(Observer, 'format_lo', 153);
        Run(Observer, 'format_hi_lo', 154);
        Run(Observer, 'format_hi_len5', 155);
        Observer.Expect('selectstr_1', '1:55357');
        Observer.Expect('selectstr_2', '1:56832');
        Observer.Expect('selectstr_2_ahi', '1:55357');
        Observer.Expect('selectstr_1_hi', '1:55357');
        Observer.Expect('format_hi', '1:55357');
        Observer.Expect('format_lo', '1:56832');
        Observer.Expect('format_hi_lo', '2:55357,56832');
        Observer.Expect('format_hi_len5', '5:55357,32,32,32,32');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_8()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'delstr_hi_1_1', 156);
        Run(Observer, 'insstr_ab_hi_2', 157);
        Run(Observer, 'insstr_hi_x_1', 158);
        Run(Observer, 'incstr_hi', 159);
        Run(Observer, 'incstr_ahi', 160);
        Run(Observer, 'replace_b_hi', 161);
        Run(Observer, 'replace_ahib_hi_x', 162);
        Run(Observer, 'replace_pair_hi_x', 163);
        Observer.Expect('delstr_hi_1_1', '0:');
        Observer.Expect('insstr_ab_hi_2', '3:97,55357,98');
        Observer.Expect('insstr_hi_x_1', '2:120,55357');
        Observer.Expect('incstr_hi', '0:');
        Observer.Expect('incstr_ahi', '0:');
        Observer.Expect('replace_b_hi', '3:97,55357,99');
        Observer.Expect('replace_ahib_hi_x', '3:97,120,98');
        Observer.Expect('replace_pair_hi_x', '2:120,56832');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_9()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'replace_pair_lo_x', 164);
        Run(Observer, 'replace_hi_hi_lo', 165);
        Run(Observer, 'replace_hi_a_b', 166);
        Run(Observer, 'trim_hi', 167);
        Run(Observer, 'trim_sp_hi_sp', 168);
        Run(Observer, 'trimend_ahi_hi', 169);
        Run(Observer, 'trimstart_hia_hi', 170);
        Run(Observer, 'trim_hi_hi', 171);
        Observer.Expect('replace_pair_lo_x', '2:55357,120');
        Observer.Expect('replace_hi_hi_lo', '1:56832');
        Observer.Expect('replace_hi_a_b', '1:55357');
        Observer.Expect('trim_hi', '1:55357');
        Observer.Expect('trim_sp_hi_sp', '1:55357');
        Observer.Expect('trimend_ahi_hi', '1:97');
        Observer.Expect('trimstart_hia_hi', '1:97');
        Observer.Expect('trim_hi_hi', '1:55357');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_10()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'substring_ahi_2', 172);
        Run(Observer, 'substring_hi_1_1', 173);
        Run(Observer, 'padleft_hi_3', 174);
        Run(Observer, 'padright_a_3_ch', 175);
        Run(Observer, 'padleft_a_3_ch', 176);
        Run(Observer, 'concat_hi_a', 177);
        Run(Observer, 'concat_a_hi', 178);
        Run(Observer, 'concat_hi_lo', 179);
        Observer.Expect('substring_ahi_2', '1:55357');
        Observer.Expect('substring_hi_1_1', '1:55357');
        Observer.Expect('padleft_hi_3', '3:120,120,55357');
        Observer.Expect('padright_a_3_ch', '3:55357,55357,55357');
        Observer.Expect('padleft_a_3_ch', '3:55357,55357,55357');
        Observer.Expect('concat_hi_a', '2:55357,97');
        Observer.Expect('concat_a_hi', '2:97,55357');
        Observer.Expect('concat_hi_lo', '2:55357,56832');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_11()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'concat_lo_hi', 180);
        Run(Observer, 'concat_hi_hi', 181);
        Run(Observer, 'substno_1', 182);
        Run(Observer, 'substno_12', 183);
        Run(Observer, 'substno_x1y', 184);
        Run(Observer, 'substno_hi_fmt', 185);
        Run(Observer, 'substno_1_pair', 186);
        Run(Observer, 't1_assign_hi', 187);
        Observer.Expect('concat_lo_hi', '2:56832,55357');
        Observer.Expect('concat_hi_hi', '2:55357,55357');
        Observer.Expect('substno_1', '1:55357');
        Observer.Expect('substno_12', '2:55357,56832');
        Observer.Expect('substno_x1y', '3:120,56832,121');
        Observer.Expect('substno_hi_fmt', '1:55357');
        Observer.Expect('substno_1_pair', '2:55357,56832');
        Observer.Expect('t1_assign_hi', '1:55357');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_12()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 't1_assign_lo', 188);
        Run(Observer, 't1_assign_pair', 189);
        Run(Observer, 't1_char_assign', 190);
        Run(Observer, 'code10_assign_hi', 191);
        Run(Observer, 'code10_assign_lo', 192);
        Run(Observer, 'code10_eq_hi', 193);
        Run(Observer, 'code10_copystr', 194);
        Run(Observer, 'code10_a_hi', 195);
        Observer.Expect('t1_assign_lo', '1:56832');
        Observer.Expect('t1_assign_pair', 'ERR:The length of the string is 2, but it must be less than or equal to 1 characters. Value: \uD83D\uDE00');
        Observer.Expect('t1_char_assign', '1:55357');
        Observer.Expect('code10_assign_hi', '1:55357');
        Observer.Expect('code10_assign_lo', '1:56832');
        Observer.Expect('code10_eq_hi', 'Yes');
        Observer.Expect('code10_copystr', '1:55357');
        Observer.Expect('code10_a_hi', '2:65,55357');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_13()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'hi_char_int', 196);
        Run(Observer, 'lo_char_int', 197);
        Run(Observer, 'eval_int_hi', 198);
        Run(Observer, 'eval_text_hi', 199);
        Run(Observer, 'eval_char_hi', 200);
        Run(Observer, 'eval_code_hi', 201);
        Run(Observer, 'eval_bool_hi', 202);
        Run(Observer, 'eval_text_lo', 203);
        Observer.Expect('hi_char_int', '55357');
        Observer.Expect('lo_char_int', '56832');
        Observer.Expect('eval_int_hi', 'No|0');
        Observer.Expect('eval_text_hi', 'Yes|1:55357');
        Observer.Expect('eval_char_hi', 'Yes|55357');
        Observer.Expect('eval_code_hi', 'Yes|1:55357');
        Observer.Expect('eval_bool_hi', 'No|No');
        Observer.Expect('eval_text_lo', 'Yes|1:56832');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_14()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'indexofany_ahib', 204);
        Run(Observer, 'split_hi_hi', 205);
        Run(Observer, 'split_ahib_hi', 206);
        Run(Observer, 'split_pair_hi', 207);
        Run(Observer, 'split_alob_lo', 208);
        Run(Observer, 'split_comma_ahi', 209);
        Run(Observer, 'split_hi_comma', 210);
        Run(Observer, 'list_contains_hi', 211);
        Observer.Expect('indexofany_ahib', '0|2');
        Observer.Expect('split_hi_hi', '2|0:|0:');
        Observer.Expect('split_ahib_hi', '2|1:97|1:98');
        Observer.Expect('split_pair_hi', '2|0:|1:56832');
        Observer.Expect('split_alob_lo', '2|1:97|1:98');
        Observer.Expect('split_comma_ahi', '2|1:97|1:55357');
        Observer.Expect('split_hi_comma', '1|1:55357');
        Observer.Expect('list_contains_hi', 'YesNoNo1');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_15()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'list_add_both', 212);
        Run(Observer, 'dict_hi_lo', 213);
        Run(Observer, 'dict_keys_codes', 214);
        Run(Observer, 'dict_hi_dup', 215);
        Run(Observer, 'json_setvalue_hi', 216);
        Run(Observer, 'json_setvalue_lo', 217);
        Run(Observer, 'json_obj_add_hi', 218);
        Run(Observer, 'json_obj_get_hi', 219);
        Observer.Expect('list_add_both', '22No');
        Observer.Expect('dict_hi_lo', '2YesYesNo2');
        Observer.Expect('dict_keys_codes', '1|1:55357');
        Observer.Expect('dict_hi_dup', 'No');
        Observer.Expect('json_setvalue_hi', '3:34,55357,34|1:55357');
        Observer.Expect('json_setvalue_lo', '3:34,56832,34|1:56832');
        Observer.Expect('json_obj_add_hi', '9:123,34,107,34,58,34,55357,34,125');
        Observer.Expect('json_obj_get_hi', '1:55357');
        Observer.Verify();
    end;

    [Test]
    procedure Ops_fn_16()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Run(Observer, 'json_arr_add_lo', 220);
        Run(Observer, 'json_obj_key_hi', 221);
        Run(Observer, 'json_read_escaped_hi', 222);
        Run(Observer, 'json_read_escaped_lo', 223);
        Run(Observer, 'json_obj_hi_text', 224);
        Run(Observer, 'base64_hi', 225);
        Run(Observer, 'base64_lo', 226);
        Run(Observer, 'base64_roundtrip_hi', 227);
        Observer.Expect('json_arr_add_lo', '5:91,34,56832,34,93');
        Observer.Expect('json_obj_key_hi', '9:123,34,55357,34,58,34,120,34,125Yes');
        Observer.Expect('json_read_escaped_hi', 'Yes|1:65533');
        Observer.Expect('json_read_escaped_lo', 'Yes|1:65533');
        Observer.Expect('json_obj_hi_text', 'Yes|1:55357');
        Observer.Expect('base64_hi', '77+9');
        Observer.Expect('base64_lo', '77+9');
        Observer.Expect('base64_roundtrip_hi', '1:65533');
        Observer.Verify();
    end;

    local procedure Attempt(Id: Integer; Src: Integer): Text
    begin
        if Op(Id, Source(Src, 1), Source(Src, 2)) then
            exit(Res);
        exit('ERR:' + GetLastErrorText());
    end;

    // The same operations on halves built from a Char variable give the same result as the halves cut from a pair.
    [Test]
    procedure Source_CharVariable_SameAsCopyStr()
    var
        Observer: Codeunit "Probe Observer";
        Id: Integer;
        Differing: Text;
    begin
        for Id := 1 to 229 do
            if Attempt(Id, 1) <> Attempt(Id, 2) then
                Differing += ',' + Format(Id);
        Observer.Observe('differing ids', Differing);
        Observer.Expect('differing ids', '');
        Observer.Verify();
    end;

    [Test]
    procedure Error_Text_WithHalf()
    var
        Observer: Codeunit "Probe Observer";
        Hi: Text;
        Lo: Text;
    begin
        Hi := Source(1, 1);
        Lo := Source(1, 2);
        asserterror Error(Hi);
        Observer.Observe('error_hi', Codes(GetLastErrorText()));
        asserterror Error(Lo);
        Observer.Observe('error_lo', Codes(GetLastErrorText()));
        asserterror Error('x%1y', Hi);
        Observer.Observe('error_fmt_hi', Codes(GetLastErrorText()));
        asserterror Error(Hi + 'a');
        Observer.Observe('error_hi_a', Codes(GetLastErrorText()));
        Observer.Expect('error_hi', '1:55357');
        Observer.Expect('error_lo', '1:56832');
        Observer.Expect('error_fmt_hi', '3:120,55357,121');
        Observer.Expect('error_hi_a', '2:55357,97');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('MessageCapture')]
    procedure Message_Text_WithHalf()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Message(Source(1, 1));
        Observer.Observe('message_hi', Codes(MsgText));
        Message(Source(1, 2));
        Observer.Observe('message_lo', Codes(MsgText));
        Message('x%1y', Source(1, 1));
        Observer.Observe('message_fmt_hi', Codes(MsgText));
        Observer.Expect('message_hi', '1:55357');
        Observer.Expect('message_lo', '1:56832');
        Observer.Expect('message_fmt_hi', '3:120,55357,121');
        Observer.Verify();
    end;

    [MessageHandler]
    procedure MessageCapture(Msg: Text[1024])
    begin
        MsgText := Msg;
    end;

    local procedure Fresh(var Rec: Record "LT SH Rec")
    begin
        Rec.Reset();
        Rec.DeleteAll();
    end;

    local procedure Bytes(var Rec: Record "LT SH Rec"): Text
    var
        Ins: InStream;
        B: Byte;
        R: Text;
        n: Integer;
    begin
        Rec.CalcFields(Data);
        Rec.Data.CreateInStream(Ins);
        while not Ins.EOS() do begin
            Ins.Read(B);
            n := B;
            R += Format(n) + ' ';
        end;
        exit(R);
    end;

    local procedure WriteRead(Value: Text; Enc: Integer): Text
    var
        Rec: Record "LT SH Rec";
        Out: OutStream;
        Ins: InStream;
        T: Text;
    begin
        Fresh(Rec);
        Rec.Name := 'A';
        Rec.Insert();
        case Enc of
            0:
                Rec.Data.CreateOutStream(Out);
            1:
                Rec.Data.CreateOutStream(Out, TextEncoding::UTF8);
            2:
                Rec.Data.CreateOutStream(Out, TextEncoding::UTF16);
            3:
                Rec.Data.CreateOutStream(Out, TextEncoding::Windows);
        end;
        Out.WriteText(Value);
        Rec.Modify();
        case Enc of
            0:
                Rec.Data.CreateInStream(Ins);
            1:
                Rec.Data.CreateInStream(Ins, TextEncoding::UTF8);
            2:
                Rec.Data.CreateInStream(Ins, TextEncoding::UTF16);
            3:
                Rec.Data.CreateInStream(Ins, TextEncoding::Windows);
        end;
        Ins.ReadText(T);
        exit('bytes=' + Bytes(Rec) + '|read=' + Codes(T));
    end;

    local procedure ObserveStreams(var Observer: Codeunit "Probe Observer"; Name: Text; Value: Text)
    begin
        Observer.Observe(Name + ' none', WriteRead(Value, 0));
        Observer.Observe(Name + ' utf8', WriteRead(Value, 1));
        Observer.Observe(Name + ' utf16', WriteRead(Value, 2));
        Observer.Observe(Name + ' windows', WriteRead(Value, 3));
    end;

    [Test]
    procedure Stream_WriteText_HalfHi()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ObserveStreams(Observer, 'hi', Source(1, 1));
        ObserveStreams(Observer, 'ahib', 'a' + Source(1, 1) + 'b');
        Observer.Expect('hi none', 'bytes=63 |read=1:63');
        Observer.Expect('hi utf8', 'bytes=239 191 189 |read=1:65533');
        Observer.Expect('hi utf16', 'bytes=253 255 |read=1:65533');
        Observer.Expect('hi windows', 'bytes=63 |read=1:63');
        Observer.Expect('ahib none', 'bytes=97 63 98 |read=3:97,63,98');
        Observer.Expect('ahib utf8', 'bytes=97 239 191 189 98 |read=3:97,65533,98');
        Observer.Expect('ahib utf16', 'bytes=97 0 253 255 98 0 |read=3:97,65533,98');
        Observer.Expect('ahib windows', 'bytes=97 63 98 |read=3:97,63,98');
        Observer.Verify();
    end;

    [Test]
    procedure Stream_WriteText_HalfLo()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ObserveStreams(Observer, 'lo', Source(1, 2));
        ObserveStreams(Observer, 'lo_hi', Source(1, 2) + Source(1, 1));
        Observer.Expect('lo none', 'bytes=63 |read=1:63');
        Observer.Expect('lo utf8', 'bytes=239 191 189 |read=1:65533');
        Observer.Expect('lo utf16', 'bytes=253 255 |read=1:65533');
        Observer.Expect('lo windows', 'bytes=63 |read=1:63');
        Observer.Expect('lo_hi none', 'bytes=63 63 |read=2:63,63');
        Observer.Expect('lo_hi utf8', 'bytes=239 191 189 239 191 189 |read=2:65533,65533');
        Observer.Expect('lo_hi utf16', 'bytes=253 255 253 255 |read=2:65533,65533');
        Observer.Expect('lo_hi windows', 'bytes=63 63 |read=2:63,63');
        Observer.Verify();
    end;

    [Test]
    procedure Table_Fields_WithHalf()
    var
        Rec: Record "LT SH Rec";
        Observer: Codeunit "Probe Observer";
        Hi: Text;
        Lo: Text;
    begin
        Hi := Source(1, 1);
        Lo := Source(1, 2);
        Fresh(Rec);
        Rec.Name := 'A';
        Rec.T50 := Hi;
        Rec.T1 := Lo;
        Rec.C10 := Hi;
        Observer.Observe('insert', Format(Rec.Insert()));
        Observer.Observe('after insert', Codes(Rec.T50) + ' ' + Codes(Rec.T1) + ' ' + Codes(Rec.C10));
        Clear(Rec);
        Observer.Observe('get', Format(Rec.Get('A')));
        Observer.Observe('read back', Codes(Rec.T50) + ' ' + Codes(Rec.T1) + ' ' + Codes(Rec.C10));
        Rec.SetRange(T50, Hi);
        Observer.Observe('filter hi count', Format(Rec.Count()));
        Rec.SetRange(T50, Lo);
        Observer.Observe('filter lo count', Format(Rec.Count()));
        Rec.SetRange(T50);
        Rec.SetRange(T50, 'a');
        Observer.Observe('filter a count', Format(Rec.Count()));
        Rec.SetRange(T50);
        Rec.T50 := Lo + Hi;
        Rec.Modify();
        Clear(Rec);
        Rec.Get('A');
        Observer.Observe('modified', Codes(Rec.T50));
        Observer.Expect('insert', 'Yes');
        Observer.Expect('after insert', '1:55357 1:56832 1:55357');
        Observer.Expect('get', 'Yes');
        Observer.Expect('read back', '1:65533 1:65533 1:65533');
        Observer.Expect('filter hi count', '1');
        Observer.Expect('filter lo count', '1');
        Observer.Expect('filter a count', '0');
        Observer.Expect('modified', '2:65533,65533');
        Observer.Verify();
    end;

    [Test]
    procedure Table_Key_WithHalf()
    var
        Rec: Record "LT SH Rec";
        Observer: Codeunit "Probe Observer";
    begin
        Fresh(Rec);
        Rec.Name := CopyStr(Source(1, 1), 1, 20);
        Observer.Observe('insert hi key', Format(Rec.Insert()));
        Rec.Name := CopyStr(Source(1, 2), 1, 20);
        Observer.Observe('insert lo key', Format(Rec.Insert()));
        Rec.Name := CopyStr(Source(1, 1), 1, 20);
        Observer.Observe('insert hi key again', Format(Rec.Insert(false)));
        Observer.Observe('count', Format(Rec.Count()));
        Rec.Name := CopyStr(Source(1, 2), 1, 20);
        Observer.Observe('get lo', Format(Rec.Get(Rec.Name)));
        Observer.Observe('name codes', Codes(Rec.Name));
        Observer.Expect('insert hi key', 'Yes');
        Observer.Expect('insert lo key', 'No');
        Observer.Expect('insert hi key again', 'No');
        Observer.Expect('count', '1');
        Observer.Expect('get lo', 'Yes');
        Observer.Expect('name codes', '1:65533');
        Observer.Verify();
    end;
}
