//! A difference of two reads of one SQL counter with work between them whose change the corpus does not pin (al-runner issue
//! 6924, the reporter's performance-telemetry helper): a SingleInstance codeunit stores the first read in a
//! `Dictionary of [Guid, Integer]`, real business logic runs, and the second read minus the stored one goes, formatted with
//! `Format(x, 0, 9)`, into a telemetry custom dimension. The difference is a tainted value of unknown size: it may be
//! stored, passed on, formatted and logged (SqlCounters.al P6924_Counters_FormattedInTelemetryDimension_NoError), and every
//! use that would read it (a comparison, more arithmetic, the digits) stays refused.

use al_builtins::native::Signal;
use al_testkit::{boolean, run_in};

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } }
    trigger OnModify() begin Name := 'changed'; end; }";

/// The reporter's helper (issue 6924, codeunit "AppPerformanceSignal"), with `Session.LogMessage` for the System
/// Application's `Telemetry.LogMessage`.
const SIGNAL: &str = "codeunit 50102 Signal
{
    SingleInstance = true;
    var
        ProcessIdentifier: List of [Guid];
        StartDateTime: Dictionary of [Guid, DateTime];
        StartNoOfSqlStatements: Dictionary of [Guid, Integer];
        StartRows: List of [BigInteger];
        Message: Dictionary of [Guid, Text];

    procedure StartMeasure(ProcessId: Guid; pMessage: Text)
    begin
        if ProcessIdentifier.Contains(ProcessId) then exit;
        ProcessIdentifier.Add(ProcessId);
        StartDateTime.Add(ProcessId, CurrentDateTime);
        StartNoOfSqlStatements.Add(ProcessId, SessionInformation.SqlStatementsExecuted);
        StartRows.Add(SessionInformation.SqlRowsRead);
        Message.Add(ProcessId, pMessage);
    end;

    procedure StopMeasure(ProcessId: Guid)
    begin
        if not ProcessIdentifier.Contains(ProcessId) then exit;
        EmitPerformanceSignal(ProcessId);
        Cleanup(ProcessId);
    end;

    procedure Spent(ProcessId: Guid): Integer
    begin
        exit(SessionInformation.SqlStatementsExecuted - StartNoOfSqlStatements.Get(ProcessId));
    end;

    local procedure EmitPerformanceSignal(ProcessId: Guid)
    var
        CustomDimensions: Dictionary of [Text, Text];
        Dur: Duration;
        TotalNoOfSqlStatements: Integer;
        Rows: BigInteger;
    begin
        Dur := CurrentDateTime - StartDateTime.Get(ProcessId);
        TotalNoOfSqlStatements := SessionInformation.SqlStatementsExecuted - StartNoOfSqlStatements.Get(ProcessId);
        Rows := SessionInformation.SqlRowsRead - StartRows.Get(1);
        CustomDimensions.Add('Duration', Format(Dur / 1, 0, 9));
        CustomDimensions.Add('NoSqlStatements', Format(TotalNoOfSqlStatements, 0, 9));
        CustomDimensions.Add('NoSqlRows', Format(Rows));
        Session.LogMessage('IFCTLM0002', Message.Get(ProcessId), Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::All, CustomDimensions);
    end;

    local procedure Cleanup(ProcessId: Guid)
    begin
        ProcessIdentifier.Remove(ProcessId);
        StartDateTime.Remove(ProcessId);
        StartNoOfSqlStatements.Remove(ProcessId);
        StartRows.RemoveAt(1);
        Message.Remove(ProcessId);
    end;
}";

fn src(body: &str) -> String {
    format!(
        "{TABLE}
{SIGNAL}
codeunit 50101 C {{
    local procedure Work()
    var Row: Record Row; I: Integer;
    begin
        // Work whose change of the counters no corpus test pins: a Commit, and a Modify that runs a trigger.
        for I := 1 to 3 do begin Row.Id := I; Row.Name := 'row'; if not Row.Insert() then; end;
        Commit();
        Row.Get(1);
        Row.Modify(true);
        Row.SetRange(Name, 'row');
        if Row.FindFirst() then;
    end;
    {body}
}}"
    )
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    refused_by(r, "")
}

