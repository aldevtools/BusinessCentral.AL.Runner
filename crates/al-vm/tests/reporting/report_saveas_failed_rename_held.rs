//! A SaveAs of a report with an RDLC layout whose answer is used answers false when a trigger raises an error, and the error
//! stays the last error (streams/TestReportSaveAsRecordRefFilter.al:TestReport_SaveAs_NoRecordRef_LeavesDataItemUnfiltered).
//! When the error is the SQL range error of a Rename and the renamed record still lives (here a global of a SingleInstance
//! codeunit), what that record holds is not measured (record/filters/SqlDateFloor1753.al:2468-2600 reads only the stored
//! row): the SaveAs refuses, as `asserterror` does (crates/al-vm/tests/store/sql_date_floor.rs
//! `a_record_alive_where_its_failed_rename_is_caught_refuses`).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 50100 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
table 50103 "Date Key" { fields { field(1; Dk; Date) { } } keys { key(PK; Dk) { } } }
codeunit 50104 Keeper
{
    SingleInstance = true;
    var R: Record "Date Key";
    procedure Fail(What: Text)
    begin
        R.Dk := 20240102D;
        R.Insert();
        if What = 'rename' then
            R.Rename(17530615D);
        Error('plain');
    end;
}
report 50101 Rep
{
    DefaultRenderingLayout = Rdlc;
    dataset { dataitem(B; Buf) { column(BId; Id) { } } }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './x.rdl'; } }
    trigger OnPreReport()
    var K: Codeunit Keeper;
    begin
        K.Fail(What);
    end;
    var What: Text;
    procedure SetWhat(W: Text)
    begin
        What := W;
    end;
}
codeunit 50102 C
{
    procedure Guarded(W: Text): Text
    var B: Record Buf temporary; P: Report Rep; OutS: OutStream;
    begin
        B.Data.CreateOutStream(OutS);
        P.SetWhat(W);
        if P.SaveAs('', ReportFormat::Xml, OutS) then
            exit('true');
        exit('false|' + GetLastErrorText());
    end;
}
"#;

fn guarded(what: &str) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", "Guarded", vec![Value::Text(what.into())]).map(|r| r.0)
}

#[test]
fn a_used_saveas_whose_trigger_fails_keeps_the_error() {
    let r = guarded("other");
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "false|plain"), "{r:?}");
}

#[test]
fn a_used_saveas_whose_rename_failed_while_the_record_lives_refuses() {
    let r = guarded("rename");
    assert!(
        matches!(&r, Err(Signal::Unsupported(why)) if why.contains("Rename raised the SQL range error")),
        "{r:?}"
    );
}
