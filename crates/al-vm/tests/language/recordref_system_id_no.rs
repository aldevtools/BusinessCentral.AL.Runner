//! `RecordRef.SystemIdNo()` on a RecordRef that is not open answers 2000000000 without an error: never opened, after
//! Close (Number 0) and after Clear, as when it is open (corpus branch al-runtime-rs-staging-y30 at 2f7609a1,
//! tests/al-language/recordref/W4RecordRefGetUnopened.al:178-194, Y30_SystemIdNo_NotOpen, measured on BC 29). The
//! other `System*No` members on a closed RecordRef are not pinned and stay refused.

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

    procedure CreatedAt(): Integer
    var
        RR: RecordRef;
    begin
        exit(RR.SystemCreatedAtNo());
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
fn system_created_at_no_on_a_record_ref_that_is_not_open_stays_refused() {
    assert!(matches!(
        run_in(SRC, "LT SysNo", "CreatedAt"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
