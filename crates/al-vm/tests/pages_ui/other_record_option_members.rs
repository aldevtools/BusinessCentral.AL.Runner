//! A List page control bound to a field of another record variable of the page (al-runner issue 7005; corpus
//! pages/controls/OtherRecordOptionMembers.al, measured on the CI legs 27.5, 28.5 and 29.0; the reporter's Windows BC 28.5 run
//! agrees): an Option field with OptionCaption shows its member name before BC 29 and its caption from BC 29; every such control
//! reads blank while the list shows no row.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69950 "OM Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Avail. Filter"; Option) { OptionMembers = All,Short,Covered; OptionCaption = 'Every Line,Short Lines,Covered Lines'; }
        field(3; Plain; Option) { OptionMembers = A,B,C; }
        field(4; Kind; Enum "OM Kind") { }
    }
    keys { key(PK; "Primary Key") { } }
}
enum 69951 "OM Kind" { value(0; First) { Caption = 'First Kind'; } value(1; Second) { Caption = 'Second Kind'; } }
table 69952 "OM Line" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 69953 "OM List"
{
    PageType = List; SourceTable = "OM Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(VarOpt; FilterSetup."Avail. Filter") { }
                field(VarPlain; FilterSetup.Plain) { }
                field(VarKind; FilterSetup.Kind) { }
            }
            repeater(Rows) { field(Id; Rec.Id) { } field(RowVarOpt; FilterSetup."Avail. Filter") { } }
        }
    }
    trigger OnOpenPage()
    begin
        FilterSetup."Avail. Filter" := FilterSetup."Avail. Filter"::Short;
        FilterSetup.Plain := FilterSetup.Plain::B;
        FilterSetup.Kind := FilterSetup.Kind::Second;
    end;
    var FilterSetup: Record "OM Setup";
}
codeunit 69954 C
{
    local procedure Shows(Rows: Integer): Text
    var P: TestPage "OM List"; Line: Record "OM Line"; i: Integer; T: Text;
    begin
        for i := 1 to Rows do begin Line.Id := i; Line.Insert(); end;
        P.OpenView();
        T := '[' + P.VarOpt.Value() + '|' + P.VarPlain.Value() + '|' + P.VarKind.Value() + '|' + P.RowVarOpt.Value() + ']';
        P.Close();
        exit(T);
    end;
    procedure OneRow(): Text begin exit(Shows(1)); end;
    procedure NoRows(): Text begin exit(Shows(0)); end;
}
"#;

fn run_on(bc: &str, routine: &str) -> Result<Value, Signal> {
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

// OtherRecordOptionMembers.al OtherRecordOption_OneRow_Values: `[Short]` on 28.5 (also the reporter's Windows run), the enum's
// caption on every version.
#[test]
fn before_29_the_option_control_of_another_record_shows_the_member_name() {
    assert_eq!(
        al_testkit::text(run_on("28.5", "OneRow")),
        "[Short|B|Second Kind|Short]"
    );
}

// OtherRecordOption_OneRow_Values on 29.0: `[Short Lines]`.
#[test]
fn from_29_it_shows_the_option_caption() {
    assert_eq!(
        al_testkit::text(run_on("29.0", "OneRow")),
        "[Short Lines|B|Second Kind|Short Lines]"
    );
}

// OtherRecordOption_NoRows_Values: every control bound to a field of the other record reads blank on a list without rows, on
// every version.
#[test]
fn without_rows_every_such_control_reads_blank() {
    assert_eq!(al_testkit::text(run_on("29.0", "NoRows")), "[|||]");
    assert_eq!(al_testkit::text(run_on("28.5", "NoRows")), "[|||]");
}

// 27.5 is measured on the Linux CI legs only: still refused.
#[test]
fn on_27_it_stays_refused() {
    assert!(matches!(run_on("27.5", "OneRow"), Err(Signal::Unsupported(_))));
}
