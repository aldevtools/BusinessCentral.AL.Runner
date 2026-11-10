//! A query Sum or Average whose sum does not fit the column's type.
//! Pinned: a BigInteger Sum of 9223372036854775807 and 1 raises `CalculationOverflow:Sql`, from BC 29 only
//! (queries/W7QueryAggregates.al:168, inside the `#if BC29PLUS` guard at :148). Not pinned: the same Sum below
//! BC 29, and an Average whose sum overflows (BC averages in SQL; the corpus query that overflows also has a Sum
//! column, W7QueryObjects.al:183-186). Both are refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 68970 "Probe AggOvf Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Int; Integer) { }
        field(3; Big; BigInteger) { }
        field(4; Dur; Duration) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

query 89970 "Probe AggOvf Sum Big"
{
    elements { dataitem(R; "Probe AggOvf Row") { column(SumBig; Big) { Method = Sum; } } }
}

query 89971 "Probe AggOvf Avg Big"
{
    elements { dataitem(R; "Probe AggOvf Row") { column(AvgBig; Big) { Method = Average; } } }
}

query 89972 "Probe AggOvf Avg Dur"
{
    elements { dataitem(R; "Probe AggOvf Row") { column(AvgDur; Dur) { Method = Average; } } }
}

query 89973 "Probe AggOvf Avg Int"
{
    elements { dataitem(R; "Probe AggOvf Row") { column(AvgInt; Int) { Method = Average; } } }
}

codeunit 50100 C
{
    local procedure Seed()
    var
        R: Record "Probe AggOvf Row";
        Big: BigInteger;
    begin
        Evaluate(Big, '9223372036854775807', 9);
        R.Id := 1; R.Int := 2147483647; R.Big := Big; R.Dur := Big; R.Insert();
        R.Id := 2; R.Int := 1; R.Big := 1; R.Dur := 1; R.Insert();
    end;

    procedure SumBig(): Text
    var
        Q: Query "Probe AggOvf Sum Big";
    begin
        Seed();
        Q.Open();
        Q.Read();
        exit(Format(Q.SumBig, 0, 9));
    end;

    procedure AvgBig(): Text
    var
        Q: Query "Probe AggOvf Avg Big";
    begin
        Seed();
        Q.Open();
        Q.Read();
        exit(Format(Q.AvgBig, 0, 9));
    end;

    procedure AvgDur(): Text
    var
        Q: Query "Probe AggOvf Avg Dur";
    begin
        Seed();
        Q.Open();
        Q.Read();
        exit(Format(Q.AvgDur, 0, 9));
    end;

    procedure AvgInt(): Text
    var
        Q: Query "Probe AggOvf Avg Int";
    begin
        Seed();
        Q.Open();
        Q.Read();
        exit(Format(Q.AvgInt, 0, 9));
    end;
}
"#;

fn run_on(bc: &str, routine: &str) -> String {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
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
        .position(|r| r.name == routine)
        .unwrap();
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![]).map(|(v, _)| v) {
        Ok(Value::Text(t)) => t.to_string(),
        Ok(other) => panic!("expected a text, got {other:?}"),
        Err(Signal::Unsupported(why)) => format!("unsupported: {why}"),
        Err(Signal::Error(e)) => format!("error: {}|{}", e.message, e.code),
        Err(other) => panic!("{other:?}"),
    }
}

#[test]
fn a_big_integer_sum_that_overflows_raises_the_pinned_error_from_bc_29() {
    assert_eq!(
        run_on("29.0", "SumBig"),
        "error: An arithmetic overflow occurred while calculating a value, for example when a sum of an Integer field exceeds the maximum value the data type can hold. Use a larger data type, such as BigInteger or Decimal, for the field or the calculation.|CalculationOverflow:Sql"
    );
}

#[test]
fn a_big_integer_sum_that_overflows_is_refused_below_bc_29() {
    for bc in ["27.5", "28.5"] {
        let got = run_on(bc, "SumBig");
        assert!(got.starts_with("unsupported:") && got.contains("overflow"), "BC {bc}: {got}");
    }
}

#[test]
fn an_average_whose_sum_overflows_is_refused() {
    for routine in ["AvgBig", "AvgDur", "AvgInt"] {
        let got = run_on("29.0", routine);
        assert!(got.starts_with("unsupported:") && got.contains("overflow"), "{routine}: {got}");
    }
}
