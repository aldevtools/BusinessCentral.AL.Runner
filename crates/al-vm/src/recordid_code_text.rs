//! `Evaluate(RecordId, 'Table: A,B')` on a table whose primary key is a Code field and a Text field
//! (misc/lt-y68-small-probes/Y68Small.al:65-121 Q5_Evaluate_RecordId_CodeAndText_Forms, :123-180 the round trips; corpus
//! master 0d01b0cc, BC 27.5, 28.5, 29.0).

use al_builtins::value::{RtType, Value};
use al_data::TableDef;

/// The key values of the parts (text, whether it was quoted) of a key text when `t`'s primary key is a Code and a Text field:
/// `None` when this module does not cover the key, `Some(None)` when Evaluate fails, else the values.
pub(crate) fn code_text_key(t: &TableDef, parts: &[(String, bool)]) -> Option<Option<Vec<Value>>> {
    let (code_len, text_len) = match t.keys.first().map(Vec::as_slice) {
        Some(&[a, b]) => match (&t.fields[a].ty, &t.fields[b].ty) {
            (RtType::Code(c), RtType::Text(x)) => (*c, *x),
            _ => return None,
        },
        _ => return None,
    };
    // Three parts: Evaluate fails (`three parts`). `A1;Hello` is one Code part (`semicolon`: `A1;HELLO`).
    if parts.len() > 2 {
        return Some(None);
    }
    // A part that is not quoted but starts with a quote after blanks (`A1, "x"`) is not pinned.
    if parts
        .iter()
        .any(|(v, quoted)| !quoted && v.trim_start().starts_with('"'))
    {
        return None;
    }
    let mut key = Vec::new();
    if let Some((v, quoted)) = parts.first() {
        // A Code part is uppercased; blanks around it are dropped, blanks and quotes inside kept (`blanks around comma`,
        // `leading blank first`, `blank in code`, `quote inside code`); an empty one, quoted or not, is an empty Code
        // (`empty code part`, `empty quoted code part`, `both empty`); one longer than the field fails (`code too long`).
        let v = if *quoted { v.as_str() } else { v.trim() };
        if code_len.is_some_and(|n| v.chars().count() > n as usize) {
            return Some(None);
        }
        key.push(Value::Code(v.to_uppercase().as_str().into()));
    }
    if let Some((v, quoted)) = parts.get(1) {
        // A Text part keeps its case; blanks around one that is not quoted are dropped (`trailing blank second`), and an empty
        // one that is not quoted gives no value (`empty text part`: `A1` reads back `A1`). A quoted one is kept as written:
        // `""`, `" "`, `"x,y"`, and doubled quotes read as one (`empty quoted text part`, `quoted blank text`,
        // `quoted text with quote`).
        let v = if *quoted { v.as_str() } else { v.trim() };
        if !v.is_empty() || *quoted {
            // A Text part longer than the field is not pinned.
            if text_len.is_some_and(|n| v.chars().count() > n as usize) {
                return None;
            }
            key.push(Value::Text(v.into()));
        }
    }
    Some(Some(key))
}
