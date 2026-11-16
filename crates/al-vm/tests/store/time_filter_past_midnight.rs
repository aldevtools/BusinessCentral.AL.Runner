//! A Time of 23:59:59.999 in a database table rounds past the day in SQL: a row outside the primary key reads 00:00:00 but filters,
//! counts and deletes after every time of the day, and so does a filter value of 23:59:59.999. Assumption
//! `time-rolled-past-midnight`, corpus pull request 1364 (record/filters/TimeRolledPastMidnight.al), measured on BC 28.5.

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
codeunit 69971 C
{
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
