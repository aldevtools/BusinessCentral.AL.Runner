//! Record store values and record state (lane T11 of the bump-29 triage): ReadIsolation after record operations, the
//! LockTable Wait argument, Get with an Integer on a Code key, TransferFields and the row version, SQL datetime precision.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69980 "SV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { } }
}
table 69990 "SV Other Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { } }
}
table 69982 "SV Code Key"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69981 C
{
    // record/lt-at-partner-gaps/AtGetIntegerOnCodeKey.al:27-58.
    procedure GetIntegerOnCodeKey(): Text
    var Row: Record "SV Code Key"; N: Integer; s: Text;
    begin
        Row."No." := 'A1'; Row.Name := 'other'; Row.Insert();
        Row."No." := '123'; Row.Name := 'hit'; Row.Insert();
        Clear(Row);
        s := Format(Row.Get(123)) + Row."No." + Row.Name;
        Clear(Row);
        N := 123;
        s += Format(Row.Get(N)) + Row."No." + Row.Name;
        Clear(Row);
        s += Format(Row.Get(124)) + Row."No." + '|' + Row.Name;
        exit(s);
    end;

    procedure GetIntegerOnCodeKeyBare()
    var Row: Record "SV Code Key";
    begin
        Row.Get(123);
    end;

    procedure GetNegativeIntegerOnCodeKey()
    var Row: Record "SV Code Key";
    begin
        if Row.Get(-5) then;
    end;

    // record/lt-cn-transferfields-modify/CnTransferFieldsModify.al:133-173.
    procedure TransferFromNewer(WithKey: Boolean): Text
    var R1: Record "SV Row"; R2: Record "SV Row";
    begin
        R1.Id := 1; R1.Amount := 1; R1.Insert();
        Commit();
        R2.Get(1); R2.Amount := 2; R2.Modify();
        Commit();
        R1.TransferFields(R2, WithKey);
        R1.Amount += 7;
        R1.Modify();
        R2.Get(1);
        exit(Format(R2.Amount));
    end;

    // Not pinned: whether TransferFields from another table takes that row's version. A Modify whose outcome depends on it is
    // refused (CnTransferFieldsModify.al:194-214 pins only a buffer that is current either way).
    procedure TransferFromOtherTableStale()
    var R1: Record "SV Row"; R2: Record "SV Row"; O: Record "SV Other Row";
    begin
        R1.Id := 1; R1.Amount := 1; R1.Insert();
        Commit();
        R2.Get(1); R2.Amount := 2; R2.Modify();
        O.Id := 1; O.Amount := 3; O.Insert();
        Commit();
        O.Get(1);
        R1.TransferFields(O);
        R1.Modify();
    end;

    procedure TransferFromOtherTableCurrent(): Text
    var R1: Record "SV Row"; O: Record "SV Other Row";
    begin
        R1.Id := 1; R1.Amount := 1; R1.Insert();
        O.Id := 1; O.Amount := 3; O.Insert();
        Commit();
        O.Get(1);
        R1.TransferFields(O);
        R1.Modify();
        R1.Get(1);
        exit(Format(R1.Amount));
    end;

    // misc/lt-cv-odd-leads/CvReadIsolationState.al:36-66.
    procedure IsolationAfterOps(): Text
    var Rec: Record "SV Row"; Other: Record "SV Row"; s: Text;
    begin
        Rec.ReadIsolation := IsolationLevel::ReadUncommitted;
        Rec.Reset();
        s := Format(Rec.ReadIsolation);
        Rec.ReadIsolation := IsolationLevel::ReadUncommitted;
        Rec.Init();
        s += ',' + Format(Rec.ReadIsolation);
        Rec.LockTable();
        s += ',' + Format(Rec.ReadIsolation);
        Rec.ReadIsolation := IsolationLevel::ReadCommitted;
        Other.Copy(Rec);
        s += ',' + Format(Other.ReadIsolation);
        Clear(Other);
        Rec.ReadIsolation := IsolationLevel::RepeatableRead;
        Other := Rec;
        s += ',' + Format(Other.ReadIsolation);
        Clear(Other);
        Other.TransferFields(Rec);
        s += ',' + Format(Other.ReadIsolation);
        Clear(Rec);
        s += ',' + Format(Rec.ReadIsolation);
        exit(s);
    end;

    local procedure LockOutcome(Arm: Integer): Text
    var RecRef: RecordRef; Rec: Record "SV Row";
    begin
        RecRef.Open(69980);
        ClearLastError();
        asserterror begin
            case Arm of
                1: RecRef.LockTable(true);
                2: RecRef.LockTable(true, true);
                3: RecRef.LockTable(true, false);
                4: RecRef.LockTable(false);
                5: RecRef.LockTable(false, true);
                6: RecRef.LockTable(false, false);
                7: Rec.LockTable(false);
                8: Rec.LockTable(false, false);
            end;
            Error('no error');
        end;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure LockOutcomes(): Text
    var i: Integer; s: Text;
    begin
        for i := 1 to 8 do
            s += LockOutcome(i) + ';';
        exit(s);
    end;

    // record/lt-cn-transferfields-modify/CnTransferFieldsModify.al:543 QH_TF_Other_SourceInMemory_AuditFields.
    procedure TransferAuditFromMemory(): Text
    var R1: Record "SV Row"; R4: Record "SV Row"; O: Record "SV Other Row"; s: Text;
    begin
        R1.Id := 1; R1.Amount := 1; R1.Insert();
        R1.Get(1);
        O.Init(); O.Id := 1; O.Amount := 4;
        R1.TransferFields(O);
        s := Format(R1.SystemCreatedAt = 0DT) + Format(R1.SystemModifiedAt = 0DT) + Format(IsNullGuid(R1.SystemCreatedBy)) + Format(IsNullGuid(R1.SystemModifiedBy));
        R1.Modify();
        s += '|' + Format(R1.SystemCreatedAt = 0DT) + Format(R1.SystemModifiedAt = 0DT) + Format(IsNullGuid(R1.SystemCreatedBy)) + Format(R1.SystemModifiedBy = UserSecurityId());
        R4.Get(1);
        s += '|' + Format(R4.SystemCreatedAt = 0DT) + Format(R4.SystemCreatedBy = UserSecurityId()) + Format(R4.SystemModifiedBy = UserSecurityId());
        exit(s);
    end;

    procedure RecordLockTrueTrue()
    var Rec: Record "SV Row";
    begin
        Rec.LockTable(true, true);
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

// CvReadIsolationState.al:58-64: Reset, LockTable and Clear set it back to Default; Init keeps it; Copy passes it;
// assignment and TransferFields do not.
#[test]
fn read_isolation_after_record_operations() {
    assert_eq!(
        text(run("IsolationAfterOps")),
        "Default,ReadUncommitted,Default,ReadCommitted,Default,Default,Default"
    );
}

// recordref/lt-refs/RfTViews.al:316-322 and :332-360 (RecordRef: every Wait = false form and (true, true));
// record/lt-record-write/RcBMeta.al:415-419 (Record.LockTable(false)); session/TestBCLockingContracts.al:124 (false, false).
#[test]
fn locktable_with_wait_false_fails() {
    let e = "The request to lock the SV Row table with a Wait parameter of FALSE is not supported.|AL:LockTableBadParm";
    let ok = "no error|Dialog";
    let want = [ok, e, ok, e, e, e, e, e].map(|s| format!("{s};")).concat();
    assert_eq!(text(run("LockOutcomes")), want);
}

// Not pinned: Record.LockTable(true, true) (only the RecordRef form is, RfTViews.al:314-315).
#[test]
fn record_locktable_true_true_is_refused() {
    assert!(
        matches!(run("RecordLockTrueTrue"), Err(Signal::Unsupported(_))),
        "{:?}",
        run("RecordLockTrueTrue")
    );
}

// AtGetIntegerOnCodeKey.al:86-140: an Integer (literal or variable) on a Code key reads as its digits; found, not found, and
// the bare Get's error.
#[test]
fn get_with_an_integer_on_a_code_key() {
    assert_eq!(text(run("GetIntegerOnCodeKey")), "Yes123hitYes123hitNo|");
    let e = al_testkit::error_message(run("GetIntegerOnCodeKeyBare"));
    assert_eq!(
        e,
        "The SV Code Key does not exist. Identification fields and values: No.='123'"
    );
}

// Not pinned: a negative Integer on a Code key.
#[test]
fn get_with_a_negative_integer_on_a_code_key_is_refused() {
    assert!(matches!(
        run("GetNegativeIntegerOnCodeKey"),
        Err(Signal::Unsupported(_))
    ));
}

// CnTransferFieldsModify.al:133-150: TransferFields with the primary key takes the source's row version, so the Modify of a
// stale buffer succeeds; :153-173: without the key it does not, and the Modify fails with DB:RecordChanged.
#[test]
fn transferfields_with_the_key_takes_the_row_version() {
    let with = al_testkit::run_with(
        SRC,
        "C",
        "TransferFromNewer",
        vec![al_builtins::value::Value::Boolean(true)],
    );
    assert_eq!(text(with.map(|r| r.0)), "9");
    let without = al_testkit::run_with(
        SRC,
        "C",
        "TransferFromNewer",
        vec![al_builtins::value::Value::Boolean(false)],
    );
    match without {
        Err(Signal::Error(e)) => assert_eq!(e.code.as_str(), "DB:RecordChanged"),
        other => panic!("{other:?}"),
    }
}

const MEDIA: &str = r#"
table 2000000184 "Tenant Media"
{
    fields
    {
        field(1; ID; Guid) { } field(2; Description; Text[250]) { } field(3; Content; Blob) { }
        field(4; "Mime Type"; Text[100]) { } field(5; Height; Integer) { } field(6; Width; Integer) { }
    }
    keys { key(PK; ID) { } }
}
table 69983 "SV Media"
{
    fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Src; Blob) { } }
    keys { key(PK; Id) { } }
}
table 69984 "SV Media Other"
{
    fields { field(1; Id; Integer) { } field(2; Pic; Media) { } }
    keys { key(PK; Id) { } }
}
table 69989 "SV Media Set"
{
    fields { field(1; Id; Integer) { } field(2; Pics; MediaSet) { } field(3; Src; Blob) { } }
    keys { key(PK; Id) { } }
}
codeunit 69985 M
{
    local procedure Import(var Row: Record "SV Media"; Content: Text)
    var ins: InStream; outs: OutStream;
    begin
        Row.Src.CreateOutStream(outs); outs.WriteText(Content); Row.Src.CreateInStream(ins);
        Row.Pic.ImportStream(ins, Content);
    end;

    // media/TestMediaPngImport.al:297-334.
    procedure SharedReplace(): Text
    var Row1: Record "SV Media"; Row2: Record "SV Media"; TenantMedia: Record "Tenant Media"; First: Guid; s: Text;
    begin
        Row1.Id := 1; Row1.Insert(); Import(Row1, 'one'); Row1.Modify();
        Row1.Get(1);
        First := Row1.Pic.MediaId();
        Row2.Init(); Row2.Id := 2; Row2.Pic := Row1.Pic; Row2.Insert();
        Import(Row1, 'two');
        Row1.Modify();
        s := Format(TenantMedia.Get(First));
        Row2.Get(2);
        s += Format(Row2.Pic.MediaId() = First) + Format(Row2.Pic.HasValue());
        Row2.Delete();
        exit(s + Format(TenantMedia.Get(First)));
    end;

    // A rollback brings back the Tenant Media row a Delete removed, with the row.
    procedure DeleteThenRollback(): Text
    var Row: Record "SV Media"; TenantMedia: Record "Tenant Media"; First: Guid;
    begin
        Row.Id := 1; Row.Insert(); Import(Row, 'one'); Row.Modify();
        Commit();
        First := Row.Pic.MediaId();
        asserterror begin
            Row.Delete();
            Error('undo');
        end;
        Row.Get(1);
        exit(Format(TenantMedia.Get(First)) + Format(Row.Pic.HasValue()));
    end;

    // Not pinned: what a Delete does to the Tenant Media rows of a MediaSet.
    procedure DeleteWithMediaSet()
    var Row: Record "SV Media Set"; ins: InStream; outs: OutStream;
    begin
        Row.Id := 1; Row.Insert();
        Row.Src.CreateOutStream(outs); outs.WriteText('one'); Row.Src.CreateInStream(ins);
        Row.Pics.ImportStream(ins, 'one');
        Row.Modify();
        Row.Delete();
    end;

    // media/TestMediaPngImport.al:441-470 QE_Media_OtherTable_Assigned_SourceRowDeleted.
    procedure SharedWithOtherTable(): Text
    var Row: Record "SV Media"; Other: Record "SV Media Other"; TenantMedia: Record "Tenant Media"; Id: Guid; outs: OutStream;
    begin
        Row.Id := 1; Row.Insert(); Import(Row, 'one'); Row.Modify();
        Row.Get(1);
        Id := Row.Pic.MediaId();
        Other.Id := 1; Other.Pic := Row.Pic; Other.Insert();
        Row.Delete();
        Other.Get(1);
        Row.Src.CreateOutStream(outs);
        exit(Format(TenantMedia.Get(Id)) + Format(Other.Pic.MediaId() = Id) + Format(Other.Pic.HasValue())
            + Format(Other.Pic.ExportStream(outs)));
    end;
}
"#;

