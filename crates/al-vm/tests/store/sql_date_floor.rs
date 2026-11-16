//! A Date or DateTime in the year 1753 written to a database table (al-runner issue 8006, record/filters/SqlDateFloor1753.al). The pinned Insert cells stay clean (record/filters/P130DateFilters.al:255-293).

use al_builtins::assumptions::{clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "SDF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dt; Date) { }
        field(3; Tm; Time) { }
        field(4; Dtm; DateTime) { }
    }
    keys { key(PK; Id) { } }
}
table 69991 "SDF Trig Row"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin end;
}
table 69993 "SDF Blob Row"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } field(3; B; Blob) { } }
    keys { key(PK; Id) { } }
}
table 69994 "SDF Sub Row"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
}
table 69995 "SDF Date Key"
{
    fields { field(1; Dk; Date) { } }
    keys { key(PK; Dk) { } }
}
table 69996 "SDF Date Key Trig"
{
    fields { field(1; Dk; Date) { } }
    keys { key(PK; Dk) { } }
    trigger OnRename() begin end;
}
table 69997 "SDF After Only"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
}
table 69980 "SDF Trig Log"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() var L: Codeunit "SDF Log"; begin L.Add('trigger'); end;
}
table 69981 "SDF Trig Date"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } field(3; Other; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin Other := 20240101D; end;
}
table 69982 "SDF Two Before"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
}
table 69983 "SDF Sub Reads"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
}
table 69984 "SDF Date Key Amt"
{
    fields { field(1; Dk; Date) { } field(2; Amount; Integer) { } }
    keys { key(PK; Dk) { } }
}
table 69970 "SDF Trig Logs Dt"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() var L: Codeunit "SDF Log"; begin L.Add('trig:' + Format(Dt, 0, 9)); end;
}
table 69971 "SDF Trig Sets Dt"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin if Dt = 20240103D then Dt := 17530615D else Dt := 20240104D; end;
}
table 69972 "SDF Trig Sets Both"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin if Dt = 20240103D then Dt := 17530615D else Dt := 17530616D; end;
}
table 69973 "SDF Sub Read"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } field(3; Dtm; DateTime) { } }
    keys { key(PK; Id) { } }
}
codeunit 69974 "SDF Subs 3"
{
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Read", 'OnAfterInsertEvent', '', false, false)]
    local procedure RD(var Rec: Record "SDF Sub Read"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('read:' + Format(Rec.Dt, 0, 9)); end;
}
tableextension 69985 "SDF Row Ext" extends "SDF Row"
{
    fields { field(50; XDt; Date) { } }
}
codeunit 69979 "SDF Subs 2"
{
    [EventSubscriber(ObjectType::Table, Database::"SDF Two Before", 'OnBeforeModifyEvent', '', false, false)]
    local procedure B1(var Rec: Record "SDF Two Before"; var xRec: Record "SDF Two Before"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('before1'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Two Before", 'OnBeforeModifyEvent', '', false, false)]
    local procedure B2(var Rec: Record "SDF Two Before"; var xRec: Record "SDF Two Before"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('before2'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Two Before", 'OnAfterModifyEvent', '', false, false)]
    local procedure A1(var Rec: Record "SDF Two Before"; var xRec: Record "SDF Two Before"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('after'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Reads", 'OnBeforeInsertEvent', '', false, false)]
    local procedure RBI(var Rec: Record "SDF Sub Reads"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('beforeInsert'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Reads", 'OnAfterInsertEvent', '', false, false)]
    local procedure RAI(var Rec: Record "SDF Sub Reads"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add(Format(Rec.Dt, 0, 9)); end;
}
codeunit 69998 "SDF Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(What: Text) begin Entries += What + ';'; end;
    procedure Dump(): Text begin exit(Entries); end;
    procedure Reset() begin Entries := ''; end;
}
codeunit 69999 "SDF Subs"
{
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure BI(var Rec: Record "SDF Sub Row"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('beforeInsert'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure AI(var Rec: Record "SDF Sub Row"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('afterInsert'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Row", 'OnBeforeModifyEvent', '', false, false)]
    local procedure BM(var Rec: Record "SDF Sub Row"; var xRec: Record "SDF Sub Row"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('beforeModify'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF Sub Row", 'OnAfterModifyEvent', '', false, false)]
    local procedure AM(var Rec: Record "SDF Sub Row"; var xRec: Record "SDF Sub Row"; RunTrigger: Boolean) var L: Codeunit "SDF Log"; begin L.Add('afterModify'); end;
    [EventSubscriber(ObjectType::Table, Database::"SDF After Only", 'OnAfterModifyEvent', '', false, false)]
    local procedure AO(var Rec: Record "SDF After Only"; var xRec: Record "SDF After Only"; RunTrigger: Boolean) begin end;
    [EventSubscriber(ObjectType::Table, Database::"SDF After Only", 'OnAfterInsertEvent', '', false, false)]
    local procedure AOI(var Rec: Record "SDF After Only"; RunTrigger: Boolean) begin end;
}
codeunit 69992 C
{
    procedure ModifyTrigLog(Dt: Date)
    var R: Record "SDF Trig Log";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure ModifyTrigDate(Dt: Date)
    var R: Record "SDF Trig Date";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure TrigModifyAllEmpty(Dt: Date)
    var R: Record "SDF Trig Row";
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dt, Dt);
    end;

    procedure ModifyAllDtmEmpty(Dtm: DateTime)
    var R: Record "SDF Row";
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dtm, Dtm);
    end;

    procedure TempModifyAllEmpty(Dt: Date)
    var R: Record "SDF Row" temporary;
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dt, Dt);
    end;

    procedure ExtModifyAll(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.ModifyAll(XDt, Dt);
    end;

    procedure RenameMissing(Dt: Date)
    var R: Record "SDF Date Key";
    begin
        R.Dk := 20240102D;
        R.Rename(Dt);
    end;

    procedure RenameChanged(Dt: Date)
    var R: Record "SDF Date Key Amt";
    begin
        R.Dk := 20240102D; R.Insert();
        R.Get(20240102D);
        R.Amount := 5;
        R.Rename(Dt);
    end;

    procedure RenameNotRead(Dt: Date)
    var R: Record "SDF Date Key Amt"; Other: Record "SDF Date Key Amt";
    begin
        Other.Dk := 20240102D; Other.Amount := 3; Other.Insert();
        R.Dk := 20240102D;
        R.Rename(Dt);
    end;

    procedure RenameAfterGet(Dt: Date)
    var R: Record "SDF Date Key Amt";
    begin
        R.Dk := 20240102D; R.Amount := 3; R.Insert();
        Clear(R);
        R.Get(20240102D);
        R.Rename(Dt);
    end;

    procedure BlobInsertTrue(Dt: Date)
    var R: Record "SDF Blob Row";
    begin
        R.Id := 1; R.Dt := Dt; R.Insert(true);
    end;

    procedure BlobInsertDuplicate(Dt: Date)
    var R: Record "SDF Blob Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Insert();
    end;

    procedure BlobInsertContent(Dt: Date)
    var R: Record "SDF Blob Row"; OS: OutStream;
    begin
        R.Id := 1; R.Dt := Dt;
        R.B.CreateOutStream(OS); OS.WriteText('x');
        R.Insert();
    end;

    procedure TwoBeforeModify(Dt: Date)
    var R: Record "SDF Two Before";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure SubReadsInsert(Dt: Date)
    var R: Record "SDF Sub Reads";
    begin
        R.Id := 1; R.Dt := Dt; R.Insert(true);
    end;

    procedure RecRefInsertDtm(Dtm: DateTime): Text
    var RR: RecordRef; Back: Record "SDF Row"; Stage: Text;
    begin
        asserterror begin
            Stage := 'insert';
            RR.Open(Database::"SDF Row");
            RR.Field(1).Value := 900; RR.Field(4).Value := Dtm; RR.Insert();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored');
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ModifyAllEmpty(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dt, Dt);
    end;

    procedure ModifyAllTrueEmpty(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dt, Dt, true);
    end;

    procedure BlobModifyAllEmpty(Dt: Date)
    var R: Record "SDF Blob Row";
    begin
        R.SetRange(Id, 99);
        R.ModifyAll(Dt, Dt);
    end;

    procedure AfterOnlyModifyAll(Dt: Date)
    var R: Record "SDF After Only";
    begin
        R.Id := 1; R.Insert();
        R.ModifyAll(Dt, Dt);
    end;

    procedure AfterOnlyModify(Dt: Date)
    var R: Record "SDF After Only";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify();
    end;

    procedure AfterOnlyInsert(Dt: Date)
    var R: Record "SDF After Only";
    begin
        R.Id := 1; R.Dt := Dt; R.Insert(true);
    end;

    procedure BlobInsert(Dt: Date): Text
    var R: Record "SDF Blob Row"; Stage: Text;
    begin
        asserterror begin
            Stage := 'insert';
            R.Id := 1; R.Dt := Dt; R.Insert();
            Stage := 'after';
            Error('stored');
        end;
        exit(Stage + '|' + GetLastErrorText());
    end;

    procedure BlobInsertUsed(Dt: Date): Boolean
    var R: Record "SDF Blob Row";
    begin
        R.Id := 1; R.Dt := Dt;
        exit(R.Insert());
    end;

    procedure SubInsert(Dt: Date; Run: Boolean): Text
    var R: Record "SDF Sub Row"; Back: Record "SDF Sub Row"; L: Codeunit "SDF Log"; Stage: Text;
    begin
        asserterror begin
            Stage := 'insert';
            R.Id := 1; R.Dt := Dt; R.Insert(Run);
            Stage := 'next';
            Back.Get(1);
            Stage := 'after';
            Error('stored');
        end;
        exit(Stage + '|' + L.Dump());
    end;

    procedure SubModify(Dt: Date): Text
    var R: Record "SDF Sub Row"; L: Codeunit "SDF Log"; Stage: Text;
    begin
        R.Id := 1; R.Insert();
        L.Reset();
        asserterror begin
            Stage := 'modify';
            R.Dt := Dt; R.Modify(true);
            Stage := 'after';
            Error('stored');
        end;
        exit(Stage + '|' + L.Dump());
    end;

    procedure RenameKey(Dt: Date): Text
    var R: Record "SDF Date Key"; Back: Record "SDF Date Key"; Stage: Text;
    begin
        R.Dk := 20240102D; R.Insert();
        Commit();
        asserterror begin
            Stage := 'rename';
            R.Rename(Dt);
            Stage := 'after';
            Error('stored');
        end;
        Back.FindFirst();
        exit(Stage + '|' + GetLastErrorText() + '|' + Format(Back.Dk, 0, 9));
    end;

    procedure RenameKeyUsed(Dt: Date): Boolean
    var R: Record "SDF Date Key";
    begin
        R.Dk := 20240102D; R.Insert();
        exit(R.Rename(Dt));
    end;

    procedure RenameKeyTrig(Dt: Date)
    var R: Record "SDF Date Key Trig";
    begin
        R.Dk := 20240102D; R.Insert();
        R.Rename(Dt);
    end;


    procedure ModifyMissingRow(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 901; R.Dt := Dt; R.Modify();
    end;

    procedure ModifyAllPlain(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.ModifyAll(Dt, Dt);
    end;

    procedure ModifyAllTrue(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.ModifyAll(Dt, Dt, true);
    end;

    procedure RecRefModify(Dt: Date)
    var R: Record "SDF Row"; RR: RecordRef;
    begin
        R.Id := 1; R.Insert();
        RR.GetTable(R);
        RR.Field(2).Value := Dt;
        RR.Modify();
    end;

    procedure BlobModify(Dt: Date)
    var R: Record "SDF Blob Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify();
    end;

    procedure ModifyDtm(Dtm: DateTime)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dtm := Dtm; R.Modify();
    end;

    procedure StampAfterFailedModify(Dt: Date): Boolean
    var R: Record "SDF Row"; Back: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        Back.Get(1);
        R.Dt := Dt;
        asserterror R.Modify();
        exit(R.SystemModifiedAt = Back.SystemModifiedAt);
    end;

    // The reporter's Insert shape, as record/filters/P130DateFilters.al:234-251 writes it.
    procedure Insert(Dt: Date; Dtm: DateTime): Text
    var R: Record "SDF Row"; Back: Record "SDF Row"; Stage: Text;
    begin
        ClearLastError();
        asserterror begin
            Stage := 'insert';
            R.Init(); R.Id := 900; R.Dt := Dt; R.Dtm := Dtm; R.Insert();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored:' + Format(Back.Dt, 0, 9) + '|' + Format(Back.Dtm, 0, 9));
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    // The reporter's Modify shape: Modify of an inserted row.
    procedure Modify(Dt: Date; Dtm: DateTime): Text
    var R: Record "SDF Row"; Back: Record "SDF Row"; Stage: Text;
    begin
        R.Init(); R.Id := 900; R.Insert();
        ClearLastError();
        asserterror begin
            Stage := 'modify';
            R.Dt := Dt; R.Dtm := Dtm; R.Modify();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored:' + Format(Back.Dt, 0, 9) + '|' + Format(Back.Dtm, 0, 9));
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure TempInsertModify(Dt: Date): Text
    var R: Record "SDF Row" temporary;
    begin
        R.Id := 1; R.Dt := Dt; R.Insert();
        R.Id := 2; R.Insert(); R.Dt := Dt; R.Modify();
        R.Get(1);
        exit(Format(R.Dt, 0, 9));
    end;

    procedure ModifyUsed(Dt: Date): Boolean
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt;
        exit(R.Modify());
    end;

    procedure ModifyWithTrigger(Dt: Date)
    var R: Record "SDF Trig Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure InsertTrue(Dt: Date; Dtm: DateTime)
    var R: Record "SDF Row";
    begin
        R.Id := 900; R.Dt := Dt; R.Dtm := Dtm; R.Insert(true);
    end;

    procedure ModifyTrue(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure TrigLogModify(Dt: Date; Run: Boolean): Text
    var R: Record "SDF Trig Logs Dt"; L: Codeunit "SDF Log"; Stage: Text;
    begin
        R.Id := 1; R.Insert();
        L.Reset();
        asserterror begin
            Stage := 'modify';
            R.Dt := Dt; R.Modify(Run);
            Stage := 'after';
            Error('stored');
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + L.Dump());
    end;

    procedure TrigLogModifyUsed(Dt: Date): Boolean
    var R: Record "SDF Trig Logs Dt";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt;
        exit(R.Modify(true));
    end;

    procedure SetDtModify(Dt: Date; Run: Boolean): Text
    var R: Record "SDF Trig Sets Dt"; Back: Record "SDF Trig Sets Dt"; Stage: Text;
    begin
        R.Id := 1; R.Insert();
        asserterror begin
            Stage := 'modify';
            R.Dt := Dt; R.Modify(Run);
            Stage := 'after';
            Back.Get(1);
            Error('stored:' + Format(Back.Dt, 0, 9));
        end;
        exit(Stage + '|' + GetLastErrorText());
    end;

    procedure SetBothModify(Dt: Date)
    var R: Record "SDF Trig Sets Both";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure SubReadInsert(Dt: Date; Dtm: DateTime; Run: Boolean): Text
    var R: Record "SDF Sub Read"; Back: Record "SDF Sub Read"; L: Codeunit "SDF Log"; Stage: Text;
    begin
        L.Reset();
        asserterror begin
            Stage := 'insert';
            R.Id := 1; R.Dt := Dt; R.Dtm := Dtm; R.Insert(Run);
            Stage := 'next';
            Back.Get(1);
            Stage := 'after';
            Error('stored');
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + L.Dump());
    end;

    procedure SubReadInsertStored(Dt: Date)
    var R: Record "SDF Sub Read";
    begin
        R.Id := 1; R.Insert(false);
        R.Dt := Dt; R.Insert(true);
    end;

    procedure InsertStoredKey(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Insert();
    end;

    procedure RenameKeyCallee(Dt: Date): Text
    var Seed: Record "SDF Date Key"; Back: Record "SDF Date Key"; Stage: Text;
    begin
        Seed.Dk := 20240102D; Seed.Insert();
        Commit();
        asserterror RenameKeyIn(Dt, Stage);
        Back.FindFirst();
        exit(Stage + '|' + GetLastErrorText() + '|' + Format(Back.Dk, 0, 9));
    end;

    local procedure RenameKeyIn(Dt: Date; var Stage: Text)
    var R: Record "SDF Date Key";
    begin
        R.Get(20240102D);
        Stage := 'rename';
        R.Rename(Dt);
        Stage := 'after';
        Error('stored');
    end;

    procedure RenameMissingText(Dt: Date): Text
    var R: Record "SDF Date Key"; Other: Record "SDF Date Key";
    begin
        Other.Dk := 20240102D; Other.Insert();
        R.Dk := 20250505D;
        asserterror R.Rename(Dt);
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(R.Dk, 0, 9));
    end;

    procedure RenameTo(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Rename(2);
    end;
}
"#;

fn date(y: i32, m: u32, d: u32, closing: bool) -> Value {
    Value::Date(al_builtins::format::day_from_civil(y, m, d), closing)
}

fn datetime(y: i32, m: u32, d: u32, ms: i64) -> Value {
    Value::DateTime((i64::from(al_builtins::format::day_from_civil(y, m, d)) - 1) * 86_400_000 + ms + 1)
}

const BLANK_DT: Value = Value::Date(0, false);
const BLANK_DTM: Value = Value::DateTime(0);

/// The routine's result and whether the run passed through the assumption.
fn run(routine: &str, args: Vec<Value>) -> (Result<Value, Signal>, bool) {
    clear_hits();
    let r = al_testkit::run_with(SRC, "C", routine, args).map(|r| r.0);
    (r, !take_hits().is_empty())
}

fn unflagged(routine: &str, args: Vec<Value>) -> String {
    let (r, hit) = run(routine, args);
    assert!(!hit, "{routine} passed through the assumption: {r:?}");
    text(r)
}

fn refused(routine: &str, args: Vec<Value>) {
    let (r, _) = run(routine, args);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
}

fn bad(stage: &str, value: &str, field: &str) -> String {
    format!(
        "{stage}|Cannot write the value {value} to the field {field} in the table SDF Row because the value is either too long or the content is invalid.|NDBCS:BadCSideData"
    )
}

// al-runner issue 8006: the Insert fails as the pinned values do, at the next access (P130DateFilters.al:255-275).
#[test]
fn an_insert_of_a_1753_date_fails_at_the_next_access() {
    assert_eq!(
        unflagged("Insert", vec![date(1753, 1, 2, false), BLANK_DTM]),
        bad("get", "01/02/1753", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![date(1753, 12, 31, false), BLANK_DTM]),
        bad("get", "12/31/1753", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 12, 31, 86_399_000)]),
        bad("get", "12/31/1753 11:59 PM", "Dtm")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 1, 1, 1_000)]),
        bad("get", "01/01/1753 12:00 AM", "Dtm")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 1, 4, 12 * 3_600_000)]),
        bad("get", "01/04/1753 12:00 PM", "Dtm")
    );
}

// The values P130DateFilters.al:255-293 pins stay clean passes.
#[test]
fn the_pinned_insert_values_stay_clean() {
    assert_eq!(
        unflagged("Insert", vec![date(1753, 1, 1, false), BLANK_DTM]),
        bad("get", "01/01/1753", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 1, 1, 0)]),
        bad("get", "01/01/1753 12:00 AM", "Dtm")
    );
}

// al-runner issue 8006: 1754-01-01 is the first date BC stores, as Date and as DateTime.
#[test]
fn the_first_day_of_1754_stores() {
    assert_eq!(
        unflagged("Insert", vec![date(1754, 1, 1, false), BLANK_DTM]),
        "stored|stored:1754-01-01||Dialog"
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1754, 1, 1, 1_000)]),
        "stored|stored:|1754-01-01T00:00:01Z|Dialog"
    );
    assert_eq!(
        unflagged("Modify", vec![date(1754, 1, 1, false), BLANK_DTM]),
        "stored|stored:1754-01-01||Dialog"
    );
}

// al-runner issue 8006: the Modify of a 1753 value fails with the same text.
#[test]
fn a_modify_of_a_1753_date_fails() {
    assert_eq!(
        unflagged("Modify", vec![date(1753, 1, 2, false), BLANK_DTM]),
        bad("modify", "01/02/1753", "Dt")
    );
    assert_eq!(
        unflagged("Modify", vec![date(1753, 1, 1, false), BLANK_DTM]),
        bad("modify", "01/01/1753", "Dt")
    );
    assert_eq!(
        unflagged("Modify", vec![BLANK_DT, datetime(1753, 12, 31, 86_399_000)]),
        bad("modify", "12/31/1753 11:59 PM", "Dtm")
    );
    assert_eq!(
        unflagged("Modify", vec![BLANK_DT, datetime(1753, 1, 1, 1_000)]),
        bad("modify", "01/01/1753 12:00 AM", "Dtm")
    );
}

// Not measured: a temporary record keeps main's result (it stores the value).
#[test]
fn a_temporary_record_keeps_the_value() {
    assert_eq!(
        unflagged("TempInsertModify", vec![date(1753, 1, 2, false)]),
        "1753-01-02"
    );
}

// Not measured, refused: a millisecond the column rounds into another minute (other than 1753-12-31 23:59:59.999), a Modify
// of a row that does not exist, RecordRef.Modify, and a Rename that writes a 1753 value into a field outside the key.
#[test]
fn the_unmeasured_neighbors_are_refused() {
    refused(
        "Insert",
        vec![BLANK_DT, datetime(1753, 6, 1, 23 * 3_600_000 + 59 * 60_000 + 59_999)],
    );
    refused("ModifyMissingRow", vec![date(1753, 6, 15, false)]);
    refused("RecRefModify", vec![date(1753, 6, 15, false)]);
    refused("RenameTo", vec![date(1753, 1, 2, false)]);
}

fn bad_in(stage: &str, value: &str, field: &str, table: &str) -> String {
    format!(
        "{stage}|Cannot write the value {value} to the field {field} in the table {table} because the value is either too long or the content is invalid."
    )
}

fn raised(routine: &str, args: Vec<Value>, value: &str, field: &str, table: &str) {
    let (r, hit) = run(routine, args);
    assert!(!hit, "{routine} passed through an assumption: {r:?}");
    let Err(Signal::Error(e)) = r else {
        panic!("{routine}: {r:?}")
    };
    assert_eq!(
        e.message,
        format!(
            "Cannot write the value {value} to the field {field} in the table {table} because the value is either too long or the content is invalid."
        )
    );
}

// Corpus PR 1398 (SqlDateFloor1753.al) measured the cells main refused: a closing date and one-digit hours on Insert
// (Insert_Date_1753_1, Insert_DateTime_MorningHours_1, Insert_DateTime_AfternoonHours_1, Insert_DateTime_RoundingEdge_1), the
// Modify result used, with a table trigger, of the DateTime 1752-12-31 12:00, in a Blob table (Modify_Date_Forms_1,
// Trigger_Date_1, Modify_DateTime_RoundingEdge_2, Blob_Date_1) and ModifyAll (ModifyAll_Forms_1).
#[test]
fn the_probe_cells_run() {
    assert_eq!(
        unflagged("Insert", vec![date(1753, 6, 1, true), BLANK_DTM]),
        bad("get", "C06/01/1753", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 6, 1, 3 * 3_600_000)]),
        bad("get", "06/01/1753 03:00 AM", "Dtm")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 6, 1, 13 * 3_600_000)]),
        bad("get", "06/01/1753 01:00 PM", "Dtm")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 12, 31, 86_399_999)]),
        bad("get", "12/31/1753 11:59 PM", "Dtm")
    );
    raised(
        "ModifyUsed",
        vec![date(1753, 1, 2, false)],
        "01/02/1753",
        "Dt",
        "SDF Row",
    );
    raised(
        "ModifyWithTrigger",
        vec![date(1753, 1, 2, false)],
        "01/02/1753",
        "Dt",
        "SDF Trig Row",
    );
    assert_eq!(
        unflagged("Modify", vec![BLANK_DT, datetime(1752, 12, 31, 12 * 3_600_000)]),
        bad("modify", "12/31/1752 12:00 PM", "Dtm")
    );
    raised(
        "BlobModify",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Blob Row",
    );
    raised(
        "ModifyDtm",
        vec![datetime(1753, 6, 15, 3 * 3_600_000)],
        "06/15/1753 03:00 AM",
        "Dtm",
        "SDF Row",
    );
    raised(
        "ModifyAllPlain",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Row",
    );
    raised(
        "ModifyAllTrue",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Row",
    );
    // ModifyAll_Forms_2: with no row in the filter too.
    raised(
        "ModifyAllEmpty",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Row",
    );
}

