//! `Evaluate` into a TableFilter field whose condition is on an Enum field, or a list, `<>` or range on an Option field
//! (al-runner issue 6951). Pinned by corpus branch al-runtime-rs-staging-r6951
//! format-evaluate/tablefilter-neutral/TableFilterEnumForms.al (BC 29 Linux CI): TableFilterEvaluate_EnumMemberForms,
//! _EnumExpressions, _OptionExpressions, _EnumRoundTrip. Every member is stored by its caption.

use al_builtins::native::Signal;

const OBJECTS: &str = "enum 95450 \"LT R6951 Cost Kind\" { Extensible = false;
        value(0; Labor) { Caption = 'Labor Cap'; } value(1; Setting) { Caption = 'Setting Cap'; } value(5; Travel) { Caption = 'Travel Cap'; } }
    enum 95451 \"LT R6951 Config Kind\" { Extensible = false; value(0; Item) { } value(1; Variant) { } value(2; Service) { } }
    table 95452 \"LT R6951 Target\" { Caption = 'LT R6951 Target';
        fields { field(1; \"Code\"; Code[20]) { } field(2; \"Cost Type\"; Enum \"LT R6951 Cost Kind\") { Caption = 'Cost Type Cap'; }
        field(3; \"Config Type\"; Enum \"LT R6951 Config Kind\") { } field(4; \"Employee No.\"; Code[20]) { }
        field(5; Status; Option) { OptionMembers = Open,Released,Closed; OptionCaption = 'Open Cap,Released Cap,Closed Cap'; } }
        keys { key(PK; \"Code\") { Clustered = true; } } }
    table 95453 \"LT R6951 Holder\" { fields { field(1; Id; Integer) { } field(2; Flt; TableFilter) { } }
        keys { key(PK; Id) { Clustered = true; } } }";

/// `<answer>|[<Format of the field>]|<last error up to its first backslash>`, as the probe's `Eval`.
fn eval(text: &str) -> Result<String, Signal> {
    let s = format!(
        "{OBJECTS} codeunit 95460 C {{ procedure F(): Text var R: Record \"LT R6951 Holder\"; Ok: Boolean; E: Text; begin ClearLastError();
        Ok := Evaluate(R.Flt, '{}');
        E := GetLastErrorText(); if StrPos(E, '\\') > 0 then E := CopyStr(E, 1, StrPos(E, '\\') - 1);
        exit(Format(Ok) + '|[' + Format(R.Flt) + ']|' + E); end; }}",
        text.replace('\'', "''")
    );
    al_testkit::run_in(&s, "C", "F").map(|v| al_testkit::text(Ok(v)))
}

fn yes(stored: &str) -> String {
    format!("Yes|[LT R6951 Target: {stored}]|")
}

#[test]
fn an_enum_member_is_stored_by_its_caption() {
    // TableFilterEvaluate_EnumMemberForms: name, name in lower case, caption, ordinal, first member, an enum without captions.
    for (text, stored) in [
        ("Cost Type=Setting", "Cost Type Cap=Setting Cap"),
        ("Cost Type=setting", "Cost Type Cap=Setting Cap"),
        ("Cost Type=Setting Cap", "Cost Type Cap=Setting Cap"),
        ("Cost Type=5", "Cost Type Cap=Travel Cap"),
        ("Cost Type=Labor", "Cost Type Cap=Labor Cap"),
        ("Config Type=Variant", "Config Type=Variant"),
    ] {
        assert_eq!(
            eval(&format!("LT R6951 Target: {text}")).unwrap(),
            yes(stored),
            "{text}"
        );
    }
}

#[test]
fn an_enum_bad_member_and_a_wrapper_are_rejected() {
    assert_eq!(
        eval("LT R6951 Target: Cost Type=Nope").unwrap(),
        "No|[]|The value \"LT R6951 Target: Cost Type=Nope\" can't be evaluated into type TableFilter. 'Nope' is not an option. The existing options are: Labor Cap,Setting Cap,Travel Cap"
    );
    assert_eq!(
        eval("LT R6951 Target: Cost Type=FILTER(Setting)").unwrap(),
        "No|[]|The value \"LT R6951 Target: Cost Type=FILTER(Setting)\" can't be evaluated into type TableFilter. An error occurred while interpreting the filter: Did not expect '('."
    );
    assert_eq!(
        eval("LT R6951 Target: Config Type=Item|Nope").unwrap(),
        "No|[]|The value \"LT R6951 Target: Config Type=Item|Nope\" can't be evaluated into type TableFilter. 'Nope' is not an option. The existing options are: Item,Variant,Service"
    );
}

#[test]
fn enum_lists_not_equal_and_ranges_are_stored_by_caption() {
    // TableFilterEvaluate_EnumExpressions.
    for (text, stored) in [
        ("Config Type=Item|Variant", "Config Type=Item|Variant"),
        ("Config Type=Item|Variant|Service", "Config Type=Item|Variant|Service"),
        ("Cost Type=Labor Cap|Travel Cap", "Cost Type Cap=Labor Cap|Travel Cap"),
        ("Cost Type=Labor|Travel", "Cost Type Cap=Labor Cap|Travel Cap"),
        ("Cost Type=<>Setting", "Cost Type Cap=<>Setting Cap"),
        ("Config Type=Item..Variant", "Config Type=Item..Variant"),
        ("Code=A,Config Type=Item|Variant", "Code=A,Config Type=Item|Variant"),
    ] {
        assert_eq!(
            eval(&format!("LT R6951 Target: {text}")).unwrap(),
            yes(stored),
            "{text}"
        );
    }
}

#[test]
fn option_lists_not_equal_and_ranges_are_stored_by_caption() {
    // TableFilterEvaluate_OptionExpressions.
    for (text, stored) in [
        ("Status=Open|Released", "Status=Open Cap|Released Cap"),
        ("Status=Open Cap|Closed Cap", "Status=Open Cap|Closed Cap"),
        ("Status=<>Closed", "Status=<>Closed Cap"),
        ("Status=Open..Released", "Status=Open Cap..Released Cap"),
    ] {
        assert_eq!(
            eval(&format!("LT R6951 Target: {text}")).unwrap(),
            yes(stored),
            "{text}"
        );
    }
}

// A quoted blank filter: `''` on an Enum with a blank member and `<>''` on a Code field (al-runner issue 7132). Pinned by corpus
// master format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:468 TableFilter_Evaluate_QuotedBlankEnum_SelectsBlankMember
// and :485 TableFilter_Evaluate_NotEqualQuotedBlankCode_SelectsNonemptyCode (corpus pull request 1152): the stored text, fed to
// the Base Application page "Table Filter" (which hands each field filter to FieldRef.SetFilter as written), selects exactly the
// one row. The test reads the filter after `=` as that page does and sets it on a RecordRef.
const BLANK_OBJECTS: &str = "enum 69004 \"FMT Blank Member Choice\" { Extensible = false;
        value(0; \" \") { Caption = ' '; } value(1; Marked) { Caption = 'Marked'; } }
    table 69024 \"FMT Blank Filter Row\" { fields { field(1; Code; Code[20]) { } field(2; Choice; Enum \"FMT Blank Member Choice\") { } }
        keys { key(PK; Code) { Clustered = true; } } }
    table 95453 \"LT R6951 Holder\" { fields { field(1; Id; Integer) { } field(2; Flt; TableFilter) { } }
        keys { key(PK; Id) { Clustered = true; } } }";

