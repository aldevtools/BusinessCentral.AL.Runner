//! Filter behaviors that external users reported as refused (AL Runner issues 5577 and 5578), each pinned by a corpus probe:
//! record/lt-bl-invalid-decimal-filter/InvalidDecimalFilter.al and record/lt-bl-wildcard-getfilter/WildcardGetFilter.al.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(src, "C", routine)
}

const BAD: &str = r#"
table 50400 "Bad Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Amount; Decimal) { }
        field(3; Big; BigInteger) { }
        field(4; Dur; Duration) { }
        field(6; Tm; Time) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50401 C
{
    [TryFunction]
    local procedure TrySet(var Row: Record "Bad Row" temporary; FieldNo: Integer; Filter: Text)
    begin
        case FieldNo of
            2: Row.SetFilter(Amount, Filter);
            3: Row.SetFilter(Big, Filter);
            4: Row.SetFilter(Dur, Filter);
            6: Row.SetFilter(Tm, Filter);
        end;
    end;

    local procedure Probe(FieldNo: Integer; Filter: Text): Text
    var
        Row: Record "Bad Row" temporary;
    begin
        ClearLastError();
        if TrySet(Row, FieldNo, Filter) then
            exit('ok [' + Row.GetFilters() + ']');
        exit('error [' + GetLastErrorText() + '] [' + GetLastErrorCode() + ']');
    end;

    procedure Repro(): Text
    var
        Row: Record "Bad Row" temporary;
    begin
        asserterror Row.SetFilter(Amount, 'NOT-A-NUMBER');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Forms(): Text
    var
        R: Text;
        F: Text;
    begin
        foreach F in 'abc#1..x#x..1#1|abc#>abc#<>abc#1;5#1,5#1e3#1.2.3#1..#..1#''abc''#*#-'.Split('#') do
            R += F + ' => ' + Probe(2, F) + '\';
        exit(R);
    end;

    procedure OtherTypes(): Text
    begin
        exit(Probe(3, 'NOT-A-NUMBER') + '\' + Probe(4, 'NOT-A-NUMBER') + '\' + Probe(6, 'NOT-A-NUMBER'));
    end;

    procedure Kept(): Text
    var
        Row: Record "Bad Row" temporary;
        R: Text;
    begin
        Row."No." := 1; Row.Amount := 5; Row.Insert();
        Row."No." := 2; Row.Amount := 7; Row.Insert();
        Row.SetFilter(Amount, '5');
        if TrySet(Row, 2, 'NOT-A-NUMBER') then R := 'ok' else R := 'error';
        exit(R + '|' + Row.GetFilter(Amount) + '|' + Format(Row.Count()));
    end;
}
"#;

fn msg(filter: &str, field: &str, value: &str, ty: &str) -> String {
    format!(
        "error [The filter \"{filter}\" is not valid for the {field} field on the Bad Row table. The value \"{value}\" can't be evaluated into type {ty}.] [Filter:CSide]"
    )
}

/// InvalidDecimalFilter.al:66 (Repro_InvalidDecimalFilter): a Decimal filter of a text with hyphens raises the evaluate error.
#[test]
fn invalid_decimal_filter_with_hyphens_raises_the_evaluate_error() {
    assert_eq!(
        text(run(BAD, "Repro")),
        "The filter \"NOT-A-NUMBER\" is not valid for the Amount field on the Bad Row table. The value \"NOT-A-NUMBER\" can't be evaluated into type Decimal.|Filter:CSide"
    );
}

/// InvalidDecimalFilter.al:92-126 (Decimal_Forms): which texts raise the error, which value it names, and the forms that read.
#[test]
fn decimal_filter_forms_match_the_corpus() {
    let d = |f: &str, v: &str| msg(f, "Amount", v, "Decimal");
    let want = [
        format!("abc => {}", d("abc", "abc")),
        format!("1..x => {}", d("1..x", "x")),
        format!("x..1 => {}", d("x..1", "x")),
        format!("1|abc => {}", d("1|abc", "abc")),
        format!(">abc => {}", d(">abc", "abc")),
        format!("<>abc => {}", d("<>abc", "abc")),
        format!("1;5 => {}", d("1;5", "1;5")),
        "1,5 => ok [Amount: 15]".to_string(),
        "1e3 => ok [Amount: 1,000]".to_string(),
        format!("1.2.3 => {}", d("1.2.3", "1.2.3")),
        "1.. => ok [Amount: 1..]".to_string(),
        "..1 => ok [Amount: ..1]".to_string(),
        format!("'abc' => {}", d("'abc'", "abc")),
        format!("* => {}", d("*", "*")),
        format!("- => {}", d("-", "-")),
    ];
    let got = text(run(BAD, "Forms"));
    let got: Vec<&str> = got.split('\\').filter(|s| !s.is_empty()).collect();
    assert_eq!(got, want.iter().map(String::as_str).collect::<Vec<_>>());
}

/// InvalidDecimalFilter.al:130-145 (Other_Types): the same text on BigInteger, Duration and Time fields.
#[test]
fn not_a_number_on_other_number_types() {
    assert_eq!(
        text(run(BAD, "OtherTypes")),
        [
            msg("NOT-A-NUMBER", "Big", "NOT-A-NUMBER", "BigInteger"),
            msg("NOT-A-NUMBER", "Dur", "NOT-A-NUMBER", "Duration"),
            msg("NOT-A-NUMBER", "Tm", "NOT-A-NUMBER", "Time"),
        ]
        .join("\\")
    );
}

/// InvalidDecimalFilter.al:148-170 (FilterKept_AfterError): the filter in force before the error stays.
#[test]
fn the_filter_before_the_error_stays() {
    assert_eq!(text(run(BAD, "Kept")), "error|5|1");
}

const WILD: &str = r#"
table 50410 "Wild Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "No.") { } }
}
codeunit 50411 C
{
    local procedure Seed(var Row: Record "Wild Row" temporary)
    begin
        Row.Reset(); Row.DeleteAll();
        Add(Row, '', ''); Add(Row, 'A1', 'apple'); Add(Row, 'AB', 'Avocado'); Add(Row, 'B1', 'banana'); Add(Row, 'XA', 'xa');
    end;

    local procedure Add(var Row: Record "Wild Row" temporary; No: Code[20]; Name: Text[50])
    begin
        Row.Init(); Row."No." := No; Row.Name := Name; Row.Insert();
    end;

    local procedure Kept(var Row: Record "Wild Row" temporary) Result: Text
    begin
        Result := '{';
        if Row.FindSet() then
            repeat
                Result += '[' + Row."No." + ']';
            until Row.Next() = 0;
        Result += '}';
    end;

    procedure Repro(): Text
    var
        Row: Record "Wild Row" temporary;
    begin
        Row.SetFilter("No.", '<>*');
        exit(Row.GetFilter("No.") + '|' + Format(Row.GetFilter("No.") = '<>*'));
    end;

    procedure CodeForms(): Text
    var
        Row: Record "Wild Row" temporary;
        F: Text;
        R: Text;
    begin
        Seed(Row);
        foreach F in '<>*#*#<>A*#*A*#<>*A*#A*|B*#<>A1&<>B1#?#<>?#??#@*a*#<>''''&<>*#a*'.Split('#') do begin
            Row.Reset();
            Row.SetFilter("No.", F);
            R += F + ' => ' + Row.GetFilter("No.") + ' | ' + Row.GetFilters() + ' | ' + Kept(Row) + '\';
        end;
        exit(R);
    end;

    procedure TextForms(): Text
    var
        Row: Record "Wild Row" temporary;
        F: Text;
        R: Text;
    begin
        Seed(Row);
        foreach F in '<>*#*#<>a*#*a*#<>*a*#a*|b*#<>apple&<>xa#??#<>??#@*A*#<>@a*#<>''''&<>*'.Split('#') do begin
            Row.Reset();
            Row.SetFilter(Name, F);
            R += F + ' => ' + Row.GetFilter(Name) + ' | ' + Row.GetFilters() + ' | ' + Kept(Row) + '\';
        end;
        exit(R);
    end;

    procedure Placeholders(): Text
    var
        Row: Record "Wild Row" temporary;
        R: Text;
    begin
        Seed(Row);
        Row.SetFilter("No.", '<>%1', '*');
        R += Row.GetFilter("No.") + ' | ' + Kept(Row) + '\';
        Row.Reset();
        Row.SetFilter("No.", '%1', 'A*');
        R += Row.GetFilter("No.") + ' | ' + Kept(Row) + '\';
        Row.Reset();
        Row.SetFilter("No.", '<>%1*', 'A');
        R += Row.GetFilter("No.") + ' | ' + Kept(Row) + '\';
        Row.Reset();
        Row.SetFilter(Name, '<>%1', '*');
        R += Row.GetFilter(Name) + ' | ' + Kept(Row) + '\';
        Row.Reset();
        Row.SetFilter(Name, '%1&<>%2', '*a*', 'xa');
        R += Row.GetFilter(Name) + ' | ' + Kept(Row);
        exit(R);
    end;

    procedure View(): Text
    var
        Row: Record "Wild Row" temporary;
        Copy: Record "Wild Row" temporary;
        R: Text;
    begin
        Row.SetFilter("No.", '<>*');
        R := Row.GetView(false);
        Copy.SetView(Row.GetView(false));
        R += '|' + Copy.GetFilter("No.");
        Copy.Reset();
        Copy.CopyFilters(Row);
        exit(R + '|' + Copy.GetFilter("No."));
    end;
}
"#;