// Blob_Date_1: the Insert into a table with a Blob field raises at the Insert. Its result used is not measured.
#[test]
fn a_blob_table_raises_at_the_insert() {
    assert_eq!(
        unflagged("BlobInsert", vec![date(1753, 6, 15, false)]),
        bad_in("insert", "06/15/1753", "Dt", "SDF Blob Row")
    );
    refused("BlobInsertUsed", vec![date(1753, 6, 15, false)]);
}

// Subscriber_Date_1: Insert(true) fires the After event and fails at the next access; Modify(true) fires only the Before event.
// Insert(false) with these subscribers is not measured.
#[test]
fn the_events_around_a_failed_write() {
    assert_eq!(
        unflagged("SubInsert", vec![date(1753, 6, 15, false), Value::Boolean(true)]),
        "next|beforeInsert;afterInsert;"
    );
    refused("SubInsert", vec![date(1753, 6, 15, false), Value::Boolean(false)]);
    assert_eq!(
        unflagged("SubModify", vec![date(1753, 6, 15, false)]),
        "modify|beforeModify;"
    );
}

// Rename_DateKey_1, Rename_DateKey_2: the Rename raises and the row keeps its key. Its result used, a closing date and a table
// with an OnRename trigger are not measured.
#[test]
fn a_rename_to_a_1753_key_raises() {
    assert_eq!(
        unflagged("RenameKeyCallee", vec![date(1753, 6, 15, false)]),
        bad_in("rename", "06/15/1753", "Dk", "SDF Date Key") + "|2024-01-02"
    );
    assert_eq!(
        unflagged("RenameKeyCallee", vec![date(1752, 12, 31, false)]),
        bad_in("rename", "12/31/1752", "Dk", "SDF Date Key") + "|2024-01-02"
    );
    refused("RenameKey", vec![date(1753, 6, 15, true)]);
    refused("RenameKeyUsed", vec![date(1753, 6, 15, false)]);
    refused("RenameKeyTrig", vec![date(1753, 6, 15, false)]);
}

