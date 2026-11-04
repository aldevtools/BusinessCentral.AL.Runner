//! The three corpus cases corpus bump 28 added that failed or were refused: a bare `IsDirty()` in the `OnRun` of a codeunit
//! with `TableNo` (BC 29), the TestField text of a Byte above 9 (BC 28 and later) and a lookup FlowField shorter than its source.

use al_builtins::native::Signal;
use al_builtins::value::Value;

/// Runs `routine` of codeunit `C` on a BC `bc` target. A routine the lowering refused comes back as `Signal::Unsupported`.
fn run_on(bc: &str, src: &str, routine: &str) -> Result<Value, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
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
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("expected a text, got {other:?}"),
    }
}

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const DIRTY: &str = r#"
table 60950 "B28 Row"
{
    fields { field(1; Id; Integer) { } field(2; Description; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 60951 "B28 Bare"
{
    TableNo = "B28 Row";
    var Log: Text;
    trigger OnRun()
    begin
        Rec.Description := 'changed';
        if IsDirty() then Log += 'IsDirty()=true' else Log += 'IsDirty()=false';
    end;
    local procedure IsDirty(): Boolean
    begin
        Log += 'local IsDirty;';
        exit(false);
    end;
    procedure Trace(): Text begin exit(Log); end;
}
codeunit 60952 "B28 Elsewhere"
{
    TableNo = "B28 Row";
    var Log: Text;
    trigger OnRun()
    begin
        Check();
    end;
    local procedure Check()
    begin
        if IsDirty() then Log += 'true' else Log += 'false';
    end;
    local procedure IsDirty(): Boolean
    begin
        exit(false);
    end;
}
codeunit 60953 C
{
    procedure Bare(): Text
    var Row: Record "B28 Row"; Bare: Codeunit "B28 Bare";
    begin
        Row.Id := 1; Row.Description := 'stored'; Row.Insert();
        Row.Get(1);
        Bare.Run(Row);
        exit(Bare.Trace());
    end;
    procedure Elsewhere(): Text
    var Row: Record "B28 Row"; E: Codeunit "B28 Elsewhere";
    begin
        Row.Id := 1; Row.Insert();
        E.Run(Row);
        exit('ran');
    end;
}
"#;

#[test]
fn a_bare_is_dirty_in_on_run_of_a_table_no_codeunit_is_the_record_method_on_bc_29() {
    // events/binding-senders/WithStatementAmbiguous.al:277-279 (BN_With_ImplicitWithInTableNoCodeunit, codeunit
    // 82156 at :412-434): Record.IsDirty wins over the codeunit's own local IsDirty, so the local procedure never runs.
    assert_eq!(text(run_on("29.0", DIRTY, "Bare")), "IsDirty()=true");
}

#[test]
fn a_bare_is_dirty_outside_on_run_of_a_table_no_codeunit_is_refused() {
    // Only OnRun is pinned (WithStatementAmbiguous.al:412-434); which IsDirty another procedure of the codeunit calls is not.
    let m = refused(run_on("29.0", DIRTY, "Elsewhere"));
    assert!(m.contains("IsDirty"), "{m}");
}

const BYTES: &str = r#"
table 60960 "B28 TF"
{
    fields { field(1; Id; Integer) { } field(2; Num; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 60961 C
{
    local procedure Try(V: Byte): Text
    var Row: Record "B28 TF"; R: RecordRef; F: FieldRef;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Num := 7; Row.Insert();
        R.Open(60960); R.FindFirst(); F := R.Field(2);
        asserterror F.TestField(V);
        exit(GetLastErrorText());
    end;
    procedure Ten(): Text begin exit(Try(10)); end;
    procedure TwoHundred(): Text begin exit(Try(200)); end;
    procedure Three(): Text begin exit(Try(3)); end;
    procedure Eleven(): Text begin exit(Try(11)); end;
    procedure Seven(): Text
    var Row: Record "B28 TF"; R: RecordRef; F: FieldRef; B: Byte;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Num := 7; Row.Insert();
        R.Open(60960); R.FindFirst(); F := R.Field(2);
        B := 7;
        F.TestField(B);
        exit('ok');
    end;
}
"#;

#[test]
fn the_test_field_text_of_a_byte_of_10_and_200_is_the_pinned_character_from_bc_28() {
    // record/testfield/AsTestFieldTexts.al:41-45 (Byte10: `\n`) and :60-65 (Byte200: U+02DC); before BC 28 the number.
    assert_eq!(
        text(run_on("28.5", BYTES, "Ten")),
        "Num must be equal to '\\n'  in B28 TF: Id=1. Current value is '7'."
    );
    assert_eq!(
        text(run_on("29.0", BYTES, "TwoHundred")),
        "Num must be equal to '\u{02DC}'  in B28 TF: Id=1. Current value is '7'."
    );
    assert_eq!(
        text(run_on("27.5", BYTES, "TwoHundred")),
        "Num must be equal to '200'  in B28 TF: Id=1. Current value is '7'."
    );
}

#[test]
fn the_test_field_text_of_an_unpinned_byte_is_refused_from_bc_28() {
    // Pinned are 6 and 8 (`\u000N`), 10 and 200; 3 and 11 are not. An equal value raises nothing, whatever it is.
    assert!(refused(run_on("29.0", BYTES, "Three")).contains("Byte"));
    assert!(refused(run_on("29.0", BYTES, "Eleven")).contains("Byte"));
    assert_eq!(text(run_on("29.0", BYTES, "Seven")), "ok");
}

const LOOKUP: &str = r#"
table 60970 "B28 Src"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } field(3; Cd; Code[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 60971 "B28 Host"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Short; Text[10]) { FieldClass = FlowField; CalcFormula = lookup("B28 Src".Txt where(Id = field(Id))); }
        field(3; ShortCd; Code[10]) { FieldClass = FlowField; CalcFormula = lookup("B28 Src".Cd where(Id = field(Id))); }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 60972 C
{
    local procedure Seed()
    var S: Record "B28 Src";
    begin
        S.Id := 1; S.Txt := 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123'; S.Cd := 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123'; S.Insert();
    end;
    procedure LongText(): Text
    var H: Record "B28 Host";
    begin
        Seed(); H.Id := 1; H.CalcFields(Short);
        exit(H.Short + '|' + Format(StrLen(H.Short)));
    end;
    procedure LongCode(): Text
    var H: Record "B28 Host";
    begin
        Seed(); H.Id := 1; H.CalcFields(ShortCd);
        exit(H.ShortCd);
    end;
}
"#;

#[test]
fn a_lookup_text_flowfield_shorter_than_its_source_holds_the_whole_value() {
    // record/flowfields/BmFlowFieldConversions.al:201-217 (BM_LookupFlowField_ShorterThanSource,
    // `short_30_chars`): a Text[10] lookup over a Text[30] source reads all 30 characters, on every version.
    for bc in ["27.5", "28.5", "29.0"] {
        assert_eq!(
            text(run_on(bc, LOOKUP, "LongText")),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123|30",
            "{bc}"
        );
    }
}

#[test]
fn a_lookup_code_flowfield_shorter_than_its_value_is_refused() {
    // Only Text over Text is pinned (BmFlowFieldConversions.al:50-58).
    assert!(refused(run_on("29.0", LOOKUP, "LongCode")).contains("lookup"));
}
