//! ADR-023: until a probe on real BC pins an access, extension code that names the base object's
//! members is refused, and the reason names the probe. This binary puts the checker scope on and
//! pins nothing (`AL_UNPINNED_BASE_ACCESS=none`); `base_access.rs` is the same code with every
//! case pinned.

use al_sema::workspace::AppSource;

fn reasons(src: &str) -> Vec<String> {
    // SAFETY: every test of this binary sets the same value before the first compile.
    unsafe { std::env::set_var("AL_UNPINNED_BASE_ACCESS", "none") };
    let apps = vec![AppSource {
        id: "app0".into(),
        name: "app0".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    l.unsupported
        .iter()
        .flatten()
        .flatten()
        .map(|r| r.to_string())
        .collect()
}

const PAGE: &str = r#"
table 53000 "BA Row" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
page 53001 "BA Card"
{
    PageType = Card;
    SourceTable = "BA Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    protected procedure Bump(By: Integer): Integer begin exit(By); end;
    protected var Counter: Integer;
}
"#;

#[test]
fn a_variable_read_is_refused_and_names_its_probe() {
    let r = reasons(&format!(
        "{PAGE} pageextension 53002 E extends \"BA Card\" {{ trigger OnOpenPage() var X: Integer; begin X := Counter; end; }}"
    ));
    assert!(
        r.iter().any(|x| x.contains("`counter`") && x.contains("probe P1/P2")),
        "{r:?}"
    );
}

#[test]
fn a_variable_write_is_refused_and_names_its_probe() {
    let r = reasons(&format!(
        "{PAGE} pageextension 53002 E extends \"BA Card\" {{ trigger OnOpenPage() begin Counter := 1; end; }}"
    ));
    assert!(r.iter().any(|x| x.contains("probe P1/P2")), "{r:?}");
}

#[test]
fn a_base_procedure_call_is_refused_and_names_its_probe() {
    let r = reasons(&format!(
        "{PAGE} pageextension 53002 E extends \"BA Card\" {{ trigger OnOpenPage() begin Bump(1); end; }}"
    ));
    assert!(
        r.iter().any(|x| x.contains("`Bump`") && x.contains("probe P4")),
        "{r:?}"
    );
}

#[test]
fn a_control_source_is_refused() {
    let r = reasons(&format!(
        "{PAGE} pageextension 53002 E extends \"BA Card\" {{ layout {{ addlast(Content) {{ field(C; Format(Counter)) {{ }} }} }} }}"
    ));
    assert!(r.iter().any(|x| x.contains("probe P1/P2")), "{r:?}");
}

#[test]
fn a_report_variable_is_refused() {
    let r = reasons(
        r#"
table 53000 "BA Row" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
report 53010 "BA Report" { ProcessingOnly = true; dataset { dataitem(Rows; "BA Row") { } } protected var Stage: Text; }
reportextension 53011 E extends "BA Report" { trigger OnPreReport() begin Stage := 'x'; end; }"#,
    );
    assert!(r.iter().any(|x| x.contains("probe R1")), "{r:?}");
}
