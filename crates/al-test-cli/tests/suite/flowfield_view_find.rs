//! Issue 6807: a view set after a Get narrows the next Find, on a Record and on a RecordRef, with a filter on a FlowField and on a
//! stored field. Values measured on BC 28.5 in corpus record/views/R6807ViewFind.al (Rec_svFlowZero, Rec_svStored,
//! Rec_copyFlowZero, Ref_svFlowZero, Ref_svStored, Ref_viewRoundTripFlowZero, Ref_GetTableAfterRecordView, and the controls
//! Rec_svFlowSome, Ref_svFlowSome).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62700 "FV Parent"
{
    fields {
        field(1; "No."; Code[20]) { }
        field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("FV Child" where(Par = field("No."))); }
    }
    keys { key(PK; "No.") { } }
}
table 62701 "FV Child"
{
    fields { field(1; Id; Integer) { } field(2; Par; Code[20]) { } }
    keys { key(PK; Id) { } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62702 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn a_view_set_after_get_narrows_find_on_record_and_recordref() {
    let o = outcomes(
        "procedure Seed() var P: Record \"FV Parent\"; C: Record \"FV Child\"; begin
             P.DeleteAll(); C.DeleteAll();
             P.\"No.\" := 'A'; P.Insert(); P.\"No.\" := 'B'; P.Insert();
             C.Id := 1; C.Par := 'A'; C.Insert();
         end;
         procedure ViewZero(): Text var Q: Record \"FV Parent\"; begin Q.SetRange(Cnt, 0); exit(Q.GetView(false)); end;
         procedure ViewSome(): Text var Q: Record \"FV Parent\"; begin Q.SetFilter(Cnt, '>%1', 0); exit(Q.GetView(false)); end;
         procedure ViewB(): Text var Q: Record \"FV Parent\"; begin Q.SetRange(\"No.\", 'B'); exit(Q.GetView(false)); end;
         [Test] procedure RecordRefFlowField() var P: Record \"FV Parent\"; R: RecordRef; begin
             Seed(); P.Get('A');
             R.Open(Database::\"FV Parent\"); R.Get(P.RecordId()); R.SetView(ViewZero());
             if R.Find() then Error('Find true under Cnt = 0');
             if R.Count() <> 1 then Error('count %1', R.Count());
             R.Close(); R.Open(Database::\"FV Parent\"); R.Get(P.RecordId()); R.SetView(ViewSome());
             if not R.Find() then Error('Find false under Cnt > 0');
             R.Close(); R.Open(Database::\"FV Parent\"); R.Get(P.RecordId()); R.SetView(ViewB());
             if R.Find() then Error('Find true under No. = B');
         end;
         [Test] procedure RecordFlowField() var P: Record \"FV Parent\"; begin
             Seed(); P.Get('A'); P.SetView(ViewZero());
             if P.Find() then Error('Find true under Cnt = 0');
             Clear(P); P.Get('A'); P.SetView(ViewSome());
             if not P.Find() then Error('Find false under Cnt > 0');
             Clear(P); P.Get('A'); P.SetView(ViewB());
             if P.Find() then Error('Find true under No. = B');
         end;
         [Test] procedure CopyFilters() var P: Record \"FV Parent\"; Q: Record \"FV Parent\"; begin
             Seed(); P.Get('A'); Q.SetRange(Cnt, 0); P.CopyFilters(Q);
             if P.Find() then Error('Find true after CopyFilters of Cnt = 0');
         end;
         [Test] procedure CopyFilterFieldForm() var P: Record \"FV Parent\"; Q: Record \"FV Parent\"; begin
             Seed(); P.Get('A'); Q.SetRange(Cnt, 0); Q.CopyFilter(Cnt, P.Cnt);
             if P.Find() then Error('Find true after CopyFilter of Cnt = 0');
         end;
         [Test] procedure GetTableThenView() var P: Record \"FV Parent\"; R: RecordRef; begin
             Seed(); P.Get('A'); P.SetView(ViewZero()); R.GetTable(P);
             if R.Find() then Error('Find true after GetTable');
             Clear(R); P.Get('A'); R.GetTable(P); R.SetView(ViewZero());
             if R.Find() then Error('Find true after SetView on the ref');
         end;
         [Test] procedure ResetStillFinds() var P: Record \"FV Parent\"; begin
             Seed(); P.Get('A'); P.SetView(ViewZero()); P.Reset();
             if not P.Find() then Error('Find false after Reset');
         end;",
    );
    for (name, outcome) in &o {
        assert_eq!(*outcome, Outcome::Pass, "{name}");
    }
    assert_eq!(o.len(), 6);
}
