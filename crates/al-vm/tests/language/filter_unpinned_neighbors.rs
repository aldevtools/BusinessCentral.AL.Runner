//! The neighbors of the pinned wildcard and Media filter forms keep the answer they had before those forms were pinned, or are
//! refused (owner rule for allow-list fixes, 2026-11-11):
//! - The text `?` alone in a placeholder is the literal text only in the pinned shape `[op]%1` as the whole filter on a Text or
//!   Code field (record/filters/PlaceholderWildcardValue.al:225-293); elsewhere it stays a wildcard, and on an Integer field the
//!   value raises the evaluate error as before.
//! - GetFilter of a wildcard text written as a literal (`=A*B`, `>A*B`, `A*B..AXB`, `..A*`) is not pinned and stays refused;
//!   the placeholder and SetRange forms read back quoted (PlaceholderWildcardValue.al:249-262, :413). So is `<>'A *'`, a
//!   quoted pattern with a space (only `<>'A*B'` and such without one are pinned, :363-377).
//! - Record.SetFilter with a text placeholder argument on a MediaSet field is not pinned (only the written Guid text and the
//!   FieldRef argument error are, recordref-fieldref/fieldref/FieldRefFilterMediaGuidText.al:227-296).
//! - `<>` with blank space around it is not pinned; `<>` alone raises the filter error (FieldRefFilterMatrix.al:1140, :1310).
//! - A range or `<`, `>` of Guids on a Media or MediaSet field, from SetFilter or SetRange, is refused until BC's Guid order is
//!   probed (owner decision 2026-11-11; FieldRefFilterMediaGuidText.al:244, :383 show that it is not the text order).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69981 "NB Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; CodeF; Code[20]) { }
        field(4; M; Media) { }
        field(5; MS; MediaSet) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69981 C
{
    local procedure Seed(var R: Record "NB Row")
    begin
        R.DeleteAll();
        Add(R, 1, 'A');
        Add(R, 2, 'AXB');
        Add(R, 3, '?');
        Add(R, 4, 'B');
        Add(R, 5, 'A*B');
    end;

    local procedure Add(var R: Record "NB Row"; Id: Integer; Name: Text)
    begin
        R.Init();
        R.Id := Id;
        R.Name := Name;
        R.CodeF := Name;
        R.Insert();
    end;

    local procedure Ids(var R: Record "NB Row"): Text
    var
        T: Text;
    begin
        if R.FindSet() then
            repeat
                T += Format(R.Id) + ',';
            until R.Next() = 0;
        exit(T);
    end;

    [TryFunction]
    local procedure TrySet(var R: Record "NB Row"; Which: Text)
    var
        G1: Guid;
        G2: Guid;
    begin
        Evaluate(G1, '{11F319D9-0000-0000-0000-6E725A266949}');
        Evaluate(G2, '{F1F319D9-0000-0000-0000-6E725A266949}');
        case Which of
            'q or name': R.SetFilter(Name, '%1|%2', '?', 'AXB');
            'q or code': R.SetFilter(CodeF, '%1|%2', '?', 'AXB');
            'q and ne': R.SetFilter(Name, '%1&<>%2', '?', 'AXB');
            'q alone name': R.SetFilter(Name, '%1', '?');
            'q alone int': R.SetFilter(Id, '%1', '?');
            'q or int': R.SetFilter(Id, '%1|%2', '?', '1');
            'w eq': R.SetFilter(Name, '=A*B');
            'w gt': R.SetFilter(Name, '>A*B');
            'w range': R.SetFilter(Name, 'A*B..AXB');
            'w upto': R.SetFilter(Name, '..A*');
            'w eq star': R.SetFilter(Name, '=*');
            'ne quoted space': R.SetFilter(Name, '<>''A *''');
            'ne quoted': R.SetFilter(Name, '<>''A*B''');
            'ph eq': R.SetFilter(Name, '=%1', 'A*B');
            'setrange': R.SetRange(Name, 'A*B');
            'ms text arg': R.SetFilter(MS, '%1', Format(G1));
            'ms written': R.SetFilter(MS, Format(G1));
            'ne alone': R.SetFilter(CodeF, '<>');
            'ne space after': R.SetFilter(CodeF, '<> ');
            'ne space before': R.SetFilter(CodeF, ' <>');
            'media range': R.SetFilter(M, Format(G1) + '..' + Format(G2));
            'media gt': R.SetFilter(M, '>' + Format(G1));
            'media eq': R.SetFilter(M, Format(G1));
            'media setrange two': R.SetRange(M, G1, G2);
            'media setrange one': R.SetRange(M, G1);
            'ms setrange two': R.SetRange(MS, G1, G2);
        end;
    end;

    procedure Rows(Which: Text): Text
    var
        R: Record "NB Row";
    begin
        Seed(R);
        if not TrySet(R, Which) then
            exit('E: ' + GetLastErrorText());
        exit(Ids(R));
    end;

    procedure ReadBack(Which: Text): Text
    var
        R: Record "NB Row";
    begin
        Seed(R);
        if not TrySet(R, Which) then
            exit('E: ' + GetLastErrorText());
        exit(R.GetFilters());
    end;
}
"#;

