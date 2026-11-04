//! A SQL counter value stored in an Integer, a Decimal, a Dictionary or a List value, and formatted for telemetry (al-runner
//! issue 6924). The corpus pins (session-system/session/SqlCounters.al, the P6924 tests) that such storage raises no error,
//! keeps the value not negative and a later read not smaller, and that Format and `Session.LogMessage` with the formatted
//! value in a custom dimension raise no error. The digits themselves are not pinned: every use that reads them is refused.

use al_builtins::native::Signal;
use al_testkit::{boolean, run, text};

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }";

fn src(body: &str) -> String {
    format!(
        "{TABLE}
codeunit 50101 C {{
    var Started: Dictionary of [Guid, Integer];
    local procedure Seed()
    var Row: Record Row; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 3 do begin Row.Id := I; Row.Name := 'row'; Row.Insert(); end;
        Commit();
    end;
    local procedure Start(Id: Guid) begin Started.Add(Id, SessionInformation.SqlStatementsExecuted()); end;
    local procedure Spent(Id: Guid): Integer
    var Count: Integer;
    begin
        Count := SessionInformation.SqlStatementsExecuted() - Started.Get(Id);
        exit(Count);
    end;
    {body}
}}"
    )
}

fn refused(r: Result<al_builtins::value::Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal containing `{needle}`, got {other:?}"),
    }
}

/// SqlCounters.al P6924_Counters_StoredInInteger_NotNegative: a read stored in an Integer is not negative, and it is an
/// Integer after the assignment.
#[test]
fn a_read_in_an_integer_is_not_negative() {
    let s = src("procedure F(): Text var Stmt: Integer; V: Variant;
        begin
            Stmt := SessionInformation.SqlStatementsExecuted();
            V := Stmt;
            exit(Format(Stmt >= 0) + Format(V.IsInteger()) + Format(V.IsBigInteger()));
        end;");
    assert_eq!(text(run(&s, "F")), "YesYesNo");
}

/// SqlCounters.al P6924_Counters_StoredInDecimal_NotNegativeAndWhole: a read in a Decimal is not negative, whole, and a later
/// read minus it is not negative.
#[test]
fn a_read_in_a_decimal_is_not_negative_and_whole() {
    let s = src("procedure F(): Boolean var Stmt: Decimal;
        begin
            Stmt := SessionInformation.SqlStatementsExecuted();
            exit((Stmt >= 0) and (Stmt = Round(Stmt, 1)) and (SessionInformation.SqlStatementsExecuted() - Stmt >= 0));
        end;");
    assert!(boolean(run(&s, "F")));
}

/// SqlCounters.al P6924_Counters_DictionaryOfGuidInteger_AddGetAndLaterDifference: stored in a Dictionary of [Guid, Integer]
/// value, the later read minus the stored value keeps the sign of the change (a Get on a table after a Commit adds to it).
#[test]
fn a_read_in_a_dictionary_value_keeps_the_sign_of_the_change() {
    let s = src("procedure F(): Text var Row: Record Row; Id: Guid; R: Text;
        begin
            Seed();
            Id := CreateGuid();
            Start(Id);
            R := Format(Spent(Id) >= 0);
            Row.Get(2);
            R += Format(Spent(Id) > 0);
            exit(R);
        end;");
    assert_eq!(text(run(&s, "F")), "YesYes");
}

/// A List value holds a read the same way.
#[test]
fn a_read_in_a_list_value_keeps_the_sign_of_the_change() {
    let s = src("procedure F(): Boolean var L: List of [BigInteger]; Row: Record Row;
        begin
            Seed();
            L.Add(SessionInformation.SqlStatementsExecuted());
            Row.Get(2);
            exit(SessionInformation.SqlStatementsExecuted() - L.Get(1) > 0);
        end;");
    assert!(boolean(run(&s, "F")));
}

/// SqlCounters.al P6924_Counters_FormattedInTelemetryDimension_NoError: the formatted difference goes into a custom dimension
/// of `Session.LogMessage` without an error.
#[test]
fn a_formatted_read_goes_to_telemetry() {
    let s = src("procedure F(): Boolean var Id: Guid; Dimensions: Dictionary of [Text, Text];
        begin
            Id := CreateGuid();
            Start(Id);
            Dimensions.Add('SqlStatements', Format(Spent(Id)));
            Session.LogMessage('0000ABC', 'done', Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::ExtensionPublisher, Dimensions);
            Session.LogMessage('0000ABD', 'done ' + Format(SessionInformation.SqlRowsRead()), Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::ExtensionPublisher, 'SqlRows', Format(SessionInformation.SqlRowsRead()));
            exit(true);
        end;");
    assert!(boolean(run(&s, "F")));
}

/// The digits are not pinned: any use of the formatted text, or of the stored number, that shows them is refused.
#[test]
fn uses_that_show_the_digits_are_refused() {
    for body in [
        "procedure F(): Integer begin exit(StrLen(Format(SessionInformation.SqlStatementsExecuted()))); end;",
        "procedure F(): Boolean begin exit(Format(SessionInformation.SqlStatementsExecuted()) = '5'); end;",
        "procedure F(): Text begin Message(Format(SessionInformation.SqlStatementsExecuted())); end;",
        "procedure F(): Text begin Error('Count %1', Format(SessionInformation.SqlStatementsExecuted())); end;",
        "procedure F(): Boolean var I: Integer; begin exit(Evaluate(I, Format(SessionInformation.SqlStatementsExecuted()))); end;",
        "procedure F(): Boolean var I: Integer; begin I := SessionInformation.SqlStatementsExecuted(); exit(I = 3); end;",
        "procedure F(): Boolean var I: Integer; begin I := SessionInformation.SqlStatementsExecuted(); exit(I > 0); end;",
        "procedure F(): Text var I: Integer; begin I := SessionInformation.SqlStatementsExecuted(); exit(Format(I * 2)); end;",
        "procedure F(): Text var D: Decimal; begin D := SessionInformation.SqlStatementsExecuted(); exit(Format(Round(D, 0.5))); end;",
        "procedure F(): Text var T: Text; begin T := CopyStr(Format(SessionInformation.SqlRowsRead()), 1, 1); exit(T); end;",
        "procedure F(): Text var Row: Record Row; I: Integer; begin I := SessionInformation.SqlRowsRead(); Row.Id := I; end;",
    ] {
        refused(run(&src(body), "F"), "");
    }
}

/// A key is compared with the other keys, which reads the number: a read in key position stays refused.
#[test]
fn a_read_as_a_dictionary_key_is_refused() {
    let s = src(
        "procedure F(): Text var D: Dictionary of [BigInteger, Text]; begin D.Add(SessionInformation.SqlStatementsExecuted(), 'x'); end;",
    );
    refused(run(&s, "F"), "SQL counter");
    let s = src(
        "procedure F(): Boolean var L: List of [BigInteger]; begin L.Add(1); exit(L.Contains(SessionInformation.SqlStatementsExecuted())); end;",
    );
    refused(run(&s, "F"), "SQL counter");
}
