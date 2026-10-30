//! Host values end to end: the clock, the identity and the system field stamps (ADR-006, ADR-013).
//! Each test names the corpus test that pins the behaviour it asserts.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

const TABLE: &str = r#"
table 50900 "ALT T"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
table 50902 "ALT FF" { fields { field(1; K; Integer) { } field(2; Flt; Integer) { FieldClass = FlowFilter; } field(3; Txt; Text[10]) { } } keys { key(PK; K) { } } }
table 50903 "ALT FF2" { fields { field(1; K; Integer) { } field(2; FltA; Integer) { FieldClass = FlowFilter; } field(3; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("ALT T"); } field(4; FltB; Integer) { FieldClass = FlowFilter; } field(5; Txt; Text[10]) { } } keys { key(PK; K) { } } }
table 50904 "ALT Blb" { fields { field(1; K; Integer) { } field(2; Pic; Blob) { } } keys { key(PK; K) { } } }
table 50905 "ALT Dt" { fields { field(1; K; Integer) { } field(2; At; DateTime) { } } keys { key(PK; K) { } } }
table 50901 "ALT Temp" { TableType = Temporary; fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
"#;

fn compile(body: &str) -> (std::sync::Arc<Engine>, usize) {
    let src = format!("{TABLE}\ncodeunit 50100 C {{ {body} }}");
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    for (ri, r) in l.program.objects[oi].routines.iter().enumerate() {
        if let Some(Some(why)) = blocked.get(oi).and_then(|o| o.get(ri)) {
            panic!("{} is blocked: {why}", r.name);
        }
    }
    (Engine::new(l.program, blocked), oi)
}

fn call(s: &mut Session, oi: usize, routine: &str) -> Result<Value, Signal> {
    let ri = s.engine.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn run(body: &str) -> Result<Value, Signal> {
    let (engine, oi) = compile(body);
    call(&mut Session::new(engine), oi, "F")
}

/// `procedure F(): Text var <vars> begin <body> end;`
fn text(vars: &str, body: &str) -> String {
    match run(&format!("procedure F(): Text var {vars} begin {body} end;")) {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

fn truth(vars: &str, body: &str) -> bool {
    text(vars, &format!("exit(Format({body}));")) == "Yes"
}

// ---------------------------------------------------------------- the clock

#[test]
fn today_time_and_current_datetime_come_from_one_instant() {
    // session/TestSessionEvaluateRegionalPatterns.al:52-67, session/TestDateTimeContracts.al:19-21:
    // the three views of the clock agree.
    assert!(truth("D: DateTime;", "DT2Date(CurrentDateTime()) = Today()"));
    assert!(truth("", "CurrentDateTime() <> 0DT"));
    assert!(truth("", "Today() <> 0D"));
    assert!(truth("", "Time() <> 0T"));
}

#[test]
fn the_default_host_clock_is_frozen_at_a_known_instant() {
    // ADR-006: tests are deterministic by default. The instant is al-host's default.
    assert_eq!(text("", "exit(Format(Today(), 0, 9));"), "2024-01-15");
    assert_eq!(
        text("", "exit(Format(CurrentDateTime(), 0, 9));"),
        "2024-01-15T12:00:00Z"
    );
}

#[test]
fn sleep_moves_the_clock_by_the_time_waited() {
    // record/TestRecordSystemId.al:588 and others call Sleep(200) between two writes and expect
    // the second stamp to be later.
    assert_eq!(
        text(
            "A: DateTime; B: DateTime;",
            "A := CurrentDateTime(); Sleep(200); B := CurrentDateTime(); exit(Format(B - A = 201));"
        ),
        "Yes"
    );
}

#[test]
fn the_clock_never_reads_earlier_than_the_last_stamp() {
    // ADR-045 item 4 and its 2026-10-27 amendment: a stamp written between two reads of the clock lies between them. Stamps are strictly increasing
    // (record/TestRecordSystemId.al:598, TestXRecStampFields.al:179), so many writes between two reads move the stamps past
    // the clock; the clock then answers the last stamp. SystemCreatedAt is the current time
    // (record/TestSystemFieldStampsContracts.al:67, within 5 seconds of a read before the insert).
    assert_eq!(
        text(
            "R: Record \"ALT T\"; I: Integer; Last: DateTime;",
            "for I := 1 to 50 do begin R.Init(); R.\"No.\" := I; R.Insert(); end; Last := R.SystemCreatedAt; \
             exit(Format(CurrentDateTime() >= Last) + Format(CurrentDateTime() > Last));"
        ),
        "YesYes"
    );
}

#[test]
fn a_stamp_is_a_value_the_datetime_column_holds() {
    // A DateTime field keeps 1/300 s (errors/lt-ms-refusals-130/P130DateFilters.al:162-183). Whether a record's buffer holds
    // a stamp rounded is not pinned (sqltime.rs `store`), so a stamp is always a value the column holds as it is: copied into
    // a DateTime field and read back, it is still equal.
    assert_eq!(
        text(
            "R: Record \"ALT T\"; D: Record \"ALT Dt\"; I: Integer;",
            "for I := 1 to 20 do begin R.Init(); R.\"No.\" := I; R.Insert(); D.K := I; D.At := R.SystemCreatedAt; D.Insert(); \
             D.Get(I); if D.At <> R.SystemCreatedAt then exit('No ' + Format(I)); end; exit('Yes');"
        ),
        "Yes"
    );
}

#[test]
fn every_read_of_the_clock_is_one_millisecond_later_than_the_last() {
    // ADR-045: a read moves the clock on by one step. W5SessionFacts.al:37-40 and
    // TestProbeSessionMiscContracts.al:784-786 only rely on "later".
    assert_eq!(
        text(
            "A: DateTime; B: DateTime;",
            "A := CurrentDateTime(); B := CurrentDateTime(); exit(Format(B - A = 1));"
        ),
        "Yes"
    );
    // Time() and CurrentDateTime() read the same clock: the read in between counts.
    assert_eq!(
        text(
            "A: DateTime; T: Time; B: DateTime;",
            "A := CurrentDateTime(); T := Time(); B := CurrentDateTime(); exit(Format(B - A = 2));"
        ),
        "Yes"
    );
    // Sleep(0) moves nothing; the reads around it are one step apart.
    assert_eq!(
        text(
            "A: DateTime; B: DateTime;",
            "A := CurrentDateTime(); Sleep(0); B := CurrentDateTime(); exit(Format(B - A = 1));"
        ),
        "Yes"
    );
}

#[test]
fn work_date_defaults_to_today_and_can_be_set() {
    // session/TestSessionFunctions.al:53 (non-zero), :62-65 (set and read back),
    // handlers/TestPageTemporalSetValue_Tests.al:285 (WorkDate is the working date shorthand).
    assert!(truth("", "WorkDate() = Today()"));
    assert!(truth("", "WorkDate() <> 0D"));
    assert_eq!(
        text(
            "Old: Date;",
            "Old := WorkDate(); WorkDate(20240101D); exit(Format(WorkDate(), 0, 9));"
        ),
        "2024-01-01"
    );
}

#[test]
fn work_date_of_the_undefined_date_resets_to_today() {
    // session/TestProbeSessionMiscContracts.al:WorkDate_SetUndefined (`after_set_0D_minus_today` is 0).
    assert_eq!(text("", "WorkDate(0D); exit(Format(WorkDate() - Today()));"), "0");
}

// ---------------------------------------------------------------- the identity

#[test]
fn user_id_is_the_fixed_test_user() {
    // session/TestSessionFunctions.al:18,75-77: not empty and stable (ADR-013 decision 9).
    assert_eq!(text("", "exit(UserId());"), "TESTUSER");
}

#[test]
fn user_security_id_is_the_fixed_guid() {
    // session/TestUserSecurityId.al:34-58: not empty and stable (ADR-013 decision 9).
    assert_eq!(
        text("", "exit(Format(UserSecurityId(), 0, 9));"),
        "{C0A1BDFA-0000-0000-0000-545553545553}"
    );
    assert!(truth("", "not IsNullGuid(Database.UserSecurityId())"));
    assert!(truth("", "UserSecurityId() = Database.UserSecurityId()"));
}

#[test]
fn session_and_service_instance_ids_are_positive() {
    // session/TestSessionIdentityValues.al:24-59.
    assert!(truth("", "ServiceInstanceId() > 0"));
    assert!(truth("", "Database.ServiceInstanceId() = ServiceInstanceId()"));
    assert!(truth("", "SessionId() > 0"));
    assert!(truth("", "Database.SessionId() = SessionId()"));
}

#[test]
fn tenant_id_is_not_empty() {
    // session/TestDatabaseExtended.al:63-70.
    assert!(truth("", "Database.TenantId() <> ''"));
}

#[test]
fn serial_number_is_stable_not_blank_and_at_most_250_long() {
    // platform/lt-system-database-members/DbSerialNumber.al:33-41 (al-runner issue 6055).
    assert!(truth("S: Text;", "Database.SerialNumber() = Database.SerialNumber()"));
    assert!(truth("", "Database.SerialNumber() <> ''"));
    assert!(truth("", "StrLen(Database.SerialNumber()) <= 250"));
}

#[test]
fn platform_answers_of_a_test_session() {
    // session/TestBCPlatformContracts.al:27-42,205-211, session/TestFinalCoverage.al:62-73.
    assert!(truth("", "GuiAllowed()"));
    assert!(truth("", "IsServiceTier()"));
    assert!(truth("", "WindowsLanguage() > 0"));
    assert!(truth("", "GlobalLanguage() > 0"));
}

#[test]
fn global_language_can_be_set() {
    // session/TestSessionSettings.al:534 reads it; setting it is the other form of the builtin.
    assert_eq!(
        text("", "GlobalLanguage(1031); exit(Format(GlobalLanguage()));"),
        "1031"
    );
}

#[test]
fn global_language_can_be_assigned_like_a_property() {
    // `GlobalLanguage := Id` is the other spelling of `GlobalLanguage(Id)`; System Application's
    // "Reten. Pol. Telemetry Impl." sets and restores it that way. The setter itself is pinned by
    // session/TestSessionSettings.al:534.
    assert_eq!(
        text(
            "Saved: Integer;",
            "Saved := GlobalLanguage(); GlobalLanguage := 1031; exit(Format(Saved) + '>' + Format(GlobalLanguage));"
        ),
        "1033>1031"
    );
}

#[test]
fn client_types_are_callable() {
    // The test session is a web client session whose default client type is Windows
    // (session/W5SessionFacts.al:135-138; callable: session/TestSessionExtended.al:21,31).
    assert!(truth("C: ClientType;", "Session.CurrentClientType() = ClientType::Web"));
    assert!(truth(
        "C: ClientType;",
        "Session.DefaultClientType() = ClientType::Windows"
    ));
}

// ---------------------------------------------------------------- system field stamps

const STAMPS: &str = "
procedure Stamps(): Text
var R: Record \"ALT T\";
begin
    R.\"No.\" := 1;
    R.Insert();
    exit(Format(R.SystemCreatedAt, 0, 9) + '|' + Format(R.SystemModifiedAt, 0, 9) + '|' + Format(R.SystemCreatedBy, 0, 9) + '|' + Format(R.SystemModifiedBy, 0, 9));
end;
";

#[test]
fn insert_stamps_all_four_fields() {
    // session/TestTransactionContracts.al:131-170, record/TestBCSystemFieldContracts.al:78-102:
    // the stamps are the clock and UserSecurityId().
    let (engine, oi) = compile(STAMPS);
    let v = call(&mut Session::new(engine), oi, "Stamps").unwrap();
    let Value::Text(t) = v else { panic!() };
    assert_eq!(
        t,
        "2024-01-15T12:00:00Z|2024-01-15T12:00:00Z|{C0A1BDFA-0000-0000-0000-545553545553}|{C0A1BDFA-0000-0000-0000-545553545553}"
    );
}

#[test]
fn a_changing_modify_moves_modified_and_keeps_created() {
    // record/TestRecordSystemId.al:726-750, session/TestTransactionContracts.al:172-190.
    let t = text(
        "R: Record \"ALT T\"; C0: DateTime; M0: DateTime;",
        "R.\"No.\" := 1; R.Insert(); R.Get(1); C0 := R.SystemCreatedAt; M0 := R.SystemModifiedAt; \
         Sleep(200); R.Amount := 8; R.Modify(); R.Get(1); \
         exit(Format(R.SystemCreatedAt = C0) + Format(R.SystemModifiedAt > M0));",
    );
    assert_eq!(t, "YesYes");
}

#[test]
fn a_modify_that_changes_nothing_does_not_stamp() {
    // record/TestRecordSystemId.al:677-722.
    let t = text(
        "R: Record \"ALT T\"; M0: DateTime;",
        "R.\"No.\" := 1; R.Amount := 7; R.Insert(); R.Get(1); M0 := R.SystemModifiedAt; \
         Sleep(200); R.Modify(); R.Get(1); exit(Format(R.SystemModifiedAt = M0));",
    );
    assert_eq!(t, "Yes");
    let t = text(
        "R: Record \"ALT T\"; M0: DateTime;",
        "R.\"No.\" := 1; R.Amount := 7; R.Insert(); R.Get(1); M0 := R.SystemModifiedAt; \
         Sleep(200); R.Amount := 7; R.Modify(); R.Get(1); exit(Format(R.SystemModifiedAt = M0));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn rename_stamps_modified_even_to_the_same_key_and_keeps_created() {
    // record/TestRecordSystemId.al:580-648,752-775.
    let t = text(
        "R: Record \"ALT T\"; C0: DateTime; M0: DateTime; B0: Guid;",
        "R.\"No.\" := 504; R.Insert(); R.Get(504); C0 := R.SystemCreatedAt; M0 := R.SystemModifiedAt; B0 := R.SystemCreatedBy; \
         Sleep(200); R.Rename(504); Clear(R); R.Get(504); \
         exit(Format(R.SystemModifiedAt > M0) + Format(R.SystemCreatedAt = C0) + Format(R.SystemCreatedBy = B0) + Format(R.SystemModifiedBy = UserSecurityId()));",
    );
    assert_eq!(t, "YesYesYesYes");
}

#[test]
fn two_writes_in_one_millisecond_still_get_distinct_increasing_stamps() {
    // record/TestRecordSystemId.al:449 and TestXRecStampFields.al:179 need strictly later stamps.
    // With a frozen clock the second write is within the same instant.
    let t = text(
        "R: Record \"ALT T\"; A: DateTime; B: DateTime;",
        "R.\"No.\" := 1; R.Insert(); A := R.SystemModifiedAt; R.Amount := 1; R.Modify(); B := R.SystemModifiedAt; exit(Format(B > A));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn the_insert_trigger_and_before_event_see_default_stamps_the_buffer_has_them_after() {
    // record/TestXRecStampFields.al:75-99: Rec carries no stamp inside OnInsert; the stamps are
    // written below the trigger and the buffer has them when Insert returns.
    let t = text(
        "R: Record \"ALT T\";",
        "R.\"No.\" := 1; Clear(R.SystemCreatedAt); R.Insert(); exit(Format(R.SystemCreatedAt <> 0DT));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn temporary_records_are_not_stamped() {
    // Assumption (docs/probes-needed.md): the SQL layer writes the stamps, a temporary record
    // never reaches it. Source: AL Runner RowVersionPatches.cs (NCL decompilation).
    let t = text(
        "R: Record \"ALT Temp\" temporary;",
        "R.K := 1; R.Insert(); exit(Format(R.SystemCreatedAt = 0DT) + Format(IsNullGuid(R.SystemCreatedBy)));",
    );
    assert_eq!(t, "YesYes");
}

// ---------------------------------------------------------------- SessionSettings.Init

#[test]
fn session_settings_init_reads_language_locale_and_time_zone_from_the_host() {
    // session/TestSessionSettings.al:371-455: Init populates a positive language id and a
    // non-empty time zone, overwrites what was set before, and is repeatable. The values are the
    // host's (en-US, UTC), the zone the runtime models (docs/probes-needed.md "Dates").
    assert_eq!(
        text(
            "S: SessionSettings;",
            "S.TimeZone('Sentinel'); S.LanguageId(1030); S.Init(); \
             exit(Format(S.LanguageId()) + '|' + Format(S.LocaleId()) + '|' + S.TimeZone() + '|' + S.Company());"
        ),
        "1033|1033|UTC|"
    );
    assert_eq!(
        text(
            "A: SessionSettings; B: SessionSettings;",
            "A.Init(); B.TimeZone('x'); B.Init(); exit(Format(Format(A) = Format(B)));"
        ),
        "Yes"
    );
}

#[test]
fn company_name_is_the_hosts_company() {
    // session/TestSessionFunctions.al:25 (not empty), session/TestDatabase.al:74 (equals
    // CurrentCompany); the name is the oracle's (session/TestEnvironmentUserTenantContracts.al:238).
    assert_eq!(text("", "exit(CompanyName());"), "CRONUS International Ltd.");
    assert!(truth("R: Record \"ALT T\";", "R.CurrentCompany() = CompanyName()"));
}

// ---------------------------------------------------------------- mutation-testing additions

#[test]
fn time_is_the_time_of_day_of_the_frozen_clock() {
    // The default instant is 12:00:00.000 UTC; Time() is the offset into the day, one-based
    // because 0T is the undefined time (Value::Time(0)).
    assert!(truth("", "Time() = 120000T"));
    assert!(truth("", "Time() > 115959.999T"));
    assert!(truth("", "Time() < 120000.001T"));
}

#[test]
fn sleep_of_zero_is_allowed_and_a_negative_wait_is_refused() {
    // Only a non-negative wait is pinned (hostvalues.rs); zero changes nothing.
    assert_eq!(
        text(
            "A: DateTime;",
            "A := CurrentDateTime(); Sleep(0); exit(Format(CurrentDateTime() - A = 1));"
        ),
        "Yes"
    );
    match run("procedure F() begin Sleep(-1); end;") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("non-negative"), "{m}"),
        other => panic!("{other:?}"),
    }
}

// platform/lt-platform-members/PlTSession.al:245-266 (GlobalLanguage_InvalidIds): 0, -1 and 99999 raise NotSupportedLanguage and
// leave the language unchanged, and so do 1000, 4096, 8192, 9999, 32767 and 65535 (:326-360, GlobalLanguage_PositiveIds); other ids
// that are not positive are not pinned.
#[test]
fn global_language_raises_for_the_pinned_invalid_ids() {
    for bad in ["0", "-1", "99999", "1000", "4096", "8192", "9999", "32767", "65535"] {
        match run(&format!("procedure F() begin GlobalLanguage({bad}); end;")) {
            Err(Signal::Error(e)) => {
                assert_eq!(
                    e.message,
                    format!(
                        "'{bad}' is not a valid language code. You must specify a locale identifier (LCID) or code in the format of language-country/region, such as en-US, de-DE, or da-DK."
                    )
                );
                assert_eq!(e.code, "NotSupportedLanguage");
            }
            other => panic!("{bad}: {other:?}"),
        }
    }
    let r = run("[TryFunction] local procedure TrySet() begin GlobalLanguage(0); end; \
         procedure F(): Text var Old: Integer; begin Old := GlobalLanguage(); \
         exit(Format(TrySet()) + Format(GlobalLanguage() = Old)); end;");
    assert!(matches!(&r, Ok(Value::Text(t)) if t.as_str() == "NoYes"), "{r:?}");
    match run("procedure F() begin GlobalLanguage(-2); end;") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("are pinned"), "{m}"),
        other => panic!("{other:?}"),
    }
}

// session/TestSessionComplete.al:45-52: an id no session has is not active; platform/lt-platform-members/PlTSession.al:294
// (SessionId_Relations): the running session is.
#[test]
fn is_session_active_is_false_for_an_unknown_id_and_true_for_the_running_session() {
    let r = run("procedure F(): Boolean begin exit(Session.IsSessionActive(99999)); end;");
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
    let r = run("procedure F(): Boolean begin exit(Session.IsSessionActive(Database.SessionId())); end;");
    assert!(matches!(r, Ok(Value::Boolean(true))), "{r:?}");
}

// ------------------------------------------------------------------ the transaction type

// session/TestClearAllTransactionTypeContracts.al:224-238 and session/ProbeSystemEnumMembers.al:78-92 (BC 27.5): a
// test starts as UpdateNoLocks (not Update), and a set before any database call takes effect.
#[test]
fn the_transaction_type_starts_as_update_no_locks_and_a_set_before_any_call_takes_effect() {
    assert_eq!(
        text(
            "",
            "exit(Format(Database.CurrentTransactionType()) + Format(Database.CurrentTransactionType(), 0, 2));"
        ),
        "UpdateNoLocks0"
    );
    assert_eq!(
        text(
            "",
            "Database.CurrentTransactionType(TransactionType::Report); \
             exit(Format(Database.CurrentTransactionType()) + '|' + Format(Database.CurrentTransactionType(), 0, 2));"
        ),
        "Report|4"
    );
}

// The ordinals: UpdateNoLocks 0, Update 1, Snapshot 2, Browse 3, Report 4 (ProbeSystemEnumMembers.al:60-76).
#[test]
fn the_transaction_type_members_have_the_platforms_ordinals() {
    assert_eq!(
        text(
            "",
            "exit(Format(TransactionType::Update, 0, 2) + Format(TransactionType::Snapshot, 0, 2) + Format(TransactionType::Report, 0, 2) + Format(TransactionType::UpdateNoLocks, 0, 2) + Format(TransactionType::Browse, 0, 2));"
        ),
        "12403"
    );
}

// ProbeSystemEnumMembers.al:8-41: ClientType ordinals, with gaps.
#[test]
fn the_client_type_members_have_the_platforms_ordinals() {
    let body = [
        "Default",
        "Windows",
        "SOAP",
        "OData",
        "Api",
        "Background",
        "Management",
        "Web",
        "Tablet",
        "Phone",
        "Desktop",
        "ChildSession",
        "ODataV4",
    ]
    .iter()
    .map(|m| format!("Format(ClientType::{m}, 0, 2) + ','"))
    .collect::<Vec<_>>()
    .join(" + ");
    assert_eq!(text("", &format!("exit({body});")), "1,2,4,5,14,7,8,3,9,10,11,15,13,");
}

// ProbeSystemEnumMembers.al:94-112: a read starts the transaction; a set of Report after it reads as Update, and the
// Commit leaves Update as the type of the next transaction, which a set changes again.
#[test]
fn a_set_after_a_read_and_after_a_commit() {
    assert_eq!(
        text(
            "R: Record \"ALT T\";",
            "if R.FindFirst() then; Database.CurrentTransactionType(TransactionType::Report); \
             exit(Format(Database.CurrentTransactionType()));"
        ),
        "Update"
    );
    assert_eq!(
        text(
            "R: Record \"ALT T\";",
            "Database.CurrentTransactionType(TransactionType::Report); Commit(); \
             exit(Format(Database.CurrentTransactionType()));"
        ),
        "Update"
    );
    assert_eq!(
        text(
            "R: Record \"ALT T\";",
            "Commit(); Database.CurrentTransactionType(TransactionType::Browse); \
             exit(Format(Database.CurrentTransactionType()));"
        ),
        "Browse"
    );
}

// :114-141 and :143-161: after a write call (also a DeleteAll that finds no row) the type stays as it was, a set to
// Report or to the same type goes through without effect, and a set to Update raises
// DB:CannotChangeTransactionType.
#[test]
fn after_a_write_a_set_is_ignored_or_refused() {
    let out = text(
        "R: Record \"ALT T\"; S: Text;",
        "R.\"No.\" := 1; R.Insert(); S := Format(Database.CurrentTransactionType()); \
         Database.CurrentTransactionType(TransactionType::Report); S += '|' + Format(Database.CurrentTransactionType()); \
         Database.CurrentTransactionType(TransactionType::UpdateNoLocks); \
         asserterror Database.CurrentTransactionType(TransactionType::Update); \
         exit(S + '|' + GetLastErrorCode() + '|' + GetLastErrorText() + '|' + Format(Database.CurrentTransactionType()));",
    );
    assert_eq!(
        out,
        "UpdateNoLocks|UpdateNoLocks|DB:CannotChangeTransactionType|The transaction type cannot be changed after a transaction has started.\\\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\\\The error occurred because the transaction type was changed from UpdateNoLocks to Update.|UpdateNoLocks"
    );
    // A DeleteAll of no row starts it as well; after a Commit the type is Update and a set to Snapshot is ignored
    // (TestClearAllTransactionTypeContracts.al:245-253).
    let out = text(
        "R: Record \"ALT T\";",
        "R.SetRange(\"No.\", -77); R.DeleteAll(); Database.CurrentTransactionType(TransactionType::Report); \
         exit(Format(Database.CurrentTransactionType()));",
    );
    assert_eq!(out, "UpdateNoLocks");
    let out = text(
        "R: Record \"ALT T\";",
        "Commit(); R.DeleteAll(); Database.CurrentTransactionType(TransactionType::Snapshot); \
         exit(Format(Database.CurrentTransactionType()));",
    );
    assert_eq!(out, "Update");
}

#[test]
fn hyperlink_raises_unhandled_ui_when_it_runs() {
    // record/ProbeClearAllSetKeyHyperlink.al:322-338 (BC 27.5): there is no handler for a Hyperlink, so a
    // call raises `Unhandled UI: Hyperlink <url>` with the code MissingUIHandler; a routine that only
    // mentions it runs.
    assert_eq!(
        text("", "if false then Hyperlink('https://example.com'); exit('ran');"),
        "ran"
    );
    assert_eq!(
        text(
            "",
            "asserterror Hyperlink('https://example.com/probe'); exit(GetLastErrorText() + '|' + GetLastErrorCode());"
        ),
        "Unhandled UI: Hyperlink https://example.com/probe|MissingUIHandler"
    );
}

// ---------------------------------------------------------------- randomness
// No corpus test pins `Random` or `Randomize`; the range 1..MaxNumber is the documented one
// (corpus/scripts/al-surface.json, system-random-method) and the rest is recorded in
// docs/probes-needed.md ("Probe batch 4"). The reference sequences are .NET's `new Random(seed)`.

#[test]
fn random_stays_in_the_documented_range() {
    assert_eq!(
        text(
            "I: Integer; N: Integer; Bad: Integer;",
            "for I := 1 to 500 do begin N := Random(10); if (N < 1) or (N > 10) then Bad += 1; end; exit(Format(Bad));"
        ),
        "0"
    );
    assert_eq!(text("", "exit(Format(Random(1)));"), "1");
}

#[test]
fn a_seeded_sequence_repeats_and_follows_dotnets_random() {
    // `new Random(42)`: Next(100)+1, Next(1000)+1, Next(7)+1 = 67, 141, 1.
    assert_eq!(
        text(
            "",
            "Randomize(42); exit(Format(Random(100)) + ',' + Format(Random(1000)) + ',' + Format(Random(7)));"
        ),
        "67,141,1"
    );
    assert_eq!(
        text(
            "",
            "Randomize(1); exit(Format(Random(100)) + ',' + Format(Random(1000)) + ',' + Format(Random(7)));"
        ),
        "25,111,4"
    );
    assert_eq!(
        text(
            "A: Integer; B: Integer;",
            "Randomize(7); A := Random(1000000); Randomize(7); B := Random(1000000); exit(Format(A = B));"
        ),
        "Yes"
    );
}

#[test]
fn the_sequence_without_a_seed_is_the_same_in_every_session() {
    // ADR-009: determinism where BC's unseeded sequence is not observable.
    let first = text("", "exit(Format(Random(1000000)) + ',' + Format(Random(1000000)));");
    let second = text("", "exit(Format(Random(1000000)) + ',' + Format(Random(1000000)));");
    assert_eq!(first, second);
}

#[test]
fn randomize_without_a_seed_moves_to_a_new_sequence() {
    // BC draws a fresh seed from the clock, so two calls differ; here the seed is a counter.
    assert_eq!(
        text(
            "A: Integer; B: Integer;",
            "Randomize(); A := Random(1000000); Randomize(); B := Random(1000000); exit(Format(A <> B));"
        ),
        "Yes"
    );
}

#[test]
fn zero_and_negative_bounds_follow_the_service_tier() {
    // Not pinned by the corpus (docs/probes-needed.md): 0 acts as 1, a negative bound as its
    // absolute value, as in ALSystemNumeric.ALRandom.
    assert_eq!(text("", "exit(Format(Random(0)));"), "1");
    assert_eq!(
        text(
            "I: Integer; N: Integer; Bad: Integer;",
            "for I := 1 to 300 do begin N := Random(-5); if (N < 1) or (N > 5) then Bad += 1; end; exit(Format(Bad));"
        ),
        "0"
    );
}

// types/TestRandomSeededContracts.al:84-92 (BC 27.5): the bound -2147483648 has no absolute value.
#[test]
fn random_of_the_smallest_integer_overflows() {
    let r = run("procedure F() var N: Integer; begin N := Random(-2147483647 - 1); end;");
    let Err(Signal::Error(e)) = r else {
        panic!("expected an AL error, got {r:?}")
    };
    assert_eq!(e.message, "Arithmetic operation resulted in an overflow.");
    assert_eq!(e.code, "AL:Overflow");
}

// ---------------------------------------------------------------- Guids
// The corpus pins that CreateGuid is not null and differs on each call (types/TestGuid.al:12-45).
// Microsoft's Tests-ERM take the first 20 characters of `Format(CreateGuid())` as a key
// (Pos. Pay Test Unit, ERM Delete Documents), so the first bytes must differ too. The version and
// variant bits are an assumption (docs/probes-needed.md, "Probe batch 6"): BC's CreateGuid is
// the platform's random (version 4, RFC 4122) Guid.

const GUID_SCAN: &str = "I: Integer; G: Text; Seen: List of [Text]; Bad: Integer; Dup: Integer;";

/// 300 Guids: how many break the `{xxxxxxxx-xxxx-4xxx-[89AB]xxx-...}` form, how many repeat the
/// first 20 characters of an earlier one.
fn guid_scan(body_after: &str) -> String {
    text(
        GUID_SCAN,
        &format!(
            "for I := 1 to 300 do begin G := Format({body_after}); \
             if (StrLen(G) <> 38) or (CopyStr(G, 16, 1) <> '4') or (StrPos('89AB', CopyStr(G, 21, 1)) = 0) then Bad += 1; \
             if Seen.Contains(CopyStr(G, 1, 20)) then Dup += 1 else Seen.Add(CopyStr(G, 1, 20)); end; \
             exit(Format(Bad) + '/' + Format(Dup));"
        ),
    )
}

#[test]
fn create_guid_is_random_looking_in_every_byte_with_version_4_bits() {
    assert_eq!(guid_scan("CreateGuid()"), "0/0");
}

#[test]
fn create_sequential_guid_has_the_same_form() {
    assert_eq!(guid_scan("Guid.CreateSequentialGuid()"), "0/0");
}

#[test]
fn the_system_id_of_an_inserted_row_is_random_looking_but_not_version_4() {
    // types/TestGuidFormContracts.al:SystemId_OfNewRows_IsNotVersion4:
    // the version and variant positions vary, and the first 20 characters never repeat.
    let r = text(
        "I: Integer; T: Record \"ALT T\"; Seen: List of [Text]; Bad: Integer; Dup: Integer; G: Text;",
        "for I := 1 to 300 do begin T.Init(); T.\"No.\" := I; T.Insert(); G := Format(T.SystemId); \
         if (CopyStr(G, 16, 1) <> '4') or (StrPos('89AB', CopyStr(G, 21, 1)) = 0) then Bad += 1; \
         if Seen.Contains(CopyStr(G, 1, 20)) then Dup += 1 else Seen.Add(CopyStr(G, 1, 20)); end; \
         exit(Format(Bad) + '/' + Format(Dup));",
    );
    let (bad, dup) = r.split_once('/').expect("bad/dup");
    assert!(bad.parse::<u32>().unwrap() > 200, "{r}");
    assert_eq!(dup, "0", "{r}");
}

fn first_guids(seed: Option<i32>) -> String {
    let (engine, oi) = compile("procedure F(): Text begin exit(Format(CreateGuid()) + Format(CreateGuid())); end;");
    let mut host = al_host::Host::deterministic();
    if let Some(s) = seed {
        host.random_seed = s;
    }
    match call(&mut Session::with_host(engine, host), oi, "F") {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_guid_sequence_is_the_same_in_every_session_and_follows_the_host_seed() {
    // ADR-009: BC's Guids are random, which AL cannot tell from a seeded sequence.
    assert_eq!(first_guids(None), first_guids(None));
    assert_eq!(first_guids(Some(5)), first_guids(Some(5)));
    assert_ne!(first_guids(Some(5)), first_guids(Some(6)));
}

// ---------------------------------------------------------------- refusal texts

#[test]
fn format_of_a_value_without_a_text_form_names_its_type_not_its_contents() {
    // Tests-ERM refused 198 tests with a message that dumped the whole record (`Format of
    // Record(RefCell { value: RecordState { ... } })`). The reason is a type name: it groups, and
    // it fits on a line.
    for (vars, body, kind) in [
        ("L: List of [Integer];", "exit(Format(L));", "a List"),
        (
            "D: Dictionary of [Integer, Integer];",
            "exit(Format(D));",
            "a Dictionary",
        ),
    ] {
        let r = run(&format!("procedure F(): Text var {vars} begin {body} end;"));
        let Err(Signal::Unsupported(m)) = r else {
            panic!("{kind}: {r:?}");
        };
        assert!(
            m.contains(kind) && !m.contains("RefCell") && !m.contains('{'),
            "{kind}: {m}"
        );
    }
}

#[test]
fn format_of_a_record_joins_the_formats_of_its_fields_by_tab() {
    // record/TestFormatRecordContracts.al:111-145.
    let t = text(
        "R: Record \"ALT T\"; Ref: RecordRef;",
        "R.\"No.\" := 5; R.Amount := 2; R.Note := 'x'; Ref.GetTable(R); \
         if Format(R) <> Format(Ref) then exit('ref differs'); exit(Format(R));",
    );
    assert_eq!(t, "5\t2\tx");
}

// types/ProbeTypeFormsAndFilters.al:FormatRecord_WithFlowFilter_BeforeAndAfterSetRange (BC 27.5): a FlowFilter field is
// the last of the text, with the value it holds (0), whatever filter was set on it.
#[test]
fn format_of_a_record_ends_with_its_flow_filter_fields_at_their_value() {
    assert_eq!(
        text(
            "R: Record \"ALT FF\"; S: Text;",
            "R.K := 1; R.Txt := 'x'; R.Insert(); R.Get(1); S := Format(R); R.SetRange(Flt, 3); S += '|' + Format(R); \
             R.SetRange(Flt, 3, 8); S += '|' + Format(R); exit(S);"
        ),
        "1\tx\t0|1\tx\t0|1\tx\t0"
    );
}

// misc/lt-probes-needed-5/PnE08RecordMisc.al:FormatRecord_FlowFilterPlacement `afterCalc`: after the stored fields come all
// FlowFilter fields, then the FlowFields, whatever their field numbers.
#[test]
fn format_of_a_record_puts_flow_filter_fields_before_flow_fields() {
    assert_eq!(
        text(
            "R: Record \"ALT FF2\"; T: Record \"ALT T\";",
            "T.\"No.\" := 1; T.Insert(); R.K := 1; R.Txt := 'n'; R.Insert(); R.Get(1); R.CalcFields(Cnt); exit(Format(R));"
        ),
        "1\tn\t0\t0\t1"
    );
}

// platform/lt-xmlport-session-navapp-cookie/P94Session.al: StopSession of an id no session has answers false; the running
// session's own id is refused.
#[test]
fn stop_session_is_false_for_an_unknown_id_and_refused_for_the_running_session() {
    let r = run("procedure F(): Boolean begin exit(Session.StopSession(99999)); end;");
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
    let r = run("procedure F(): Boolean begin exit(Session.StopSession(0, 'why')); end;");
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
    let r = run("procedure F(): Boolean begin exit(Session.StopSession(Database.SessionId())); end;");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// ADR-042 (assumption geturl-web-base-address): GetUrl(ClientType::Web) is the host's server base address as set; the forms
// that add a company, an object or another client type are not pinned and are refused.
fn get_url(body: &str, url: Option<&str>) -> Result<Value, Signal> {
    let (engine, oi) = compile(&format!("procedure F(): Text begin {body} end;"));
    let mut host = al_host::Host::deterministic();
    if let Some(u) = url {
        host = host.with_server_url(u);
    }
    al_builtins::assumptions::clear_hits();
    call(&mut Session::with_host(engine, host), oi, "F")
}

#[test]
fn geturl_web_is_the_host_server_address_and_marks_its_assumption() {
    let r = get_url("exit(GetUrl(ClientType::Web));", None);
    assert!(
        matches!(&r, Ok(Value::Text(t)) if &**t == "http://localhost:8080/BC"),
        "{r:?}"
    );
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::GetUrlWebBaseAddress]
    );
    let r = get_url("exit(GetUrl(ClientType::Web));", Some("https://bc.example:7048/BC1/"));
    assert!(
        matches!(&r, Ok(Value::Text(t)) if &**t == "https://bc.example:7048/BC1/"),
        "{r:?}"
    );
}

// platform/lt-system-database-members/GetUrlCompany.al:67-77, session/lt-geturl-shape/GetUrlShape.al:121-130: the query after
// the host's address, with the company encoded.
#[test]
fn geturl_with_a_company_or_an_object_builds_the_pinned_query() {
    for (body, want) in [
        (
            "exit(GetUrl(ClientType::Web, 'My Company'));",
            "http://localhost:8080/BC/?company=My%20Company",
        ),
        (
            "exit(GetUrl(ClientType::Web, 'A&B'));",
            "http://localhost:8080/BC/?company=A%26B",
        ),
        ("exit(GetUrl(ClientType::Web, ''));", "http://localhost:8080/BC"),
        (
            "exit(GetUrl(ClientType::Web, 'C', ObjectType::Page, 21));",
            "http://localhost:8080/BC/?company=C&page=21",
        ),
        (
            "exit(GetUrl(ClientType::Web, '', ObjectType::Report, 6));",
            "http://localhost:8080/BC/?report=6",
        ),
        ("exit(GetUrl(ClientType::Web, 'C', ObjectType::Codeunit, 80));", ""),
        ("exit(GetUrl(ClientType::Web, 'C', ObjectType::Page, 0));", ""),
    ] {
        let r = get_url(body, None);
        assert!(matches!(&r, Ok(Value::Text(t)) if &**t == want), "{body}: {r:?}");
    }
}

// session/lt-geturl-shape/GetUrlShape.al:270 (QA_GetUrl_Web_NoArgument_Whole `default alone`): GetUrl(ClientType::Default)
// alone is the web address with the scheme dynamicsnav and the path followed by `//`. Only an http address and only the
// form without arguments are pinned; the rest is refused.
#[test]
fn geturl_default_alone_is_the_dynamicsnav_form_of_the_address() {
    let r = get_url("exit(GetUrl(ClientType::Default));", None);
    assert!(
        matches!(&r, Ok(Value::Text(t)) if &**t == "dynamicsnav://localhost:8080/BC//"),
        "{r:?}"
    );
    assert_eq!(
        al_builtins::assumptions::take_hits(),
        vec![al_builtins::assumptions::Assumption::GetUrlWebBaseAddress]
    );
    for (body, url) in [
        (
            "exit(GetUrl(ClientType::Default));",
            Some("https://bc.example:7048/BC1"),
        ),
        ("exit(GetUrl(ClientType::Default, 'A&B'));", None),
        ("exit(GetUrl(ClientType::Default, 'C', ObjectType::Table, 18));", None),
        ("exit(GetUrl(ClientType::Default, 'C', ObjectType::Page, 0));", None),
        (
            "exit(GetUrl(ClientType::Default, 'C', ObjectType::Codeunit, 80));",
            None,
        ),
    ] {
        let r = get_url(body, url);
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("GetUrl")),
            "{body}: {r:?}"
        );
    }
}

// platform/lt-t-geturl/TGetUrl.al:203-210 (GetUrl_ClientDefault): the Default forms with a company and a page or report are
// `dynamicsnav://<address>/<company>/` followed by `runpage?page=N` or `runreport?report=N`; the company is percent-encoded
// (only the space is pinned).
#[test]
fn geturl_default_with_a_company_or_an_object_builds_the_pinned_path() {
    for (body, want) in [
        (
            "exit(GetUrl(ClientType::Default, 'My Company'));",
            "dynamicsnav://localhost:8080/BC/My%20Company/",
        ),
        (
            "exit(GetUrl(ClientType::Default, ''));",
            "dynamicsnav://localhost:8080/BC//",
        ),
        (
            "exit(GetUrl(ClientType::Default, '', ObjectType::Page, 21));",
            "dynamicsnav://localhost:8080/BC//runpage?page=21",
        ),
        (
            "exit(GetUrl(ClientType::Default, 'My Company', ObjectType::Report, 69367));",
            "dynamicsnav://localhost:8080/BC/My%20Company/runreport?report=69367",
        ),
    ] {
        let r = get_url(body, None);
        assert!(matches!(&r, Ok(Value::Text(t)) if &**t == want), "{body}: {r:?}");
    }
}

// The host part of the other client types, and a character the corpus does not pin in a company, are refused.
#[test]
fn geturl_of_another_client_type_or_an_unpinned_character_is_refused() {
    for body in [
        "exit(GetUrl(ClientType::Phone, 'C', ObjectType::Table, 18));",
        "exit(GetUrl(ClientType::Tablet, 'C', ObjectType::Page, 0));",
        "exit(GetUrl(ClientType::Desktop, 'A&B'));",
        "exit(GetUrl(ClientType::Background, 'C', ObjectType::Page, 21));",
        "exit(GetUrl(ClientType::SOAP, 'C', ObjectType::Table, 18));",
        "exit(GetUrl(ClientType::Web, 'A`B'));",
    ] {
        let r = get_url(body, None);
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("GetUrl")),
            "{body}: {r:?}"
        );
    }
}