fn refused_by(r: Result<al_builtins::value::Value, Signal>, what: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("SQL counter"), "{what}: {why}"),
        other => panic!("{what}: expected a SQL counter refusal, got {other:?}"),
    }
}

/// The reporter's three steps across unpinned work, the helper held in a SingleInstance codeunit's globals.
#[test]
fn the_telemetry_helper_runs_across_unpinned_work() {
    let s = src("procedure F(): Boolean var Signal: Codeunit Signal; Id: Guid;
        begin
            Id := CreateGuid();
            Signal.StartMeasure(Id, 'measure');
            Work();
            Signal.StopMeasure(Id);
            exit(true);
        end;");
    assert!(boolean(run_in(&s, "C", "F")));
}

/// The SingleInstance codeunit keeps its globals between two variables of it: Start through one, Stop through another.
#[test]
fn the_helper_is_one_instance() {
    let s = src(
        "procedure F(): Boolean var A: Codeunit Signal; B: Codeunit Signal; Id: Guid;
        begin
            Id := CreateGuid();
            A.StartMeasure(Id, 'measure');
            Work();
            B.StopMeasure(Id);
            exit(true);
        end;",
    );
    assert!(boolean(run_in(&s, "C", "F")));
}

/// A List value and a local Integer hold the first read the same way.
#[test]
fn a_list_and_an_integer_hold_the_first_read_across_unpinned_work() {
    let s = src("procedure F(): Boolean var L: List of [Integer]; First: Integer; Total: BigInteger; Dims: Dictionary of [Text, Text];
        begin
            L.Add(SessionInformation.SqlStatementsExecuted());
            L.Set(1, L.Get(1));
            First := SessionInformation.SqlRowsRead();
            Work();
            Total := SessionInformation.SqlStatementsExecuted() - L.Get(1);
            Dims.Add('Statements', Format(Total, 0, 9));
            Dims.Add('Rows', Format(SessionInformation.SqlRowsRead() - First));
            Session.LogMessage('0000ABC', 'done', Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::ExtensionPublisher, Dims);
            exit(true);
        end;");
    assert!(boolean(run_in(&s, "C", "F")));
}

/// Nothing pins the size or the sign of a change across unpinned work: every use that reads the difference is refused.
#[test]
fn reading_a_difference_across_unpinned_work_is_refused() {
    for read in [
        "exit(Format(Signal.Spent(Id) >= 0));",
        "exit(Format(Signal.Spent(Id) = 0));",
        "exit(Format(Signal.Spent(Id) + 1));",
        "exit(Format(Signal.Spent(Id) - Signal.Spent(Id)));",
        "exit(Format(Signal.Spent(Id) * 2));",
        "exit(Format(StrLen(Format(Signal.Spent(Id), 0, 9))));",
        "exit(Format(Signal.Spent(Id), 5));",
        "Message(Format(Signal.Spent(Id)));",
    ] {
        let s = src(&format!(
            "procedure F(): Text var Signal: Codeunit Signal; Id: Guid;
            begin
                Id := CreateGuid();
                Signal.StartMeasure(Id, 'measure');
                Work();
                {read}
            end;"
        ));
        refused_by(run_in(&s, "C", "F"), read);
    }
}

/// Two different counters do not subtract, across unpinned work or not.
#[test]
fn reads_of_two_counters_do_not_subtract() {
    let s = src("procedure F(): Text var First: Integer;
        begin
            First := SessionInformation.SqlRowsRead();
            Work();
            exit(Format(SessionInformation.SqlStatementsExecuted() - First));
        end;");
    refused(run_in(&s, "C", "F"));
}