// Shapes outside the measured cells keep main's outcome: a ModifyAll(.., false) row by row (an After subscriber declared)
// refuses as before; a table with only After subscribers refuses its Insert and Modify as before; a closing date into a
// table with insert subscribers refuses as before. ModifyAll over no row of a value the column holds, and of a temporary
// record, raises nothing.
#[test]
fn the_neighbors_keep_main() {
    refused("AfterOnlyModifyAll", vec![date(1753, 6, 15, false)]);
    refused("AfterOnlyModify", vec![date(1753, 6, 15, false)]);
    refused("AfterOnlyInsert", vec![date(1753, 6, 15, false)]);
    refused("SubInsert", vec![date(1753, 6, 15, true), Value::Boolean(true)]);
    let (r, hit) = run("ModifyAllTrueEmpty", vec![date(1754, 1, 1, false)]);
    assert!(!hit && r.is_ok(), "{r:?}");
    let (r, hit) = run("TempModifyAllEmpty", vec![date(1753, 6, 15, false)]);
    assert!(!hit && r.is_ok(), "{r:?}");
}

// A ModifyAll of AL code over no row, of a value the column cannot hold, that no measured cell takes: main raised nothing; the
// measured cells next to it (ModifyAll_Forms_2, Extra_ModifyAllNoRow: ModifyAll(.., false) over no row raises) make that
// doubtful, so it refuses: ModifyAll(.., true), a Blob table, a closing date, a Date before 1753.
#[test]
fn a_modify_all_over_no_row_outside_the_cells_refuses() {
    refused("ModifyAllTrueEmpty", vec![date(1753, 6, 15, false)]);
    refused("BlobModifyAllEmpty", vec![date(1753, 6, 15, false)]);
    refused("ModifyAllEmpty", vec![date(1753, 6, 15, true)]);
    refused("ModifyAllEmpty", vec![date(1700, 1, 1, false)]);
}