// TestMediaPngImport.al:322-332: a replaced media another row still holds stays in Tenant Media; the Delete of its last row
// removes it.
#[test]
fn a_shared_media_stays_until_its_last_row_goes() {
    assert_eq!(text(al_testkit::run_in(MEDIA, "M", "SharedReplace")), "YesYesYesNo");
}

#[test]
fn a_rollback_restores_the_media_a_delete_removed() {
    assert_eq!(text(al_testkit::run_in(MEDIA, "M", "DeleteThenRollback")), "YesYes");
}

// TestMediaPngImport.al:441-470: a Delete of one holder removes the media from Tenant Media although a row of another
// table holds it; that row keeps the MediaId, HasValue is false and ExportStream returns false.
#[test]
fn a_media_shared_with_another_table_leaves_with_its_first_holder() {
    assert_eq!(
        text(al_testkit::run_in(MEDIA, "M", "SharedWithOtherTable")),
        "NoYesNoNo"
    );
}

const DATES: &str = r#"
table 69986 "SV Dates"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dt; Date) { }
        field(4; Tm; Time) { }
        field(5; Dtm; DateTime) { }
    }
    keys { key(PK; Id) { } }
}
table 69987 "SV Dates Blob"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } field(3; B; Blob) { } }
    keys { key(PK; Id) { } }
}
codeunit 69988 D
{
    // errors/lt-ms-refusals-130/P130DateFilters.al:123-186.
    procedure Milliseconds(): Text
    var R: Record "SV Dates"; Back: Record "SV Dates"; Ms: List of [Integer]; M: Integer; T: Time; DT: DateTime; s: Text;
    begin
        Ms.AddRange(1, 2, 3, 4, 5, 6, 7, 8, 9, 998, 999);
        foreach M in Ms do begin
            T := 000000T + M;
            DT := CreateDateTime(20240325D, T);
            R.Init(); R.Id := M; R.Tm := T; R.Dtm := DT; R.Insert();
            Back.Get(M);
            s += Format(Back.Dtm, 0, 9) + '|' + Format(Back.Dtm = DT) + '|' + Format(R.Dtm = DT) + '/';
            s += Format(Back.Tm, 0, 9) + '|' + Format(Back.Tm = T) + '|' + Format(R.Tm = T) + ';';
        end;
        exit(s);
    end;

    // P130DateFilters.al:190-232.
    procedure Filters(): Text
    var R: Record "SV Dates"; s: Text;
    begin
        R.Init(); R.Id := 1; R.Dtm := CreateDateTime(20240325D, 000000T); R.Insert();
        R.Init(); R.Id := 2; R.Dtm := CreateDateTime(20240324D, 235959.997T); R.Insert();
        R.Init(); R.Id := 3; R.Tm := 120000T; R.Insert();
        R.Reset();
        R.SetRange(Dtm, CreateDateTime(20240301D, 000000T), CreateDateTime(20240324D, 235959.999T));
        s := Format(R.Count()) + '|' + R.GetFilters() + '|';
        R.SetRange(Dtm, CreateDateTime(20240301D, 000000T), CreateDateTime(20240324D, 235959.998T));
        s += Format(R.Count());
        R.SetRange(Dtm, CreateDateTime(20240301D, 000000T), CreateDateTime(20240324D, 235959.996T));
        s += Format(R.Count());
        R.SetRange(Dtm, CreateDateTime(20240325D, 000000.001T));
        s += Format(R.Count());
        R.SetRange(Dtm, CreateDateTime(20240324D, 235959.999T));
        s += Format(R.Count());
        R.SetRange(Dtm);
        R.SetRange(Tm, 120000.001T);
        s += Format(R.Count());
        R.SetRange(Tm, 120000.002T);
        s += Format(R.Count());
        exit(s);
    end;

    // P130DateFilters.al:234-275: the Insert does not fail; the next access of the table does.
    procedure Store(Dt: Date; Dtm: DateTime): Text
    var R: Record "SV Dates"; Back: Record "SV Dates"; Stage: Text;
    begin
        ClearLastError();
        asserterror begin
            Stage := 'insert';
            R.Init(); R.Id := 900; R.Dt := Dt; R.Dtm := Dtm; R.Insert();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored:' + Format(Back.Dt, 0, 9) + '|' + Format(Back.Dtm, 0, 9) + '|' + Format(Back.Dt = Dt) + '|' + Format(Back.Dtm = Dtm));
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure StoreBlobTable(Dt: Date)
    var R: Record "SV Dates Blob";
    begin
        R.Id := 1; R.Dt := Dt; R.Insert();
    end;

    procedure ModifyTo(Dt: Date)
    var R: Record "SV Dates";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify();
    end;

    procedure TimeRollsOver(): Text
    var R: Record "SV Dates";
    begin
        R.Id := 1; R.Tm := 235959.999T; R.Insert();
        R.Get(1);
        exit(Format(R.Tm, 0, 9));
    end;

    procedure TempOnTheGrid(): Text
    var R: Record "SV Dates" temporary;
    begin
        R.Id := 1; R.Tm := 120000.003T; R.Insert();
        R.Get(1);
        exit(Format(R.Tm, 0, 9));
    end;

    procedure FilterSeconds(): Text
    var R: Record "SV Dates";
    begin
        R.SetRange(Dtm, CreateDateTime(20240301D, 120030T));
        exit(R.GetFilters());
    end;

    procedure TempKeepsMilliseconds(): Text
    var R: Record "SV Dates" temporary;
    begin
        R.Id := 1; R.Tm := 120000.001T; R.Insert();
        R.Get(1);
        exit(Format(R.Tm, 0, 9));
    end;
}
"#;