/// Seeds two rows, evaluates `text`, sets the stored field filter on the field it names and answers
/// `<Format of the field>|<count>|<first code>`.
fn blank_filter_selects(seed: &str, text: &str, field_no: i32) -> String {
    let s = format!(
        "{BLANK_OBJECTS} codeunit 95460 C {{ procedure F(): Text var H: Record \"LT R6951 Holder\"; Row: Record \"FMT Blank Filter Row\";
        Ref: RecordRef; Fld: FieldRef; T: Text; begin {seed}
        Evaluate(H.Flt, '{}'); T := Format(H.Flt);
        Ref.Open(Database::\"FMT Blank Filter Row\"); Fld := Ref.Field({field_no}); Fld.SetFilter(CopyStr(T, StrPos(T, '=') + 1));
        Ref.FindFirst(); Fld := Ref.Field(1);
        exit(T + '|' + Format(Ref.Count()) + '|' + Format(Fld.Value())); end; }}",
        text.replace('\'', "''")
    );
    al_testkit::text(al_testkit::run_in(&s, "C", "F"))
}

#[test]
fn a_quoted_blank_on_an_enum_with_a_blank_member_selects_the_blank_member() {
    let got = blank_filter_selects(
        "Row.Code := 'BLANK42'; Row.Insert(false); Row.Init(); Row.Code := 'MARKED42'; Row.Choice := Row.Choice::Marked; Row.Insert(false);",
        "FMT Blank Filter Row: Choice=''",
        2,
    );
    assert_eq!(got, "FMT Blank Filter Row: Choice=''|1|BLANK42");
}

#[test]
fn not_equal_a_quoted_blank_on_a_code_field_selects_the_nonempty_code() {
    let got = blank_filter_selects(
        "Row.Code := ''; Row.Insert(false); Row.Init(); Row.Code := 'NEUTRAL42'; Row.Insert(false);",
        "FMT Blank Filter Row: Code=<>''",
        1,
    );
    assert_eq!(got, "FMT Blank Filter Row: Code=<>''|1|NEUTRAL42");
}
