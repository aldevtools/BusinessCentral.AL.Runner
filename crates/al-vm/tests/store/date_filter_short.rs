//! Date filter texts of month and day only, impossible slash forms, DateTime texts, and `0D` in a query's static filters.
//! Every literal is what real BC answered in the corpus: filters/lt-y62-date-filter-short/Y62DateFilterShort.al
//! (codeunit 93260, writer Y62).

use al_testkit::text;

const SRC: &str = r#"
table 50196 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; D; Date) { }
        field(3; Dt; DateTime) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

query 50196 NotZero
{
    OrderBy = ascending(Id);
    elements
    {
        dataitem(Row; R)
        {
            DataItemTableFilter = D = filter(<> 0D);
            column(Id; Id) { }
        }
    }
}

query 50197 Zero
{
    OrderBy = ascending(Id);
    elements
    {
        dataitem(Row; R)
        {
            DataItemTableFilter = D = filter(= 0D);
            column(Id; Id) { }
        }
    }
}

query 50198 ZeroRange
{
    OrderBy = ascending(Id);
    elements
    {
        dataitem(Row; R)
        {
            DataItemTableFilter = D = filter(0D .. 20260101D);
            column(Id; Id) { }
        }
    }
}

query 50199 ColNotZero
{
    OrderBy = ascending(Id);
    elements
    {
        dataitem(Row; R)
        {
            column(Id; Id) { }
            column(D; D) { ColumnFilter = D = filter(<> 0D); }
        }
    }
}

codeunit 50196 C
{
    local procedure Seed()
    begin
        Add(1, 0D, 0DT);
        Add(2, 20260101D, CreateDateTime(20240101D, 103000T));
        Add(7, 20300128D, CreateDateTime(20300128D, 103000T));
        Add(8, 20310128D, CreateDateTime(20310128D, 103000T));
    end;

    local procedure Add(NewId: Integer; NewDate: Date; NewDt: DateTime)
    var
        Row: Record R;
    begin
        Row.Id := NewId;
        Row.D := NewDate;
        Row.Dt := NewDt;
        Row.Insert();
    end;

    [TryFunction]
    local procedure TryFilter(var Row: Record R; Dt: Boolean; Txt: Text)
    begin
        if Dt then
            Row.SetFilter(Dt, Txt)
        else
            Row.SetFilter(D, Txt);
    end;

    local procedure Apply(Dt: Boolean; Txt: Text): Text
    var
        Row: Record R;
        Out: Text;
    begin
        if not TryFilter(Row, Dt, Txt) then
            exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
        if Dt then
            Out := Row.GetFilter(Dt)
        else
            Out := Row.GetFilter(D);
        Out += '|rows=';
        if Row.FindSet() then
            repeat
                Out += Format(Row.Id) + ',';
            until Row.Next() = 0;
        exit(Out);
    end;

    procedure Record(): Text
    begin
        Seed();
        WorkDate(20300615D);
        exit(Apply(false, '1/28') + ';' + Apply(false, '1-28') + ';' + Apply(false, '1.28') + ';' + Apply(false, '>1/28') + ';'
            + Apply(false, '1/1/0') + ';' + Apply(false, '28/1') + ';' + Apply(false, '02/30/2026') + ';' + Apply(false, '99/99') + ';'
            + Apply(true, '2024-01-01 10:30:00') + ';' + Apply(true, '01/01/2024 10:30:00 AM') + ';' + Apply(true, '2024-01-01') + ';'
            + Apply(true, 'not a datetime'));
    end;

    [TryFunction]
    local procedure TryRange(var Txt: Text)
    var
        Q: Query ZeroRange;
    begin
        Q.Open();
        while Q.Read() do
            Txt += Format(Q.Id) + ',';
    end;

    procedure Queries(): Text
    var
        N: Query NotZero;
        Z: Query Zero;
        C: Query ColNotZero;
        C2: Query ColNotZero;
        Txt: Text;
    begin
        Seed();
        N.Open();
        while N.Read() do
            Txt += Format(N.Id) + ',';
        Z.Open();
        Txt += '/';
        while Z.Read() do
            Txt += Format(Z.Id) + ',';
        C.Open();
        Txt += '/';
        while C.Read() do
            Txt += Format(C.Id) + ',';
        C2.SetRange(D, 0D);
        C2.Open();
        Txt += '/';
        while C2.Read() do
            Txt += Format(C2.Id) + ',';
        Txt += '/';
        if not TryRange(Txt) then
            Txt += GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Txt);
    end;
}
"#;

const INVALID: &str = " does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.";

/// SetFilter_Date_MonthDayOnly_WorkDate2030, SetFilter_Date_ImpossibleSlashForms, SetFilter_Date_ImpossibleShortForms and
/// SetFilter_DateTime_TextForms.
#[test]
fn month_day_texts_take_the_work_date_year_and_impossible_forms_raise() {
    let bad = |t: &str| {
        format!(
            "error:The filter \"{t}\" is not valid for the D field on the R table. The format of the Date '{t}'{INVALID}|Filter:CSide"
        )
    };
    let expected = [
        "01/28/30|rows=7,".to_string(),
        "01/28/30|rows=7,".to_string(),
        "01/28/30|rows=7,".to_string(),
        ">01/28/30|rows=8,".to_string(),
        "01/01/00|rows=".to_string(),
        bad("28/1"),
        bad("02/30/2026"),
        bad("99/99"),
        "01/01/24 10:30 AM|rows=2,".to_string(),
        "01/01/24 10:30 AM|rows=2,".to_string(),
        "01/01/24 12:00 AM..01/01/24 11:59:59 PM|rows=2,".to_string(),
        "error:The filter \"not a datetime\" is not valid for the Dt field on the R table. The value \"not a datetime\" can't be evaluated into type DateTime.|Filter:CSide".to_string(),
    ]
    .join(";");
    assert_eq!(text(al_testkit::run(SRC, "Record")), expected);
}

/// Query_DataItemTableFilter_NotZeroDate, Query_DataItemTableFilter_ZeroDateForms and Query_ColumnFilter_ZeroDate_AndRuntimeFilters:
/// `0D` in a static query filter is the empty date; a date literal such as `20260101D` there raises at Open.
#[test]
fn zero_date_in_a_static_query_filter_is_the_empty_date() {
    assert_eq!(
        text(al_testkit::run(SRC, "Queries")),
        format!("2,7,8,/1,/2,7,8,/1,/The format of the Date '20260101D'{INVALID}|InvalidFilterExpression:Evaluate")
    );
}
