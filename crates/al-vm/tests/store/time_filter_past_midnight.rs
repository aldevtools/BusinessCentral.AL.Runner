//! A Time of 23:59:59.999 in a database table rounds past the day in SQL: a row of the field shape
//! record/filters/TimeRolledPastMidnight.al:11-25 measures reads 00:00:00 but filters, counts and deletes after every time of the
//! day, and so does a filter value of 23:59:59.999. Other field shapes keep main's rounding to 00:00:00.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69970 "TF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tm; Time) { }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; Id) { } key(TmKey; Tm) { } }
}
table 69972 "TF Key"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Tm; Time) { }
    }
    keys { key(PK; Code, Tm) { } }
}
table 69973 "TF Sum"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; MaxTm; Time) { FieldClass = FlowField; CalcFormula = max("TF Row".Tm); }
        field(3; MinTm; Time) { FieldClass = FlowField; CalcFormula = min("TF Row".Tm); }
        field(4; LookupTm; Time) { FieldClass = FlowField; CalcFormula = lookup("TF Row".Tm where(Id = field(Id))); }
    }
    keys { key(PK; Id) { } }
}
table 69974 "TF Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tm; Time) { }
        field(3; Tm2; Time) { }
    }
    keys { key(PK; Id) { } key(Both; Tm2, Id) { } }
}
table 69975 "TF Trig"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tm; Time) { }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; Id) { } key(TmKey; Tm) { } }
    trigger OnModify() begin end;
}
table 69976 "TF Base"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
tableextension 69977 "TF Ext" extends "TF Base"
{
    fields { field(69977; Tm; Time) { } field(69978; Note; Text[30]) { } }
    keys { key(TmKey; Tm) { } }
}
table 69978 "TF Sub"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tm; Time) { }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; Id) { } key(TmKey; Tm) { } }
}
codeunit 69979 "TF Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"TF Sub", 'OnBeforeModifyEvent', '', false, false)]
    local procedure OnBeforeModify(var Rec: Record "TF Sub")
    begin
    end;
}
table 69980 "TF Ext Key Base"
{
    fields { field(1; Id; Integer) { } field(2; Tm; Time) { } }
    keys { key(PK; Id) { } }
}
tableextension 69981 "TF Ext Key" extends "TF Ext Key Base"
{
    keys { key(TmKeyExt; Tm) { } }
}
table 69982 "TF Key Props"
{
    fields { field(1; Id; Integer) { } field(2; Tm; Time) { } }
    keys { key(PK; Id) { } key(TmKey; Tm) { MaintainSQLIndex = false; } }
}
table 69983 "TF Count"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cnt0; Integer) { FieldClass = FlowField; CalcFormula = count("TF Row" where(Tm = const(000000T))); }
        field(3; TmFilter; Time) { FieldClass = FlowFilter; }
        field(4; CntF; Integer) { FieldClass = FlowField; CalcFormula = count("TF Row" where(Tm = field(TmFilter))); }
        field(5; CntId; Integer) { FieldClass = FlowField; CalcFormula = count("TF Row" where(Id = filter(1..3))); }
    }
    keys { key(PK; Id) { } }
}
query 69984 "TF Q"
{
    elements
    {
        dataitem(R; "TF Row")
        {
            column(Id; Id) { }
            column(Tm; Tm) { }
        }
    }
}
codeunit 69971 C
{
    procedure ModifyWithoutRead()
    var R: Record "TF Row";
    begin
        Seed();
        R.Init(); R.Id := 4; R.Tm := 000000T; R.Modify();
    end;

    procedure ModifyAfterReadingAnotherRow()
    var R: Record "TF Row"; R2: Record "TF Row";
    begin
        Seed();
        R2.Get(1); R := R2; R.Id := 4; R.Note := 'n'; R.Modify();
    end;

    procedure ModifyTwiceAfterOneGet()
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(4); R.Note := 'a'; R.Modify(); R.Note := 'b'; R.Modify();
    end;

    procedure ModifyAfterGetAndInit()
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(4); R.Init(); R.Tm := 000000T; R.Modify();
    end;

    procedure ModifyAfterGetOfACopy()
    var R: Record "TF Row"; R2: Record "TF Row";
    begin
        Seed();
        R2.Get(4); R := R2; R.Note := 'n'; R.Modify();
    end;

    procedure ModifyAfterSetTable()
    var R: Record "TF Row"; RR: RecordRef;
    begin
        Seed();
        R.Get(4); RR.GetTable(R); RR.SetTable(R); R.Note := 'n'; R.Modify();
    end;

    procedure ModifyOfAByValueCopy()
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(4); ModifyCopy(R);
    end;

    local procedure ModifyCopy(R: Record "TF Row")
    begin
        R.Note := 'n'; R.Modify();
    end;

    procedure ModifyWithoutReadWritesAnotherTime(): Text
    var R: Record "TF Row";
    begin
        Seed();
        R.Init(); R.Id := 4; R.Tm := 120000T; R.Modify();
        R.Reset(); R.SetRange(Tm, 120000T); exit(Ids(R));
    end;

    procedure QueryFilterEq0(): Text
    var Q: Query "TF Q"; s: Text;
    begin
        Seed();
        Q.SetRange(Tm, 000000T); Q.Open();
        while Q.Read() do s += Format(Q.Id);
        exit(s);
    end;

    procedure QueryFilterLt999(): Text
    var Q: Query "TF Q"; s: Text;
    begin
        Seed();
        Q.SetFilter(Tm, '<%1', 235959.999T); Q.Open();
        while Q.Read() do s += Format(Q.Id);
        exit(s);
    end;

    procedure QueryFilterOnId(): Text
    var Q: Query "TF Q"; s: Text;
    begin
        Seed();
        Q.SetRange(Id, 1, 3); Q.Open();
        while Q.Read() do s += Format(Q.Id) + ':' + Format(Q.Tm, 0, 9) + ';';
        exit(s);
    end;

    procedure CalcFormulaConst(): Text
    var S: Record "TF Count";
    begin
        Seed();
        S.Id := 1; S.Insert(); S.CalcFields(Cnt0);
        exit(Format(S.Cnt0));
    end;

    procedure FlowFilterEq0(): Text
    var S: Record "TF Count";
    begin
        Seed();
        S.Id := 1; S.Insert(); S.SetRange(TmFilter, 000000T); S.CalcFields(CntF);
        exit(Format(S.CntF));
    end;

    procedure CalcFormulaOnId(): Text
    var S: Record "TF Count";
    begin
        Seed();
        S.Id := 1; S.Insert(); S.CalcFields(CntId);
        exit(Format(S.CntId));
    end;

    procedure RenameWrites999()
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(2); R.Tm := 235959.999T; R.Rename(20);
    end;

    procedure RenameWritesAnotherTime(): Text
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(2); R.Tm := 130000T; R.Rename(20);
        R.Reset(); R.SetRange(Tm, 130000T); exit(Ids(R));
    end;

    procedure ExtensionKey(): Text
    var E: Record "TF Ext Key Base"; s: Text;
    begin
        E.Id := 1; E.Tm := 000000T; E.Insert(); E.Id := 4; E.Tm := 235959.999T; E.Insert();
        E.SetRange(Tm, 235959.999T); s := 'eq999=' + Format(E.Count());
        E.SetRange(Tm, 000000T); s += ';eq0=' + Format(E.Count());
        E.SetFilter(Tm, '<%1', 235959.999T); exit(s + ';lt999=' + Format(E.Count()));
    end;

    procedure KeyWithProperties(): Text
    var E: Record "TF Key Props"; s: Text;
    begin
        E.Id := 1; E.Tm := 000000T; E.Insert(); E.Id := 4; E.Tm := 235959.999T; E.Insert();
        E.SetRange(Tm, 235959.999T); s := 'eq999=' + Format(E.Count());
        E.SetRange(Tm, 000000T); s += ';eq0=' + Format(E.Count());
        E.SetFilter(Tm, '<%1', 235959.999T); exit(s + ';lt999=' + Format(E.Count()));
    end;

    procedure PlainFields(): Text
    var R: Record "TF Plain"; s: Text;
    begin
        R.Id := 1; R.Tm := 000000T; R.Tm2 := 000000T; R.Insert();
        R.Id := 4; R.Tm := 235959.999T; R.Tm2 := 235959.999T; R.Insert();
        R.SetRange(Tm, 000000T); s += 'tm_eq0=' + Format(R.Count()) + ';';
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'tm_lt999=' + Format(R.Count()) + ';';
        R.Reset();
        R.SetRange(Tm2, 000000T); s += 'tm2_eq0=' + Format(R.Count()) + ';';
        R.SetFilter(Tm2, '<%1', 235959.999T); exit(s + 'tm2_lt999=' + Format(R.Count()));
    end;

    procedure ExtensionField(): Text
    var R: Record "TF Base"; s: Text;
    begin
        R.Id := 1; R.Tm := 000000T; R.Insert();
        R.Id := 4; R.Tm := 235959.999T; R.Insert();
        R.SetRange(Tm, 000000T); s += 'eq0=' + Format(R.Count()) + ';';
        R.SetFilter(Tm, '<%1', 235959.999T); exit(s + 'lt999=' + Format(R.Count()));
    end;

    procedure ModifyTrueWithTrigger()
    var R: Record "TF Trig";
    begin
        R.Id := 4; R.Tm := 235959.999T; R.Insert();
        R.Get(4); R.Note := 'x'; R.Modify(true);
    end;

    procedure ModifyWithSubscriber()
    var R: Record "TF Sub";
    begin
        R.Id := 4; R.Tm := 235959.999T; R.Insert();
        R.Get(4); R.Note := 'x'; R.Modify();
    end;

    procedure ModifyFalseWithTrigger(): Text
    var R: Record "TF Trig";
    begin
        R.Id := 4; R.Tm := 235959.999T; R.Insert();
        R.Get(4); R.Note := 'x'; R.Modify(false);
        R.Reset(); R.SetRange(Tm, 235959.999T); exit(Format(R.Count()));
    end;

    procedure Filters(): Text
    var R: Record "TF Row"; s: Text;
    begin
        Ins(1, 000000T); Ins(2, 120000T); Ins(3, 235959.997T); Ins(5, 000000.001T);
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'lt999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<=%1', 235959.999T); s += 'le999=' + Ids(R) + ';';
        R.SetFilter(Tm, '>%1', 235959.999T); s += 'gt999=' + Ids(R) + ';';
        R.SetFilter(Tm, '>=%1', 235959.999T); s += 'ge999=' + Ids(R) + ';';
        R.SetRange(Tm, 235959.999T); s += 'eq999=' + Ids(R) + ';';
        R.SetRange(Tm, 000000T, 235959.999T); s += 'rng0to999=' + Ids(R) + ';';
        R.SetRange(Tm, 120000T, 235959.999T); s += 'rng12to999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<>%1', 235959.999T); s += 'ne999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<%1', 235959.998T); s += 'lt998=' + Ids(R) + ';';
        R.SetFilter(Tm, '>%1', 000000T); s += 'gt0=' + Ids(R) + ';';
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'filters=' + R.GetFilters();
        exit(s);
    end;

    procedure FiltersWithRolledOverRow(): Text
    begin
        Ins(4, 235959.999T);
        exit(Filters());
    end;

    procedure ReadCountDelete(): Text
    var R: Record "TF Row"; s: Text;
    begin
        Seed();
        if R.FindSet() then
            repeat
                s += Format(R.Id) + ':' + Format(R.Tm, 0, 9) + ';';
            until R.Next() = 0;
        R.SetRange(Tm, 000000T); s += 'eq0=' + Ids(R) + '/' + Format(R.Count()) + ';';
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'lt999count=' + Format(R.Count()) + ';';
        R.DeleteAll();
        R.Reset();
        exit(s + 'left=' + Ids(R));
    end;

    procedure ModifyKeeps(): Text
    var R: Record "TF Row"; s: Text;
    begin
        Seed();
        R.Get(4); R.Note := 'x'; R.Modify();
        R.Reset(); R.SetRange(Tm, 235959.999T); s += 'note=' + Ids(R) + ';';
        R.Reset(); R.Get(4); R.Tm := 000000T; R.Modify();
        R.Reset(); R.SetRange(Tm, 235959.999T); s += 'midnight=' + Ids(R) + ';';
        R.Reset(); R.Get(2); R.Tm := 235959.999T; R.Modify();
        R.Reset(); R.SetRange(Tm, 235959.999T); s += 'set=' + Ids(R) + ';';
        R.Get(2); s += 'read2=' + Format(R.Tm, 0, 9) + ';';
        R.Reset(); R.SetRange(Id, 1); R.ModifyAll(Tm, 235959.999T);
        R.Reset(); R.SetRange(Tm, 235959.999T); s += 'modifyall=' + Ids(R) + ';';
        R.Reset(); R.ModifyAll(Note, 'y');
        R.SetRange(Tm, 235959.999T); s += 'modifyallnote=' + Ids(R) + ';';
        exit(s);
    end;

    procedure FlowFields(): Text
    var S: Record "TF Sum";
    begin
        Seed();
        S.Id := 4; S.Insert();
        S.CalcFields(MaxTm, MinTm, LookupTm);
        exit(Format(S.MaxTm, 0, 9) + ';' + Format(S.MinTm, 0, 9) + ';' + Format(S.LookupTm, 0, 9));
    end;

    procedure Temporary(): Text
    var R: Record "TF Row" temporary; s: Text;
    begin
        R.Id := 1; R.Tm := 000000T; R.Insert();
        R.Id := 4; R.Tm := 235959.999T; R.Insert();
        R.Get(4); s := Format(R.Tm, 0, 9) + ';';
        R.SetRange(Tm, 000000T); s += 'eq0=' + Ids(R) + ';';
        R.SetRange(Tm, 235959.999T); exit(s + 'eq999=' + Ids(R));
    end;

    procedure PrimaryKey(): Text
    var K: Record "TF Key"; G: Record "TF Key"; s: Text;
    begin
        K.Code := 'A'; K.Tm := 235959.999T; K.Insert();
        s := Format(G.Get('A', 235959.999T)) + ';';
        Clear(G);
        s += Format(G.Get('A', 000000T)) + ';';
        K.SetRange(Tm, 235959.999T);
        exit(s + Format(K.Count()));
    end;

    procedure SortByTime()
    var R: Record "TF Row";
    begin
        Seed();
        R.SetCurrentKey(Tm);
        R.FindLast();
    end;

    procedure ModifyAllMidnight()
    var R: Record "TF Row";
    begin
        Seed();
        R.ModifyAll(Tm, 000000T);
    end;

    procedure RenameRolled()
    var R: Record "TF Row";
    begin
        Seed();
        R.Get(4);
        R.Rename(6);
    end;

    local procedure Seed()
    begin
        Ins(1, 000000T); Ins(2, 120000T); Ins(3, 235959.997T); Ins(4, 235959.999T); Ins(5, 000000.001T);
    end;

    local procedure Ins(K: Integer; T: Time)
    var R: Record "TF Row";
    begin
        R.Id := K; R.Tm := T; R.Insert();
    end;

    local procedure Ids(var R: Record "TF Row") s: Text
    begin
        if R.FindSet() then
            repeat
                s += Format(R.Id);
            until R.Next() = 0;
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