fn lines(v: Result<Value, Signal>) -> Vec<String> {
    text(v)
        .split('\\')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:92 (Repro_NotWildcard_RoundTrips): `<>*` reads back as written.
#[test]
fn not_wildcard_filter_reads_back() {
    assert_eq!(text(run(WILD, "Repro")), "<>*|Yes");
}

/// WildcardGetFilter.al:106-139 (Forms_CodeField): GetFilter, GetFilters and the rows kept on a Code field.
#[test]
fn wildcard_forms_on_a_code_field() {
    assert_eq!(
        lines(run(WILD, "CodeForms")),
        [
            "<>* => <>* | No.: <>* | {}",
            "* => * | No.: * | {[][A1][AB][B1][XA]}",
            "<>A* => <>A* | No.: <>A* | {[][B1][XA]}",
            "*A* => *A* | No.: *A* | {[A1][AB][XA]}",
            "<>*A* => <>*A* | No.: <>*A* | {[][B1]}",
            "A*|B* => A*|B* | No.: A*|B* | {[A1][AB][B1]}",
            "<>A1&<>B1 => <>A1&<>B1 | No.: <>A1&<>B1 | {[][AB][XA]}",
            "? => ? | No.: ? | {}",
            "<>? => <>? | No.: <>? | {[][A1][AB][B1][XA]}",
            "?? => ?? | No.: ?? | {[A1][AB][B1][XA]}",
            "@*a* => @*A* | No.: @*A* | {[A1][AB][XA]}",
            "<>''&<>* => <>''&<>* | No.: <>''&<>* | {}",
            "a* => A* | No.: A* | {[A1][AB]}",
        ]
    );
}

/// WildcardGetFilter.al:142-173 (Forms_TextField): the same on a Text field, case-sensitive unless `@`.
#[test]
fn wildcard_forms_on_a_text_field() {
    assert_eq!(
        lines(run(WILD, "TextForms")),
        [
            "<>* => <>* | Name: <>* | {}",
            "* => * | Name: * | {[][A1][AB][B1][XA]}",
            "<>a* => <>a* | Name: <>a* | {[][AB][B1][XA]}",
            "*a* => *a* | Name: *a* | {[A1][AB][B1][XA]}",
            "<>*a* => <>*a* | Name: <>*a* | {[]}",
            "a*|b* => a*|b* | Name: a*|b* | {[A1][B1]}",
            "<>apple&<>xa => <>apple&<>xa | Name: <>apple&<>xa | {[][AB][B1]}",
            "?? => ?? | Name: ?? | {[XA]}",
            "<>?? => <>?? | Name: <>?? | {[][A1][AB][B1]}",
            "@*A* => @*A* | Name: @*A* | {[A1][AB][B1][XA]}",
            "<>@a* => <>@a* | Name: <>@a* | {[][B1][XA]}",
            "<>''&<>* => <>''&<>* | Name: <>''&<>* | {}",
        ]
    );
}

/// WildcardGetFilter.al:176-202 (Placeholders): `<>%1` with `*` is the literal text, `%1` with `A*` a pattern, and `<>%1*` keeps
/// the placeholder as written.
#[test]
fn wildcard_placeholders() {
    assert_eq!(
        lines(run(WILD, "Placeholders")),
        [
            "<>'*' | {[][A1][AB][B1][XA]}",
            "A* | {[A1][AB]}",
            "<>%1* | {[][A1][AB][B1][XA]}",
            "<>'*' | {[][A1][AB][B1][XA]}",
            "*a*&<>xa | {[A1][AB][B1]}",
        ]
    );
}

/// WildcardGetFilter.al:205-222 (NotWildcard_GetView): the view, SetView and CopyFilters keep `<>*`.
#[test]
fn not_wildcard_in_a_view() {
    assert_eq!(
        text(run(WILD, "View")),
        "VERSION(1) SORTING(Field1) WHERE(Field1=1(<>*))|<>*|<>*"
    );
}