fn dates(routine: &str, args: Vec<al_builtins::value::Value>) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_with(DATES, "D", routine, args).map(|r| r.0)
}

fn date(y: i32, m: u32, d: u32, closing: bool) -> al_builtins::value::Value {
    al_builtins::value::Value::Date(al_builtins::format::day_from_civil(y, m, d), closing)
}

fn datetime(y: i32, m: u32, d: u32, ms: i64) -> al_builtins::value::Value {
    al_builtins::value::Value::DateTime(
        (i64::from(al_builtins::format::day_from_civil(y, m, d)) - 1) * 86_400_000 + ms + 1,
    )
}

// P130DateFilters.al:162-183: a non-temporary DateTime and Time are stored in 1/300 s; the variable keeps what it was given.
#[test]
fn stored_datetime_and_time_round_to_three_hundredths() {
    let want = [
        ("2024-03-25T00:00:00Z|No|Yes", "00:00:00|No|Yes"),
        ("2024-03-25T00:00:00.003Z|No|Yes", "00:00:00.003|No|Yes"),
        ("2024-03-25T00:00:00.003Z|Yes|Yes", "00:00:00.003|Yes|Yes"),
        ("2024-03-25T00:00:00.003Z|No|Yes", "00:00:00.003|No|Yes"),
        ("2024-03-25T00:00:00.007Z|No|Yes", "00:00:00.007|No|Yes"),
        ("2024-03-25T00:00:00.007Z|No|Yes", "00:00:00.007|No|Yes"),
        ("2024-03-25T00:00:00.007Z|Yes|Yes", "00:00:00.007|Yes|Yes"),
        ("2024-03-25T00:00:00.007Z|No|Yes", "00:00:00.007|No|Yes"),
        ("2024-03-25T00:00:00.01Z|No|Yes", "00:00:00.01|No|Yes"),
        ("2024-03-25T00:00:00.997Z|No|Yes", "00:00:00.997|No|Yes"),
        ("2024-03-25T00:00:01Z|No|Yes", "00:00:01|No|Yes"),
    ]
    .map(|(a, b)| format!("{a}/{b};"))
    .concat();
    assert_eq!(text(dates("Milliseconds", vec![])), want);
}