// TimeRolledPastMidnight.al:99-158 without row 4: 23:59:59.999 as a filter value compares after the day.
#[test]
fn time_filter_rounded_past_midnight_stays_after_the_day() {
    assert_eq!(
        text(run("Filters")),
        "lt999=1235;le999=1235;gt999=;ge999=;eq999=;rng0to999=1235;rng12to999=23;ne999=1235;lt998=125;gt0=23;\
         filters=Tm: <11:59:59.999 PM"
    );
}

// TimeRolledPastMidnight.al:99-158: row 4 holds 23:59:59.999 and is filtered after the other rows.
#[test]
fn time_filter_rounded_past_midnight_selects_a_stored_rollover() {
    assert_eq!(
        text(run("FiltersWithRolledOverRow")),
        "lt999=1235;le999=12345;gt999=;ge999=4;eq999=4;rng0to999=12345;rng12to999=234;ne999=1235;lt998=125;gt0=234;\
         filters=Tm: <11:59:59.999 PM"
    );
}

// TimeRolledPastMidnight.al:142 `read`, :150 `eq0`, :186-199 DeleteAll: the row reads 00:00:00 and is not midnight.
#[test]
fn a_stored_rollover_reads_midnight_and_counts_after_the_day() {
    assert_eq!(
        text(run("ReadCountDelete")),
        "1:00:00:00;2:12:00:00;3:23:59:59.997;4:00:00:00;5:00:00:00;eq0=15/2;lt999count=4;left=4"
    );
}

