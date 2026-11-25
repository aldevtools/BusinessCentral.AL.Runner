// Neighbor tests of the review of al-runner PR 8894: routes a, b, c, e, f and gr of the values whose probe test in
// SrcFilterConvertKinds.al also runs route v (FieldRef.Value :=), which al-runner refuses for some of them. Same expected values
// as the probe test (corpus PR 1552), route v left out; no test here is refused, so a failure here is never hidden.
codeunit 57413 "SFK Split Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Txt15_Int_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '1.5';
        Probe.FiltersTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code1000_Int_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '1,000';
        Probe.FiltersCode(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Txt15_Big_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '1.5';
        Probe.FiltersTxt(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code1000_Big_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '1,000';
        Probe.FiltersCode(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code0_Bool_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '0';
        Probe.FiltersCode(7, X);
        Probe.Expect('a', 'ok: No');
        Probe.Expect('b', 'ok: No');
        Probe.Expect('c', 'ok: No');
        Probe.Expect('e', 'ok: No');
        Probe.Expect('f', 'ok: No');
        Probe.Expect('gr', 'ok: No');
        Probe.Verify();
    end;

    [Test]
    procedure Int0_Bool_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 0;
        Probe.FiltersInt(7, X);
        Probe.Expect('a', 'ok: No');
        Probe.Expect('b', 'ok: No');
        Probe.Expect('c', 'ok: No');
        Probe.Expect('e', 'ok: No');
        Probe.Expect('f', 'ok: No');
        Probe.Expect('gr', 'ok: No');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_G_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.FiltersCode(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code123_G_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.FiltersCode(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_G_Filters()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.FiltersTxt(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Verify();
    end;
}