// Extra_ModifyAllNoRow: ModifyAll(.., false) over no row raises for the trigger table with a Date in 1753 and for the plain
// table with a DateTime in 1753 (main refused both; these were refused here before the cells were measured).
#[test]
fn a_modify_all_over_no_row_raises_in_the_measured_cells() {
    raised(
        "TrigModifyAllEmpty",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Trig Row",
    );
    raised(
        "ModifyAllDtmEmpty",
        vec![datetime(1753, 6, 15, 3 * 3_600_000)],
        "06/15/1753 03:00 AM",
        "Dtm",
        "SDF Row",
    );
}

// A field a table extension adds is not measured: its ModifyAll goes row by row as on main and refuses.
#[test]
fn a_modify_all_of_a_table_extension_field_refuses() {
    refused("ExtModifyAll", vec![date(1753, 6, 15, false)]);
}

// Only the measured OnModify body (`Hits += 10`) is taken: a trigger that calls code or writes a Date field refuses.
#[test]
fn an_unmeasured_trigger_body_refuses() {
    refused("ModifyTrigLog", vec![date(1753, 6, 15, false)]);
    refused("ModifyTrigDate", vec![date(1753, 6, 15, false)]);
}

// Subscribers: exactly one Before and one After subscriber that only log (the measured bodies). Two Before subscribers, and an
// After insert subscriber that reads Rec, refuse.
#[test]
fn unmeasured_subscribers_refuse() {
    refused("TwoBeforeModify", vec![date(1753, 6, 15, false)]);
    refused("SubReadsInsert", vec![date(1753, 6, 15, false)]);
}

