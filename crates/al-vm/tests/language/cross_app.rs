//! Objects of one app used by another: table extensions of another app's table (corpus
//! tableextension/lt-bw-cross-app/BwCrossAppTests.al with the cross-app fixtures tests/al-language-xapp-a, -b).

use al_sema::workspace::AppSource;
use al_testkit::text;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const BASE: &str = r#"
table 61101 "XA Base"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
    procedure BwSame(): Text begin exit('table of app A'); end;
    procedure BwOverload(Value: Integer): Text begin exit('table of app A, Integer ' + Format(Value)); end;
    procedure BwCallSameFromA(): Text begin exit(BwSame()); end;
}
"#;

const EXT: &str = r#"
tableextension 61113 "XB Base Same Ext" extends "XA Base"
{
    procedure BwSame(): Text begin exit('extension of app B'); end;
    procedure BwOverload(Value: Text): Text begin exit('extension of app B, Text ' + Value); end;
    procedure BwCallSameFromB(): Text begin exit(BwSame()); end;
}
"#;

const CALLER: &str = r#"
codeunit 83951 T
{
    procedure FromA(): Text var B: Record "XA Base"; begin exit(B.BwCallSameFromA()); end;
    procedure FromB(): Text var B: Record "XA Base"; begin exit(B.BwCallSameFromB()); end;
    procedure OverloadInt(): Text var B: Record "XA Base"; begin exit(B.BwOverload(5)); end;
    procedure OverloadText(): Text var B: Record "XA Base"; begin exit(B.BwOverload('x')); end;
}
"#;

fn built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![
        app("a", &[], BASE),
        app("b", &["a"], EXT),
        app("m", &["a", "b"], CALLER),
    ]))
}

/// A table extension of another app declares a procedure with the name and parameters of one of the base table
/// (BC compiles it with warning AL0523). A bare call in the table's own code runs the table's procedure, one in the
/// extension's code the extension's (BwCrossAppTests.al:34-39); overloads that differ in parameter type are picked by
/// the argument (BwCrossAppTests.al:36-41).
#[test]
fn a_bare_call_runs_the_procedure_of_the_object_whose_code_makes_it() {
    let mut b = built();
    assert_eq!(text(b.run_in("T", "FromA")), "table of app A");
    assert_eq!(text(b.run_in("T", "FromB")), "extension of app B");
    assert_eq!(text(b.run_in("T", "OverloadInt")), "table of app A, Integer 5");
    assert_eq!(text(b.run_in("T", "OverloadText")), "extension of app B, Text x");
}

/// A bare call of that name in a third extension of the table (another app that sees both) is not pinned: refused.
/// (A call from a third app on a record variable is AL0196, must-not-compile case 1581.)
#[test]
fn a_bare_call_of_a_name_two_other_objects_declare_alike_is_refused() {
    let c = r#"
tableextension 61123 "XC Base Ext" extends "XA Base"
{
    procedure BwCallSameFromC(): Text begin exit(BwSame()); end;
}
codeunit 83952 TC
{
    procedure FromC(): Text var B: Record "XA Base"; begin exit(B.BwCallSameFromC()); end;
}
"#;
    let (_ws, l) = al_sema::lower::compile(
        vec![app("a", &[], BASE), app("b", &["a"], EXT), app("c", &["a", "b"], c)],
        &Default::default(),
    );
    let mut b = al_testkit::Built::new(l);
    let r = b.run_in("TC", "FromC");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("BwSame")),
        "{r:?}"
    );
}

const RENAME_TABLE: &str = r#"
table 61102 "XA Rename Blob"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
"#;

const RENAME_EXT: &str = r#"
tableextension 61111 "XB Rename Blob Ext" extends "XA Rename Blob"
{
    fields { field(61110; "BW XB Blob"; Blob) { } }
}
"#;

const RENAME_CALLER: &str = r#"
codeunit 83953 R
{
    procedure BlobText(var Row: Record "XA Rename Blob"): Text
    var InS: InStream; T: Text;
    begin
        if not Row."BW XB Blob".HasValue() then exit('<none>');
        Row."BW XB Blob".CreateInStream(InS); InS.ReadText(T); exit(T);
    end;
    procedure WrittenBeforeInsert(): Text
    var Row: Record "XA Rename Blob"; Fresh: Record "XA Rename Blob"; OutS: OutStream;
    begin
        Row."No." := 'OLD';
        Row."BW XB Blob".CreateOutStream(OutS); OutS.WriteText('blob of OLD');
        Row.Insert();
        Row.Rename('NEW');
        Fresh.Get('NEW'); Fresh.CalcFields("BW XB Blob");
        exit(BlobText(Row) + '|' + BlobText(Fresh));
    end;
    procedure WrittenAfterInsert(): Text
    var Row: Record "XA Rename Blob"; OutS: OutStream;
    begin
        Row."No." := 'OLD';
        Row.Insert();
        Row."BW XB Blob".CreateOutStream(OutS); OutS.WriteText('late');
        Row.Rename('NEW');
        exit(BlobText(Row));
    end;
}
"#;

fn rename_built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![
        app("a", &[], RENAME_TABLE),
        app("b", &["a"], RENAME_EXT),
        app("m", &["a", "b"], RENAME_CALLER),
    ]))
}

/// A Blob that another app's table extension adds, written before the Insert: after a Rename the record variable shows
/// no value, while the renamed row keeps the bytes (BwCrossAppTests.al:89-115, `blob in variable after rename = <none>`,
/// `NEW blob = blob of OLD`).
#[test]
fn a_rename_leaves_no_value_in_another_apps_extension_blob_of_the_variable() {
    assert_eq!(
        text(rename_built().run_in("R", "WrittenBeforeInsert")),
        "<none>|blob of OLD"
    );
}

/// Written after the last Insert or Modify, nothing pins what the variable or the row hold after the Rename: refused.
#[test]
fn a_rename_after_an_unsaved_write_to_another_apps_extension_blob_is_refused() {
    let r = rename_built().run_in("R", "WrittenAfterInsert");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("Rename")),
        "{r:?}"
    );
}
