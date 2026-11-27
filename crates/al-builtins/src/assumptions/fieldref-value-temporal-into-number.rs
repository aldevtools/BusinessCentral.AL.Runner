FieldRefValueTemporalIntoNumber => {
    id: "fieldref-value-temporal-into-number",
    assumed: "On every supported BC major, in an en-US session, FieldRef.Value() of a Date, Time or DateTime field converted straight into an Integer, BigInteger, Decimal, Byte or Char (a local, a var parameter, an argument, a named return value, exit, a global, a record field, an array element, List.Add of the list's element type, Abs as a Decimal) raises 'Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav<Date|Time|DateTime> to <System.Int32|System.Int64|Microsoft.Dynamics.Nav.Runtime.Decimal18|System.Byte|System.Char>.' (Conversion). A Date or Time into a Duration, Option or Boolean and a DateTime into a Duration convert as the Variant conversion does; a DateTime into a Boolean raises the Int32 error, into an Option from BC 28 'The value \"<Format>\" can't be evaluated into type Option.' (Evaluate:Conversion), on BC 27 the Int32 error. The probe measured a blank and a set value of each kind; the rule is taken for every value of the kind.",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1629 (merged at 778b8fc1, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcTemporalIntoNumberPlaces.al:296-680, the <Source>_Targets and <Source>_Places tests for 0D, 2026-01-15, 0T, 12:00:00, 0DT and 2026-01-15 12:00, measured on BC 29.0 and asserted by its CI on 27.5, 28 and 29; al-runner issue 9265 measured the blank Date into an Integer on BC 28.4.",
    probe: "Pin corpus PR 1629 with the next corpus bump. Not measured: values of each kind other than the blank and the set one (taken by the rule), another session language or format region (refused), I += FR.Value() (does not compile).",
    awaiting: Awaiting {
        issues: &[9265],
        files: &["tests/al-language/recordref-fieldref/fieldref-value-convert/FvcTemporalIntoNumberPlaces.al"],
        since: "2026-11-27",
    },
}