// P130DateFilters.al:220-227: the filter values of a non-temporary record are rounded the same way; GetFilters keeps them.
#[test]
fn datetime_and_time_filter_values_round_too() {
    assert_eq!(
        text(dates("Filters", vec![])),
        "2|Dtm: 03/01/24 12:00 AM..03/24/24 11:59:59.999 PM|111110"
    );
}

// P130DateFilters.al:255-275, :285-293.
#[test]
fn dates_out_of_the_sql_range_fail_at_the_next_access() {
    let msg = |v: &str, f: &str| {
        format!(
            "get|Cannot write the value {v} to the field {f} in the table SV Dates because the value is either too long or the content is invalid.|NDBCS:BadCSideData"
        )
    };
    let blank_dtm = al_builtins::value::Value::DateTime(0);
    let blank_dt = al_builtins::value::Value::Date(0, false);
    assert_eq!(
        text(dates("Store", vec![blank_dt.clone(), blank_dtm.clone()])),
        "stored|stored:||Yes|Yes|Dialog"
    );
    assert_eq!(
        text(dates("Store", vec![date(1753, 1, 1, false), blank_dtm.clone()])),
        msg("01/01/1753", "Dt")
    );
    assert_eq!(
        text(dates("Store", vec![date(1752, 12, 31, false), blank_dtm.clone()])),
        msg("12/31/1752", "Dt")
    );
    assert_eq!(
        text(dates("Store", vec![date(1752, 12, 31, true), blank_dtm.clone()])),
        msg("C12/31/1752", "Dt")
    );
    assert_eq!(
        text(dates("Store", vec![blank_dt.clone(), datetime(1753, 1, 1, 0)])),
        msg("01/01/1753 12:00 AM", "Dtm")
    );
    assert_eq!(
        text(dates(
            "Store",
            vec![blank_dt.clone(), datetime(1752, 12, 31, 12 * 3_600_000)]
        )),
        msg("12/31/1752 12:00 PM", "Dtm")
    );
    // :306-318.
    assert_eq!(
        text(dates(
            "Store",
            vec![blank_dt.clone(), datetime(9999, 12, 31, 86_399_997)]
        )),
        "stored|stored:|9999-12-31T23:59:59.997Z|Yes|Yes|Dialog"
    );
    assert_eq!(
        text(dates(
            "Store",
            vec![blank_dt.clone(), datetime(9999, 12, 31, 86_399_999)]
        )),
        msg("12/31/9999 11:59 PM", "Dtm")
    );
}

