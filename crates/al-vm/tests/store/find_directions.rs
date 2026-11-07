//! `Record.Find` and `RecordRef.Find` with every direction text (corpus record/PZFindDirections.al, BC 29.0): the
//! characters are tried in the order written and the first that finds a row wins; `-` and `+` stand alone; a failed Find
//! leaves the key as it was; any other text raises `DB:BadSearchMethod`.

use al_builtins::value::Value;
use al_testkit::{Built, text};

const SRC: &str = r#"
table 69353 "PZ Find Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 69354 "PZ Find Empty" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69353 "PZ Find Directions"
{
    procedure Seed()
    var Row: Record "PZ Find Row"; i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to 3 do begin Row.Id := i * 10; Row.Txt := 't'; Row.Insert(); end;
    end;

    [TryFunction]
    local procedure TryRecordFind(Direction: Text; StartId: Integer; var Found: Boolean; var AfterId: Integer; Empty: Boolean)
    var Row: Record "PZ Find Row"; EmptyRow: Record "PZ Find Empty";
    begin
        if Empty then begin EmptyRow.Id := StartId; Found := EmptyRow.Find(Direction); AfterId := EmptyRow.Id; end
        else begin Row.Id := StartId; Found := Row.Find(Direction); AfterId := Row.Id; end;
    end;

    [TryFunction]
    local procedure TryRecordRefFind(Direction: Text; StartId: Integer; var Found: Boolean; var AfterId: Integer; Empty: Boolean)
    var RecRef: RecordRef;
    begin
        if Empty then RecRef.Open(Database::"PZ Find Empty") else RecRef.Open(Database::"PZ Find Row");
        RecRef.Field(1).Value := StartId;
        Found := RecRef.Find(Direction);
        AfterId := RecRef.Field(1).Value;
    end;

    procedure One(Direction: Text; StartId: Integer; UseRecordRef: Boolean; Empty: Boolean): Text
    var Found: Boolean; AfterId: Integer; Ok: Boolean;
    begin
        if UseRecordRef then Ok := TryRecordRefFind(Direction, StartId, Found, AfterId, Empty)
        else Ok := TryRecordFind(Direction, StartId, Found, AfterId, Empty);
        if not Ok then exit('error ' + GetLastErrorText() + '|' + GetLastErrorCode());
        exit(Format(Found) + '/' + Format(AfterId));
    end;
}
"#;

/// What the corpus pins, as a model: the rows are 10, 20 and 30.
fn model(direction: &str, start: i32, empty: bool) -> String {
    let rows = if empty { vec![] } else { vec![10, 20, 30] };
    let valid = match direction {
        "-" | "+" => true,
        d => {
            !d.is_empty()
                && d.chars().all(|c| matches!(c, '=' | '<' | '>'))
                && d.chars().enumerate().all(|(i, c)| !d[i + 1..].contains(c))
        }
    };
    if !valid {
        return format!(
            "error There is an invalid character in the Find expression '{direction}'.\\\\In Find expressions, you can use =, >, and < in combinations, where each sign can only appear once, or you can use - or +.|DB:BadSearchMethod"
        );
    }
    let mut found = None;
    for c in direction.chars() {
        found = match c {
            '=' => rows.iter().copied().find(|r| *r == start),
            '<' => rows.iter().copied().filter(|r| *r < start).max(),
            '>' => rows.iter().copied().filter(|r| *r > start).min(),
            '-' => rows.first().copied(),
            _ => rows.last().copied(),
        };
        if found.is_some() {
            break;
        }
    }
    match found {
        Some(id) => format!("Yes/{id}"),
        None => format!("No/{start}"),
    }
}

#[test]
fn every_direction_text_gives_what_the_corpus_pins_for_record_and_recordref() {
    let mut b = Built::from_source(SRC);
    b.run_in("PZ Find Directions", "Seed").unwrap();
    let (oi, one) = b.routine_in("PZ Find Directions", "One");
    let mut texts: Vec<String> = vec!["-".into(), "+".into()];
    // Every text of up to four characters over the three signs, plus the invalid ones of the corpus.
    let signs = ["=", "<", ">"];
    let mut level: Vec<String> = signs.iter().map(|s| s.to_string()).collect();
    for _ in 0..3 {
        texts.extend(level.iter().cloned());
        level = level
            .iter()
            .flat_map(|p| signs.iter().map(move |s| format!("{p}{s}")))
            .collect();
    }
    texts.extend(["", "x", "=x", "X", " ", "-+", "=-", "+-", "<-", "=<>x"].map(String::from));
    for d in &texts {
        for use_ref in [false, true] {
            for (start, empty) in [(20, false), (25, false), (5, false), (35, false), (20, true)] {
                let args = vec![
                    Value::Text(d.as_str().into()),
                    Value::Integer(start),
                    Value::Boolean(use_ref),
                    Value::Boolean(empty),
                ];
                let got = text(b.call(oi, one, args).map(|(v, _)| v));
                assert_eq!(
                    got,
                    model(d, start, empty),
                    "Find('{d}') from {start}, recordref {use_ref}, empty {empty}"
                );
            }
        }
    }
}

#[test]
fn integer_to_text_is_the_digits_for_the_values_the_corpus_uses() {
    // record/PZFindDirections.al:91 (`StartId.ToText()` for 5, 20, 25 and 35); no thousands separator
    // (format-evaluate/P93ScalarToText.al:219-255: 1000 and 2147483647).
    let src = "codeunit 50100 C { procedure F(): Text var N: Integer; begin N := 25; exit(N.ToText() + (-7).ToText()); end; procedure G(): Text var N: Integer; begin N := 1234; exit(N.ToText()); end; }";
    let mut b = Built::from_source(src);
    assert_eq!(text(b.run_in("C", "F")), "25-7");
    assert_eq!(text(b.run_in("C", "G")), "1234");
}