fn call(routine: &str, which: &str) -> String {
    match al_testkit::run_with(SRC, "C", routine, vec![Value::Text(which.into())]).map(|x| x.0) {
        Ok(Value::Text(s)) => s.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused: {why}"),
        other => format!("{other:?}"),
    }
}

fn refused(routine: &str, which: &str) {
    let got = call(routine, which);
    assert!(got.starts_with("refused: "), "{which}: {got}");
}

#[test]
fn a_question_mark_placeholder_outside_the_pinned_shape_stays_a_wildcard() {
    assert_eq!(call("Rows", "q or name"), "1,2,3,4,");
    assert_eq!(call("Rows", "q or code"), "1,2,3,4,");
    assert_eq!(call("Rows", "q and ne"), "1,3,4,");
    // The pinned shape: `%1` with `?` is the literal text (PlaceholderWildcardValue.al:242).
    assert_eq!(call("ReadBack", "q alone name"), "Name: ='?'");
    assert_eq!(call("Rows", "q alone name"), "3,");
}

#[test]
fn a_question_mark_placeholder_on_an_integer_raises_the_evaluate_error() {
    for which in ["q alone int", "q or int"] {
        let got = call("Rows", which);
        assert!(got.starts_with("E: "), "{which}: {got}");
    }
}

#[test]
fn the_read_back_of_a_written_literal_wildcard_text_is_refused() {
    for which in ["w eq", "w gt", "w range", "w upto", "ne quoted space"] {
        refused("ReadBack", which);
    }
    // Its rows are read as before.
    assert_eq!(call("Rows", "w eq"), "5,");
    assert_eq!(call("ReadBack", "w eq star"), "Name: ='*'");
    // The pinned forms (PlaceholderWildcardValue.al:249-262, :413).
    assert_eq!(call("ReadBack", "ph eq"), "Name: ='A*B'");
    assert_eq!(call("ReadBack", "setrange"), "Name: ='A*B'");
    // `<>'A*B'` written reads back as the pattern (PlaceholderWildcardValue.al:363-377); with a space it is not pinned.
    assert_eq!(call("ReadBack", "ne quoted"), "Name: <>A*B");
}

#[test]
fn a_text_placeholder_on_a_media_set_field_is_refused() {
    refused("Rows", "ms text arg");
    assert_eq!(call("Rows", "ms written"), "");
}

#[test]
fn not_equal_with_blank_space_is_refused() {
    assert!(call("Rows", "ne alone").starts_with("E: "));
    refused("Rows", "ne space after");
    refused("Rows", "ne space before");
}

#[test]
fn a_guid_order_on_a_media_field_is_refused() {
    for which in ["media range", "media gt", "media setrange two", "ms setrange two"] {
        refused("Rows", which);
    }
    assert_eq!(call("Rows", "media eq"), "");
    assert_eq!(call("Rows", "media setrange one"), "");
}
