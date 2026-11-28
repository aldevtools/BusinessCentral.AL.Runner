//! The text scanners of the property grammars (CalcFormula, TableRelation, SubPageLink, DataItemLink, query
//! filters, report and xmlport links): one splitter at top-level separators and one unquoter, each with the explicit
//! rules the grammars differ in (ADR-037 item 4). A rule set names what a grammar does; the differential tests at
//! the end keep a verbatim copy of every scanner this replaced and compare them on generated text.

/// Which quote characters hide separators and parentheses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quotes {
    /// `"` and `'`: a quote opens at either and closes at the same character.
    Both,
    /// `"` only.
    Double,
}

/// Whether parentheses hide separators.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Parens {
    Hide,
    Count,
}

/// The byte ranges of the pieces of `text` split at `sep` outside quotes (and, with `Parens::Hide`, parentheses); every piece,
/// empty ones too.
fn split_ranges(text: &str, sep: char, quotes: Quotes, parens: Parens) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    let mut quote: Option<char> = None;
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') => quote = Some(c),
            (None, '\'') if quotes == Quotes::Both => quote = Some(c),
            (None, '(') if parens == Parens::Hide => depth += 1,
            (None, ')') if parens == Parens::Hide => depth -= 1,
            (None, c) if c == sep && depth == 0 => {
                out.push((start, i));
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push((start, text.len()));
    out
}

