//! A quoted empty text alone as a filter value on a DateTime field shows in GetFilters as the range `''..''`; after an operator, as a
//! range end and given as %1 it shows as `''`. Real BC's values: corpus master 3eb812ed,
//! record/filters/TimeFilterEmptyText.al:153-173 (SetFilter_DateTimeField_QuotedEmptyText), BC 27.5, 28.5 and 29.0.

use al_testkit::text;

const SRC: &str = r#"
table 50102 "Dtf Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(10; "DateTime Field"; DateTime) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 50103 C
{
    var Row: Record "Dtf Row";

    [TryFunction]
    local procedure TryFilter(Filter: Text; WithArg: Boolean; var Result: Text)
    begin
        if WithArg then
            Row.SetFilter("DateTime Field", Filter, 0DT)
        else
            Row.SetFilter("DateTime Field", Filter);
        Result := Format(Row.Count()) + ' | ' + Row.GetFilters();
    end;

    local procedure Probe(Filter: Text; WithArg: Boolean): Text
    var Result: Text;
    begin
        Row.Reset();
        if TryFilter(Filter, WithArg, Result) then begin
            Row.Reset();
            exit('[' + Result + ']');
        end;
        Row.Reset();
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
    end;

    procedure Run(): Text
    var T: Text;
    begin
        Row.DeleteAll();
        Row."Entry No." := 1; Row."DateTime Field" := CreateDateTime(20240305D, 100000T); Row.Insert();
        Row.Init(); Row."Entry No." := 2; Row."DateTime Field" := 0DT; Row.Insert();
        Row.Init(); Row."Entry No." := 3; Row.Insert();
        T += Probe('<>''''', false) + '/';
        T += Probe('''''', false) + '/';
        T += Probe('=''''', false) + '/';
        T += Probe('<''''', false) + '/';
        T += Probe('>''''', false) + '/';
        T += Probe('<=''''', false) + '/';
        T += Probe('>=''''', false) + '/';
        T += Probe('''''..', false) + '/';
        T += Probe('..''''', false) + '/';
        T += Probe('''''|%1', true) + '/';
        T += Probe('<>%1', true) + '/';
        T += Probe('<>', false) + '/';
        T += Probe('<>''''&<>%1', true) + '/';
        T += 'end';
        exit(T);
    end;
}
"#;

#[test]
fn a_quoted_empty_text_alone_on_a_datetime_field_shows_as_a_range() {
    let want = [
        "[1 | DateTime Field: <>'']",
        "[2 | DateTime Field: ''..'']",
        "[2 | DateTime Field: '']",
        "[0 | DateTime Field: <'']",
        "[1 | DateTime Field: >'']",
        "[2 | DateTime Field: <='']",
        "[3 | DateTime Field: >='']",
        "[3 | DateTime Field: ''..]",
        "[2 | DateTime Field: ..'']",
        "[2 | DateTime Field: ''..''|'']",
        "[1 | DateTime Field: <>'']",
        "error: The filter \"<>\" is not valid for the DateTime Field field on the Dtf Row table. A value is expected after the '<>'. | Filter:CSide",
        "[1 | DateTime Field: <>''&<>'']",
        "end",
    ];
    let got = text(al_testkit::run_in(SRC, "C", "Run"));
    let got: Vec<&str> = got.split('/').collect();
    assert_eq!(got, want);
}