// text/lt-foreach-text/ForeachText.al:Foreach_Text_Char_Order: a Text visits its characters in order, a Code in upper case.
#[test]
fn foreach_over_a_text_visits_each_character() {
    let r = run(
        "procedure F(): Text var T: Text; Ch: Char; V: Text; begin T := 'a+b'; foreach Ch in T do V += Format(Ch) + ','; exit(V); end;",
    );
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "a,+,b,"), "{r:?}");
}

// types/lt-evaluate-wide-decimal/EvaluateWideDecimal.al:Evaluate_Decimal_IntegerDigits: 15 digits before the point read, 16 do not.
#[test]
fn evaluate_decimal_takes_fifteen_integer_digits_and_not_sixteen() {
    let r = run(
        "procedure F(): Text var D: Decimal; A: Boolean; B: Boolean; begin A := Evaluate(D, '999999999999999'); B := Evaluate(D, '1000000000000000'); exit(Format(A) + Format(B)); end;",
    );
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "YesNo"), "{r:?}");
}

// types/lt-page-style/PageStyle.al: the members have the ordinals 1 to 11 and a fresh variable is 0.
#[test]
fn page_style_members_and_default() {
    let r = run(
        "procedure F(): Text var S: PageStyle; begin exit(Format(PageStyle::Subordinate, 0, 2) + Format(PageStyle::None) + Format(S)); end;",
    );
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "11None0"), "{r:?}");
}

