//! A `TableRelation ... where(F = filter('<>'''''))`: the filter text written as one AL string with doubled quotes in it,
//! on a field that a tableextension adds to the related table (al-runner issue 6924, item 3.2). Measured on BC 29 by the
//! corpus writer (record/table-relation/TableRelationQuotes.al TRX_*, assumption table-relation-quoted-literal-filter):
//! `<>''` selects the filled rows, `''` the empty one, and `filter('<>''')` raises InvalidFilterExpression, so it is refused.

use al_builtins::native::Signal;
use al_testkit::{run, text};

const SRC: &str = r#"
table 69980 "QL Target" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
tableextension 69981 "QL Target Ext" extends "QL Target" { fields { field(50; "Ext Code"; Code[10]) { } } }
table 69982 "QL Line"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = A,Target; }
        field(4; Empty; Code[20]) { TableRelation = "QL Target" where("Ext Code" = filter('''''')); }
        field(5; Bad; Code[20]) { TableRelation = "QL Target" where("Ext Code" = filter('<>''')); }
        field(3; Ref; Code[20])
        {
            TableRelation = if (Kind = const(A)) "QL Line" else
            if (Kind = const(Target)) "QL Target" where("Ext Code" = filter('<>'''''));
        }
    }
    keys { key(PK; Id) { } }
}
codeunit 69983 C
{
    local procedure Try(Value: Code[20]): Text
    var L: Record "QL Line";
    begin
        L.Init(); L.Kind := L.Kind::Target;
        ClearLastError();
        asserterror begin L.Validate(Ref, Value); Error('ok'); end;
        exit(GetLastErrorText());
    end;
    procedure F(): Text
    var T: Record "QL Target";
    begin
        T.Code := 'FILLED'; T."Ext Code" := 'X'; T.Insert();
        T.Code := 'EMPTY'; T."Ext Code" := ''; T.Insert();
        exit(Try('FILLED') + '|' + Try('EMPTY'));
    end;
    procedure E(): Text
    var T: Record "QL Target"; L: Record "QL Line";
    begin
        T.Code := 'FILLED'; T."Ext Code" := 'X'; T.Insert();
        T.Code := 'EMPTY'; T."Ext Code" := ''; T.Insert();
        L.Validate(Empty, 'EMPTY');
        ClearLastError();
        asserterror L.Validate(Empty, 'FILLED');
        exit(GetLastErrorText());
    end;
    procedure B()
    var T: Record "QL Target"; L: Record "QL Line";
    begin
        T.Code := 'FILLED'; T."Ext Code" := 'X'; T.Insert();
        L.Validate(Bad, 'FILLED');
    end;
}
"#;

#[test]
fn a_not_empty_filter_written_as_one_quoted_literal_accepts_a_filled_row() {
    assert_eq!(
        text(run(SRC, "F")),
        "ok|The field Ref of table QL Line contains a value (EMPTY) that cannot be found in the related table (QL Target)."
    );
}

#[test]
fn a_quoted_empty_literal_filter_selects_the_empty_row() {
    assert_eq!(
        text(run(SRC, "E")),
        "The field Empty of table QL Line contains a value (FILLED) that cannot be found in the related table (QL Target)."
    );
}

#[test]
fn another_quoted_literal_filter_is_refused() {
    let r = run(SRC, "B");
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}