// TimeRolledPastMidnight.al:202-254: a Modify that leaves the field as read keeps the rollover.
#[test]
fn a_modify_keeps_a_stored_rollover_it_did_not_change() {
    assert_eq!(
        text(run("ModifyKeeps")),
        "note=4;midnight=4;set=24;read2=00:00:00;modifyall=124;modifyallnote=124;"
    );
}

// TimeRolledPastMidnight.al:362-364: max, min and lookup read 00:00:00.
#[test]
fn flowfields_over_a_stored_rollover_read_midnight() {
    assert_eq!(text(run("FlowFields")), "00:00:00;00:00:00;00:00:00");
}

// TimeRolledPastMidnight.al:289-292: a temporary table keeps 23:59:59.999.
#[test]
fn a_temporary_table_keeps_the_milliseconds() {
    assert_eq!(text(run("Temporary")), "23:59:59.999;eq0=1;eq999=4");
}

// P130DateFilters.al:614-616 and TimeRolledPastMidnight.al:331-332: a primary key keeps main's behavior.
#[test]
fn a_primary_key_rollover_is_found_by_both_values() {
    assert_eq!(text(run("PrimaryKey")), "Yes;Yes;1");
}

// Not modelled or not probed: a read in the order of a key holding the field, ModifyAll of 00:00:00 over it, a Rename.
#[test]
fn unmodelled_rollover_shapes_are_refused() {
    for routine in ["SortByTime", "ModifyAllMidnight", "RenameRolled"] {
        let r = run(routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}

// TimeRolledPastMidnight.al:17, :23: a Time field without a secondary key of its own keeps main's rounding to 00:00:00.
#[test]
fn a_time_field_without_a_key_of_its_own_keeps_main_rounding() {
    assert_eq!(text(run("PlainFields")), "tm_eq0=2;tm_lt999=0;tm2_eq0=2;tm2_lt999=0");
}

// TimeRolledPastMidnight.al:17: a Time field a table extension declares keeps main's rounding to 00:00:00.
#[test]
fn a_table_extension_time_field_keeps_main_rounding() {
    assert_eq!(text(run("ExtensionField")), "eq0=2;lt999=0");
}

// TimeRolledPastMidnight.al:12-25, :203-262 measure a Modify with no trigger code or subscriber around it.
#[test]
fn a_modify_of_a_rolled_row_with_trigger_code_or_a_subscriber_is_refused() {
    for routine in ["ModifyTrueWithTrigger", "ModifyWithSubscriber"] {
        let r = run(routine);
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("trigger code or subscribers")),
            "{routine}: {r:?}"
        );
    }
}