// types/lt-secret-text-from-text/SecretTextFromText.al:SecretStrSubstNo_TextArgument `text_var_in_template`.
#[test]
fn secret_str_subst_no_takes_a_text_variable() {
    let r = run(
        "procedure F(): Boolean var P: Text; S: SecretText; begin P := 'v'; S := SecretStrSubstNo('x=%1', P); exit(S.IsEmpty()); end;",
    );
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
}

// platform/lt-t-geturl/TGetUrl.al:383-388 GetUrl_Web_ReportQueryCodeunitWithRecord: a report, a query or a codeunit
// with a record and UseFilters false has no address, the empty text. A query with UseFilters true is not pinned.
#[test]
fn geturl_of_report_query_or_codeunit_with_a_record_is_empty_and_a_query_with_filters_is_refused() {
    for obj in ["Report, 6", "Query, 9150", "Codeunit, 80"] {
        let body = format!(
            "var R: Record \"ALT T\"; begin exit(GetUrl(ClientType::Web, 'C', ObjectType::{obj}, R, false)); end;"
        );
        let (engine, oi) = compile(&format!("procedure F(): Text {body}"));
        let r = call(&mut Session::with_host(engine, al_host::Host::deterministic()), oi, "F");
        assert!(matches!(&r, Ok(Value::Text(t)) if t.is_empty()), "{obj}: {r:?}");
    }
    let body =
        "var R: Record \"ALT T\"; begin exit(GetUrl(ClientType::Web, 'C', ObjectType::Query, 9150, R, true)); end;";
    let (engine, oi) = compile(&format!("procedure F(): Text {body}"));
    let r = call(&mut Session::with_host(engine, al_host::Host::deterministic()), oi, "F");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("no query part")),
        "{r:?}"
    );
}

