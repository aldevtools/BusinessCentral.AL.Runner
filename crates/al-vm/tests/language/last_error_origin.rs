//! `GetLastErrorText(true)` answers a text that depends on how the error was raised. Each expectation cites the corpus line
//! that pins it (format-evaluate/P86LastError.al, errors/last-error/P70_02LastErrFlag.al,
//! errors/PnF01LastErrorFlag.al). An origin the corpus does not pin is refused.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69870 "LO Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { DataClassification = CustomerContent; } field(3; Amount; Decimal) { } field(4; Kind; Option) { OptionMembers = Zero,One; } } keys { key(PK; Id) { } } }
table 69872 "LO Blob" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } field(3; Data; Blob) { } } keys { key(PK; Code) { } key(Sec; Name) { Unique = true; } } }
codeunit 69871 "LO C"
{
    var Lbl: Label 'a label text', Locked = true;
        ArgLbl: Label 'label with %1';
        GlobalText: Text;

    procedure NoError(): Text begin exit(GetLastErrorText(true)); end;

    procedure Info(): Text
    var E: ErrorInfo;
    begin
        E.Message := 'secret';
        asserterror Error(E);
        exit(GetLastErrorText(true));
    end;

    procedure Literal(): Text begin asserterror Error('plain %1', 'argument'); exit(GetLastErrorText(true)); end;
    procedure EmptyLiteral(): Text begin asserterror Error(''); exit(GetLastErrorText(true)); end;
    procedure FromLabel(): Text begin asserterror Error(Lbl); exit(GetLastErrorText(true)); end;
    procedure FromLabelArgs(): Text begin asserterror Error(ArgLbl, 'x'); exit(GetLastErrorText(true)); end;
    procedure FromGlobalText(): Text begin GlobalText := 'g'; asserterror Error(GlobalText); exit(GetLastErrorText(true)); end;
    procedure FromLocalText(): Text var T: Text; begin T := 'v %1'; asserterror Error(T, 1); exit(GetLastErrorText(true)); end;
    procedure FromConcat(): Text var T: Text; begin T := 'v'; asserterror Error(T + ' w'); exit(GetLastErrorText(true)); end;
    procedure FromStrSubstNo(): Text begin asserterror Error(StrSubstNo(ArgLbl, 'y')); exit(GetLastErrorText(true)); end;

    procedure FieldErrLabel(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1; R.Name := 'Jane';
        asserterror R.FieldError(Name, Lbl);
        exit(GetLastErrorText(true));
    end;

    procedure FieldErrLiteral(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1;
        asserterror R.FieldError(Name, 'some text');
        exit(GetLastErrorText(true));
    end;

    procedure FieldErrBlank(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1;
        asserterror R.FieldError(Name);
        exit(GetLastErrorText(true));
    end;

    procedure FieldErrGlobalText(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1; GlobalText := 'g';
        asserterror R.FieldError(Name, GlobalText);
        exit(GetLastErrorText(true));
    end;

    procedure TestFieldZeroOption(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1;
        asserterror R.TestField(Kind);
        exit(GetLastErrorText(true));
    end;

    procedure InsertDuplicate(): Text
    var R: Record "LO Blob";
    begin
        R.Code := 'A'; R.Name := 'n1'; R.Insert();
        R.Name := 'n2';
        asserterror R.Insert();
        exit(GetLastErrorText(true));
    end;

    procedure InsertDuplicateUnique(): Text
    var R: Record "LO Blob";
    begin
        R.Code := 'B'; R.Name := 'same'; R.Insert();
        R.Code := 'C';
        asserterror R.Insert();
        exit(GetLastErrorText(true));
    end;

    procedure NothingInFilter(): Text
    var R: Record "LO Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.SetRange(Id, 5);
        asserterror R.FindFirst();
        exit(GetLastErrorText(true));
    end;

    procedure TestFieldBlank(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1;
        asserterror R.TestField(Name);
        exit(GetLastErrorText(true));
    end;

    procedure TestFieldValue(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1; R.Name := 'Jane';
        asserterror R.TestField(Name, 'John');
        exit(GetLastErrorText(true));
    end;

    procedure FieldErr(): Text
    var R: Record "LO Row";
    begin
        R.Id := 1; R.Name := 'Jane';
        asserterror R.FieldError(Name);
        exit(GetLastErrorText(true));
    end;

    procedure Missing(): Text
    var R: Record "LO Row";
    begin
        R.DeleteAll();
        asserterror R.Get(99);
        exit(GetLastErrorText(true));
    end;

    procedure Divide(): Text
    var A: Integer; Z: Integer;
    begin
        A := 1;
        asserterror A := A div Z;
        exit(GetLastErrorText(true));
    end;
}
"#;

fn flag(routine: &str) -> String {
    text(run_in(SRC, "LO C", routine))
}

#[test]
fn the_pinned_origins_answer_the_pinned_texts() {
    // P86LastError.al:410 (cleared), :189/:192 (empty Error text).
    assert_eq!(flag("NoError"), "");
    assert_eq!(flag("EmptyLiteral"), "");
    // P86LastError.al:54 and the other ErrorInfo cases.
    assert_eq!(
        flag("Info"),
        "This message had personal data removed. The original may still be in transient telemetry. Find it using the transientTelemetryId."
    );
    // P86LastError.al:189, PnF01LastErrorFlag.al:85, :103.
    assert_eq!(
        flag("Literal"),
        "Use ERROR with a text constant to improve telemetry details"
    );
    // P86LastError.al:225 (blank), :233 (value), P70_02LastErrFlag.al:123, :135.
    assert_eq!(
        flag("TestFieldBlank"),
        "Name must have a value in LO Row: <redacted>. It cannot be zero or empty."
    );
    assert_eq!(
        flag("TestFieldValue"),
        "Name must be equal to '<redacted>'  in LO Row: <redacted>. Current value is '<redacted>'."
    );
    // P86LastError.al:270.
    assert_eq!(flag("FieldErr"), "FieldError for incorrect value of LO Row.Name");
    // P86LastError.al:309, P70_02LastErrFlag.al:88.
    assert_eq!(
        flag("Missing"),
        "The LO Row does not exist. Identification fields and values: <redacted>"
    );
    // P86LastError.al:340.
    assert_eq!(flag("Divide"), "Attempted to divide by zero.");
}

#[test]
fn the_text_forms_of_error_and_fielderror_answer_the_pinned_texts() {
    // errors/last-error/LastErrorTextExcludeLabel.al:136, :155 (a global Label, before substitution).
    assert_eq!(flag("FromLabel"), "a label text");
    assert_eq!(flag("FromLabelArgs"), "label with %1");
    // lt-last-error-text-forms/LastErrorTextExcludeTextVariable.al:134, :140; LastErrorTextExcludeLabel.al:163.
    let use_error = "Use ERROR with a text constant to improve telemetry details";
    assert_eq!(flag("FromLocalText"), use_error);
    assert_eq!(flag("FromConcat"), use_error);
    assert_eq!(flag("FromStrSubstNo"), use_error);
    // LastErrorTextExcludeLabel.al:246; P86LastError.al:273; FieldErrorTextVariable.al:106.
    assert_eq!(
        flag("FieldErrLabel"),
        "FieldError on field LO Row.Name, message: a label text"
    );
    assert_eq!(
        flag("FieldErrLiteral"),
        "FieldError on field LO Row.Name, message: Use FIELDERROR with a text constant to improve Telemetry details."
    );
    assert_eq!(flag("FieldErrBlank"), "FieldError for unspecified value of LO Row.Name");
    // TestFieldOptionEnumExcludeFlag.al:83.
    assert_eq!(
        flag("TestFieldZeroOption"),
        "Kind must not be Zero in LO Row: <redacted>"
    );
    // InsertDuplicateKeyExcludeFlag.al:68, :121.
    assert_eq!(
        flag("InsertDuplicate"),
        "The record in table LO Blob already exists. Identification fields and values: <redacted>"
    );
    assert_eq!(
        flag("InsertDuplicateUnique"),
        "There is already a record in table LO Blob that has the same values in a unique index for the following fields: <redacted>"
    );
    // FindEmptyTableExcludeFlag.al:148.
    assert_eq!(
        flag("NothingInFilter"),
        "There is no LO Row within the filter.\\\\Filters: <redacted>"
    );
}

#[test]
fn an_error_text_of_an_unpinned_form_is_refused() {
    for routine in ["FromGlobalText", "FieldErrGlobalText"] {
        match run_in(SRC, "LO C", routine) {
            Err(Signal::Unsupported(m)) => assert!(m.contains("does not pin"), "{routine}: {m}"),
            other => panic!("{routine}: {other:?}"),
        }
    }
}
