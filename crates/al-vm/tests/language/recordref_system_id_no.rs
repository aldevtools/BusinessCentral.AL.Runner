//! `RecordRef.SystemIdNo()` on a RecordRef that is not open answers 2000000000 without an error: never opened, after
//! Close (Number 0) and after Clear, as when it is open (corpus branch al-runtime-rs-staging-y30 at 2f7609a1,
//! tests/al-language/recordref/W4RecordRefGetUnopened.al:178-194, Y30_SystemIdNo_NotOpen, measured on BC 29). The
//! audit members answer 2000000001-2000000004 in every state, without an error (corpus master ac3499d6,
//! W4RecordRefGetUnopened.al:317, Y40_SystemAuditFieldNos_NotOpen, BC 27.5/28.5/29.0).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50730 "LT SysNo Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

codeunit 50731 "LT SysNo"
{
    procedure Go(): Text
    var
        RR: RecordRef;
        Out: Text;
    begin
        Out := Format(RR.SystemIdNo());
        RR.Open(Database::"LT SysNo Row");
        Out += ' ' + Format(RR.SystemIdNo());
        RR.Close();
        Out += ' ' + Format(RR.SystemIdNo()) + ' ' + Format(RR.Number());
        RR.Open(Database::"LT SysNo Row");
        Clear(RR);
        Out += ' ' + Format(RR.SystemIdNo());
        exit(Out);
    end;

    procedure AuditNos(): Text
    var
        RR: RecordRef;
        Out: Text;
    begin
        Out := Nos(RR);
        RR.Open(Database::"LT SysNo Row");
        Out += Nos(RR);
        RR.Close();
        Out += Nos(RR);
        RR.Open(Database::"LT SysNo Row");
        Clear(RR);
        Out += Nos(RR);
        exit(Out);
    end;

    local procedure Nos(var RR: RecordRef): Text
    begin
        exit(Format(RR.SystemCreatedAtNo()) + ';' + Format(RR.SystemCreatedByNo()) + ';' + Format(RR.SystemModifiedAtNo()) + ';' + Format(RR.SystemModifiedByNo()) + '|');
    end;
}
"#;

#[test]
fn system_id_no_on_a_record_ref_that_is_not_open() {
    assert_eq!(
        text(run_in(SRC, "LT SysNo", "Go")),
        "2000000000 2000000000 2000000000 0 2000000000"
    );
}

#[test]
fn audit_field_nos_on_a_record_ref_that_is_not_open() {
    let one = "2000000001;2000000002;2000000003;2000000004|";
    assert_eq!(text(run_in(SRC, "LT SysNo", "AuditNos")), one.repeat(4));
}