// misc/lt-probes-needed-5/PnE08RecordMisc.al:TestField_Blob `af219`: an error inside ASSERTERROR rolls the Insert back, and a
// later CalcFields of the Blob finds no row, so the content a stream wrote is gone.
#[test]
fn calcfields_of_a_blob_of_a_rolled_back_row_empties_the_blob() {
    assert_eq!(
        text(
            "R: Record \"ALT Blb\"; Out: OutStream; S: Text;",
            "R.K := 1; R.Insert(); asserterror R.TestField(Pic); R.Pic.CreateOutStream(Out); Out.WriteText('x'); \
             R.CalcFields(Pic); if R.Pic.HasValue() then S := 'kept' else S := 'empty'; exit(S);"
        ),
        "empty"
    );
}

#[test]
fn random_of_a_decimal() {
    // types/lt-aj-probes/AjScalars.al:375-433: a whole Decimal draws as that Integer, a fraction is the conversion error.
    assert_eq!(text("D: Decimal;", "D := 1.0; exit(Format(Random(D)));"), "1");
    match run("procedure F(): Text var D: Decimal; I: Integer; begin D := 2.5; I := Random(D); exit(''); end;") {
        Err(Signal::Error(e)) => {
            assert_eq!(e.code, "ConversionCast");
            assert_eq!(
                e.message,
                "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 2.5 to System.Int32."
            );
        }
        other => panic!("{other:?}"),
    }
}

