//! A guarded `XmlPort.Import` that fails keeps the error as the last error (xmlports/TestXmlPortImportWriteTx.al:
//! StaticGuardedImport_FailingSecondElement_ReturnsFalse_AndRollsBackTheFirstRow). When the error is the SQL range error of a
//! Rename and the renamed record still lives (here a global of the XmlPort variable), what that record holds is not measured
//! (record/filters/SqlDateFloor1753.al:2468-2600 reads only the stored row): the import refuses, as `asserterror` does
//! (crates/al-vm/tests/store/sql_date_floor.rs `a_record_alive_where_its_failed_rename_is_caught_refuses`).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 50100 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
table 50103 "Date Key" { fields { field(1; Dk; Date) { } } keys { key(PK; Dk) { } } }
xmlport 50101 Imp
{
    Format = Xml;
    Direction = Import;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(N)
            {
                trigger OnAfterAssignVariable()
                begin
                    R.Dk := 20240102D;
                    R.Insert();
                    if N = 'rename' then
                        R.Rename(17530615D);
                    Error('plain');
                end;
            }
        }
    }
    var R: Record "Date Key";
}
codeunit 50102 C
{
    procedure Guarded(What: Text): Text
    var B: Record Buf temporary; P: XmlPort Imp; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><N>' + What + '</N></Root>');
        B.Data.CreateInStream(InS);
        P.SetSource(InS);
        if P.Import() then
            exit('true');
        exit('false|' + GetLastErrorText());
    end;
}
"#;

fn guarded(what: &str) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", "Guarded", vec![Value::Text(what.into())]).map(|r| r.0)
}

#[test]
fn a_guarded_import_that_fails_keeps_the_error() {
    let r = guarded("other");
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "false|plain"), "{r:?}");
}

#[test]
fn a_guarded_import_whose_rename_failed_while_the_record_lives_refuses() {
    let r = guarded("rename");
    assert!(
        matches!(&r, Err(Signal::Unsupported(why)) if why.contains("Rename raised the SQL range error")),
        "{r:?}"
    );
}
