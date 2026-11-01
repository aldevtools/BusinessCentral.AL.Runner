//! Lowering gaps that stopped about a thousand Tests-ERM tests after CaptionClassTranslate (issue 48).

use al_sema::workspace::AppSource;

fn lowered(src: &str) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    )
}

/// Why `routine` of object `object` cannot be lowered ("" when it can).
fn refusal(src: &str, object: &str, routine: &str) -> String {
    let (ws, l) = lowered(src);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == object).unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name.eq_ignore_ascii_case(routine))
        .unwrap();
    l.unsupported[oi][ri].first().cloned().unwrap_or_default()
}

// Base Application's Sales Header passes CurrFieldNo to an event publisher
// (`OnBeforeShouldSearchForCustomerByName(CustomerNo, Result, IsHandled, CurrFieldNo, Rec, xRec)`): a value argument,
// so no place is needed. The value itself is pinned by handlers/TestPageCurrFieldNo_Tests.al (the field number a
// TestPage write validates, 0 elsewhere).
#[test]
fn curr_field_no_can_be_an_argument_of_a_call() {
    let src = "table 63000 T { fields { field(1; A; Integer) { } field(2; B; Integer) { } } keys { key(PK; A) { } }
        procedure P(): Integer var Handled: Boolean; begin Observe(CurrFieldNo, Rec, xRec, Handled); exit(CurrFieldNo); end;
        [IntegrationEvent(false, false)] local procedure Observe(var No: Integer; var R: Record T; var X: Record T; var H: Boolean) begin end; }";
    assert_eq!(refusal(src, "T", "P"), "");
}

// Base Application's Sales Line calls `GetSalesHeader();` as a statement: a procedure that returns a record, its result
// dropped (`exit(SalesHeader)` after loading a global). Nothing in the corpus calls a record-returning procedure as a
// statement; dropping the result changes nothing a test can see.
#[test]
fn a_record_returning_procedure_can_be_called_as_a_statement() {
    let src = "table 63001 H { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }
        table 63002 L { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } }
          var Hdr: Record H;
          procedure Header(): Record H begin Hdr.K := 'A'; exit(Hdr); end;
          procedure P() begin Header(); end;
          procedure Q(): Code[10] begin exit(Header().K); end; }
        codeunit 63003 U { procedure R(): Boolean var Ln: Record L; begin exit(Ln.Header().K = 'A'); end;
          procedure S(): Boolean var Ln: Record L; begin if Ln.Header().K = 'A' then exit(true); exit(false); end;
          procedure T() var Ln: Record L; begin Ln.Header(); end; }";
    assert_eq!(refusal(src, "L", "P"), "");
    assert_eq!(refusal(src, "L", "Q"), "");
    // the same through a record variable, in a condition
    for r in ["R", "S", "T"] {
        assert_eq!(refusal(src, "U", r), "", "{r}");
    }
}