/// SubPageLink and RunPageLink: quotes of both kinds, each part trimmed, empty parts dropped.
pub(crate) fn split_trimmed(text: &str, sep: char) -> Vec<String> {
    split_ranges(text, sep, Quotes::Both, Parens::Hide)
        .into_iter()
        .map(|(a, b)| text[a..b].trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Query properties (DataItemLink, OrderBy, filters): quotes of both kinds, parts as written, empty middle parts kept,
/// a blank last part dropped.
pub(crate) fn split_query(text: &str, sep: char) -> Vec<String> {
    let mut ranges = split_ranges(text, sep, Quotes::Both, Parens::Hide);
    if let Some(&(a, b)) = ranges.last() {
        if text[a..b].trim().is_empty() {
            ranges.pop();
        }
    }
    ranges.into_iter().map(|(a, b)| text[a..b].to_string()).collect()
}

/// Report and xmlport links: double quotes only, parts as written, empty parts kept.
pub(crate) fn split_double(text: &str, sep: char) -> Vec<String> {
    split_ranges(text, sep, Quotes::Double, Parens::Hide)
        .into_iter()
        .map(|(a, b)| text[a..b].to_string())
        .collect()
}

/// Names (`Item."Field"`, a report column source, `Interface = Codeunit` of an enum value): double quotes only, parentheses not
/// counted, the parts as slices of `text`, as written, empty parts kept.
pub(crate) fn split_names(text: &str, sep: char) -> Vec<&str> {
    split_ranges(text, sep, Quotes::Double, Parens::Count)
        .into_iter()
        .map(|(a, b)| &text[a..b])
        .collect()
}

/// TableRelation: quotes of both kinds, the parts as slices of `text`, as written, empty parts kept.
pub(crate) fn split_slices(text: &str, sep: char) -> Vec<&str> {
    split_ranges(text, sep, Quotes::Both, Parens::Hide)
        .into_iter()
        .map(|(a, b)| &text[a..b])
        .collect()
}

/// The index of the `)` that closes the `(` at `open`; quotes of both kinds hide parentheses (a quote closes at its own
/// character, so `''` inside a single-quoted text closes and reopens it). `None` for a group that is not closed.
pub(crate) fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut i = open;
    while let Some(&c) = chars.get(i) {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `s` trimmed, without one pair of surrounding double quotes. A doubled quote inside is kept as written.
pub(crate) fn unquote_double(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(t)
        .to_string()
}

/// TableRelation: `s` trimmed, without one pair of surrounding double quotes, or else one pair of single quotes, when
/// the text between holds no such quote.
pub(crate) fn unquote_relation(s: &str) -> String {
    let s = s.trim();
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|x| x.strip_suffix(q)) {
            if !inner.contains(q) {
                return inner.to_string();
            }
        }
    }
    s.to_string()
}

/// Xmlport properties: `s` trimmed; a single-quoted text loses its quotes and reads `''` as `'`; a double-quoted
/// one loses its quotes.
pub(crate) fn unquote_xmlport(s: &str) -> String {
    let t = s.trim();
    if let Some(inner) = t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
        return inner.replace("''", "'");
    }
    if let Some(inner) = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        return inner.to_string();
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquote_double_trims_and_strips_one_pair() {
        assert_eq!(unquote_double("  \"No.\" "), "No.");
        assert_eq!(unquote_double("Plain"), "Plain");
        assert_eq!(unquote_double("'x'"), "'x'");
        assert_eq!(unquote_double("\"a\"\"b\""), "a\"\"b");
    }

    #[test]
    fn split_double_keeps_empty_parts_and_ignores_quoted_and_nested_separators() {
        assert_eq!(split_double("a,,b", ','), ["a", "", "b"]);
        assert_eq!(split_double("f(a,b),\"x,y\",z", ','), ["f(a,b)", "\"x,y\"", "z"]);
        assert_eq!(split_double("", ','), [""]);
        assert_eq!(split_double(" a , b ", ','), [" a ", " b "]);
    }

    // ---- The scanners this module replaced, verbatim, as the oracle of the differential tests.

    fn legacy_pages_split_top(text: &str, sep: char) -> Vec<String> {
        let mut out = Vec::new();
        let (mut depth, mut start) = (0i32, 0usize);
        let mut quote: Option<char> = None;
        for (i, c) in text.char_indices() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '\'' | '"') => quote = Some(c),
                (None, '(') => depth += 1,
                (None, ')') => depth -= 1,
                (None, c) if c == sep && depth == 0 => {
                    out.push(text[start..i].trim().to_string());
                    start = i + c.len_utf8();
                }
                _ => {}
            }
        }
        out.push(text[start..].trim().to_string());
        out.retain(|s| !s.is_empty());
        out
    }

    fn legacy_query_split_top(text: &str, sep: char) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let (mut depth, mut quote) = (0i32, None::<char>);
        for ch in text.chars() {
            match (quote, ch) {
                (Some(q), c) if c == q => {
                    quote = None;
                    cur.push(c);
                }
                (Some(_), c) => cur.push(c),
                (None, '"' | '\'') => {
                    quote = Some(ch);
                    cur.push(ch);
                }
                (None, '(') => {
                    depth += 1;
                    cur.push(ch);
                }
                (None, ')') => {
                    depth -= 1;
                    cur.push(ch);
                }
                (None, c) if c == sep && depth == 0 => out.push(std::mem::take(&mut cur)),
                (None, c) => cur.push(c),
            }
        }
        if !cur.trim().is_empty() {
            out.push(cur);
        }
        out
    }

    fn legacy_split_double(text: &str, sep: char) -> Vec<String> {
        let (mut out, mut cur, mut quoted, mut depth) = (Vec::new(), String::new(), false, 0);
        for c in text.chars() {
            match c {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => depth -= 1,
                c if c == sep && !quoted && depth == 0 => {
                    out.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
            cur.push(c);
        }
        out.push(cur);
        out
    }

    /// members.rs `split_outside_quotes` (enum implementation pairs and object names).
    fn legacy_members_split_outside_quotes(text: &str, sep: char) -> Vec<String> {
        let mut out = vec![String::new()];
        let mut in_quote = false;
        for c in text.chars() {
            match c {
                '"' => {
                    in_quote = !in_quote;
                    out.last_mut().expect("non-empty").push(c);
                }
                c if c == sep && !in_quote => out.push(String::new()),
                c => out.last_mut().expect("non-empty").push(c),
            }
        }
        out
    }

    /// reports.rs `split_outside_quotes` (report column sources, split at dots).
    fn legacy_reports_split_outside_quotes(s: &str, sep: char) -> Vec<&str> {
        let mut out = Vec::new();
        let (mut start, mut quoted) = (0, false);
        for (i, c) in s.char_indices() {
            match c {
                '"' => quoted = !quoted,
                c if c == sep && !quoted => {
                    out.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(&s[start..]);
        out
    }

    fn legacy_split_commas(s: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let (mut depth, mut start, mut quote) = (0i32, 0usize, None::<char>);
        for (i, ch) in s.char_indices() {
            match (quote, ch) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(ch),
                (None, '(') => depth += 1,
                (None, ')') => depth -= 1,
                (None, ',') if depth == 0 => {
                    out.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(&s[start..]);
        out
    }

    fn legacy_unquote_relation(s: &str) -> String {
        let s = s.trim();
        for q in ['"', '\''] {
            if let Some(inner) = s.strip_prefix(q).and_then(|x| x.strip_suffix(q)) {
                if !inner.contains(q) {
                    return inner.to_string();
                }
            }
        }
        s.to_string()
    }

    fn legacy_unquote_xmlport(s: &str) -> String {
        let t = s.trim();
        if let Some(inner) = t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
            return inner.replace("''", "'");
        }
        if let Some(inner) = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
            return inner.to_string();
        }
        t.to_string()
    }

    /// Every text of up to `len` characters over an alphabet that exercises the scanners: both quotes, parentheses,
    /// both separators, a space, a letter, a non-ASCII character.
    fn texts(len: usize) -> Vec<String> {
        const A: [char; 9] = ['"', '\'', '(', ')', ',', '=', ' ', 'a', 'é'];
        let mut all = vec![String::new()];
        let mut layer = vec![String::new()];
        for _ in 0..len {
            let mut next = Vec::new();
            for s in &layer {
                for c in A {
                    let mut t = s.clone();
                    t.push(c);
                    next.push(t);
                }
            }
            all.extend(next.iter().cloned());
            layer = next;
        }
        all
    }

    #[test]
    fn the_splitters_give_what_the_scanners_they_replaced_gave() {
        for t in texts(6) {
            for sep in [',', '='] {
                assert_eq!(split_trimmed(&t, sep), legacy_pages_split_top(&t, sep), "pages {t:?}");
                assert_eq!(split_query(&t, sep), legacy_query_split_top(&t, sep), "query {t:?}");
                assert_eq!(split_double(&t, sep), legacy_split_double(&t, sep), "double {t:?}");
                assert_eq!(
                    split_names(&t, sep),
                    legacy_members_split_outside_quotes(&t, sep),
                    "members {t:?}"
                );
                assert_eq!(
                    split_names(&t, sep),
                    legacy_reports_split_outside_quotes(&t, sep),
                    "reports {t:?}"
                );
            }
            assert_eq!(split_slices(&t, ','), legacy_split_commas(&t), "slices {t:?}");
        }
    }

    #[test]
    fn the_unquoters_give_what_the_unquoters_they_replaced_gave() {
        for t in texts(6) {
            assert_eq!(unquote_relation(&t), legacy_unquote_relation(&t), "relation {t:?}");
            assert_eq!(unquote_xmlport(&t), legacy_unquote_xmlport(&t), "xmlport {t:?}");
        }
    }
    /// `Cursor::raw_until_close` of calcformula.rs, verbatim in effect: the raw text of the group whose `(` is at
    /// `open`, trimmed, and the index after its `)`.
    fn legacy_raw_until_close(c: &[char], open: usize) -> Option<(String, usize)> {
        let mut i = open + 1;
        while c.get(i).is_some_and(|x| x.is_whitespace()) {
            i += 1;
        }
        let start = i;
        let mut depth = 0usize;
        while let Some(&ch) = c.get(i) {
            match ch {
                '"' => {
                    i += 1;
                    while c.get(i).is_some_and(|x| *x != '"') {
                        i += 1;
                    }
                }
                '\'' => {
                    i += 1;
                    loop {
                        match (c.get(i), c.get(i + 1)) {
                            (Some('\''), Some('\'')) => i += 2,
                            (Some('\''), _) | (None, _) => break,
                            _ => i += 1,
                        }
                    }
                }
                '(' => depth += 1,
                ')' if depth == 0 => {
                    let raw: String = c[start..i].iter().collect();
                    return Some((raw.trim().to_string(), i + 1));
                }
                ')' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// The `Group` token of relation.rs's lexer, verbatim in effect: the text between the `(` at `i` and its `)`.
    fn legacy_lex_group(c: &[char], mut i: usize) -> Option<(String, usize)> {
        let mut depth = 0;
        let start = i + 1;
        let mut quote: Option<char> = None;
        loop {
            let x = *c.get(i)?;
            match (quote, x) {
                (Some(q), x) if x == q => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(x),
                (None, '(') => depth += 1,
                (None, ')') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Some((c[start..i].iter().collect(), i + 1))
    }

    #[test]
    fn matching_paren_gives_what_the_two_group_scanners_gave() {
        for t in texts(7) {
            let c: Vec<char> = t.chars().collect();
            for open in (0..c.len()).filter(|&i| c[i] == '(') {
                let new =
                    matching_paren(&c, open).map(|close| (c[open + 1..close].iter().collect::<String>(), close + 1));
                let calc = new.clone().map(|(raw, next)| (raw.trim().to_string(), next));
                assert_eq!(calc, legacy_raw_until_close(&c, open), "calcformula {t:?} at {open}");
                assert_eq!(new, legacy_lex_group(&c, open), "relation {t:?} at {open}");
            }
        }
    }
}
