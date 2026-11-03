//! Behaviour that differs between BC versions (ADR-015 3.4). Each test runs the same AL on a
//! BC 27 target and a BC 29 target and cites the corpus `#if BC29PLUS` guard that pins both sides.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_version::{BcVersion, Target};
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 60900 "VG Line"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Doc No."; Code[20]) { }
        field(3; Qty; Integer) { }
        field(4; Big; BigInteger) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60901 "VG Head"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; IdInt; Integer) { }
        field(14; CntBig; Integer) { FieldClass = FlowField; CalcFormula = count("VG Line" where(Big = field(IdInt))); }
        field(10; SumBig; BigInteger) { FieldClass = FlowField; CalcFormula = sum("VG Line".Qty where("Doc No." = field("No."))); }
        field(11; MaxRv; BigInteger) { FieldClass = FlowField; CalcFormula = max("VG Line".SystemRowVersion where("Doc No." = field("No."))); }
        field(12; MinRv; BigInteger) { FieldClass = FlowField; CalcFormula = min("VG Line".SystemRowVersion where("Doc No." = field("No."))); }
        field(13; LookRv; BigInteger) { FieldClass = FlowField; CalcFormula = lookup("VG Line".SystemRowVersion where("Entry No." = const(1))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50100 C
{
    procedure Seed()
    var H: Record "VG Head"; L: Record "VG Line";
    begin
        H."No." := 'D1'; H.Insert();
        L."Entry No." := 1; L."Doc No." := 'D1'; L.Qty := 3; L.Insert();
        L."Entry No." := 2; L."Doc No." := 'D1'; L.Qty := 4; L.Insert();
    end;
    procedure SumBig(): BigInteger
    var H: Record "VG Head";
    begin Seed(); H.Get('D1'); H.CalcFields(SumBig); exit(H.SumBig); end;
    procedure Max(): Boolean
    var H: Record "VG Head";
    begin Seed(); H.Get('D1'); H.CalcFields(MaxRv); exit(H.MaxRv > 0); end;
    procedure MinMax(): Boolean
    var H: Record "VG Head";
    begin Seed(); H.Get('D1'); H.CalcFields(MaxRv, MinRv); exit(H.MaxRv > H.MinRv); end;
    procedure RowVersionExists(): Boolean
    var R: RecordRef;
    begin R.Open(60900); exit(R.FieldExist(2000000005)); end;
    procedure UserNameFieldsExist(): Text
    var R: RecordRef; No: Integer; T: Text;
    begin
        R.Open(60900);
        for No := 2000000004 to 2000000009 do T += Format(No - 2000000000) + '=' + Format(R.FieldExist(No), 0, 9) + ';';
        exit(T);
    end;

    procedure CountBig(): Integer
    var H: Record "VG Head";
    begin Seed(); H.Get('D1'); H.CalcFields(CntBig); exit(H.CntBig); end;
    procedure Look(): Boolean
    var H: Record "VG Head";
    begin Seed(); H.Get('D1'); H.CalcFields(LookRv); exit(H.LookRv > 0); end;
}
"#;

/// `bc` is a BC version, or `None` for a run with no target (the newest behaviour).
fn run(bc: Option<&str>, routine: &str) -> Result<Value, Signal> {
    let target = bc.map(|v| Target::for_bc(BcVersion::parse(v).unwrap()));
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = match &target {
        Some(t) => al_sema::lower::compile_for(apps, &Default::default(), t),
        None => al_sema::lower::compile(apps, &Default::default()),
    };
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("{routine} is blocked: {r}");
    }
    let engine = Engine::with_target(l.program, blocked, target);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn ok(r: Result<Value, Signal>) -> Value {
    r.unwrap_or_else(|e| panic!("expected a value, got {e:?}"))
}

fn message(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message,
        other => panic!("expected an AL error, got {other:?}"),
    }
}

// corpus record/TestCalcFormulaValidationTests.Codeunit.al:205,227 (#if BC29PLUS): BC 27 and 28
// refuse a sum of an Integer source into a BigInteger FlowField; BC 29 accepts it.
#[test]
fn integer_sum_into_a_biginteger_flowfield_is_refused_before_bc_29_and_calculated_from_it() {
    for bc in ["27.5", "28.5"] {
        let m = message(run(Some(bc), "SumBig"));
        assert!(
            m.starts_with("The following fields must have the same type:"),
            "BC {bc}: {m}"
        );
        assert!(m.contains("Type: BigInteger <-- Integer"), "BC {bc}: {m}");
    }
    for bc in [Some("29.0"), Some("30"), None] {
        let v = ok(run(bc, "SumBig"));
        assert!(matches!(v, Value::BigInteger(7)), "BC {bc:?}: {v:?}");
    }
}

// corpus calcfields/lt-bm-flowfield-conversions/BmFlowFieldConversions.al:166-186 (#if BC29PLUS): a count FlowField
// whose filter links a BigInteger source field to an Integer field (`Big = field(Id)`) is refused when calculated on
// BC 27 and 28 (DB:TypeMisMatch, source first); BC 29 counts.
#[test]
fn a_count_filter_biginteger_from_integer_is_refused_before_bc_29_and_counted_from_it() {
    for bc in ["27.5", "28.5"] {
        let m = message(run(Some(bc), "CountBig"));
        assert_eq!(
            m,
            "The following fields must have the same type:\\\\Field: Big <-- IdInt\\Table: VG Line <-- VG Head\\Type: BigInteger <-- Integer",
            "BC {bc}"
        );
    }
    for bc in [Some("29.0"), None] {
        let v = ok(run(bc, "CountBig"));
        assert!(matches!(v, Value::Integer(2)), "BC {bc:?}: {v:?}");
    }
}

// corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:298,306,353 (#if BC29PLUS): min and
// max over SystemRowVersion calculate on BC 27 and 28 and are refused on BC 29. The message is the
// pinned fragments (lines 311-317, 358-360) joined by newlines; the glue is recorded in
// docs/probes-needed.md.
#[test]
fn min_and_max_over_systemrowversion_calculate_before_bc_29_and_are_refused_from_it() {
    for bc in ["27.5", "28.5"] {
        assert!(matches!(ok(run(Some(bc), "Max")), Value::Boolean(true)), "BC {bc}");
        assert!(matches!(ok(run(Some(bc), "MinMax")), Value::Boolean(true)), "BC {bc}");
    }
    for bc in [Some("29.0"), Some("30"), None] {
        let m = message(run(bc, "Max"));
        assert!(
            m.starts_with(
                "Table content in the VG Head table on SQL Server contains a value that is not compatible with the corresponding data type"
            ),
            "BC {bc:?}: {m}"
        );
        assert!(m.contains("Field: MaxRv"), "{m}");
        assert!(m.contains("Type: BigInteger"), "{m}");
        assert!(m.contains("SQL type: timestamp"), "{m}");
        // The first refused field is named when several are calculated together (line 311).
        let m = message(run(bc, "MinMax"));
        assert!(m.contains("Field: MaxRv"), "{m}");
    }
}

// corpus calcfields/W4CalcFields.al:134-135 and misc/lt-probes-needed-1/PnACalcFieldsBC29.al:225-243
// (#if BC29PLUS) pin the whole text, with BC's backslash line breaks, and the code NDBCS:IncompatibleSqlType.
#[test]
fn min_and_max_over_systemrowversion_refusal_has_the_pinned_text_and_code_on_bc_29() {
    for bc in [Some("29.0"), None] {
        match run(bc, "Max") {
            Err(Signal::Error(e)) => {
                assert_eq!(
                    e.message,
                    "Table content in the VG Head table on SQL Server contains a value that is not compatible with the corresponding data type that is defined for the table.\\\\Field: MaxRv\\Type: BigInteger\\SQL type: timestamp\\\\The table design may have been changed outside Business Central.",
                    "BC {bc:?}"
                );
                assert_eq!(e.code, "NDBCS:IncompatibleSqlType", "BC {bc:?}");
            }
            other => panic!("expected an AL error, got {other:?}"),
        }
    }
}

// corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:300-303: lookup() of the same field
// still calculates on BC 29 (Record_CalcFields_LooksUpSystemRowVersion).
#[test]
fn lookup_of_systemrowversion_calculates_on_every_version() {
    for bc in [Some("27.5"), Some("29.0"), None] {
        assert!(matches!(ok(run(bc, "Look")), Value::Boolean(true)), "BC {bc:?}");
    }
}

// corpus recordref/lt-recordref-fieldref-members/P92Fields.al:72 (#if BC29PLUS): on BC 29 RecordRef.FieldExist(2000000005),
// the rowversion's number, is true. Before 29 it is false (fieldref/lt-y80-user-name-fields/Y80UserNameFieldExist.al:55-57).
#[test]
fn field_exist_of_the_rowversion_number_is_true_on_bc_29_and_false_before() {
    assert!(matches!(
        ok(run(Some("29.0"), "RowVersionExists")),
        Value::Boolean(true)
    ));
    for bc in ["27.5", "28.5"] {
        assert!(matches!(ok(run(Some(bc), "RowVersionExists")), Value::Boolean(false)), "BC {bc}");
    }
}

// fieldref/lt-y80-user-name-fields/Y80UserNameFieldExist.al:55-57 (own table): 2000000005 to 2000000008 exist on BC 29 only,
// 2000000004 on every version, 2000000009 never.
#[test]
fn field_exist_of_the_user_name_system_fields_is_true_on_bc_29_and_false_before() {
    assert_eq!(
        al_testkit::text(run(Some("29.0"), "UserNameFieldsExist")),
        "4=true;5=true;6=true;7=true;8=true;9=false;"
    );
    for bc in ["27.5", "28.5"] {
        assert_eq!(
            al_testkit::text(run(Some(bc), "UserNameFieldsExist")),
            "4=true;5=false;6=false;7=false;8=false;9=false;",
            "BC {bc}"
        );
    }
}
