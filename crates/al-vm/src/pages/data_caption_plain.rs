//! Which DataCaptionExpression of a Worksheet may keep main's open when BC would have run it at the open and the page is in no
//! measured cell (`data_caption_open.rs`). On BC the expression runs at the open (DataCaptionExprWorksheet.al:412, :439, :572,
//! :586), so an expression that can raise an error there (a call of a procedure, a `Get` without `if`, a division, `TestField`)
//! fails the open on BC while main never ran it. Only an expression that cannot raise is plain: a literal, a field of Rec, a
//! `Format` of one of those, or a concatenation (`+`) of those. Nothing else is measured; every other expression refuses.

/// Whether `text`, the property text of a DataCaptionExpression, is plain.
pub(super) fn plain_caption_expression(text: &str) -> bool {
    let Some(terms) = split_top_level(text.trim(), '+') else {
        return false;
    };
    terms.iter().all(|t| plain_term(t.trim()))
}

/// A leaf, or `Format(leaf)` with exactly one argument.
fn plain_term(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("format") {
        let rest = t["format".len()..].trim_start();
        return rest
            .strip_prefix('(')
            .and_then(|r| r.strip_suffix(')'))
            .is_some_and(|inner| plain_leaf(inner.trim()));
    }
    plain_leaf(t)
}

/// A string literal `'...'` (a quote doubled inside), or `Rec.Field` / `Rec."Field"`.
fn plain_leaf(t: &str) -> bool {
    if let Some(body) = t.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        return body.replace("''", "").find('\'').is_none();
    }
    let Some(field) = t
        .get(..4)
        .filter(|p| p.eq_ignore_ascii_case("rec."))
        .map(|_| t[4..].trim())
    else {
        return false;
    };
    match field.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(name) => !name.is_empty() && !name.contains('"'),
        None => !field.is_empty() && field.chars().all(|c| c.is_alphanumeric() || c == '_'),
    }
}

/// `text` cut at `sep` outside quotes `'` `"` and parentheses; `None` when quotes or parentheses do not balance.
fn split_top_level(text: &str, sep: char) -> Option<Vec<&str>> {
    let (mut parts, mut start, mut depth, mut quote) = (Vec::new(), 0, 0i32, None::<char>);
    for (i, c) in text.char_indices() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '\'' | '"' => quote = Some(c),
                '(' => depth += 1,
                ')' => depth -= 1,
                c if c == sep && depth == 0 => {
                    parts.push(&text[start..i]);
                    start = i + 1;
                }
                _ => {}
            },
        }
        if depth < 0 {
            return None;
        }
    }
    if quote.is_some() || depth != 0 {
        return None;
    }
    parts.push(&text[start..]);
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::plain_caption_expression as plain;

    #[test]
    fn literals_fields_format_and_concatenation_are_plain() {
        for t in [
            "'Lit'",
            "'It''s'",
            "Rec.\"Item No.\"",
            "rec.Name",
            "'Cap ' + Rec.\"Item No.\"",
            "Format(Rec.Quantity)",
            "'a' + Format(Rec.\"Line No.\") + 'b + c'",
        ] {
            assert!(plain(t), "{t}");
        }
    }

    #[test]
    fn calls_builtins_and_other_operators_are_not() {
        for t in [
            "GetCaption()",
            "Rec.GetFilter(\"Item No.\")",
            "Format(Rec.Quantity, 0, 1)",
            "Format(GetCaption())",
            "Rec.\"Item No.\" + GetCaption()",
            "Rec.Quantity / Rec.Amount",
            "Name",
            "'unterminated",
            "Rec.",
            "Rec.\"a\"b\"",
            "'a' + ",
            "Format(Rec.Name",
            "",
        ] {
            assert!(!plain(t), "{t}");
        }
    }
}
