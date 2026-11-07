//! A quoted empty text as the filter value on a Time field is the undefined Time 0T, with every operator, in a range, in an OR
//! list and after `<>` (al-runner issue 6900). Real BC's values: corpus master 3eb812ed,
//! record/filters/TimeFilterEmptyText.al:104-128 (SetFilter_TimeField_QuotedEmptyText), BC 27.5, 28.5 and 29.0.

use al_testkit::text;

const SRC: &str = r#"
table 50100 "Tf Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(9; "Time Field"; Time) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 50101 C
{
    var Row: Record "Tf Row";

    [TryFunction]
    local procedure TryFilter(Filter: Text; WithArg: Boolean; var Result: Text)
    begin
        if WithArg then
            Row.SetFilter("Time Field", Filter, 0T)
        else
            Row.SetFilter("Time Field", Filter);
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
        Row."Entry No." := 1; Row."Time Field" := 100000T; Row.Insert();
        Row.Init(); Row."Entry No." := 2; Row."Time Field" := 0T; Row.Insert();
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
        Row.SetRange("Time Field", 0T);
        T += Format(Row.Count()) + ' | ' + Row.GetFilters();
        exit(T);
    end;
}
"#;

#[test]
fn a_quoted_empty_text_filters_a_time_field_by_the_undefined_time() {
    let want = [
        "[1 | Time Field: <>'']",
        "[2 | Time Field: '']",
        "[2 | Time Field: '']",
        "[0 | Time Field: <'']",
        "[1 | Time Field: >'']",
        "[2 | Time Field: <='']",
        "[3 | Time Field: >='']",
        "[3 | Time Field: ''..]",
        "[2 | Time Field: ..'']",
        "[2 | Time Field: ''|'']",
        "[1 | Time Field: <>'']",
        "error: The filter \"<>\" is not valid for the Time Field field on the Tf Row table. A value is expected after the '<>'. | Filter:CSide",
        "[1 | Time Field: <>''&<>'']",
        "2 | Time Field: ''",
    ];
    let got = text(al_testkit::run_in(SRC, "C", "Run"));
    let got: Vec<&str> = got.split('/').collect();
    assert_eq!(got, want);
}
