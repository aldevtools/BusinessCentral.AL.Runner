//! A placeholder value that starts with `@` is the case-insensitive operator, as when it is written in the filter (al-runner issue
//! 7034): `SetFilter(Printer, '<>%1', '@NO PRINT')` leaves out `NO PRINT` and `no print` and keeps `@NO PRINT`
//! (record/filters/PlaceholderAtSignValue.al, staging r6980).

use al_testkit::text;

const SRC: &str = r#"
table 69940 Row
{
    fields { field(1; Id; Integer) { } field(2; Printer; Text[250]) { } field(3; "Code"; Code[50]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69940 C
{
    local procedure Seed()
    var R: Record Row;
    begin
        R.Id := 1; R.Printer := 'NO PRINT'; R."Code" := 'NO PRINT'; R.Insert();
        R.Id := 2; R.Printer := 'no print'; R."Code" := 'NO PRINTX'; R.Insert();
        R.Id := 3; R.Printer := '@NO PRINT'; R."Code" := '@NO PRINT'; R.Insert();
        R.Id := 4; R.Printer := 'ABC'; R."Code" := 'ABC'; R.Insert();
    end;

    local procedure Ids(var R: Record Row): Text
    var Result: Text;
    begin
        if not R.FindSet() then exit('none');
        repeat Result += Format(R.Id); until R.Next() = 0;
        exit(Result);
    end;

    procedure Run(): Text
    var R: Record Row; Out: Text;
    begin
        Seed();
        R.SetFilter(Printer, '<>%1', '@NO PRINT');
        Out := R.GetFilters() + ' ' + Ids(R);
        R.SetFilter(Printer, '<>%1', '@no*');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        R.SetFilter(Printer, '<>%1&<>ABC', '@NO PRINT');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        R.Reset();
        R.SetFilter("Code", '<>@NO PRINT');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        R.SetFilter("Code", '<>%1', '@no print');
        Out += ' | ' + R.GetFilters() + ' ' + Ids(R);
        exit(Out);
    end;
}
"#;

#[test]
fn an_at_sign_in_a_placeholder_value_ignores_case() {
    assert_eq!(
        text(al_testkit::run(SRC, "Run")),
        "Printer: <>@NO PRINT 34 | Printer: <>@no* 34 | Printer: <>@NO PRINT&<>ABC 3 | Code: <>@NO PRINT 234 | Code: <>@NO PRINT 234"
    );
}
