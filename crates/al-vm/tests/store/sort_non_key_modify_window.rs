//! A read in a sort that is not a key of the table, then a Modify of a field of that sort, then FindFirst with a range:
//! FindFirst finds the row Count counts, at 63, 64 and 200 rows (al-runner issue 7575).

use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "SNK Link"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Store Code"; Code[10]) { }
        field(3; Active; Boolean) { }
        field(4; "Remote ID"; Integer) { }
    }
    keys
    {
        key(PK; "No.", "Store Code") { Clustered = true; }
        key(Key2; "Store Code", Active, "Remote ID") { }
    }
}
codeunit 69991 C
{
    procedure Run(Rows: Integer): Text
    var
        Link: Record "SNK Link";
        i: Integer;
    begin
        for i := 1 to Rows - 1 do begin
            Link."No." := StrSubstNo('Z%1', 10000 + i);
            Link."Store Code" := 'X';
            Link.Insert();
        end;
        Link."No." := 'A';
        Link."Store Code" := 'X';
        Link.Insert();
        if not Find(0) then
            exit('no row before');
        Link.Get('A', 'X');
        Link."Remote ID" := 7;
        Link.Modify();
        exit(Format(Find(7)) + ' ' + Format(Cnt(7)));
    end;

    local procedure Find(RemoteId: Integer): Boolean
    var
        Link: Record "SNK Link";
    begin
        Link.SetCurrentKey("Store Code", "Remote ID");
        Link.SetRange("Store Code", 'X');
        Link.SetRange("Remote ID", RemoteId);
        exit(Link.FindFirst());
    end;

    local procedure Cnt(RemoteId: Integer): Integer
    var
        Link: Record "SNK Link";
    begin
        Link.SetCurrentKey("Store Code", "Remote ID");
        Link.SetRange("Store Code", 'X');
        Link.SetRange("Remote ID", RemoteId);
        exit(Link.Count());
    end;
}
"#;

fn run(rows: i64) -> String {
    text(al_testkit::run_with(SRC, "C", "Run", vec![Value::Integer(rows as i32)]).map(|(v, _)| v))
}

#[test]
fn modified_row_is_found_after_a_read_in_a_non_key_sort() {
    for rows in [63, 64, 200] {
        assert_eq!(run(rows), "Yes 1", "{rows} rows");
    }
}
