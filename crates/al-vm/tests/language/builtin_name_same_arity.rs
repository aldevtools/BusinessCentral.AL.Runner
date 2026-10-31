//! A table procedure named like a built-in Record method hides the built-in only for a call that fits the procedure
//! (record/lt-bm-builtin-name-procedures/Y36BuiltinNameSameArity.al:146-298): `Row.Delete(true)` next to own `Delete(Text)`,
//! `Row.Get(2)` next to own `Get(Record; Record)`, `Row.Validate(Note, 'x')` next to own `Validate(Text; Text)` and a bare
//! `Insert()` in table code next to own `Insert(Integer; Text; Boolean)` are the built-ins; `Delete('why')` and
//! `Validate('Note', 'y')` run the procedures. Microsoft's E-Document tests call `EDocImportedLine.Insert()` next to a table
//! procedure Insert with arguments.

use al_testkit::text;

const SRC: &str = r#"
table 69770 "BNS Row"
{
    fields { field(1; Id; Integer) { } field(2; Note; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }

    procedure Get(A: Record "BNS Row"; B: Record "BNS Row"): Boolean
    begin
        Note := 'own Get';
        exit(false);
    end;

    procedure Insert(A: Integer; B: Text; C: Boolean)
    begin
        Note := 'own Insert';
    end;

    procedure Delete(Reason: Text): Boolean
    begin
        Note := 'own Delete';
        exit(false);
    end;

    procedure Validate(FieldName: Text; Value: Text)
    begin
        Note := 'own Validate';
    end;

    procedure BareInsert(NewId: Integer)
    begin
        Id := NewId;
        Note := 'inside';
        Insert();
    end;
}
codeunit 69771 C
{
    procedure F(): Text
    var Row: Record "BNS Row"; Result: Text;
    begin
        Row.DeleteAll();
        Row.BareInsert(1);
        Row.BareInsert(2);
        Row.Get(2);
        Result := Row.Note;
        Row.Validate(Note, 'x');
        Result += ' ' + Row.Note;
        Row.Validate('Note', 'y');
        Result += ' ' + Row.Note;
        Result += ' ' + Format(Row.Delete('why')) + ' ' + Row.Note + ' ' + Format(Row.Count());
        Result += ' ' + Format(Row.Delete(true)) + ' ' + Format(Row.Count());
        exit(Result);
    end;
}
"#;

#[test]
fn the_builtin_runs_when_the_call_does_not_fit_the_table_procedure() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "inside x own Validate No own Delete 2 Yes 1"
    );
}
