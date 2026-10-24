//! StrSubstNo, Message and Error `%n` of a Decimal table field that declares its format (DecimalPlaces, AutoFormatType or
//! AutoFormatExpression): refused when it runs (al-runner issue 5791: BC gives `70.00` for an AutoFormatType = 1 field, the
//! runtime gave `70`; no corpus test pins it, docs/probes-needed.md). A field without them and Format of the field still run.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50760 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { AutoFormatType = 1; }
        field(3; Plain; Decimal) { }
        field(4; Places; Decimal) { DecimalPlaces = 2 : 2; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50761 C
{
    procedure SubstAmount(): Text
    var R: Record R;
    begin
        R.Amount := 70;
        exit(StrSubstNo('%1', R.Amount));
    end;

    procedure SubstPlaces(): Text
    var R: Record R;
    begin
        R.Places := 70;
        exit(StrSubstNo('%1', R.Places));
    end;

    procedure ErrorAmount(): Text
    var R: Record R;
    begin
        R.Amount := 70;
        asserterror Error('%1', R.Amount);
        exit(GetLastErrorText());
    end;

    procedure SubstPlain(): Text
    var R: Record R;
    begin
        R.Plain := 70;
        exit(StrSubstNo('%1', R.Plain));
    end;

    procedure SubstNotReached(): Text
    var R: Record R;
    begin
        if R.Id <> 0 then
            exit(StrSubstNo('%1', R.Amount));
        exit('skipped');
    end;
}
"#;

fn refused(routine: &str) {
    let r = run_in(SRC, "C", routine);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("not pinned")),
        "{routine}: {r:?}"
    );
}

#[test]
fn a_field_that_declares_its_format_is_refused_in_strsubstno_and_error() {
    refused("SubstAmount");
    refused("SubstPlaces");
    refused("ErrorAmount");
}

#[test]
fn a_plain_field_and_code_that_does_not_reach_the_call_still_run() {
    assert_eq!(text(run_in(SRC, "C", "SubstPlain")), "70");
    assert_eq!(text(run_in(SRC, "C", "SubstNotReached")), "skipped");
}