// The same row and trigger with Modify(false): no code runs, as at TimeRolledPastMidnight.al:212, and the row keeps the rollover.
#[test]
fn a_modify_false_of_a_rolled_row_runs_as_pinned() {
    assert_eq!(text(run("ModifyFalseWithTrigger")), "1");
}

fn refused(routine: &str, why: &str) {
    let r = run(routine);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains(why)),
        "{routine}: {r:?}"
    );
}

// TimeRolledPastMidnight.al:210-212, :221-223 keep the rollover only after a Get of the same row. A Modify of 00:00:00 over it by a
// record that did not read the row since (Init, a copy of another row's or of another variable's read, a second Modify, a builtin
// that wrote the buffer, a by-value copy) is not measured.
#[test]
fn a_modify_of_midnight_over_a_rolled_row_without_its_own_read_is_refused() {
    for routine in [
        "ModifyWithoutRead",
        "ModifyAfterReadingAnotherRow",
        "ModifyTwiceAfterOneGet",
        "ModifyAfterGetAndInit",
        "ModifyAfterGetOfACopy",
        "ModifyAfterSetTable",
        "ModifyOfAByValueCopy",
    ] {
        refused(routine, "did not read it");
    }
}

// A Modify without a read that writes another time over the rolled row keeps main's answer.
#[test]
fn a_modify_without_a_read_that_writes_another_time_keeps_main() {
    assert_eq!(text(run("ModifyWithoutReadWritesAnotherTime")), "24");
}