// Not pinned: other dates before 1753-01-02, an Insert into a table with a Blob field, a Modify, a Time that rounds past
// midnight.
#[test]
fn unpinned_sql_range_cases_are_refused() {
    for (routine, d) in [
        ("StoreBlobTable", date(1753, 1, 1, false)),
        ("ModifyTo", date(1753, 1, 1, false)),
        ("ModifyTo", date(1600, 6, 1, false)),
        ("ModifyTo", date(1753, 1, 1, true)),
    ] {
        let r = dates(routine, vec![d.clone()]);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine} {d:?}: {r:?}");
    }
    let r = dates(
        "Store",
        vec![date(1600, 6, 1, false), al_builtins::value::Value::DateTime(0)],
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// errors/lt-ms-refusals-132/P132ReportTimeMs.al:190, :197: a Time that rounds to midnight reads 00:00:00.
#[test]
fn a_time_that_rounds_to_midnight_reads_midnight() {
    assert_eq!(text(dates("TimeRollsOver", vec![])), "00:00:00");
}

// errors/lt-ms-refusals-130/P130DateFilters.al:324 (QA_TempRecord_Milliseconds_InsertGetModify): a temporary table keeps
// the milliseconds whole through Insert and Get; only a write to a database table rounds them.
#[test]
fn a_temporary_row_keeps_unrounded_milliseconds() {
    assert_eq!(text(dates("TempKeepsMilliseconds", vec![])), "12:00:00.001");
    assert_eq!(text(dates("TempOnTheGrid", vec![])), "12:00:00.003");
}

// Not pinned: GetFilters of a DateTime with seconds and no milliseconds.
#[test]
fn getfilters_of_a_datetime_with_seconds_is_refused() {
    let r = dates("FilterSeconds", vec![]);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_delete_of_a_row_with_a_media_set_is_refused() {
    let r = al_testkit::run_in(MEDIA, "M", "DeleteWithMediaSet");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// CnTransferFieldsModify.al:250-268 QE_TF_Other_StaleBuffer_SourceOlder: the buffer keeps its own row version, so a stale
// buffer stays stale after a TransferFields from another table.
#[test]
fn a_stale_buffer_stays_stale_after_another_tables_transferfields() {
    let r = run("TransferFromOtherTableStale");
    assert!(
        matches!(&r, Err(Signal::Error(e)) if e.code == "DB:RecordChanged"),
        "{r:?}"
    );
    // CnTransferFieldsModify.al:194-214: a current buffer modifies either way.
    assert_eq!(text(run("TransferFromOtherTableCurrent")), "3");
}

// CnTransferFieldsModify.al:543 QH_TF_Other_SourceInMemory_AuditFields: the buffer takes the in-memory source's blank At and null By
// fields; Modify stamps the modified pair on the buffer and in the database; the database keeps its own created pair.
#[test]
fn transfer_fields_from_another_table_copies_the_four_audit_fields() {
    assert_eq!(
        text(run("TransferAuditFromMemory")),
        "YesYesYesYes|YesNoYesYes|NoYesYes"
    );
}

/// `Media.ExportStream` of an empty Media field is not pinned; Custom Report Layout.InitBuiltInLayout reaches it through
/// the Layout of a built-in Report Layout List row, whose HasValue the corpus pins as No (report/PZReportLayoutMedia.al:51-53)
/// but whose export it does not. The refusal names that case (al-runner issue 5712).
#[test]
fn an_export_of_an_empty_media_field_is_refused_naming_the_built_in_layout_case() {
    let src = r#"
table 69940 "EM Row" { fields { field(1; Id; Integer) { } field(2; Pic; Media) { } } keys { key(PK; Id) { } } }
table 69941 "EM Out" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69942 C
{
    procedure F(): Boolean
    var R: Record "EM Row"; O: Record "EM Out" temporary; S: OutStream;
    begin
        O.Data.CreateOutStream(S);
        exit(R.Pic.ExportStream(S));
    end;
}
"#;
    match al_testkit::run_in(src, "C", "F") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Report Layout List"), "{why}"),
        other => panic!("{other:?}"),
    }
}