// Rename: the measured cell is a Get then the Rename. A buffer changed after the Get and a buffer not read refuse; a buffer
// read by Get raises. A key no row has raises RecordNotFound (Extra_RenameAbsentKey_1, see
// `a_rename_of_a_missing_row_raises_not_found_first`).
#[test]
fn a_rename_needs_the_row_read() {
    refused("RenameChanged", vec![date(1753, 6, 15, false)]);
    refused("RenameNotRead", vec![date(1753, 6, 15, false)]);
    raised(
        "RenameAfterGet",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dk",
        "SDF Date Key Amt",
    );
}

// Blob table: a key that is stored refuses (not measured), and so does a Date before 1753.
#[test]
fn a_blob_insert_outside_the_cell_refuses() {
    refused("BlobInsertDuplicate", vec![date(1753, 6, 15, false)]);
    refused("BlobInsertContent", vec![date(1700, 1, 1, false)]);
    refused("BlobInsertTrue", vec![date(1700, 1, 1, false)]);
}

// Extra_BlobInsertTrue, Extra_BlobContent: Insert(true) with an empty Blob, and Insert() with content in the Blob, raise at the
// Insert (main refused both; refused here before the cells were measured).
#[test]
fn a_blob_insert_raises_in_the_measured_cells() {
    raised(
        "BlobInsertTrue",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Blob Row",
    );
    raised(
        "BlobInsertContent",
        vec![date(1753, 6, 15, false)],
        "06/15/1753",
        "Dt",
        "SDF Blob Row",
    );
}

