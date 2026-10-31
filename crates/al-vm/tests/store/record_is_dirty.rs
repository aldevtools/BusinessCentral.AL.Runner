//! `Record.IsDirty` (BC 29): the sequences of lt-ac/ACRecordIsDirty.al (codeunit 77504), and the refusals of what the
//! corpus does not pin.

use al_testkit::{run, text};

const TABLE: &str = r#"
table 50100 "Dirty Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;

fn with(body: &str) -> String {
    format!(
        "{TABLE}\ncodeunit 50101 \"Dirty Probe\"\n{{\n    procedure Probe(): Text\n    var\n        Row: Record \"Dirty Row\";\n        Other: Record \"Dirty Row\";\n        S: Text;\n    begin\n{body}\n        exit(S);\n    end;\n}}\n"
    )
}

fn yn(src: &str) -> String {
    text(run(&with(src), "Probe"))
}

fn refused(src: &str) -> String {
    match run(&with(src), "Probe") {
        Err(al_builtins::native::Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// ACRecordIsDirty.al:31-87 AC_IsDirty_Lifecycle.
#[test]
fn lifecycle_matches_the_corpus() {
    let got = yn(r#"
        Row.DeleteAll();
        S += Format(Row.IsDirty());
        Row.Init();
        S += Format(Row.IsDirty());
        Row.Id := 1;
        Row.Name := 'one';
        S += Format(Row.IsDirty());
        Row.Insert();
        S += Format(Row.IsDirty());
        Row.Get(1);
        S += Format(Row.IsDirty());
        Row.Name := 'changed';
        S += Format(Row.IsDirty());
        Row.Modify();
        S += Format(Row.IsDirty());
        Row.Name := 'changed';
        S += Format(Row.IsDirty());
        Row.Name := 'again';
        Row.Name := 'changed';
        S += Format(Row.IsDirty());
        Row.Amount := 5;
        S += Format(Row.IsDirty());
        Row.Get(1);
        S += Format(Row.IsDirty());
        Row.Validate(Name, 'validated');
        S += Format(Row.IsDirty());
        Other := Row;
        S += Format(Other.IsDirty());
        Row.Get(1);
        Other.Init();
        Other.TransferFields(Row);
        S += Format(Other.IsDirty());
        Row.Get(1);
        Row.Delete();
        S += Format(Row.IsDirty());
"#);
    assert_eq!(got, "NoNoYesNoNoYesNoNoNoYesNoYesYesNoNo");
}

// ACRecordIsDirty.al:90-123 AC_IsDirty_FindAndRename.
#[test]
fn find_and_rename_match_the_corpus() {
    let got = yn(r#"
        Row.DeleteAll();
        Row.Id := 1;
        Row.Name := 'a';
        Row.Insert();
        Row.Id := 2;
        Row.Name := 'b';
        Row.Insert();
        Row.FindFirst();
        S += Format(Row.IsDirty());
        Row.Name := 'x';
        S += Format(Row.IsDirty());
        Row.FindLast();
        S += Format(Row.IsDirty());
        Row.Rename(3);
        S += Format(Row.IsDirty());
        Row.Name := 'y';
        Row.Reset();
        S += Format(Row.IsDirty());
        Row.Init();
        S += Format(Row.IsDirty());
"#);
    assert_eq!(got, "NoYesNoNoYesYes");
}

#[test]
fn unpinned_paths_are_refused() {
    let seed = "        Row.Id := 1;\n        Row.Insert();\n";
    for (case, body) in [
        // Copy of a record into itself (Y8RecordIsDirtyShapes.al pins Copy from another variable only).
        ("copy from itself", "        Row.Get(1);\n        Row.Copy(Row);\n"),
        ("failed get", "        if Row.Get(2) then;\n"),
    ] {
        let why = refused(&format!("{seed}{body}        S := Format(Row.IsDirty());"));
        assert!(why.starts_with("Record.IsDirty"), "{case}: {why}");
    }
}

#[test]
fn another_variable_writing_the_row_is_refused() {
    // Row's snapshot says clean, the stored row says dirty: nothing pins which one BC compares with.
    let why = refused(
        r#"
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        Other.Get(1);
        Other.Name := 'other';
        Other.Modify();
        S := Format(Row.IsDirty());"#,
    );
    assert!(why.contains("stored row"), "{why}");
}

#[test]
fn a_read_then_a_field_write_is_dirty_and_a_write_back_is_not() {
    // The snapshot of a read is the buffer itself until the first field write copies it (ACRecordIsDirty.al:46-56).
    let got = yn(r#"
        Row.Id := 1;
        Row.Name := 'a';
        Row.Insert();
        Row.Get(1);
        Row.Name := 'b';
        S += Format(Row.IsDirty());
        Row.Name := 'a';
        S += Format(Row.IsDirty());
        Row.Get(1);
        Row.Init();
        S += Format(Row.IsDirty());"#);
    assert_eq!(got, "YesNoYes");
}
