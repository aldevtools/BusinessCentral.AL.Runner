//! Tests-ERM refusals of issue 27: a text filter
//! on a DateFormula field and `TestField` of a Blob field. No corpus test
//! pins any of them (the corpus has no `asserterror` on `FindFirst`, no filter on a DateFormula field,
//! no `for ... := false to true` and no `TestField` of a Blob), so each is refused with a reason that
//! names the open question and `docs/probes-needed.md`.

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn compile(tests: &str) -> (al_sema::lower::Lowered, usize) {
    let src = format!(
        "table 63000 \"B7 Row\" {{ fields {{ field(1; \"No.\"; Code[20]) {{ }} field(2; Pic; Blob) {{ }} field(3; Calc; DateFormula) {{ }} }} keys {{ key(PK; \"No.\") {{ }} }} }}
         codeunit 63001 T {{ Subtype = Test; {tests} }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    (l, oi)
}

fn why_not_run(test: &str) -> String {
    let (l, oi) = compile(test);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    match &o[0].1 {
        Outcome::NotRun(why) => why.clone(),
        other => panic!("{}: {other:?}", o[0].0),
    }
}

fn names_the_open_question(why: &str, words: &[&str]) {
    for w in words.iter().chain(&["not pinned", "probes-needed"]) {
        assert!(why.contains(w), "`{w}` missing from: {why}");
    }
}

// The error is pinned on every version (record/lt-filters-keys-finds/P68C_EmptyFinds.al:215-250), so the test fails with it.
#[test]
fn find_first_on_an_empty_set_without_the_result_raises_the_table_is_empty_error() {
    let (l, oi) = compile("[Test] procedure T1() var R: Record \"B7 Row\"; begin R.FindFirst(); end;");
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    match &o[0].1 {
        Outcome::Fail(m) => assert_eq!(m, "The B7 Row table is empty."),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_text_filter_on_a_date_formula_field_is_refused_with_its_reason() {
    // The empty text is read as the empty formula (assumption filter-empty-text-dateformula); a formula stays open.
    let why = why_not_run("[Test] procedure T2() var R: Record \"B7 Row\"; begin R.SetFilter(Calc, '1M'); end;");
    names_the_open_question(&why, &["SetFilter", "DateFormula"]);
}

// A Boolean `for` loop was refused here; it runs now (registry entry for-boolean, blocked_set_3.rs).

// Pinned by misc/lt-probes-needed-5/PnE08RecordMisc.al:TestField_Blob: an empty Blob fails with the usual error.
#[test]
fn test_field_of_an_empty_blob_fails_with_the_usual_error() {
    let (l, oi) = compile("[Test] procedure T4() var R: Record \"B7 Row\"; begin R.TestField(Pic); end;");
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    match &o[0].1 {
        Outcome::Fail(m) => assert_eq!(m, "Pic must have a value in B7 Row: No.=. It cannot be zero or empty."),
        other => panic!("{other:?}"),
    }
}

// Pinned by the same corpus test: a Blob written through a stream passes TestField.
#[test]
fn test_field_of_a_blob_written_through_a_stream_passes() {
    let (l, oi) = compile(
        "[Test] procedure T5() var R: Record \"B7 Row\"; Out: OutStream; begin R.Pic.CreateOutStream(Out); Out.WriteText('x'); R.TestField(Pic); end;",
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// A Blob with stored content read by Get without CalcFields passes TestField
// (record/lt-testfield-blob-decimal/TestFieldBlobDecimal.al:TestField_Blob_StoredContent `after_get_without_calcfields`).
#[test]
fn test_field_of_a_stored_blob_not_fetched_with_calcfields_passes() {
    let (l, oi) = compile(
        "[Test] procedure T6() var R: Record \"B7 Row\"; R2: Record \"B7 Row\"; Out: OutStream; begin \
         R.\"No.\" := 'A'; R.Pic.CreateOutStream(Out); Out.WriteText('x'); R.Insert(); \
         R2.Get('A'); R2.TestField(Pic); end;",
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    assert!(matches!(&o[0].1, Outcome::Pass), "{:?}", o[0].1);
}
