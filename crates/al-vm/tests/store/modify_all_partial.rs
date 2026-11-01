//! ModifyAll on a record variable that was fetched with SetLoadFields (al-runner issue 6654): every row in the filter is
//! changed as without the partial load, also with the OnModify trigger, which sees the whole row.

use al_testkit::text;

const SRC: &str = r#"
table 69990 "MAP Row" { fields { field(1; Id; Integer) { } field(2; "My Field"; Text[30]) { } field(3; Other; Integer) { } } keys { key(PK; Id) { } }
    trigger OnModify() begin Seen += Format(Other) + ';'; end;
    var Seen: Text; }
codeunit 69991 C
{
    procedure WithTrigger(): Text
    var MyRec: Record "MAP Row"; Check: Record "MAP Row";
    begin
        MyRec.Id := 1; MyRec."My Field" := 'a'; MyRec.Other := 5; MyRec.Insert();
        Clear(MyRec);
        MyRec.SetLoadFields("My Field");
        MyRec.FindSet();
        MyRec.ModifyAll("My Field", 'new value', true);
        Check.Get(1);
        exit(Check."My Field" + '/' + Format(Check.Other));
    end;

    procedure F(): Text
    var MyRec: Record "MAP Row"; Check: Record "MAP Row"; T: Text;
    begin
        MyRec.Id := 1; MyRec."My Field" := 'a'; MyRec.Other := 5; MyRec.Insert();
        MyRec.Id := 2; MyRec."My Field" := 'b'; MyRec.Other := 6; MyRec.Insert();
        Clear(MyRec);
        MyRec.SetLoadFields("My Field");
        MyRec.FindSet();
        MyRec.ModifyAll("My Field", 'new value');
        if Check.FindSet() then
            repeat
                T += Format(Check.Id) + '=' + Check."My Field" + '/' + Format(Check.Other) + ';';
            until Check.Next() = 0;
        exit(T);
    end;
}
"#;

#[test]
fn modify_all_after_a_partial_fetch_writes_every_row() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "1=new value/5;2=new value/6;");
}

#[test]
fn modify_all_with_triggers_after_a_partial_fetch() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "WithTrigger")), "new value/5");
}