// platform/lt-t-geturl/TGetUrl.al:248-285 (GetUrl_ClientPhone, Tablet, Desktop): `ms-dynamicsnav://` and the address, then the
// web query without a `/`; TGetUrl.al:233-240 (GetUrl_ClientWindows): the Default text; GetUrlCtBackground.al,
// GetUrlCtManagement.al, GetUrlCtNAS.al:69-73: the empty text.
#[test]
fn geturl_of_the_other_client_types_builds_the_pinned_text() {
    for (body, want) in [
        ("exit(GetUrl(ClientType::Phone));", "ms-dynamicsnav://localhost:8080/BC"),
        (
            "exit(GetUrl(ClientType::Tablet, 'My Company'));",
            "ms-dynamicsnav://localhost:8080/BC?company=My%20Company",
        ),
        (
            "exit(GetUrl(ClientType::Desktop, '', ObjectType::Page, 21));",
            "ms-dynamicsnav://localhost:8080/BC?page=21",
        ),
        (
            "exit(GetUrl(ClientType::Phone, 'My Company', ObjectType::Report, 69367));",
            "ms-dynamicsnav://localhost:8080/BC?company=My%20Company&report=69367",
        ),
        (
            "exit(GetUrl(ClientType::Windows, 'My Company', ObjectType::Page, 21));",
            "dynamicsnav://localhost:8080/BC/My%20Company/runpage?page=21",
        ),
        ("exit(GetUrl(ClientType::Background));", ""),
        ("exit(GetUrl(ClientType::Management, 'My Company'));", ""),
        ("exit(GetUrl(ClientType::NAS, ''));", ""),
    ] {
        let r = get_url(body, None);
        assert!(matches!(&r, Ok(Value::Text(t)) if &**t == want), "{body}: {r:?}");
    }
}

