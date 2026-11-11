//! Where a table procedure called on Rec from a field trigger shares the trigger's xRec (review of runner PR 7800).
//! record/validate/XRecTableProcCalls.al:186-339 pins a procedure of the base table called from code (`Record.Validate`). From a
//! TestPage SetValue it stays an assumption (`xrec-table-proc-from-field-validate`, a flagged pass, as before the pin), and a
//! table extension's own procedure keeps the blank xRec of a plain call (not measured).

use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50300 "Z Log" { SingleInstance = true; var Log: Text; procedure Add(V: Text) begin Log += V + ';'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }
table 50301 "Z Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; A; Text[30]) { trigger OnValidate() begin Rec.Proc(); end; }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
    procedure Proc() var L: Codeunit "Z Log"; begin L.Add('P x=' + xRec.A); end;
}
tableextension 50304 "Z Ext" extends "Z Row"
{
    fields
    {
        field(50300; ExtA; Text[30]) { trigger OnValidate() begin Rec.ExtProc(); end; }
    }
    procedure ExtProc() var L: Codeunit "Z Log"; begin L.Add('X x=' + xRec.A); end;
}
page 50302 "Z Card" { PageType = Card; SourceTable = "Z Row"; ApplicationArea = All; UsageCategory = None;
  layout { area(Content) { field("Code"; Rec."Code") { ApplicationArea = All; } field(A; Rec.A) { ApplicationArea = All; } } } }
codeunit 50303 "Z Page Tests"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure PageSetValue()
    var Row: Record "Z Row"; P: TestPage "Z Card"; L: Codeunit "Z Log";
    begin
        Row.DeleteAll(); Row."Code" := 'R1'; Row.A := 'stored'; Row.Insert(); Commit();
        P.OpenEdit(); P.GoToRecord(Row); P.A.SetValue('v'); P.Close();
        if L.Take() <> 'P x=stored;' then Error('other');
    end;
}
codeunit 50305 "Z Code Tests"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure CodeValidate()
    var Row: Record "Z Row"; L: Codeunit "Z Log";
    begin
        Row.DeleteAll(); Row."Code" := 'R1'; Row.A := 'stored'; Row.Insert(); Commit();
        Row.Get('R1'); Row.Validate(A, 'v');
        if L.Take() <> 'P x=stored;' then Error('other');
    end;
}
codeunit 50306 "Z Ext Tests"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure ExtProcFromExtValidate()
    var Row: Record "Z Row"; L: Codeunit "Z Log";
    begin
        Row.DeleteAll(); Row."Code" := 'R1'; Row.A := 'stored'; Row.Insert(); Commit();
        Row.Get('R1'); Row.Validate(ExtA, 'v');
        // The blank xRec of a plain call, as before the pin.
        if L.Take() <> 'X x=;' then Error('other');
    end;
}
"#;

fn run(codeunit: &str) -> (Vec<(String, Outcome)>, Vec<al_builtins::assumptions::Assumption>) {
    let l = al_testkit::compile_apps(vec![al_testkit::app(SRC)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect(codeunit);
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    (o, hits.into_iter().flat_map(|(_, h)| h).collect())
}

#[test]
fn a_testpage_setvalue_keeps_the_assumption_mark() {
    use al_builtins::assumptions::Assumption::XrecTableProcFromFieldValidate as A;
    let (o, hits) = run("Z Page Tests");
    assert!(matches!(o.as_slice(), [(_, Outcome::Pass)]), "{o:?}");
    assert!(hits.contains(&A), "{hits:?}");
    let (o, hits) = run("Z Code Tests");
    assert!(matches!(o.as_slice(), [(_, Outcome::Pass)]), "{o:?}");
    assert!(!hits.contains(&A), "{hits:?}");
}

#[test]
fn a_table_extensions_own_procedure_keeps_the_plain_xrec() {
    let (o, _) = run("Z Ext Tests");
    assert!(matches!(o.as_slice(), [(_, Outcome::Pass)]), "{o:?}");
}