// al-runner issue 7834, corpus PR 1398 Outside_Date_Insert, Outside_DateTime_Insert, Outside_Date_Modify,
// Outside_DateTime_Modify: a Date or DateTime before 1753 fails as a 1753 value does, the Insert at the next access, the Modify
// at the Modify.
#[test]
fn a_value_before_1753_fails_like_1753() {
    assert_eq!(
        unflagged("Insert", vec![date(1700, 1, 1, false), BLANK_DTM]),
        bad("get", "01/01/1700", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1700, 1, 1, 12 * 3_600_000)]),
        bad("get", "01/01/1700 12:00 PM", "Dtm")
    );
    assert_eq!(
        unflagged(
            "Insert",
            vec![BLANK_DT, datetime(1752, 12, 31, 23 * 3_600_000 + 59 * 60_000)]
        ),
        bad("get", "12/31/1752 11:59 PM", "Dtm")
    );
    assert_eq!(
        unflagged("Modify", vec![date(1700, 1, 1, false), BLANK_DTM]),
        bad("modify", "01/01/1700", "Dt")
    );
    assert_eq!(
        unflagged("Modify", vec![BLANK_DT, datetime(1700, 1, 1, 12 * 3_600_000)]),
        bad("modify", "01/01/1700 12:00 PM", "Dtm")
    );
    // Modify_Date_ClosingAndOutside_1 to _3 (refused here before they were measured).
    assert_eq!(
        unflagged("Modify", vec![date(1752, 12, 31, false), BLANK_DTM]),
        bad("modify", "12/31/1752", "Dt")
    );
    assert_eq!(
        unflagged("Modify", vec![date(1753, 6, 15, true), BLANK_DTM]),
        bad("modify", "C06/15/1753", "Dt")
    );
}

