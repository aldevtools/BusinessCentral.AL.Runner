//! Where a table procedure called on Rec from a field trigger shares the trigger's xRec (review of runner PR 7800).
//! record/validate/XRecTableProcCalls.al:186-339 pins a procedure of the base table called in a `Record.Validate(Field, Value)`
//! that AL code calls, alone on its record. Every other Validate call site (a TestPage SetValue, FieldRef.Validate, a Variant
//! value, an XmlPort field, a Validate nested in another of the record) is as before the pin: from the table's own OnValidate an
//! assumption (`xrec-table-proc-from-field-validate`, a flagged pass), from a table extension's trigger the blank xRec of a plain
//! call. A table extension's own procedure keeps the blank xRec too (not measured).

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
        field(4; B; Text[30]) { trigger OnValidate() begin Validate(A, 'n'); end; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
    procedure Proc() var L: Codeunit "Z Log"; begin L.Add('P x=' + xRec.A); end;
}
table 50307 "Z Blob" { TableType = Temporary; fields { field(1; K; Integer) { } field(2; B; Blob) { } } keys { key(PK; K) { Clustered = true; } } }
xmlport 50308 "Z Import"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; "Z Row") { AutoSave = false; AutoUpdate = false; AutoReplace = false;
        fieldelement(Code; Row."Code") { } fieldelement(A; Row.A) { FieldValidate = Yes; } } } }
}
tableextension 50304 "Z Ext" extends "Z Row"
{
    fields
    {
        field(50300; ExtA; Text[30]) { trigger OnValidate() begin Rec.ExtProc(); end; }
        field(50301; ExtB; Text[30]) { trigger OnValidate() begin Rec.Proc(); end; }
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
codeunit 50310 "Z Sites"
{
    procedure Seed(var Row: Record "Z Row")
    begin
        Row.DeleteAll(); Row."Code" := 'R1'; Row.A := 'stored'; Row.Insert(); Commit(); Row.Get('R1');
    end;
    procedure Check(Expected: Text)
    var L: Codeunit "Z Log"; T: Text;
    begin
        T := L.Take();
        if T <> Expected then Error('expected %1, got %2', Expected, T);
    end;
}
codeunit 50311 "Z Code Ext Field"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure CodeValidateExtFieldBaseProc()
    var Row: Record "Z Row"; S: Codeunit "Z Sites";
    begin
        S.Seed(Row); Row.Validate(ExtB, 'v'); S.Check('P x=stored;');
    end;
}
codeunit 50312 "Z FieldRef Base"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure FieldRefBase()
    var Row: Record "Z Row"; RR: RecordRef; FR: FieldRef; S: Codeunit "Z Sites";
    begin
        S.Seed(Row); RR.GetTable(Row); FR := RR.Field(2); FR.Validate('v'); S.Check('P x=stored;');
    end;
}
codeunit 50313 "Z FieldRef Ext Field"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure FieldRefExtFieldBaseProc()
    var Row: Record "Z Row"; RR: RecordRef; FR: FieldRef; S: Codeunit "Z Sites";
    begin
        S.Seed(Row); RR.GetTable(Row); FR := RR.Field(50301); FR.Validate('v'); S.Check('P x=;');
    end;
}
codeunit 50314 "Z Variant"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure VariantValidate()
    var Row: Record "Z Row"; V: Variant; S: Codeunit "Z Sites";
    begin
        S.Seed(Row); V := 'v'; Row.Validate(A, V); S.Check('P x=stored;');
    end;
}
codeunit 50315 "Z Nested"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure NestedValidate()
    var Row: Record "Z Row"; S: Codeunit "Z Sites";
    begin
        S.Seed(Row); Row.Validate(B, 'v'); S.Check('P x=stored;');
    end;
}
codeunit 50316 "Z XmlPort"
{
    Subtype = Test; TestPermissions = Disabled;
    [Test] procedure XmlPortImport()
    var Row: Record "Z Row"; TB: Record "Z Blob"; O: OutStream; I: InStream; S: Codeunit "Z Sites";
    begin
        Row.DeleteAll();
        TB.B.CreateOutStream(O); O.WriteText('<Root><Row><Code>R9</Code><A>v</A></Row></Root>'); TB.B.CreateInStream(I);
        Xmlport.Import(Xmlport::"Z Import", I);
        S.Check('P x=v;');
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

/// The reviewer's scenarios of round 3: each Validate call site other than a `Record.Validate` from AL code gives what it gave
/// before the pin (the values and the flag of the binary of main, ca0c74808).
#[test]
fn only_a_record_validate_from_al_code_is_a_clean_pass() {
    use al_builtins::assumptions::Assumption::XrecTableProcFromFieldValidate as A;
    let mut wrong = Vec::new();
    for (codeunit, flagged) in [
        // Pinned: Record.Validate from code, from the table's field and from a table extension's field
        // (XRecTableProcCalls.al:186, :252).
        ("Z Code Tests", false),
        ("Z Code Ext Field", false),
        // Not pinned, as before: the assumption from the table's own OnValidate.
        ("Z FieldRef Base", true),
        ("Z Variant", true),
        ("Z Nested", true),
        ("Z XmlPort", true),
        // Not pinned, as before: the blank xRec from a table extension's trigger (the test compares the value).
        ("Z FieldRef Ext Field", false),
    ] {
        let (o, hits) = run(codeunit);
        if !matches!(o.as_slice(), [(_, Outcome::Pass)]) || hits.contains(&A) != flagged {
            wrong.push(format!("{codeunit}: want flagged={flagged}, got {o:?} {hits:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