// platform/lt-t-geturl/TGetUrl.al:310-329 (GetUrl_Web_CompanyCharacters): only letters, digits and - . _ ~ stay unencoded.
#[test]
fn geturl_web_encodes_every_other_printable_character() {
    for (company, want) in [
        ("CRONUS USA, Inc.", "CRONUS%20USA%2C%20Inc."),
        ("A_B", "A_B"),
        ("A-B.C~D", "A-B.C~D"),
        ("A*B$C", "A%2AB%24C"),
        ("A[B]{C}", "A%5BB%5D%7BC%7D"),
        ("A|B^C", "A%7CB%5EC"),
        ("A\"B", "A%22B"),
    ] {
        let body = format!("exit(GetUrl(ClientType::Web, '{company}'));");
        let want = format!("http://localhost:8080/BC/?company={want}");
        let r = get_url(&body, None);
        assert!(matches!(&r, Ok(Value::Text(t)) if **t == *want), "{body}: {r:?}");
    }
}

// platform/lt-t-geturl/TGetUrl.al:481-559 on corpus master (TV_GetUrl_ClientSOAP, _ClientOData, _ClientODataV4, _ClientApi):
// the service client types answer the host's address (host and instance masked there) with their path, a company in it, and
// the empty text for an object. Api with a company (an id the pins mask) and the forms no pin shows are refused.
// The service address is not the web address: the pins mask the web address as <base> first and the service texts show
// http://<host>/<instance>/..., so their host part differs from the web one (TGetUrl.al:95-102, 504, 521, 487). The ports
// 7047 (SOAP) and 7048 (OData, ODataV4, Api) are BC's defaults and part of assumption `geturl-web-base-address`.
#[test]
fn geturl_of_the_service_client_types_builds_the_pinned_addresses() {
    for (body, want) in [
        (
            "exit(GetUrl(ClientType::SOAP));",
            "http://localhost:7047/BC/WS/Services",
        ),
        (
            "exit(GetUrl(ClientType::SOAP, 'My Company'));",
            "http://localhost:7047/BC/WS/My%20Company/Services",
        ),
        (
            "exit(GetUrl(ClientType::SOAP, ''));",
            "http://localhost:7047/BC/WS/Services",
        ),
        ("exit(GetUrl(ClientType::OData));", "http://localhost:7048/BC/ODataV4"),
        (
            "exit(GetUrl(ClientType::ODataV4, 'My Company'));",
            "http://localhost:7048/BC/ODataV4/Company('My%20Company')",
        ),
        (
            "exit(GetUrl(ClientType::ODataV4, ''));",
            "http://localhost:7048/BC/ODataV4",
        ),
        ("exit(GetUrl(ClientType::Api));", "http://localhost:7048/BC/api/"),
        ("exit(GetUrl(ClientType::Api, ''));", "http://localhost:7048/BC/api/"),
        ("exit(GetUrl(ClientType::SOAP, '', ObjectType::Page, 21));", ""),
        (
            "exit(GetUrl(ClientType::OData, 'My Company', ObjectType::Report, 69367));",
            "",
        ),
        (
            "exit(GetUrl(ClientType::SOAP, 'My Company', ObjectType::Codeunit, 69366));",
            "",
        ),
        (
            "exit(GetUrl(ClientType::ODataV4, 'My Company', ObjectType::Query, 69376));",
            "",
        ),
        (
            "exit(GetUrl(ClientType::ODataV4, 'CRONUS International Ltd.', ObjectType::Page, 21));",
            "",
        ),
    ] {
        let r = get_url(body, None);
        assert!(matches!(&r, Ok(Value::Text(t)) if &**t == want), "{body}: {r:?}");
    }
    for body in [
        "exit(GetUrl(ClientType::Api, 'My Company'));",
        "exit(GetUrl(ClientType::SOAP, 'A&B'));",
        "exit(GetUrl(ClientType::SOAP, 'CRONUS International Ltd.'));",
        "exit(GetUrl(ClientType::OData, 'My Company', ObjectType::Codeunit, 69366));",
        "exit(GetUrl(ClientType::Api, '', ObjectType::Query, 69376));",
    ] {
        let r = get_url(body, None);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{body}: {r:?}");
    }
}

/// al-runner issue 6301: `Sleep(D)` with a Duration waits D's milliseconds, the Integer the Duration converts to
/// (types/lt-filter-enum-relation-duration-guid/D_Duration.al:DateTimeMinusDateTime_To_Integer).
#[test]
fn sleep_takes_a_duration_as_its_milliseconds() {
    assert_eq!(
        text(
            "A: DateTime; B: DateTime; D: Duration;",
            "D := 200; A := CurrentDateTime(); Sleep(D); B := CurrentDateTime(); exit(Format(B - A = 201));"
        ),
        "Yes"
    );
    assert_eq!(
        text(
            "A: DateTime; D: Duration;",
            "A := CurrentDateTime(); Sleep(D); exit(Format(CurrentDateTime() - A = 1));"
        ),
        "Yes"
    );
}