// Not measured, refused as on main: a value before 1700-01-01, a closing date before 1753, a DateTime before 1753 that the
// column rounds into another minute, Insert(true), Modify(true), a Modify result used, RecordRef.Insert, a Rename and a
// ModifyAll with rows.
#[test]
fn a_value_before_1753_outside_the_cells_refuses() {
    refused("Insert", vec![date(1699, 12, 31, false), BLANK_DTM]);
    refused("Insert", vec![BLANK_DT, datetime(1699, 12, 31, 12 * 3_600_000)]);
    refused("Insert", vec![date(1700, 1, 1, true), BLANK_DTM]);
    refused(
        "Insert",
        vec![BLANK_DT, datetime(1720, 5, 5, 3 * 3_600_000 + 59 * 60_000 + 59_999)],
    );
    refused("InsertTrue", vec![date(1700, 1, 1, false), BLANK_DTM]);
    refused("ModifyTrue", vec![date(1700, 1, 1, false)]);
    refused("ModifyTrue", vec![date(1753, 6, 15, true)]);
    refused("ModifyUsed", vec![date(1700, 1, 1, false)]);
    refused("RecRefInsertDtm", vec![datetime(1700, 1, 1, 12 * 3_600_000)]);
    refused("RenameKey", vec![date(1700, 1, 1, false)]);
    refused("ModifyAllPlain", vec![date(1700, 1, 1, false)]);
}

// Extra_TriggerLog: the OnModify trigger that logs the Date runs before the error of Modify(true); Modify(false) raises without
// it. Its result used is not measured.
#[test]
fn a_logging_trigger_runs_before_the_error() {
    assert_eq!(
        unflagged("TrigLogModify", vec![date(1753, 6, 15, false), Value::Boolean(true)]),
        bad_in("modify", "06/15/1753", "Dt", "SDF Trig Logs Dt") + "|trig:1753-06-15;"
    );
    assert_eq!(
        unflagged("TrigLogModify", vec![date(1753, 6, 15, false), Value::Boolean(false)]),
        bad_in("modify", "06/15/1753", "Dt", "SDF Trig Logs Dt") + "|"
    );
    refused("TrigLogModifyUsed", vec![date(1753, 6, 15, false)]);
    refused("TrigLogModify", vec![date(1700, 1, 1, false), Value::Boolean(true)]);
}

// Extra_TriggerSetsDate: a 1753 Date the OnModify trigger sets raises at the Modify; a 1753 Date the caller set and the trigger
// replaced stores (main). Modify(false) of a caller's 1753 Date and a trigger whose `else` branch sets a 1753 Date too are not
// measured.
#[test]
fn a_date_the_trigger_sets_raises() {
    assert_eq!(
        unflagged("SetDtModify", vec![date(2024, 1, 3, false), Value::Boolean(true)]),
        bad_in("modify", "06/15/1753", "Dt", "SDF Trig Sets Dt")
    );
    assert_eq!(
        unflagged("SetDtModify", vec![date(1753, 6, 15, false), Value::Boolean(true)]),
        "after|stored:2024-01-04"
    );
    refused("SetDtModify", vec![date(1753, 6, 15, false), Value::Boolean(false)]);
    refused("SetBothModify", vec![date(2024, 1, 3, false)]);
}

