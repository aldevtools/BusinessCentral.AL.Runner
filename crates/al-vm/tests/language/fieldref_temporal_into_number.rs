//! `FieldRef.Value()` of a Date, Time or DateTime field converted straight into a number (issues 9265 and 9305): the
//! cells of corpus PR 1629 (merged at 778b8fc1),
//! recordref-fieldref/fieldref-value-convert/FvcTemporalIntoNumberPlaces.al:296-680 (assumption
//! `fieldref-value-temporal-into-number`). Into an Integer, BigInteger, Decimal, Byte or Char the conversion error
//! naming the .NET type, in every place the probe measured; into a Duration, Option or Boolean the Variant conversion,
//! except a DateTime. The same value through a Variant converts as pinned
//! (types/date-time/Y62UndefinedDate.al:292,295).

const SRC: &str = r#"
table 50716 TemporalRow { fields { field(1; K; Integer) { } field(2; D; Date) { } field(3; T; Time) { } field(4; Dt; DateTime) { } field(5; IntT; Integer) { } } keys { key(PK; K) { } } }
codeunit 50768 C {
    var GlobalInt: Integer;
    local procedure Source(var RR: RecordRef; var FR: FieldRef; Src: Text)
    begin
        RR.Open(Database::TemporalRow);
        case Src of
            'date0': FR := RR.Field(2);
            'date1': begin RR.Field(2).Value := 20260115D; FR := RR.Field(2); end;
            'date_early': begin RR.Field(2).Value := 17530101D; FR := RR.Field(2); end;
            'time0': FR := RR.Field(3);
            'time1': begin RR.Field(3).Value := 120000T; FR := RR.Field(3); end;
            'datetime0': FR := RR.Field(4);
            'datetime1': begin RR.Field(4).Value := CreateDateTime(20260115D, 120000T); FR := RR.Field(4); end;
            'date_late': begin RR.Field(2).Value := 99991231D; FR := RR.Field(2); end;
            'date_closing': begin RR.Field(2).Value := ClosingDate(20260115D); FR := RR.Field(2); end;
            'time_morning': begin RR.Field(3).Value := 083015T; FR := RR.Field(3); end;
            'time_second': begin RR.Field(3).Value := 000001T; FR := RR.Field(3); end;
            'datetime_early': begin RR.Field(4).Value := CreateDateTime(17530101D, 000001T); FR := RR.Field(4); end;
        end;
    end;
    local procedure Outcome(Ok: Boolean; Result: Text): Text
    begin
        if Ok then exit('ok: ' + Result);
        exit('E: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
    end;
    local procedure AssignVar(var FR: FieldRef; var I: Integer) begin I := FR.Value(); end;
    local procedure TakeInt(I: Integer): Text begin exit(Format(I, 0, 9)); end;
    local procedure RetNamed(var FR: FieldRef) R: Integer begin R := FR.Value(); end;
    local procedure RetExit(var FR: FieldRef): Integer begin exit(FR.Value()); end;
    local procedure RetParen(var FR: FieldRef): Integer begin exit((FR.Value())); end;
    [TryFunction]
    local procedure TryTarget(var FR: FieldRef; Target: Text; var Result: Text)
    var I: Integer; B: BigInteger; Dec: Decimal; Dur: Duration; O: Option A,B,C; Bool: Boolean; By: Byte; Ch: Char;
    begin
        case Target of
            'Integer': begin I := FR.Value(); Result := Format(I, 0, 9); end;
            'BigInteger': begin B := FR.Value(); Result := Format(B, 0, 9); end;
            'Decimal': begin Dec := FR.Value(); Result := Format(Dec, 0, 9); end;
            'Duration': begin Dur := FR.Value(); Result := Format(Dur, 0, 9); end;
            'Option': begin O := FR.Value(); Result := Format(O, 0, 9); end;
            'Boolean': begin Bool := FR.Value(); Result := Format(Bool, 0, 9); end;
            'Byte': begin By := FR.Value(); Result := Format(By, 0, 9); end;
            'Char': begin Ch := FR.Value(); Result := Format(Ch, 0, 9); end;
        end;
    end;
    [TryFunction]
    local procedure TryPlace(var FR: FieldRef; Place: Text; var Result: Text)
    var I: Integer; Row: Record TemporalRow; A: array[2] of Integer; LI: List of [Integer]; LD: List of [Decimal]; LB: List of [BigInteger]; V: Variant;
    begin
        case Place of
            'var': begin AssignVar(FR, I); Result := Format(I, 0, 9); end;
            'argument': Result := TakeInt(FR.Value());
            'named_return': Result := Format(RetNamed(FR), 0, 9);
            'exit': Result := Format(RetExit(FR), 0, 9);
            'paren_exit': Result := Format(RetParen(FR), 0, 9);
            'global': begin GlobalInt := FR.Value(); Result := Format(GlobalInt, 0, 9); end;
            'record_field': begin Row.IntT := FR.Value(); Result := Format(Row.IntT, 0, 9); end;
            'array': begin A[1] := FR.Value(); Result := Format(A[1], 0, 9); end;
            'list_integer': begin LI.Add(FR.Value()); Result := Format(LI.Get(1), 0, 9); end;
            'list_decimal': begin LD.Add(FR.Value()); Result := Format(LD.Get(1), 0, 9); end;
            'list_biginteger': begin LB.Add(FR.Value()); Result := Format(LB.Get(1), 0, 9); end;
            'abs': begin I := Abs(FR.Value()); Result := Format(I, 0, 9); end;
            'variant': begin V := FR.Value(); I := V; Result := Format(I, 0, 9); end;
        end;
    end;
    procedure Targets(Src: Text): Text
    var RR: RecordRef; FR: FieldRef; Target: Text; Result: Text; Out: Text;
    begin
        foreach Target in 'Integer,BigInteger,Decimal,Duration,Option,Boolean,Byte,Char'.Split(',') do begin
            Source(RR, FR, Src);
            ClearLastError(); Result := '';
            Out += Target + '=' + Outcome(TryTarget(FR, Target, Result), Result) + ';';
            RR.Close();
        end;
        exit(Out);
    end;
    [TryFunction]
    local procedure TryTakeInt(I: Integer) begin end;
    [TryFunction]
    local procedure TryOther(var FR: FieldRef; Place: Text; var Result: Text)
    var LI: List of [Integer]; D: Dictionary of [Integer, Integer]; Ok: Boolean;
    begin
        case Place of
            'list_contains': Result := Format(LI.Contains(FR.Value()));
            'list_indexof': Result := Format(LI.IndexOf(FR.Value()));
            'list_insert': LI.Insert(1, FR.Value());
            'dict_add': D.Add(FR.Value(), 1);
            'dict_containskey': Result := Format(D.ContainsKey(FR.Value()));
            'try_argument': Ok := TryTakeInt(FR.Value());
        end;
    end;
    procedure Other(Src: Text; Place: Text): Text
    var RR: RecordRef; FR: FieldRef; Result: Text;
    begin
        Source(RR, FR, Src);
        exit(Outcome(TryOther(FR, Place, Result), Result));
    end;
    procedure German(Src: Text): Text
    var RR: RecordRef; FR: FieldRef; I: Integer;
    begin
        Source(RR, FR, Src);
        GlobalLanguage(1031);
        I := FR.Value();
        exit(Format(I));
    end;
    procedure Places(Src: Text): Text
    var RR: RecordRef; FR: FieldRef; Place: Text; Result: Text; Out: Text;
    begin
        foreach Place in 'var,argument,named_return,exit,paren_exit,global,record_field,array,list_integer,list_decimal,list_biginteger,abs,variant'.Split(',') do begin
            Source(RR, FR, Src);
            ClearLastError(); Result := '';
            Out += Place + '=' + Outcome(TryPlace(FR, Place, Result), Result) + ';';
            RR.Close();
        end;
        exit(Out);
    end;
}
"#;

fn call(routine: &str, src: &str) -> String {
    let arg = vec![al_builtins::value::Value::Text(src.into())];
    al_testkit::text(al_testkit::run_with(SRC, "C", routine, arg).map(|r| r.0))
}

fn err(kind: &str, dotnet: &str) -> String {
    format!("E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav{kind} to {dotnet}. | Conversion")
}

fn targets(kind: &str, duration: &str, option: &str, boolean: &str) -> String {
    format!(
        "Integer={};BigInteger={};Decimal={};Duration={duration};Option={option};Boolean={boolean};Byte={};Char={};",
        err(kind, "System.Int32"),
        err(kind, "System.Int64"),
        err(kind, "Microsoft.Dynamics.Nav.Runtime.Decimal18"),
        err(kind, "System.Byte"),
        err(kind, "System.Char"),
    )
}

fn places(kind: &str, variant: &str) -> String {
    let int = err(kind, "System.Int32");
    let dec = err(kind, "Microsoft.Dynamics.Nav.Runtime.Decimal18");
    let big = err(kind, "System.Int64");
    format!(
        "var={int};argument={int};named_return={int};exit={int};paren_exit={int};global={int};record_field={int};array={int};\
         list_integer={int};list_decimal={dec};list_biginteger={big};abs={dec};variant={variant};"
    )
}

/// FvcTemporalIntoNumberPlaces.al Date0_Targets, Date1_Targets, Time0_Targets, Time1_Targets, DateTime0_Targets and
/// DateTime1_Targets (the BC 28 and later readings).
#[test]
fn a_temporal_fieldref_value_into_each_number_target() {
    assert_eq!(
        call("Targets", "date0"),
        targets("Date", "ok: P0DT0H0M0.0S", "ok: 0", "ok: false")
    );
    assert_eq!(
        call("Targets", "date1"),
        targets("Date", "ok: P0DT0H24M39.993S", "ok: 1479993", "ok: true")
    );
    assert_eq!(
        call("Targets", "time0"),
        targets("Time", "ok: P0DT0H0M0.0S", "ok: 0", "ok: false")
    );
    assert_eq!(
        call("Targets", "time1"),
        targets("Time", "ok: P0DT12H0M0.001S", "ok: 43200001", "ok: true")
    );
    let int = err("DateTime", "System.Int32");
    assert_eq!(
        call("Targets", "datetime0"),
        targets(
            "DateTime",
            "ok: P0DT0H0M0.0S",
            "E: The value \"\" can't be evaluated into type Option. | Evaluate:Conversion",
            &int
        )
    );
    assert_eq!(
        call("Targets", "datetime1"),
        targets(
            "DateTime",
            "ok: P739996DT12H0M0.001S",
            "E: The value \"01/15/26 12:00 PM\" can't be evaluated into type Option. | Evaluate:Conversion",
            &int
        )
    );
}

/// FvcTemporalIntoNumberPlaces.al <Source>_Places; `paren_exit` is issue 9305.
#[test]
fn a_temporal_fieldref_value_into_an_integer_in_every_place() {
    assert_eq!(call("Places", "date0"), places("Date", "ok: 0"));
    assert_eq!(call("Places", "date1"), places("Date", "ok: 1479993"));
    assert_eq!(call("Places", "time0"), places("Time", "ok: 0"));
    assert_eq!(call("Places", "time1"), places("Time", "ok: 43200001"));
    assert_eq!(
        call("Places", "datetime1"),
        places("DateTime", &err("DateTime", "System.Int32"))
    );
}

/// The rule is taken for every value of a kind (the probe measured a blank and a set one): an early date.
#[test]
fn another_date_takes_the_rule_of_its_kind() {
    assert!(call("Targets", "date_early").starts_with(&format!("Integer={};", err("Date", "System.Int32"))));
}

/// The follow-up corpus probe, recordref-fieldref/fieldref-value-convert/FvcTemporalIntoNumberMore.al:158-310: values the
/// first probe did not measure take the rule of their kind; a closing date is one more than its day.
#[test]
fn more_values_take_the_rule_of_their_kind() {
    assert_eq!(
        call("Targets", "date_early"),
        targets("Date", "ok: P0DT0H21M20.543S", "ok: 1280543", "ok: true")
    );
    assert_eq!(
        call("Targets", "date_late"),
        targets("Date", "ok: P0DT2H1M44.849S", "ok: 7304849", "ok: true")
    );
    assert_eq!(
        call("Targets", "date_closing"),
        targets("Date", "ok: P0DT0H24M39.994S", "ok: 1479994", "ok: true")
    );
    assert_eq!(
        call("Targets", "time_morning"),
        targets("Time", "ok: P0DT8H30M15.001S", "ok: 30615001", "ok: true")
    );
    assert_eq!(
        call("Targets", "time_second"),
        targets("Time", "ok: P0DT0H0M1.001S", "ok: 1001", "ok: true")
    );
    assert_eq!(
        call("Targets", "datetime_early"),
        targets(
            "DateTime",
            "ok: P640271DT0H0M1.001S",
            "E: The value \"01/01/1753 12:00 AM\" can't be evaluated into type Option. | Evaluate:Conversion",
            &err("DateTime", "System.Int32")
        )
    );
}

fn refusal(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Places the probes did not measure keep the refusal (review of the 9265 change): builtins other than List.Add and an
/// argument of a TryFunction.
#[test]
fn a_temporal_fieldref_value_in_a_place_the_corpus_does_not_measure_refuses() {
    for place in [
        "list_contains",
        "list_indexof",
        "list_insert",
        "dict_add",
        "dict_containskey",
        "try_argument",
    ] {
        let args = vec![
            al_builtins::value::Value::Text("date1".into()),
            al_builtins::value::Value::Text(place.into()),
        ];
        let why = refusal(al_testkit::run_with(SRC, "C", "Other", args).map(|r| r.0));
        assert!(why.contains("in a place the corpus does not measure"), "{place}: {why}");
    }
}

#[test]
fn outside_an_en_us_session_a_temporal_fieldref_value_into_a_number_refuses() {
    let arg = vec![al_builtins::value::Value::Text("date1".into())];
    let why = refusal(al_testkit::run_with(SRC, "C", "German", arg).map(|r| r.0));
    assert!(why.contains("outside an en-US session"), "{why}");
}

/// FvcTemporalIntoNumberPlaces.al:608 DateTime1_Targets, the oldest supported major: a DateTime into an Option raises the
/// Int32 conversion error, not the Evaluate text.
#[test]
fn on_the_oldest_major_a_datetime_into_an_option_raises_the_int32_error() {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse("27.5").unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Targets")
        .unwrap();
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    let arg = vec![al_builtins::value::Value::Text("datetime1".into())];
    let got = al_testkit::text(s.call(oi, ri, inst, arg).map(|(v, _)| v));
    let int = err("DateTime", "System.Int32");
    assert_eq!(got, targets("DateTime", "ok: P739996DT12H0M0.001S", &int, &int));
}
