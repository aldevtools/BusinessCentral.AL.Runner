//! NaviPartner NP Retail blockers: FieldRef.GetRangeMin/GetRangeMax and a SystemId set in OnInsert.

use al_builtins::native::Signal;
use al_testkit::text;

const T: &str = r#"
table 69980 "NB T" { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } keys { key(PK; K) { } } }
codeunit 69981 C
{
    procedure F(): Text
    var R: Record "NB T"; RR: RecordRef; FR: FieldRef; S: Text;
    begin
        RR.GetTable(R);
        FR := RR.Field(2);
        FR.SetRange(20, 40);
        S := Format(FR.GetRangeMin()) + '/' + Format(FR.GetRangeMax());
        FR.SetFilter('>20');
        S += ' ' + Format(FR.GetRangeMin());
        exit(S);
    end;
    procedure NoUpper(): Text
    var R: Record "NB T"; RR: RecordRef; FR: FieldRef;
    begin
        RR.GetTable(R);
        FR := RR.Field(2);
        FR.SetFilter('>20');
        exit(Format(FR.GetRangeMax()));
    end;
    procedure Many(): Text
    var R: Record "NB T"; RR: RecordRef; FR: FieldRef;
    begin
        RR.GetTable(R);
        FR := RR.Field(2);
        FR.SetFilter('10..20|40..50');
        exit(Format(FR.GetRangeMin()));
    end;
}
"#;

// Pins: corpus recordref/lt-refs/RfTFieldFilters.al:183, record/lt-filters-keys-finds/P68B_RangeMinMax.al.
#[test]
fn field_ref_range_bounds_follow_the_record_rules() {
    assert_eq!(text(al_testkit::run_in(T, "C", "F")), "20/40 20");
    for (routine, code, msg) in [
        (
            "NoUpper",
            "DB:Ok",
            "The filter \">20\" on the N field in the NB T table has no upper bound.",
        ),
        (
            "Many",
            "DB:NotIntervalFilter",
            "The filter expression \"10..20|40..50\" applied to the field N in the NB T table results in more than one range.",
        ),
    ] {
        match al_testkit::run_in(T, "C", routine) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.code, code);
                assert_eq!(e.message, msg);
            }
            other => panic!("{routine}: {other:?}"),
        }
    }
}

const SID: &str = r#"
table 69982 "NB S" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } }
    trigger OnInsert() var St: Codeunit "NB St"; begin if St.Mode() = 5 then SystemId := St.Wanted(); end; }
tableextension 69985 "NB S Ext" extends "NB S"
{
    trigger OnBeforeInsert() var St: Codeunit "NB St"; begin if St.Mode() = 1 then Rec.SystemId := St.Wanted(); end;
    trigger OnInsert() var St: Codeunit "NB St"; begin if St.Mode() = 2 then Rec.SystemId := St.Wanted(); end;
}
codeunit 69986 "NB St"
{
    SingleInstance = true;
    var M: Integer; W: Guid;
    procedure Arm(NewMode: Integer; NewWanted: Guid) begin M := NewMode; W := NewWanted; end;
    procedure Mode(): Integer begin exit(M); end;
    procedure Wanted(): Guid begin exit(W); end;
}
codeunit 69983 C
{
    var St: Codeunit "NB St";

    procedure Kept(Mode: Integer): Text
    var R: Record "NB S"; S: Record "NB S"; G: Guid; Empty: Guid;
    begin
        R.DeleteAll(); G := CreateGuid(); St.Arm(Mode, G);
        R.K := 1; R.Insert(true); St.Arm(0, Empty);
        S.Get(1);
        exit(Format(R.SystemId = G) + Format(S.SystemId = G));
    end;

    procedure KeptTemp(): Text
    var T: Record "NB S" temporary; G: Guid; Empty: Guid;
    begin
        G := CreateGuid(); St.Arm(2, G);
        T.K := 1; T.Insert(true); St.Arm(0, Empty);
        T.Get(1);
        exit(Format(T.SystemId = G));
    end;

    procedure Duplicate(Statement: Boolean): Text
    var R: Record "NB S"; F: Record "NB S"; Empty: Guid; Ok: Boolean;
    begin
        R.DeleteAll(); F.K := 1; F.Insert();
        St.Arm(5, F.SystemId);
        R.K := 2;
        if Statement then begin
            asserterror R.Insert(true);
            St.Arm(0, Empty);
            exit(GetLastErrorCode() + '|' + GetLastErrorText().Replace(Format(F.SystemId), '{id}'));
        end;
        Ok := R.Insert(true);
        St.Arm(0, Empty);
        exit(Format(Ok) + Format(R.SystemId = F.SystemId) + Format(R.Count()));
    end;
}
"#;

// record/lt-qr-systemid-insert-events/QrSystemIdTests.al:56-63 (a table extension's OnBeforeInsert), :71-78 (its OnInsert),
// record/lt-at-partner-gaps/AtSystemIdOnInsert.al:135-150 (the table's OnInsert): the id is kept in the variable and the stored row.
#[test]
fn a_system_id_set_in_an_insert_trigger_is_kept() {
    for mode in [1, 2, 5] {
        let r = al_testkit::run_with(SID, "C", "Kept", vec![al_builtins::value::Value::Integer(mode)]).map(|(v, _)| v);
        assert_eq!(text(r), "YesYes", "mode {mode}");
    }
}

// QrSystemIdTests.al:180-196: also on a temporary record.
#[test]
fn a_system_id_set_in_an_insert_trigger_of_a_temporary_record_is_kept() {
    assert_eq!(text(al_testkit::run_in(SID, "C", "KeptTemp")), "Yes");
}

// QrSystemIdTests.al:213-232 (Insert with a return value answers false, the caller does not hold the id, no row),
// :251-273 (the statement form raises DB:RecordExists).
#[test]
fn a_duplicate_system_id_set_in_on_insert_fails_the_insert() {
    let r =
        al_testkit::run_with(SID, "C", "Duplicate", vec![al_builtins::value::Value::Boolean(false)]).map(|(v, _)| v);
    assert_eq!(text(r), "NoNo1");
    let r = al_testkit::run_with(SID, "C", "Duplicate", vec![al_builtins::value::Value::Boolean(true)]).map(|(v, _)| v);
    assert_eq!(
        text(r),
        "DB:RecordExists|There is already a record in table NB S that has the same values in a unique index for the following fields: System ID='{id}'"
    );
}

const ISO: &str = r#"
codeunit 69984 C
{
    procedure F(): Text
    var G: Guid; V: Text;
    begin
        Evaluate(G, '{6F2C3D4E-5B6C-4D7E-9F80-AB1C2D3E4F50}');
        IsolatedStorage.Set(G, 'x', DataScope::Company);
        IsolatedStorage.Get(Format(G), DataScope::Company, V);
        exit(V + Format(IsolatedStorage.Contains(G, DataScope::Company)));
    end;
}
"#;

// session/lt-isolated-storage-guid/IsoStorageGuidKey.al: a Guid key is its Format text.
#[test]
fn an_isolated_storage_guid_key_is_its_text() {
    assert_eq!(text(al_testkit::run_in(ISO, "C", "F")), "xYes");
}