// The pin measures the record's own filters and FieldRef filters (TimeRolledPastMidnight.al:99-159, :369-376), not a query's
// filters, a CalcFormula const or a FlowFilter on the field.
#[test]
fn a_query_or_calcformula_filter_on_the_field_over_a_rolled_row_is_refused() {
    for routine in [
        "QueryFilterEq0",
        "QueryFilterLt999",
        "CalcFormulaConst",
        "FlowFilterEq0",
    ] {
        refused(routine, "filter on a Time field over a row that holds");
    }
}

// A query or a CalcFormula that filters on another field and does not reach the rolled row keeps main's answer.
#[test]
fn a_query_or_calcformula_filter_on_another_field_keeps_main() {
    assert_eq!(text(run("QueryFilterOnId")), "1:00:00:00;2:12:00:00;3:23:59:59.997;");
    assert_eq!(text(run("CalcFormulaOnId")), "3");
}

// The pin measures the rollover written by Insert, Modify and ModifyAll (TimeRolledPastMidnight.al:203-262), not by a Rename.
#[test]
fn a_rename_that_writes_a_rollover_is_refused() {
    refused("RenameWrites999", "a Rename that writes a Time of 23:59:59.999");
}

// A Rename that writes another time keeps main's answer.
#[test]
fn a_rename_that_writes_another_time_keeps_main() {
    assert_eq!(text(run("RenameWritesAnotherTime")), "20");
}

// TimeRolledPastMidnight.al:23: the key on the field is the table's own and has no properties. A key a table extension adds, or
// one with properties, keeps main's rounding to 00:00:00.
#[test]
fn a_key_of_a_table_extension_or_with_properties_keeps_main_rounding() {
    assert_eq!(text(run("ExtensionKey")), "eq999=2;eq0=2;lt999=0");
    assert_eq!(text(run("KeyWithProperties")), "eq999=2;eq0=2;lt999=0");
}