// Extra_SubscriberReadsDate: the one After insert subscriber that logs the Date sees the 1753 value; the error comes at the next
// access. Insert(false) and a DateTime are not measured.
#[test]
fn an_after_insert_subscriber_sees_the_value() {
    assert_eq!(
        unflagged(
            "SubReadInsert",
            vec![date(1753, 6, 15, false), BLANK_DTM, Value::Boolean(true)]
        ),
        bad_in("next", "06/15/1753", "Dt", "SDF Sub Read") + "|read:1753-06-15;"
    );
    refused(
        "SubReadInsert",
        vec![date(1753, 6, 15, false), BLANK_DTM, Value::Boolean(false)],
    );
    refused(
        "SubReadInsert",
        vec![BLANK_DT, datetime(1753, 6, 15, 3 * 3_600_000), Value::Boolean(true)],
    );
}

// A key a row has: which error BC reports first (the duplicate key or the value) and whether the After event fires is not
// measured, so the reader subscriber cell and the pre-1753 Insert cell refuse (Extra_SubscriberReadsDate and
// Outside_Date_Insert insert a key no row has).
#[test]
fn an_insert_of_a_stored_key_refuses_in_the_new_cells() {
    refused("SubReadInsertStored", vec![date(1753, 6, 15, false)]);
    refused("InsertStoredKey", vec![date(1700, 1, 1, false)]);
}

// Extra_RenameAbsentKey_1: the Rename of a key no row has raises RecordNotFound before the value's error. The Date 1752-12-31
// there is not measured.
#[test]
fn a_rename_of_a_missing_row_raises_not_found_first() {
    let not_found =
        "The SDF Date Key does not exist. Identification fields and values: Dk='05/05/25'|DB:RecordNotFound|2025-05-05";
    assert_eq!(
        unflagged("RenameMissingText", vec![date(1753, 6, 15, false)]),
        not_found
    );
    assert_eq!(unflagged("RenameMissingText", vec![date(1754, 1, 1, false)]), not_found);
    refused("RenameMissing", vec![date(1752, 12, 31, false)]);
}

// The values corpus PR 1398 measured and the reporter did not (a one-digit hour) are taken only by the Insert statement;
// RecordRef.Insert refuses them, and keeps main's result for a value main took (a two-digit hour).
#[test]
fn a_recordref_insert_takes_only_main_values() {
    refused("RecRefInsertDtm", vec![datetime(1753, 6, 15, 3 * 3_600_000)]);
    assert_eq!(
        unflagged("RecRefInsertDtm", vec![datetime(1753, 6, 15, 11 * 3_600_000)]),
        bad("get", "06/15/1753 11:00 AM", "Dtm")
    );
}

// A missing row keeps main's RecordNotFound for a value the column holds.
#[test]
fn a_modify_of_a_missing_row_keeps_main_for_a_storable_value() {
    let (r, hit) = run("ModifyMissingRow", vec![date(1754, 1, 1, false)]);
    assert!(!hit);
    assert!(matches!(r, Err(Signal::Error(_))), "{r:?}");
}

// A failed Modify leaves the buffer as the sibling failure does (unique violation): the stamp is restored.
#[test]
fn a_failed_modify_restores_the_buffer_stamp() {
    let (r, hit) = run("StampAfterFailedModify", vec![date(1753, 6, 15, false)]);
    assert!(!hit);
    assert!(matches!(r, Ok(Value::Boolean(true))), "{r:?}");
}

// Shapes record/filters/SqlDateFloor1753.al does not measure refuse (they ran flagged before the probe was pinned): the days
// and minutes between the measured values before 1753 (1700-01-01, 1700-01-01 12:00, 1752-12-31 23:59), a row with two
// values their columns cannot hold, and a record that still lives where the error of its failed Rename is caught.
#[test]
fn a_day_between_the_measured_values_before_1753_refuses() {
    refused("Insert", vec![date(1720, 5, 5, false), BLANK_DTM]);
    refused("Modify", vec![date(1752, 12, 30, false), BLANK_DTM]);
    refused("Insert", vec![BLANK_DT, datetime(1720, 5, 5, 12 * 3_600_000)]);
    refused(
        "Modify",
        vec![BLANK_DT, datetime(1752, 12, 31, 23 * 3_600_000 + 58 * 60_000)],
    );
}

#[test]
fn a_row_with_two_values_the_columns_cannot_hold_refuses() {
    refused(
        "Insert",
        vec![date(1753, 6, 15, false), datetime(1753, 6, 15, 3 * 3_600_000)],
    );
    refused(
        "Modify",
        vec![date(1753, 6, 15, false), datetime(1753, 6, 15, 3 * 3_600_000)],
    );
}

#[test]
fn a_record_alive_where_its_failed_rename_is_caught_refuses() {
    refused("RenameKey", vec![date(1753, 6, 15, false)]);
    refused("RenameKey", vec![date(1752, 12, 31, false)]);
}
