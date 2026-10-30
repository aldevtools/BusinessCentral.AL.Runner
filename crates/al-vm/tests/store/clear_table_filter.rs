//! `Clear` of a TableFilter field (record/lt-wc/WcClearTableFilter.al, al-runner issue 6318): the field becomes the empty
//! filter, the other fields keep their values, and the stored row changes only with Modify.

use al_testkit::text;

const SRC: &str = r#"
table 50496 "CTF Job" { fields { field(1; "Code"; Code[20]) { } field(2; "Table No."; Integer) { } field(3; "Table Filter"; TableFilter) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 50496 C
{
    procedure Empty(): Text
    var Job: Record "CTF Job";
    begin
        Job."Table No." := Database::"CTF Job";
        Clear(Job."Table Filter");
        exit(Format(Job."Table Filter") + '|' + Format(Job."Table No."));
    end;
    procedure WithValue(): Text
    var Job: Record "CTF Job"; Fresh: Record "CTF Job"; Out: Text;
    begin
        Job.DeleteAll();
        Job.Code := 'J1';
        Job."Table No." := Database::"CTF Job";
        Evaluate(Job."Table Filter", 'CTF Job: Table No.=5');
        Job.Insert();
        Fresh.Get('J1');
        Out := Format(Fresh."Table Filter");
        Clear(Fresh."Table Filter");
        Out += '|' + Format(Fresh."Table Filter");
        Job.Get('J1');
        Out += '|' + Format(Job."Table Filter");
        Fresh.Modify();
        Job.Get('J1');
        exit(Out + '|' + Format(Job."Table Filter"));
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

/// WcClearTableFilter.al:36-50 WC_Clear_TableFilter_Empty.
#[test]
fn clear_of_an_empty_table_filter_field_keeps_it_empty() {
    assert_eq!(run("Empty"), "|50496");
}

/// WcClearTableFilter.al:52-90 WC_Clear_TableFilter_WithValue and _ReadFromDatabase.
#[test]
fn clear_of_a_table_filter_field_empties_it_and_modify_stores_that() {
    assert_eq!(run("WithValue"), "CTF Job: Table No.=5||CTF Job: Table No.=5|");
}
