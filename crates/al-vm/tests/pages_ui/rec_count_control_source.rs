//! A control whose source is a Record method without parentheses (`field(Total; Rec.Count)`). The methods
//! pages/controls/RecCountControl.al measured (open corpus PR 1525, assumption `page-control-rec-method-source`, issue 7875) on a
//! List or Card page run, and the corpus case is their test. Every other method, page type and a table with a procedure of that
//! name keep main's refusal (no corpus case pins them): "control Total is bound to unknown field <name>".

use al_builtins::native::Signal;

fn src(page_type: &str, source: &str, table_proc: &str) -> String {
    format!(
        r#"
table 69960 "RC Row"
{{
    fields {{ field(1; Id; Integer) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
    {table_proc}
}}
page 69961 "RC Host"
{{
    PageType = {page_type};
    SourceTable = "RC Row";
    layout {{ area(Content) {{ repeater(Rows) {{ field(Id; Rec.Id) {{ }} field(Total; {source}) {{ }} }} }} }}
}}
codeunit 69962 C
{{
    procedure Run()
    var
        Row: Record "RC Row";
        View: TestPage "RC Host";
    begin
        Row.Id := 1;
        Row.Insert(false);
        View.OpenView();
    end;
}}
"#
    )
}

fn refused(why_has: &str, page_type: &str, source: &str, table_proc: &str) {
    match al_testkit::run_in(&src(page_type, source, table_proc), "C", "Run") {
        Err(Signal::Unsupported(why)) => assert!(why.contains(why_has), "{source} on {page_type}: {why}"),
        other => panic!("{source} on {page_type}: expected the refusal, got {other:?}"),
    }
}

#[test]
fn a_measured_rec_method_source_on_a_list_page_opens() {
    for m in [
        "Count",
        "CountApprox",
        "GetFilters",
        "IsEmpty",
        "TableCaption",
        "TableName",
    ] {
        let r = al_testkit::run_in(&src("List", &format!("Rec.{m}"), ""), "C", "Run");
        assert!(r.is_ok(), "Rec.{m}: {r:?}");
    }
}

#[test]
fn an_unmeasured_rec_method_source_stays_refused_as_an_unknown_field() {
    refused("is bound to unknown field HasFilter", "List", "Rec.HasFilter", "");
    refused("is bound to unknown field Count", "Worksheet", "Rec.Count", "");
    refused(
        "is bound to unknown field Count",
        "List",
        "Rec.Count",
        "procedure Count(): Integer begin exit(1); end;",
    );
}
