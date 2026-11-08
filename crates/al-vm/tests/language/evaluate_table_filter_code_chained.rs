//! `Evaluate` into a TableFilter field of Code alternatives that hold `-` and `/` (al-runner issue 7262) and of chained Code
//! exclusions `<>X&<>Y` (al-runner issue 7264). Pinned by corpus pull request 1199 (measured on real BC 27.5, 28.5 and 29.0):
//! format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:936 TableFilter_Evaluate_CodeExclusions_LeavesOtherRow
//! and :943 TableFilter_Evaluate_CodeAlternatives_SelectsBothMatchingRows. Only the selection through the Base Application page
//! "Table Filter" and FieldRef.SetFilter is pinned; the stored text is an assumption. The test reads the stored text back as that
//! page does: each field by its caption, its filter handed to FieldRef.SetFilter as written.

const OBJECTS: &str = r#"
table 69081 "FMT Code Chain Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Code Field"; Code[20]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 95453 "LT R6951 Holder" { fields { field(1; Id; Integer) { } field(2; Flt; TableFilter) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 95461 Apply
{
    procedure Selects(Value: Text; TableNo: Integer): Text
    var
        Ref: RecordRef;
        FRef: FieldRef;
        Part: Text;
        Cap: Text;
        I: Integer;
        No: Integer;
        First: Text;
    begin
        Part := CopyStr(Value, StrPos(Value, ': ') + 2);
        Ref.Open(TableNo);
        Cap := CopyStr(Part, 1, StrPos(Part, '=') - 1);
        for I := 1 to Ref.FieldCount() do
            if Ref.FieldIndex(I).Caption() = Cap then No := Ref.FieldIndex(I).Number();
        if No = 0 then Error('no field %1', Cap);
        FRef := Ref.Field(No);
        FRef.SetFilter(CopyStr(Part, StrPos(Part, '=') + 1));
        Ref.FindFirst();
        First := Format(Ref.Field(1).Value());
        Ref.FindLast();
        exit(Format(Ref.Count()) + '|' + First + '|' + Format(Ref.Field(1).Value()));
    end;
}
"#;

fn raw(seed: [&str; 3], condition: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "{OBJECTS}\ncodeunit 95460 T\n{{\nprocedure F(): Text var R: Record \"FMT Code Chain Row\"; H: Record \"LT R6951 Holder\"; A: Codeunit Apply;
        begin
        R.\"Entry No.\" := 101; R.\"Code Field\" := '{}'; R.Insert(false);
        R.\"Entry No.\" := 202; R.\"Code Field\" := '{}'; R.Insert(false);
        R.\"Entry No.\" := 303; R.\"Code Field\" := '{}'; R.Insert(false);
        Evaluate(H.Flt, 'FMT Code Chain Row: {condition}');
        exit(A.Selects(Format(H.Flt), Database::\"FMT Code Chain Row\")); end;\n}}\n",
        seed[0], seed[1], seed[2]
    );
    al_testkit::run_in(&src, "T", "F")
}

fn case(seed: [&str; 3], condition: &str) -> String {
    al_testkit::text(raw(seed, condition))
}

/// :943 TableFilter_Evaluate_CodeAlternatives_SelectsBothMatchingRows: rows 101 and 202, not 303 (`BIN/C`).
#[test]
fn code_alternatives_with_dash_and_slash_select_both_rows() {
    assert_eq!(case(["BIN-A", "BIN/B", "BIN/C"], "Code Field=BIN-A|BIN/B"), "2|101|202");
}

/// :936 TableFilter_Evaluate_CodeExclusions_LeavesOtherRow (seed of CheckCommaTableFilter, :715): only row 101.
#[test]
fn chained_code_exclusions_leave_the_other_row() {
    assert_eq!(
        case(["OTHER", "CORP01", "CORP02"], "Code Field=<>CORP01&<>CORP02"),
        "1|101|101"
    );
}

/// Lower case in the new forms is not pinned and stays refused.
#[test]
fn lower_case_alternatives_with_a_slash_stay_refused() {
    let r = raw(["BIN-A", "BIN/B", "BIN/C"], "Code Field=bin-a|bin/b");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
